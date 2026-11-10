# datagrid-query-rs

Rust query builder for data grids. It builds parameterised `SELECT`, `INSERT`,
`UPDATE`, `UPSERT`, and `DELETE` statements. It does not execute anything.

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

## Mutations

`build_insert`, `build_update`, `build_upsert`, `build_merge`, and
`build_delete` share `MutationOptions`. Rows are partial, and missing values use
either a configured SQL default expression or the existing column value for
updates.

`build_merge` uses PostgreSQL `MERGE` and applies patch semantics on matched
rows: omitted fields keep their existing values, while explicit `null` writes
`NULL`. Merge requires PostgreSQL 15 or newer; `MERGE ... RETURNING` requires
PostgreSQL 17 or newer.

```rust
use datagrid_query_rs::{
    build_insert, MutationColumn, MutationOptions, MutationRow, Value,
};

let mut row = MutationRow::new();
row.insert("name".to_owned(), Value::Text("Ada".to_owned()));

let options = MutationOptions {
    table: "people".to_owned(),
    schema: None,
    columns: vec![
        MutationColumn {
            name: "id".to_owned(),
            data_type: "uuid".to_owned(),
            insert_default_sql: Some("uuidv7()".to_owned()),
            ..Default::default()
        },
        MutationColumn {
            name: "name".to_owned(),
            data_type: "text".to_owned(),
            ..Default::default()
        },
        MutationColumn {
            name: "version".to_owned(),
            data_type: "int8".to_owned(),
            insert_default_sql: Some("0".to_owned()),
            ..Default::default()
        },
    ],
    primary_key: vec!["id".to_owned()],
    version_column: Some("version".to_owned()),
    returning: None,
    rows: vec![row],
};

let query = build_insert(&options).unwrap();
```

Identifiers are validated against `^[A-Za-z_][A-Za-z0-9_]*$` and quoted; user
values are returned as bind parameters. Sub-select SQL and default SQL
expressions are trusted input.

## JSON bridge

`QueryOptions` and `MutationOptions` implement serde `Serialize` and
`Deserialize`. Their JSON shapes match the TypeScript types, so options
serialized on the TypeScript side can be deserialized and reused directly:

```rust
let options: datagrid_query_rs::QueryOptions = serde_json::from_str(&json)?;
let query = datagrid_query_rs::build_query(&options)?;
```

Optional fields may be omitted, filter operators use the TypeScript names,
mutation columns use `insertDefaultSql` and `updateDefaultSql`, and JSON objects
in filter values become `Value::Json`.

The implementation is split by operation in `src/query.rs`, `src/insert.rs`,
`src/update.rs`, `src/upsert.rs`, `src/merge.rs`, and `src/delete.rs`, with
shared types in `src/lib.rs`, shared SQL helpers in `src/shared.rs`, and
mutation helpers in `src/mutation.rs`.

See [../docs/query-builder.md](../docs/query-builder.md) for the shared query
model, operator semantics, JSON bridge, and mutation details.
