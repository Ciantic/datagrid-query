import type { PGlite } from "@electric-sql/pglite";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { bindMutationRows, buildUpdate } from "../../src/index.js";
import { createDatabase, membershipOptions, widgetOptions } from "./support.js";

let db: PGlite;

beforeAll(async () => {
  db = await createDatabase();
});

afterAll(async () => {
  await db.close();
});

describe("buildUpdate", () => {
  it("patches only supplied fields and handles explicit null", async () => {
    await db.exec(`
      DELETE FROM widgets;
      INSERT INTO widgets (id, name, note, created_at, updated_at, version) VALUES
        (100, 'alpha', 'old-note', 'c1', 'u1', 1),
        (9000, 'beta', NULL, 'c2', 'u2', 1);
    `);

    const update = buildUpdate(widgetOptions());
    const rows = [
      { id: 100, version: 1, name: "alpha2", note: "n1", updated_at: "u2" },
      { id: 9000, version: 1, name: "beta2" },
    ];
    await db.query(update.sql, bindMutationRows(update, rows));

    const updated = await db.query<{
      id: number;
      name: string;
      note: string | null;
      version: number;
    }>('SELECT "id", "name", "note", "version" FROM "widgets" ORDER BY "id"');
    expect(updated.rows).toEqual([
      { id: 100, name: "alpha2", note: "n1", version: 2 },
      { id: 9000, name: "beta2", note: null, version: 2 },
    ]);

    const clearNote = buildUpdate(widgetOptions());
    const clearRows = [{ id: 100, version: 2, note: null }];
    await db.query(clearNote.sql, bindMutationRows(clearNote, clearRows));

    const cleared = await db.query<{ name: string; note: string | null; version: number }>(
      'SELECT "name", "note", "version" FROM "widgets" WHERE "id" = 100',
    );
    expect(cleared.rows).toEqual([{ name: "alpha2", note: null, version: 3 }]);

    const stale = buildUpdate(widgetOptions());
    const staleRows = [{ id: 9000, version: 1, name: "stale" }];
    await db.query(stale.sql, bindMutationRows(stale, staleRows));

    const unchanged = await db.query<{ name: string; version: number }>(
      'SELECT "name", "version" FROM "widgets" WHERE "id" = 9000',
    );
    expect(unchanged.rows).toEqual([{ name: "beta2", version: 2 }]);
  });

  it("executes composite primary key updates without a version column", async () => {
    await db.exec(`
      DELETE FROM memberships;
      INSERT INTO memberships (tenant_id, user_id, role) VALUES
        (1, 10, 'admin'),
        (1, 20, 'user');
    `);

    const update = buildUpdate(membershipOptions());
    const rows = [{ tenant_id: 1, user_id: 20, role: "manager" }];
    await db.query(update.sql, bindMutationRows(update, rows));

    const updated = await db.query<{
      tenant_id: number;
      user_id: number;
      role: string;
    }>(
      'SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id"',
    );
    expect(updated.rows).toEqual([
      { tenant_id: 1, user_id: 10, role: "admin" },
      { tenant_id: 1, user_id: 20, role: "manager" },
    ]);
  });
});
