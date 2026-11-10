# datagrid-query-rs

Rust query builder for data grids. It turns a table or sub-select, selected
columns, filters, sorting, and pagination into a parameterised `SELECT`
statement. It does not execute anything.

## Development

The integration test starts PGlite in socket-server mode through the Node
script in `../datagrid-query-ts/scripts/pglite-server.mjs`. Install the
TypeScript dependencies once before running it:

```bash
(cd ../datagrid-query-ts && pnpm install)
```

```bash
cargo test
cargo fmt
cargo clippy --all-targets -- -D warnings
```

`cargo test` runs both SQL-shape tests and real queries against PGlite.

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

## JSON bridge

`QueryOptions` implements serde `Serialize` and `Deserialize`. Its JSON shape is
the same as the TypeScript `QueryOptions` type, so options serialized on the
TypeScript side can be deserialized and reused directly:

```rust
let options: datagrid_query_rs::QueryOptions = serde_json::from_str(&json)?;
let query = datagrid_query_rs::build_query(&options)?;
```

Optional fields may be omitted, filter operators use the TypeScript names, and
JSON objects in filter values become `Value::Json`.

See [../docs/query-builder.md](../docs/query-builder.md) for the shared query
model, operator semantics, and JSON bridge details.
