import type { PGlite } from "@electric-sql/pglite";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { bindMutationRows, buildInsert } from "../../src/index.js";
import { createDatabase, membershipOptions, widgetOptions } from "./support.js";

let db: PGlite;

beforeAll(async () => {
  db = await createDatabase();
});

afterAll(async () => {
  await db.close();
});

describe("buildInsert", () => {
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
