import { describe, expect, it } from "vitest";
import { buildQuery } from "../src/index.js";

describe("buildQuery", () => {
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
