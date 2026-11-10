import { PGlite } from "@electric-sql/pglite";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { buildQuery } from "../src/index.js";

let db: PGlite;

beforeAll(async () => {
  db = new PGlite();
  await db.exec(`
    CREATE TABLE people (
      id integer PRIMARY KEY,
      name text NOT NULL,
      first_name text NOT NULL,
      age integer NOT NULL,
      status text NOT NULL,
      score integer NOT NULL,
      deleted_at timestamptz
    );

    INSERT INTO people (id, name, first_name, age, status, score, deleted_at) VALUES
      (1, 'Ada', 'Ada', 36, 'active', 90, NULL),
      (2, 'Bob', 'Bob', 17, 'pending', 70, NULL),
      (3, 'Cara', 'Cara', 28, 'inactive', 80, '2026-01-01T00:00:00Z'),
      (4, 'Dan', 'Dan', 28, 'active', 60, NULL);
  `);
});

afterAll(async () => {
  await db.close();
});

describe("buildQuery", () => {
  it("builds and executes a filtered, sorted, paginated query", async () => {
    const query = buildQuery({
      table: "people",
      columns: ["id", "name"],
      filters: [{ column: "age", operator: "gte", value: 18 }],
      sort: [{ column: "name", direction: "asc" }],
      limit: 2,
      offset: 1,
    });

    expect(query.sql).toBe(
      'SELECT "id", "name" FROM "people" WHERE "age" >= $1 ORDER BY "name" ASC LIMIT 2 OFFSET 1',
    );
    expect(query.params).toEqual([18]);

    const { rows } = await db.query<{ id: number; name: string }>(query.sql, query.params);
    expect(rows).toEqual([
      { id: 3, name: "Cara" },
      { id: 4, name: "Dan" },
    ]);
  });

  it("combines top-level AND filters with nested OR groups", () => {
    const query = buildQuery({
      table: "people",
      filters: [
        { column: "name", operator: "ilike", value: "a%" },
        {
          combinator: "or",
          filters: [
            { column: "status", operator: "in", value: ["active", "pending"] },
            { column: "deleted_at", operator: "isNull" },
          ],
        },
      ],
      sort: [{ column: "id", direction: "asc" }],
    });

    expect(query.sql).toBe(
      'SELECT * FROM "people" WHERE "name" ILIKE $1 AND ("status" IN ($2, $3) OR "deleted_at" IS NULL) ORDER BY "id" ASC',
    );
    expect(query.params).toEqual(["a%", "active", "pending"]);
  });

  it("treats eq and neq null as IS NULL and IS NOT NULL", async () => {
    const query = buildQuery({
      table: "people",
      columns: ["id"],
      filters: [{ column: "deleted_at", operator: "eq", value: null }],
      sort: [{ column: "id", direction: "asc" }],
    });

    expect(query.sql).toBe(
      'SELECT "id" FROM "people" WHERE "deleted_at" IS NULL ORDER BY "id" ASC',
    );
    expect(query.params).toEqual([]);

    const { rows } = await db.query<{ id: number }>(query.sql, query.params);
    expect(rows.map((row) => row.id)).toEqual([1, 2, 4]);
  });

  it("builds BETWEEN filters", async () => {
    const query = buildQuery({
      table: "people",
      columns: ["id", "score"],
      filters: [{ column: "score", operator: "between", value: [70, 90] }],
      sort: [{ column: "score", direction: "desc" }],
    });

    expect(query.sql).toBe(
      'SELECT "id", "score" FROM "people" WHERE "score" BETWEEN $1 AND $2 ORDER BY "score" DESC',
    );
    expect(query.params).toEqual([70, 90]);

    const { rows } = await db.query<{ id: number }>(query.sql, query.params);
    expect(rows.map((row) => row.id)).toEqual([1, 3, 2]);
  });

  it("uses constants for empty IN lists", async () => {
    const query = buildQuery({
      table: "people",
      filters: [{ column: "id", operator: "in", value: [] }],
    });

    expect(query.sql).toBe('SELECT * FROM "people" WHERE FALSE');
    expect(query.params).toEqual([]);

    const { rows } = await db.query(query.sql, query.params);
    expect(rows).toEqual([]);
  });

  it("passes an ID array as one ANY parameter", async () => {
    const query = buildQuery({
      table: "people",
      columns: ["id"],
      filters: [{ column: "id", operator: "any", value: [1, 3] }],
      sort: [{ column: "id", direction: "asc" }],
    });

    expect(query.sql).toBe(
      'SELECT "id" FROM "people" WHERE "id" = ANY($1) ORDER BY "id" ASC',
    );
    expect(query.params).toEqual([[1, 3]]);

    const { rows } = await db.query<{ id: number }>(query.sql, query.params);
    expect(rows.map((row) => row.id)).toEqual([1, 3]);
  });

  it("supports the inverse ALL form", async () => {
    const query = buildQuery({
      table: "people",
      columns: ["id"],
      filters: [{ column: "id", operator: "notAny", value: [1, 3] }],
      sort: [{ column: "id", direction: "asc" }],
    });

    expect(query.sql).toBe(
      'SELECT "id" FROM "people" WHERE "id" <> ALL($1) ORDER BY "id" ASC',
    );
    expect(query.params).toEqual([[1, 3]]);

    const { rows } = await db.query<{ id: number }>(query.sql, query.params);
    expect(rows.map((row) => row.id)).toEqual([2, 4]);
  });

  it("offsets outer parameters after a parameterised subquery", async () => {
    const query = buildQuery({
      subquery: {
        sql: 'SELECT "id", "name", "age" FROM "people" WHERE "age" >= $1',
        alias: "adults",
        params: [18],
      },
      filters: [{ column: "name", operator: "like", value: "C%" }],
      sort: [{ column: "id", direction: "asc" }],
      limit: 5,
    });

    expect(query.sql).toBe(
      'SELECT * FROM (SELECT "id", "name", "age" FROM "people" WHERE "age" >= $1) AS "adults" WHERE "name" LIKE $2 ORDER BY "id" ASC LIMIT 5',
    );
    expect(query.params).toEqual([18, "C%"]);

    const { rows } = await db.query<{ id: number; name: string; age: number }>(
      query.sql,
      query.params,
    );
    expect(rows).toEqual([{ id: 3, name: "Cara", age: 28 }]);
  });

  it("supports schema-qualified tables and column aliases", async () => {
    const query = buildQuery({
      table: "people",
      schema: "public",
      columns: [{ name: "first_name", alias: "firstName" }],
      filters: [{ column: "id", operator: "eq", value: 1 }],
    });

    expect(query.sql).toBe(
      'SELECT "first_name" AS "firstName" FROM "public"."people" WHERE "id" = $1',
    );

    const { rows } = await db.query<{ firstName: string }>(query.sql, query.params);
    expect(rows).toEqual([{ firstName: "Ada" }]);
  });

  it("uses TRUE for an empty AND group and FALSE for an empty OR group", () => {
    const andQuery = buildQuery({
      table: "people",
      filters: [{ combinator: "and", filters: [] }],
    });
    const orQuery = buildQuery({
      table: "people",
      filters: [{ combinator: "or", filters: [] }],
    });

    expect(andQuery.sql).toBe('SELECT * FROM "people" WHERE TRUE');
    expect(orQuery.sql).toBe('SELECT * FROM "people" WHERE FALSE');
  });

  it("rejects identifiers that are not plain SQL names", () => {
    expect(() => buildQuery({ table: "people; DROP TABLE people" })).toThrow(
      /Invalid table identifier/,
    );
    expect(() =>
      buildQuery({ table: "people", columns: ['name" OR 1=1 --'] }),
    ).toThrow(/Invalid column identifier/);
    expect(() =>
      buildQuery({ table: "people", sort: [{ column: "bad-name" }] }),
    ).toThrow(/Invalid sort column identifier/);
    expect(() =>
      buildQuery({
        subquery: { sql: 'SELECT * FROM "people"', alias: "bad alias" },
      }),
    ).toThrow(/Invalid subquery alias identifier/);
  });

  it("rejects invalid values and pagination", () => {
    expect(() =>
      buildQuery({ table: "people", filters: [{ column: "age", operator: "gt" }] }),
    ).toThrow(/requires a value/);
    expect(() =>
      buildQuery({
        table: "people",
        filters: [{ column: "age", operator: "in", value: 18 }],
      }),
    ).toThrow(/requires an array value/);
    expect(() =>
      buildQuery({
        table: "people",
        filters: [{ column: "age", operator: "between", value: [18] }],
      }),
    ).toThrow(/requires exactly two values/);
    expect(() => buildQuery({ table: "people", limit: -1 })).toThrow(/limit must be/);
    expect(() => buildQuery({ table: "people", offset: 1.5 })).toThrow(/offset must be/);
  });

  it("rejects an empty subquery", () => {
    expect(() =>
      buildQuery({ subquery: { sql: "   ", alias: "source" } }),
    ).toThrow(/Subquery SQL must not be empty/);
  });
});
