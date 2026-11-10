# datagrid-query-ts

TypeScript query builder for data grids, tested against real PostgreSQL via
[PGlite](https://pglite.dev). It turns a table or sub-select, selected columns,
filters, sorting, and pagination into a parameterised `SELECT` statement. It does
not execute anything.

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

`buildQuery` is the only runtime export; all other exports are types. Invalid
input throws an error named `QueryError`. Identifiers are validated against
`^[A-Za-z_][A-Za-z0-9_]*$` and quoted; user values are returned as bind
parameters. Sub-select SQL is trusted input.

The options accepted by `buildQuery` use the same JSON shape as the Rust
`QueryOptions` serde representation, so `JSON.stringify(options)` can be sent to
the Rust implementation and deserialized there.

See [../docs/query-builder.md](../docs/query-builder.md) for the shared query
model and operator semantics.
