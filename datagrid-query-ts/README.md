# datagrid-query-ts

TypeScript query builder for data grids, tested against real PostgreSQL via
[PGlite](https://pglite.dev). It builds parameterised `SELECT`, `INSERT`,
`UPDATE`, `UPSERT`, and `DELETE` statements. It does not execute anything.

## Requirements

- Node.js >= 20
- pnpm 11

## Getting started

```bash
pnpm install
pnpm test        # runs the PGlite-backed test suite
pnpm build       # emits dist/ (ESM + type declarations)
pnpm typecheck
```

## Usage

```ts
import { buildQuery } from "datagrid-query-ts";
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
  buildInsert,
  buildUpdate,
  buildUpsert,
  buildMerge,
  buildDelete,
} from "datagrid-query-ts";

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
  rows: [{ name: "Ada" }],
};

const insert = buildInsert(options);
const update = buildUpdate(options);
const upsert = buildUpsert(options);
const merge = buildMerge(options);
const remove = buildDelete(options);
```

Mutation rows are partial. `insertDefaultSql` and `updateDefaultSql` control
fallbacks for missing values, and `#present` boolean arrays carry presence
through `unnest`. `buildMerge` uses PostgreSQL `MERGE` and applies patch
semantics on matched rows: omitted fields keep their existing values, while
explicit `null` writes `NULL`.

`buildQuery`, `buildInsert`, `buildUpdate`, `buildUpsert`, `buildMerge`, and
`buildDelete` are the runtime exports; all other exports are types. Invalid input throws an error
named `QueryError`. Identifiers are validated against
`^[A-Za-z_][A-Za-z0-9_]*$` and quoted; user values are returned as bind
parameters. Sub-select SQL and default SQL expressions are trusted input.

The options accepted by the builders use the same JSON shape as the Rust serde
representations, so `JSON.stringify(options)` can be sent to the Rust
implementation and deserialized there.

The implementation is split by operation in `src/query.ts`, `src/insert.ts`,
`src/update.ts`, `src/upsert.ts`, `src/merge.ts`, and `src/delete.ts`, with
shared helpers in `src/shared.ts` and `src/mutation.ts`.

See [../docs/query-builder.md](../docs/query-builder.md) for the shared query
model and operator semantics.
