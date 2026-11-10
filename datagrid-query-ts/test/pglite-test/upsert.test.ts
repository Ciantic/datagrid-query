import type { PGlite } from "@electric-sql/pglite";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { bindMutationRows, buildUpsert } from "../../src/index.js";
import { createDatabase, membershipOptions, widgetOptions } from "./support.js";

let db: PGlite;

beforeAll(async () => {
  db = await createDatabase();
});

afterAll(async () => {
  await db.close();
});

describe("buildUpsert", () => {
  it("executes insert, full-row update, and version mismatch", async () => {
    await db.exec(`
      DELETE FROM widgets;
      INSERT INTO widgets (id, name, note, created_at, updated_at, version) VALUES
        (100, 'alpha', 'old-note', 'c1', 'u1', 2),
        (9000, 'beta', NULL, 'c9', 'u9', 2);
    `);

    const query = buildUpsert(widgetOptions());
    const rows = [
      {
        id: 100,
        name: "alpha3",
        note: "n2",
        created_at: "ignored",
        updated_at: "u3",
        version: 2,
      },
      {
        id: 200,
        name: "gamma",
        created_at: "c3",
        updated_at: "u3",
        version: 1,
      },
      {
        id: 9000,
        name: "stale",
        version: 1,
      },
    ];
    await db.query(query.sql, bindMutationRows(query, rows));

    const upserted = await db.query<{
      id: number;
      name: string;
      note: string | null;
      created_at: string;
      version: number;
    }>('SELECT "id", "name", "note", "created_at", "version" FROM "widgets" ORDER BY "id"');
    expect(upserted.rows).toEqual([
      { id: 100, name: "alpha3", note: "n2", created_at: "c1", version: 3 },
      { id: 200, name: "gamma", note: null, created_at: "c3", version: 1 },
      { id: 9000, name: "beta", note: null, created_at: "c9", version: 2 },
    ]);
  });

  it("executes composite primary key upserts without a version column", async () => {
    await db.exec(`
      DELETE FROM memberships;
      INSERT INTO memberships (tenant_id, user_id, role) VALUES
        (1, 10, 'admin'),
        (1, 20, 'user');
    `);

    const query = buildUpsert(membershipOptions());
    const rows = [
      { tenant_id: 1, user_id: 10, role: "super" },
      { tenant_id: 2, user_id: 30, role: "owner" },
    ];
    await db.query(query.sql, bindMutationRows(query, rows));

    const upserted = await db.query<{
      tenant_id: number;
      user_id: number;
      role: string;
    }>(
      'SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id"',
    );
    expect(upserted.rows).toEqual([
      { tenant_id: 1, user_id: 10, role: "super" },
      { tenant_id: 1, user_id: 20, role: "user" },
      { tenant_id: 2, user_id: 30, role: "owner" },
    ]);
  });
});
