use datagrid_query_rs::{
    Combinator, Filter, FilterCondition, FilterGroup, FilterOperator, QueryOptions, QuerySource,
    SelectColumn, Sort, SortDirection, SubquerySource, Value, build_query,
};
use postgres::types::ToSql;
use postgres::{Client, NoTls, Row};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct PgliteServer {
    child: Child,
}

impl PgliteServer {
    fn start() -> (Self, Client) {
        let port = free_port();
        let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("Rust crate should have a parent directory")
            .join("datagrid-query-ts");
        let script = project_dir.join("scripts/pglite-server.mjs");

        let child = Command::new("node")
            .arg(&script)
            .current_dir(&project_dir)
            .env("PGLITE_PORT", port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|error| {
                panic!(
                    "failed to start `node {}`: {error}. Run `pnpm install` in datagrid-query-ts first.",
                    script.display()
                )
            });

        let mut server = Self { child };
        let client = server.connect(port);
        (server, client)
    }

    fn connect(&mut self, port: u16) -> Client {
        let connection = format!(
            "host=127.0.0.1 port={port} user=postgres password=postgres dbname=postgres connect_timeout=1"
        );
        let deadline = Instant::now() + Duration::from_secs(20);

        loop {
            if let Some(status) = self.child.try_wait().expect("failed to poll PGlite server") {
                panic!("PGlite server exited during startup: {status}");
            }

            match Client::connect(&connection, NoTls) {
                Ok(client) => return client,
                Err(error) => {
                    if Instant::now() >= deadline {
                        panic!("timed out waiting for PGlite server: {error}");
                    }
                }
            }

            thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for PgliteServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("failed to reserve a local port");
    listener
        .local_addr()
        .expect("failed to read the reserved port")
        .port()
}

fn blank(source: QuerySource) -> QueryOptions {
    QueryOptions {
        source,
        columns: Vec::new(),
        filters: Vec::new(),
        sort: Vec::new(),
        limit: None,
        offset: None,
    }
}

fn table(name: &str) -> QuerySource {
    QuerySource::Table {
        schema: None,
        name: name.to_owned(),
    }
}

fn condition(column: &str, operator: FilterOperator, value: Option<Value>) -> Filter {
    Filter::Condition(FilterCondition {
        column: column.to_owned(),
        operator,
        value,
    })
}

fn sort(column: &str, direction: SortDirection) -> Sort {
    Sort {
        column: column.to_owned(),
        direction,
        nulls: None,
    }
}

fn execute(client: &mut Client, options: &QueryOptions) -> Vec<Row> {
    let query = build_query(options).expect("query should build");
    let params = to_sql_params(&query.params);
    let refs: Vec<&(dyn ToSql + Sync)> = params.iter().map(|param| param.as_ref()).collect();

    client
        .query(&query.sql, &refs)
        .unwrap_or_else(|error| panic!("query failed: {error}\n{}", query.sql))
}

fn to_sql_params(values: &[Value]) -> Vec<Box<dyn ToSql + Sync>> {
    values
        .iter()
        .map(|value| match value {
            Value::Null => boxed(Option::<String>::None),
            Value::Bool(value) => boxed(*value),
            Value::Int(value) => boxed(*value),
            Value::Float(value) => boxed(*value),
            Value::Text(value) => boxed(value.clone()),
            Value::Bytes(value) => boxed(value.clone()),
            Value::Array(values) => array_param(values),
            Value::Json(value) => boxed(value.clone()),
        })
        .collect()
}

fn boxed<T>(value: T) -> Box<dyn ToSql + Sync>
where
    T: ToSql + Sync + 'static,
{
    Box::new(value)
}

fn array_param(values: &[Value]) -> Box<dyn ToSql + Sync> {
    if values.iter().all(|value| matches!(value, Value::Int(_))) {
        let values: Vec<i64> = values
            .iter()
            .map(|value| match value {
                Value::Int(value) => *value,
                _ => unreachable!(),
            })
            .collect();
        return boxed(values);
    }

    if values.iter().all(|value| matches!(value, Value::Text(_))) {
        let values: Vec<String> = values
            .iter()
            .map(|value| match value {
                Value::Text(value) => value.clone(),
                _ => unreachable!(),
            })
            .collect();
        return boxed(values);
    }

    panic!("PGlite test helper only supports integer and text array parameters");
}

fn id_names(rows: &[Row]) -> Vec<(i64, String)> {
    rows.iter()
        .map(|row| (row.get("id"), row.get("name")))
        .collect()
}

fn ids(rows: &[Row]) -> Vec<i64> {
    rows.iter().map(|row| row.get("id")).collect()
}

#[test]
fn executes_generated_queries_against_pglite() {
    let (_server, mut client) = PgliteServer::start();

    client
        .batch_execute(
            r#"
            CREATE TABLE people (
                id bigint PRIMARY KEY,
                name text NOT NULL,
                first_name text NOT NULL,
                age bigint NOT NULL,
                status text NOT NULL,
                score bigint NOT NULL,
                deleted_at text
            );

            INSERT INTO people (id, name, first_name, age, status, score, deleted_at) VALUES
                (1, 'Ada', 'Ada', 36, 'active', 90, NULL),
                (2, 'Bob', 'Bob', 17, 'pending', 70, NULL),
                (3, 'Cara', 'Cara', 28, 'inactive', 80, '2026-01-01T00:00:00Z'),
                (4, 'Dan', 'Dan', 28, 'active', 60, NULL);
            "#,
        )
        .expect("failed to create test data");

    let mut options = blank(table("people"));
    options.columns = vec![
        SelectColumn::Name("id".to_owned()),
        SelectColumn::Name("name".to_owned()),
    ];
    options.filters = vec![condition("age", FilterOperator::Gte, Some(Value::Int(18)))];
    options.sort = vec![sort("name", SortDirection::Asc)];
    options.limit = Some(2);
    options.offset = Some(1);

    assert_eq!(
        id_names(&execute(&mut client, &options)),
        vec![(3, "Cara".to_owned()), (4, "Dan".to_owned())]
    );

    let mut options = blank(table("people"));
    options.columns = vec![SelectColumn::Name("id".to_owned())];
    options.filters = vec![Filter::Group(FilterGroup {
        combinator: Combinator::Or,
        filters: vec![
            condition(
                "status",
                FilterOperator::Eq,
                Some(Value::Text("inactive".to_owned())),
            ),
            condition("score", FilterOperator::Gte, Some(Value::Int(80))),
        ],
    })];
    options.sort = vec![sort("id", SortDirection::Asc)];

    assert_eq!(ids(&execute(&mut client, &options)), vec![1, 3]);

    let mut options = blank(table("people"));
    options.columns = vec![SelectColumn::Name("id".to_owned())];
    options.filters = vec![condition(
        "score",
        FilterOperator::Between,
        Some(Value::Array(vec![Value::Int(70), Value::Int(90)])),
    )];
    options.sort = vec![sort("score", SortDirection::Desc)];

    assert_eq!(ids(&execute(&mut client, &options)), vec![1, 3, 2]);

    let mut options = blank(table("people"));
    options.columns = vec![SelectColumn::Name("id".to_owned())];
    options.filters = vec![condition(
        "id",
        FilterOperator::Any,
        Some(Value::Array(vec![Value::Int(1), Value::Int(3)])),
    )];
    options.sort = vec![sort("id", SortDirection::Asc)];

    assert_eq!(ids(&execute(&mut client, &options)), vec![1, 3]);

    let mut options = blank(table("people"));
    options.columns = vec![SelectColumn::Name("id".to_owned())];
    options.filters = vec![condition(
        "id",
        FilterOperator::In,
        Some(Value::Array(Vec::new())),
    )];

    assert!(execute(&mut client, &options).is_empty());

    let mut options = blank(table("people"));
    options.columns = vec![SelectColumn::Name("id".to_owned())];
    options.filters = vec![condition("deleted_at", FilterOperator::Eq, None)];
    options.sort = vec![sort("id", SortDirection::Asc)];

    assert_eq!(ids(&execute(&mut client, &options)), vec![1, 2, 4]);

    let mut options = blank(QuerySource::Subquery {
        subquery: SubquerySource {
            sql: r#"SELECT "id", "name", "age" FROM "people" WHERE "age" >= $1"#.to_owned(),
            alias: "adults".to_owned(),
            params: vec![Value::Int(18)],
        },
    });
    options.columns = vec![
        SelectColumn::Name("id".to_owned()),
        SelectColumn::Name("name".to_owned()),
    ];
    options.filters = vec![condition(
        "name",
        FilterOperator::Like,
        Some(Value::Text("C%".to_owned())),
    )];
    options.sort = vec![sort("id", SortDirection::Asc)];

    assert_eq!(
        id_names(&execute(&mut client, &options)),
        vec![(3, "Cara".to_owned())]
    );
}
