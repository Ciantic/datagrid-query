use datagrid_query_rs::{
    BuiltQuery, Combinator, Filter, FilterCondition, FilterGroup, FilterOperator, MutationColumn,
    MutationOptions, MutationRow, QueryOptions, QuerySource, SelectColumn, Sort, SortDirection,
    SubquerySource, Value, build_delete, build_insert, build_query, build_update, build_upsert,
};
use postgres::types::ToSql;
use postgres::{Client, NoTls, Row};
use std::collections::BTreeMap;
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
    run(client, &query)
}

fn run(client: &mut Client, query: &BuiltQuery) -> Vec<Row> {
    let params = to_sql_params(&query.params);
    let refs: Vec<&(dyn ToSql + Sync)> = params.iter().map(|param| param.as_ref()).collect();

    client
        .query(&query.sql, &refs)
        .unwrap_or_else(|error| panic!("query failed: {error:?}\n{}", query.sql))
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
    if values
        .iter()
        .all(|value| matches!(value, Value::Int(_) | Value::Null))
    {
        let values: Vec<Option<i64>> = values
            .iter()
            .map(|value| match value {
                Value::Int(value) => Some(*value),
                Value::Null => None,
                _ => unreachable!(),
            })
            .collect();
        return boxed(values);
    }

    if values
        .iter()
        .all(|value| matches!(value, Value::Text(_) | Value::Null))
    {
        let values: Vec<Option<String>> = values
            .iter()
            .map(|value| match value {
                Value::Text(value) => Some(value.clone()),
                Value::Null => None,
                _ => unreachable!(),
            })
            .collect();
        return boxed(values);
    }

    if values
        .iter()
        .all(|value| matches!(value, Value::Bool(_) | Value::Null))
    {
        let values: Vec<Option<bool>> = values
            .iter()
            .map(|value| match value {
                Value::Bool(value) => Some(*value),
                Value::Null => None,
                _ => unreachable!(),
            })
            .collect();
        return boxed(values);
    }

    panic!("PGlite test helper only supports integer, text, and boolean array parameters");
}

fn id_names(rows: &[Row]) -> Vec<(i64, String)> {
    rows.iter()
        .map(|row| (row.get("id"), row.get("name")))
        .collect()
}

fn ids(rows: &[Row]) -> Vec<i64> {
    rows.iter().map(|row| row.get("id")).collect()
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn mutation_column(name: &str, data_type: &str) -> MutationColumn {
    MutationColumn {
        name: name.to_owned(),
        data_type: data_type.to_owned(),
        ..Default::default()
    }
}

fn widget_columns() -> Vec<MutationColumn> {
    vec![
        MutationColumn {
            insert_default_sql: Some("9000".to_owned()),
            ..mutation_column("id", "bigint")
        },
        mutation_column("name", "text"),
        mutation_column("note", "text"),
        MutationColumn {
            insert_default_sql: Some("now()::text".to_owned()),
            updatable: false,
            ..mutation_column("created_at", "text")
        },
        MutationColumn {
            insert_default_sql: Some("now()::text".to_owned()),
            update_default_sql: Some("now()::text".to_owned()),
            ..mutation_column("updated_at", "text")
        },
        MutationColumn {
            insert_default_sql: Some("1".to_owned()),
            ..mutation_column("version", "bigint")
        },
    ]
}

fn widget_options(rows: Vec<MutationRow>) -> MutationOptions {
    MutationOptions {
        table: "widgets".to_owned(),
        schema: None,
        columns: widget_columns(),
        primary_key: vec!["id".to_owned()],
        version_column: Some("version".to_owned()),
        returning: None,
        rows,
    }
}

fn membership_options(rows: Vec<MutationRow>) -> MutationOptions {
    MutationOptions {
        table: "memberships".to_owned(),
        schema: None,
        columns: vec![
            mutation_column("tenant_id", "bigint"),
            mutation_column("user_id", "bigint"),
            mutation_column("role", "text"),
        ],
        primary_key: vec!["tenant_id".to_owned(), "user_id".to_owned()],
        version_column: None,
        returning: None,
        rows,
    }
}

fn mutation_row(entries: &[(&str, Value)]) -> MutationRow {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect::<BTreeMap<_, _>>()
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

#[test]
fn executes_mutation_builders_against_pglite() {
    let (_server, mut client) = PgliteServer::start();

    client
        .batch_execute(
            r#"
            CREATE TABLE widgets (
                id bigint PRIMARY KEY,
                name text NOT NULL,
                note text,
                created_at text NOT NULL,
                updated_at text NOT NULL,
                version bigint NOT NULL
            );

            CREATE TABLE memberships (
                tenant_id bigint NOT NULL,
                user_id bigint NOT NULL,
                role text NOT NULL,
                PRIMARY KEY (tenant_id, user_id)
            );
            "#,
        )
        .expect("failed to create mutation test tables");

    let insert = build_insert(&widget_options(vec![
        mutation_row(&[
            ("id", Value::Int(100)),
            ("name", text("alpha")),
            ("note", text("n0")),
            ("created_at", text("c1")),
            ("updated_at", text("u1")),
            ("version", Value::Int(1)),
        ]),
        mutation_row(&[("name", text("beta"))]),
    ]))
    .unwrap();
    run(&mut client, &insert);

    let inserted = client
        .query(
            r#"SELECT "id", "name", "version" FROM "widgets" ORDER BY "id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        inserted
            .iter()
            .map(|row| {
                (
                    row.get::<_, i64>("id"),
                    row.get::<_, String>("name"),
                    row.get::<_, i64>("version"),
                )
            })
            .collect::<Vec<_>>(),
        vec![(100, "alpha".to_owned(), 1), (9000, "beta".to_owned(), 1),]
    );

    let update = build_update(&widget_options(vec![
        mutation_row(&[
            ("id", Value::Int(100)),
            ("version", Value::Int(1)),
            ("name", text("alpha2")),
            ("note", text("n1")),
            ("updated_at", text("u2")),
        ]),
        mutation_row(&[
            ("id", Value::Int(9000)),
            ("version", Value::Int(1)),
            ("name", text("beta2")),
        ]),
    ]))
    .unwrap();
    run(&mut client, &update);

    let updated = client
        .query(
            r#"SELECT "id", "name", "note", "version" FROM "widgets" ORDER BY "id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        updated
            .iter()
            .map(|row| {
                (
                    row.get::<_, i64>("id"),
                    row.get::<_, String>("name"),
                    row.get::<_, Option<String>>("note"),
                    row.get::<_, i64>("version"),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (100, "alpha2".to_owned(), Some("n1".to_owned()), 2),
            (9000, "beta2".to_owned(), None, 2),
        ]
    );

    let upsert = build_upsert(&widget_options(vec![
        mutation_row(&[
            ("id", Value::Int(100)),
            ("name", text("alpha3")),
            ("note", text("n2")),
            ("created_at", text("ignored")),
            ("updated_at", text("u3")),
            ("version", Value::Int(2)),
        ]),
        mutation_row(&[
            ("id", Value::Int(200)),
            ("name", text("gamma")),
            ("created_at", text("c3")),
            ("updated_at", text("u3")),
            ("version", Value::Int(1)),
        ]),
    ]))
    .unwrap();
    run(&mut client, &upsert);

    let upserted = client
        .query(
            r#"SELECT "id", "name", "note", "version" FROM "widgets" ORDER BY "id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        upserted
            .iter()
            .map(|row| {
                (
                    row.get::<_, i64>("id"),
                    row.get::<_, String>("name"),
                    row.get::<_, Option<String>>("note"),
                    row.get::<_, i64>("version"),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (100, "alpha3".to_owned(), Some("n2".to_owned()), 3),
            (200, "gamma".to_owned(), None, 1),
            (9000, "beta2".to_owned(), None, 2),
        ]
    );

    let created = client
        .query(
            r#"SELECT "id", "created_at" FROM "widgets" WHERE "id" IN (100, 200) ORDER BY "id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        created
            .iter()
            .map(|row| (row.get::<_, i64>("id"), row.get::<_, String>("created_at")))
            .collect::<Vec<_>>(),
        vec![(100, "c1".to_owned()), (200, "c3".to_owned())]
    );

    let stale_upsert = build_upsert(&widget_options(vec![mutation_row(&[
        ("id", Value::Int(9000)),
        ("name", text("stale")),
        ("note", text("stale-note")),
        ("created_at", text("stale-created")),
        ("updated_at", text("stale-updated")),
        ("version", Value::Int(1)),
    ])]))
    .unwrap();
    run(&mut client, &stale_upsert);

    let stale = client
        .query(r#"SELECT "name" FROM "widgets" WHERE "id" = 9000"#, &[])
        .unwrap();
    assert_eq!(stale[0].get::<_, String>("name"), "beta2");

    let delete = build_delete(&widget_options(vec![mutation_row(&[
        ("id", Value::Int(200)),
        ("version", Value::Int(1)),
    ])]))
    .unwrap();
    run(&mut client, &delete);

    let remaining = client
        .query(r#"SELECT "id" FROM "widgets" ORDER BY "id""#, &[])
        .unwrap();
    assert_eq!(ids(&remaining), vec![100, 9000]);

    let insert = build_insert(&membership_options(vec![
        mutation_row(&[
            ("tenant_id", Value::Int(1)),
            ("user_id", Value::Int(10)),
            ("role", text("admin")),
        ]),
        mutation_row(&[
            ("tenant_id", Value::Int(1)),
            ("user_id", Value::Int(20)),
            ("role", text("user")),
        ]),
    ]))
    .unwrap();
    run(&mut client, &insert);

    let update = build_update(&membership_options(vec![mutation_row(&[
        ("tenant_id", Value::Int(1)),
        ("user_id", Value::Int(20)),
        ("role", text("manager")),
    ])]))
    .unwrap();
    run(&mut client, &update);

    let upsert = build_upsert(&membership_options(vec![
        mutation_row(&[
            ("tenant_id", Value::Int(1)),
            ("user_id", Value::Int(10)),
            ("role", text("super")),
        ]),
        mutation_row(&[
            ("tenant_id", Value::Int(2)),
            ("user_id", Value::Int(30)),
            ("role", text("owner")),
        ]),
    ]))
    .unwrap();
    run(&mut client, &upsert);

    let memberships = client
        .query(
            r#"SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        memberships
            .iter()
            .map(|row| {
                (
                    row.get::<_, i64>("tenant_id"),
                    row.get::<_, i64>("user_id"),
                    row.get::<_, String>("role"),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (1, 10, "super".to_owned()),
            (1, 20, "manager".to_owned()),
            (2, 30, "owner".to_owned()),
        ]
    );

    let delete = build_delete(&membership_options(vec![mutation_row(&[
        ("tenant_id", Value::Int(1)),
        ("user_id", Value::Int(10)),
    ])]))
    .unwrap();
    run(&mut client, &delete);

    let remaining = client
        .query(
            r#"SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        remaining
            .iter()
            .map(|row| {
                (
                    row.get::<_, i64>("tenant_id"),
                    row.get::<_, i64>("user_id"),
                    row.get::<_, String>("role"),
                )
            })
            .collect::<Vec<_>>(),
        vec![(1, 20, "manager".to_owned()), (2, 30, "owner".to_owned()),]
    );
}
