use datagrid_query::{
    Combinator, Filter, FilterCondition, FilterGroup, FilterOperator, QueryOptions, QuerySource,
    SelectColumn, Sort, SortDirection, SubquerySource, Value, build_query,
};

#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::{PgliteServer, run_sql};

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

    let query = build_query(&options).expect("query should build");
    let rows = run_sql(&mut client, &query.sql, &query.params);
    assert_eq!(
        rows.iter()
            .map(|row| (row.get("id"), row.get("name")))
            .collect::<Vec<(i64, String)>>(),
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

    let query = build_query(&options).expect("query should build");
    let rows = run_sql(&mut client, &query.sql, &query.params);
    assert_eq!(
        rows.iter().map(|row| row.get("id")).collect::<Vec<i64>>(),
        vec![1, 3]
    );

    let mut options = blank(table("people"));
    options.columns = vec![SelectColumn::Name("id".to_owned())];
    options.filters = vec![condition(
        "score",
        FilterOperator::Between,
        Some(Value::Array(vec![Value::Int(70), Value::Int(90)])),
    )];
    options.sort = vec![sort("score", SortDirection::Desc)];

    let query = build_query(&options).expect("query should build");
    let rows = run_sql(&mut client, &query.sql, &query.params);
    assert_eq!(
        rows.iter().map(|row| row.get("id")).collect::<Vec<i64>>(),
        vec![1, 3, 2]
    );

    let mut options = blank(table("people"));
    options.columns = vec![SelectColumn::Name("id".to_owned())];
    options.filters = vec![condition(
        "id",
        FilterOperator::Any,
        Some(Value::Array(vec![Value::Int(1), Value::Int(3)])),
    )];
    options.sort = vec![sort("id", SortDirection::Asc)];

    let query = build_query(&options).expect("query should build");
    let rows = run_sql(&mut client, &query.sql, &query.params);
    assert_eq!(
        rows.iter().map(|row| row.get("id")).collect::<Vec<i64>>(),
        vec![1, 3]
    );

    let mut options = blank(table("people"));
    options.columns = vec![SelectColumn::Name("id".to_owned())];
    options.filters = vec![condition(
        "id",
        FilterOperator::In,
        Some(Value::Array(Vec::new())),
    )];

    let query = build_query(&options).expect("query should build");
    assert!(run_sql(&mut client, &query.sql, &query.params).is_empty());

    let mut options = blank(table("people"));
    options.columns = vec![SelectColumn::Name("id".to_owned())];
    options.filters = vec![condition("deleted_at", FilterOperator::Eq, None)];
    options.sort = vec![sort("id", SortDirection::Asc)];

    let query = build_query(&options).expect("query should build");
    let rows = run_sql(&mut client, &query.sql, &query.params);
    assert_eq!(
        rows.iter().map(|row| row.get("id")).collect::<Vec<i64>>(),
        vec![1, 2, 4]
    );

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

    let query = build_query(&options).expect("query should build");
    let rows = run_sql(&mut client, &query.sql, &query.params);
    assert_eq!(
        rows.iter()
            .map(|row| (row.get("id"), row.get("name")))
            .collect::<Vec<(i64, String)>>(),
        vec![(3, "Cara".to_owned())]
    );
}
