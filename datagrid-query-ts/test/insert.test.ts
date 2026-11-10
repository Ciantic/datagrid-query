import type { PGlite } from "@electric-sql/pglite";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { bindMutationRows, buildInsert } from "../src/index.js";
import { createDatabase, membershipOptions, widgetOptions } from "./support.js";

let db: PGlite;

beforeAll(async () => {
  db = await createDatabase();
});

afterAll(async () => {
  await db.close();
});

describe("buildInsert", () => {
  it("builds a batch insert with defaults and presence flags", () => {
    const query = buildInsert(widgetOptions());
    const params = bindMutationRows(query, [{ id: 1, name: "a" }]);

    expect(query.sql).toBe(
      'INSERT INTO "widgets" ("id", "name", "note", "created_at", "updated_at", "version") SELECT CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END FROM unnest($1::bigint[], $2::bool[], $3::text[], $4::text[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::bigint[], $10::bool[]) AS v("id", "id#present", "name", "note", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") RETURNING "widgets"."id"',
    );
    expect(params).toEqual([
      [1],
      [true],
      ["a"],
      [null],
      [null],
      [false],
      [null],
      [false],
      [null],
      [false],
    ]);
  });

  it("executes a batch insert with defaults", async () => {
    const query = buildInsert(widgetOptions());
    const rows = [
      {
        id: 100,
        name: "alpha",
        note: null,
        created_at: "c1",
        updated_at: "u1",
        version: 1,
      },
      { name: "beta" },
    ];
    await db.query(query.sql, bindMutationRows(query, rows));

    const inserted = await db.query<{ id: number; name: string; version: number }>(
      'SELECT "id", "name", "version" FROM "widgets" ORDER BY "id"',
    );
    expect(inserted.rows).toEqual([
      { id: 100, name: "alpha", version: 1 },
      { id: 9000, name: "beta", version: 1 },
    ]);
  });

  it("executes composite primary key inserts", async () => {
    const query = buildInsert(membershipOptions());
    const rows = [
      { tenant_id: 1, user_id: 10, role: "admin" },
      { tenant_id: 1, user_id: 20, role: "user" },
    ];
    await db.query(query.sql, bindMutationRows(query, rows));

    const inserted = await db.query<{ tenant_id: number; user_id: number; role: string }>(
      'SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id"',
    );
    expect(inserted.rows).toEqual([
      { tenant_id: 1, user_id: 10, role: "admin" },
      { tenant_id: 1, user_id: 20, role: "user" },
    ]);
  });
});
