# Query builder design

Both projects expose build functions that return SQL plus bind parameters.
Nothing connects to a database or executes a statement.

| Operation | Rust | TypeScript |
| --- | --- | --- |
| Select | `build_query(&QueryOptions)` | `buildQuery(options)` |
| Insert | `build_insert(&MutationOptions)` | `buildInsert(options)` |
| Upsert | `build_upsert(&MutationOptions)` | `buildUpsert(options)` |
| Update | `build_update(&MutationOptions)` | `buildUpdate(options)` |
| Delete | `build_delete(&MutationOptions)` | `buildDelete(options)` |

`BuiltQuery` contains `sql` and `params`. The generated statement has no
trailing semicolon.

## Source

A query has exactly one source:

- A table name with an optional schema.
- A sub-select with a required alias and optional bind parameters.

Table, schema, and alias names are identifiers. They must match
`^[A-Za-z_][A-Za-z0-9_]*$` and are always double-quoted.

Sub-select SQL is trusted developer input and is inserted verbatim inside
parentheses. Its parameters are passed before any outer parameters, so a
sub-select using `$1` makes the first outer filter use `$2`. Sub-select
parameters are expected to be numbered from `$1` and to match the SQL.

## Columns

An empty column list selects `*`. A column can be a plain name or a name with an
alias. Filter and sort columns refer to the source's column names, not to
select-list aliases.

## Filters

The top-level filter list is joined with `AND`. A condition can be wrapped in a
group to build nested `AND` / `OR` expressions. An empty `AND` group becomes
`TRUE`; an empty `OR` group becomes `FALSE`. An empty top-level list produces no
`WHERE` clause.

| Operator | SQL | Value |
| --- | --- | --- |
| `eq` | `=` | Scalar; `null` or missing becomes `IS NULL` |
| `neq` | `<>` | Scalar; `null` or missing becomes `IS NOT NULL` |
| `gt`, `gte`, `lt`, `lte` | `>`, `>=`, `<`, `<=` | Required non-null scalar |
| `like`, `ilike` | `LIKE`, `ILIKE` | Required non-null pattern |
| `notLike`, `notIlike` | `NOT LIKE`, `NOT ILIKE` | Required non-null pattern |
| `in`, `notIn` | `IN`, `NOT IN` | Required array; expanded to one parameter per item |
| `any`, `notAny` | `= ANY`, `<> ALL` | Required array; passed as one array parameter |
| `isNull`, `isNotNull` | `IS NULL`, `IS NOT NULL` | Ignored |
| `between`, `notBetween` | `BETWEEN`, `NOT BETWEEN` | Required array of exactly two values |

An empty `in` list becomes `FALSE`, and an empty `notIn` list becomes `TRUE`.
`any` and `notAny` keep the array as a single parameter, so their SQL has one
placeholder regardless of list length.

All values become numbered bind parameters. The builder does not escape or
rewrite `LIKE` patterns; callers own `%` and `_` semantics.

## Filtering by an array of IDs

Use `any` to pass the array as one parameter and render `= ANY($n)`:

```ts
buildQuery({
  table: "people",
  filters: [{ column: "id", operator: "any", value: [1, 2, 3] }],
});
// SELECT * FROM "people" WHERE "id" = ANY($1)
// params: [[1, 2, 3]]
```

```rust
let options = QueryOptions {
    source: QuerySource::Table {
        schema: None,
        name: "people".to_owned(),
    },
    columns: Vec::new(),
    filters: vec![Filter::Condition(FilterCondition {
        column: "id".to_owned(),
        operator: FilterOperator::Any,
        value: Some(Value::Array(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
        ])),
    })],
    sort: Vec::new(),
    limit: None,
    offset: None,
};

let query = build_query(&options).unwrap();
assert_eq!(
    query.sql,
    r#"SELECT * FROM "people" WHERE "id" = ANY($1)"#
);
assert_eq!(
    query.params,
    vec![Value::Array(vec![
        Value::Int(1),
        Value::Int(2),
        Value::Int(3),
    ])]
);
```

Use `in` instead when you want one placeholder per ID: `id IN ($1, $2, $3)`.
Use `notAny` for `id <> ALL($1)`.

## Sorting

Every sort entry names a source column and an optional direction (`asc` by
default) and nulls order (`first` or `last`). Sorts are emitted in the given
order.

## Pagination

`limit` and `offset` are optional non-negative integers. TypeScript rejects
values that are not safe integers. Rust represents them as `u64`. The builder
emits `LIMIT` and `OFFSET` only when a value is present.

## JSON transport from TypeScript to Rust

Rust `QueryOptions` and `MutationOptions` implement serde `Serialize` and
`Deserialize` using the same JSON shape as their TypeScript counterparts. A
TypeScript caller can serialize its options directly:

```ts
import type { QueryOptions } from "datagrid-query-ts";

const options = {
  table: "people",
  columns: ["id", { name: "first_name", alias: "firstName" }],
  filters: [{ column: "age", operator: "gte", value: 18 }],
  sort: [{ column: "name", direction: "asc" }],
  limit: 50,
} satisfies QueryOptions;

const json = JSON.stringify(options);
```

The Rust side deserializes that JSON and builds the query:

```rust
use datagrid_query_rs::{build_query, QueryOptions};

let options: QueryOptions = serde_json::from_str(&json)?;
let query = build_query(&options)?;
```

`MutationOptions` follows the same bridge, including `build_upsert`. The shared
fixture `fixtures/upsert-options.json` is consumed by both test suites: the
TypeScript suite parses and builds it, and the Rust suite deserializes it,
re-serializes it, and builds the same upsert.

The bridge uses these rules:

- `table` / `schema` or `subquery` appear at the top level, exactly as in the
  TypeScript type.
- Optional TypeScript fields may be omitted.
- Operator names use the TypeScript spelling, including `ilike` and `notIlike`.
- Sort direction defaults to `asc` when omitted.
- JSON strings, numbers, booleans, arrays, and `null` become the matching
  `Value` variants. JSON objects become `Value::Json`.
- An omitted filter value and an explicit JSON `null` are equivalent; both
  deserialize to no value.
- Serializing Rust options produces the same shape, so options can travel back
  to TypeScript. `Value::Bytes` is the exception: `serde_json` writes it as a
  number array, and it reads back as `Value::Array`.

## Mutations

All mutation builders take one `MutationOptions` object:

```ts
interface MutationOptions {
  table: string;
  schema?: string;
  columns: MutationColumn[];
  primaryKey: string[];
  versionColumn?: string;
  returning?: string[];
  rows: MutationRow[];
}

interface MutationColumn {
  name: string;
  type: string;
  insertable?: boolean;
  updatable?: boolean;
  insertDefaultSql?: string;
  updateDefaultSql?: string;
}
```

`type` is the PostgreSQL type used in the `unnest` casts, such as `uuid`,
`text`, `numeric`, `timestamptz`, or `int8`. `insertDefaultSql` and
`updateDefaultSql` are trusted SQL expressions used when a row does not provide
a value. `primaryKey` supports one or more columns. `returning` defaults to the
primary key columns; an empty array disables `RETURNING`.

Rows are partial objects. A missing value means "not provided":

- For insert columns with `insertDefaultSql`, SQL uses a `#present` boolean
  array and falls back to the expression when false.
- For update columns, SQL uses a `#present` boolean array. Missing values keep
  the existing column value, or use `updateDefaultSql` when configured.
- Missing values without a default become `NULL` parameters. Explicit `null`
  counts as provided.
- Update and delete rows must contain non-null primary key values. If
  `versionColumn` is set, they must also contain a non-null version.

Each builder groups rows into typed arrays and emits one `unnest(...)` source:

```text
INSERT INTO "widgets" ("id", "name", "updated_at", "version")
SELECT CASE WHEN v."id#present" THEN v."id" ELSE 9000 END,
       v."name",
       CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now() END,
       CASE WHEN v."version#present" THEN v."version" ELSE 1 END
FROM unnest($1::bigint[], $2::bool[], ...) AS v("id", "id#present", ...)
RETURNING "widgets"."id"
```

Update and delete match rows by primary key. When `versionColumn` is set:

- Update adds `AND u."version" = v."version"` and sets `"version" = u."version" + 1`.
- Upsert adds `WHERE u."version" = excluded."version"` and increments the version.
- Delete adds the same version match condition.

Upsert uses `ON CONFLICT (primaryKey...)`. Non-key, updatable columns are
assigned from `excluded`; `versionColumn` is incremented. If there is nothing to
update, it emits `DO NOTHING`.

The parameter order follows the generated `unnest` list. Every parameter is an
array: value arrays use the column type, and presence flags use `bool[]`.

## Security model

- Identifiers are validated and quoted, so table, column, and alias names cannot
  inject SQL.
- Values are never interpolated into the statement; they are returned in
  `params` for the caller to bind.
- Raw sub-select SQL is trusted. Do not pass end-user text as sub-select SQL.
- Mutation default expressions are trusted SQL. Do not pass end-user text as
  `insertDefaultSql` or `updateDefaultSql`.
- The caller still needs to apply authorization and row-level rules.

## Parity

Both implementations use the same SQL shape and the same operator semantics.
Their test suites cover the same cases, including parameter offsets after a
sub-select, nested filter groups, empty `IN` lists, and identifier validation.

Both suites also execute generated SQL against PGlite, including mutations.
TypeScript uses the PGlite package directly. Rust starts the PGlite socket
server from `datagrid-query-ts/scripts/pglite-server.mjs`, connects with the
`postgres` crate, and runs the generated statements against real PostgreSQL
semantics.
