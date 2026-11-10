# datagrid_query

Rust query builder for data grids. It builds parameterised `SELECT`, `INSERT`,
`UPDATE`, `UPSERT`, and `DELETE` statements. It does not execute anything.

## Development

The integration tests start PGlite in socket-server mode with a script embedded
in the test support module. Bun runs that script and caches the pinned PGlite
packages in its global module cache, so no JavaScript install step is needed.
The first run downloads them, which requires network access. Install
[Bun](https://bun.sh) first; the tests are verified with Bun 1.4.

```bash
cargo test
cargo fmt
cargo clippy --all-targets -- -D warnings
```

`cargo test` runs both the unit tests in `tests/*.rs` and the PGlite-backed
integration tests in `tests/pglite-test/`.

## Usage

```rust
use datagrid_query::{
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
`build_delete` share metadata-only `MutationOptions`. Their result is a
`BuiltMutation` containing SQL and a parameter specification. Rows are bound
separately with `bind_mutation_rows`.

`build_merge` uses PostgreSQL `MERGE` and applies patch semantics on matched
rows: omitted fields keep their existing values, while explicit `null` writes
`NULL`. Merge requires PostgreSQL 15 or newer; `MERGE ... RETURNING` requires
PostgreSQL 17 or newer.

```rust
use datagrid_query::{
    bind_mutation_rows, build_insert, MutationColumn, MutationOptions, MutationRow, Value,
};

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
};

let mutation = build_insert(&options).unwrap();

let mut row = MutationRow::new();
row.insert("name".to_owned(), Value::Text("Ada".to_owned()));
let params = bind_mutation_rows(&mutation, &[row]).unwrap();
```

Identifiers are validated against `^[A-Za-z_][A-Za-z0-9_]*$` and quoted; user
values are returned as bind parameters. Sub-select SQL and default SQL
expressions are trusted input.

## JSON bridge

`QueryOptions` and `MutationOptions` implement serde `Serialize` and
`Deserialize`. Their JSON shapes match the TypeScript types, so options
serialized on the TypeScript side can be deserialized and reused directly:

```rust
let options: datagrid_query::QueryOptions = serde_json::from_str(&json)?;
let query = datagrid_query::build_query(&options)?;
```

Optional fields may be omitted, filter operators use the TypeScript names,
mutation columns use `insertDefaultSql` and `updateDefaultSql`, and JSON objects
in filter values become `Value::Json`. Mutation metadata contains no rows, so
row data is serialized and bound separately with `bind_mutation_rows`.

The implementation is split by operation in `src/query.rs`, `src/insert.rs`,
`src/update.rs`, `src/upsert.rs`, `src/merge.rs`, and `src/delete.rs`, with
shared types in `src/lib.rs`, shared SQL helpers in `src/shared.rs`, and
mutation helpers in `src/mutation.rs`.

See [../docs/query-builder.md](../docs/query-builder.md) for the shared query
model, operator semantics, JSON bridge, and mutation details.
