# datagrid-query-rs

Rust query builder for data grids. It turns a table or sub-select, selected
columns, filters, sorting, and pagination into a parameterised `SELECT`
statement. It does not execute anything.

## Development

```bash
cargo test
cargo fmt
cargo clippy --all-targets -- -D warnings
```

## Usage

```rust
use datagrid_query_rs::{
    build_query, Filter, FilterCondition, FilterOperator, QueryOptions, QuerySource,
    SelectColumn, Sort, SortDirection, Value,
};

let options = QueryOptions {
    source: QuerySource::Table {
        schema: None,
        name: "people".to_owned(),
    },
    columns: vec![
        SelectColumn::Name("id".to_owned()),
        SelectColumn::Name("name".to_owned()),
    ],
    filters: vec![Filter::Condition(FilterCondition {
        column: "age".to_owned(),
        operator: FilterOperator::Gte,
        value: Some(Value::Int(18)),
    })],
    sort: vec![Sort {
        column: "name".to_owned(),
        direction: SortDirection::Asc,
        nulls: None,
    }],
    limit: Some(50),
    offset: None,
};

let query = build_query(&options).unwrap();
assert_eq!(
    query.sql,
    r#"SELECT "id", "name" FROM "people" WHERE "age" >= $1 ORDER BY "name" ASC LIMIT 50"#
);
assert_eq!(query.params, vec![Value::Int(18)]);
```

`build_query` is the only public function. Identifiers are validated against
`^[A-Za-z_][A-Za-z0-9_]*$` and quoted; user values are returned as bind
parameters. Sub-select SQL is trusted input.

See [../docs/query-builder.md](../docs/query-builder.md) for the shared query
model and operator semantics.
