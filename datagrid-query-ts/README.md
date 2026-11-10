# datagrid-query

TypeScript query builder for data grids, tested against real PostgreSQL via
[PGlite](https://pglite.dev). It builds parameterised `SELECT`, `INSERT`,
`UPDATE`, `UPSERT`, and `DELETE` statements. It does not execute anything.

## Requirements

- Node.js >= 20
- pnpm 11

## Getting started

```bash
pnpm install
pnpm test              # runs unit and PGlite-backed integration tests
pnpm test:unit         # runs SQL-shape tests only
pnpm test:integration  # runs PGlite-backed tests only
pnpm build             # emits dist/ (ESM + type declarations)
pnpm typecheck
```

## Usage

```ts
import { buildQuery } from "datagrid-query";
import { PGlite } from "@electric-sql/pglite";

const { sql, params } = buildQuery({
  table: "people",
  columns: ["id", "name"],
  filters: [{ column: "age", operator: "gte", value: 18 }],
  sort: [{ column: "name", direction: "asc" }],
  limit: 50,
});

const db = new PGlite();
const { rows } = await db.query(sql, params);
```

Use `subquery` instead of `table` to query a sub-select. Sub-select parameters
come first, so outer filters continue numbering after them:

```ts
const query = buildQuery({
  subquery: {
    sql: 'SELECT "id", "name" FROM "people" WHERE "age" >= $1',
    alias: "adults",
    params: [18],
  },
  filters: [{ column: "name", operator: "like", value: "A%" }],
});
```

## Mutations

```ts
import {
  bindMutationRows,
  buildInsert,
  buildUpdate,
  buildUpsert,
  buildMerge,
  buildDelete,
} from "datagrid-query";

const options = {
  table: "people",
  columns: [
    { name: "id", type: "uuid", insertDefaultSql: "uuidv7()" },
    { name: "name", type: "text" },
    { name: "updatedAt", type: "timestamptz", updateDefaultSql: "now()" },
    { name: "version", type: "int8", insertDefaultSql: "0" },
  ],
  primaryKey: ["id"],
  versionColumn: "version",
};

const insert = buildInsert(options);
const rows = [{ name: "Ada" }];
const params = bindMutationRows(insert, rows);

const update = buildUpdate(options);
const upsert = buildUpsert(options);
const merge = buildMerge(options);
const remove = buildDelete(options);
```

Mutation builders return SQL and a parameter specification without row data.
`bindMutationRows` turns partial rows into the parameter arrays used by
`unnest`. `insertDefaultSql` and `updateDefaultSql` control fallbacks for
missing values, and `#present` boolean arrays carry presence. `buildMerge` uses
PostgreSQL `MERGE` and applies patch semantics on matched rows: omitted fields
keep their existing values, while explicit `null` writes `NULL`.

`buildQuery`, `buildInsert`, `buildUpdate`, `buildUpsert`, `buildMerge`,
`buildDelete`, and `bindMutationRows` are the runtime exports; all other
exports are types. Invalid input throws an error named `QueryError`.
Identifiers are validated against `^[A-Za-z_][A-Za-z0-9_]*$` and quoted; user
values are returned as bind parameters. Sub-select SQL and default SQL
expressions are trusted input.

The options accepted by the builders use the same JSON shape as the Rust serde
representations, so `JSON.stringify(options)` can be sent to the Rust
implementation and deserialized there. Mutation options contain no rows, so
metadata and row data can be transported independently.

The implementation is split by operation in `src/query.ts`, `src/insert.ts`,
`src/update.ts`, `src/upsert.ts`, `src/merge.ts`, and `src/delete.ts`, with
shared helpers in `src/shared.ts` and `src/mutation.ts`.

See [../docs/query-builder.md](../docs/query-builder.md) for the shared query
model and operator semantics.
