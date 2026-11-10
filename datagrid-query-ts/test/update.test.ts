import type { PGlite } from "@electric-sql/pglite";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { buildUpdate } from "../src/index.js";
import {
  createDatabase,
  membershipOptions,
  widgetOptions,
} from "./support.js";

let db: PGlite;

beforeAll(async () => {
  db = await createDatabase();
});

afterAll(async () => {
  await db.close();
});

describe("buildUpdate", () => {
  it("builds a batch update with presence flags and version locking", () => {
    const query = buildUpdate(widgetOptions([{ id: 1, version: 1, name: "b" }]));

    expect(query.sql).toBe(
      'UPDATE "widgets" AS u SET "name" = CASE WHEN v."name#present" THEN v."name" ELSE u."name" END, "note" = CASE WHEN v."note#present" THEN v."note" ELSE u."note" END, "updated_at" = CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, "version" = u."version" + 1 FROM unnest($1::bigint[], $2::bigint[], $3::text[], $4::bool[], $5::text[], $6::bool[], $7::text[], $8::bool[]) AS v("id", "version", "name", "name#present", "note", "note#present", "updated_at", "updated_at#present") WHERE u."id" = v."id" AND u."version" = v."version" RETURNING u."id"',
    );
    expect(query.params).toEqual([[1], [1], ["b"], [true], [null], [false], [null], [false]]);
  });

  it("patches only supplied fields and handles explicit null", async () => {
    await db.exec(`
      DELETE FROM widgets;
      INSERT INTO widgets (id, name, note, created_at, updated_at, version) VALUES
        (100, 'alpha', 'old-note', 'c1', 'u1', 1),
        (9000, 'beta', NULL, 'c2', 'u2', 1);
    `);

    const update = buildUpdate(
      widgetOptions([
        { id: 100, version: 1, name: "alpha2", note: "n1", updated_at: "u2" },
        { id: 9000, version: 1, name: "beta2" },
      ]),
    );
    await db.query(update.sql, update.params);

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

    const clearNote = buildUpdate(
      widgetOptions([{ id: 100, version: 2, note: null }]),
    );
    await db.query(clearNote.sql, clearNote.params);

    const cleared = await db.query<{ name: string; note: string | null; version: number }>(
      'SELECT "name", "note", "version" FROM "widgets" WHERE "id" = 100',
    );
    expect(cleared.rows).toEqual([{ name: "alpha2", note: null, version: 3 }]);

    const stale = buildUpdate(
      widgetOptions([{ id: 9000, version: 1, name: "stale" }]),
    );
    await db.query(stale.sql, stale.params);

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

    const update = buildUpdate(
      membershipOptions([{ tenant_id: 1, user_id: 20, role: "manager" }]),
    );
    await db.query(update.sql, update.params);

    const rows = await db.query<{ tenant_id: number; user_id: number; role: string }>(
      'SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id"',
    );
    expect(rows.rows).toEqual([
      { tenant_id: 1, user_id: 10, role: "admin" },
      { tenant_id: 1, user_id: 20, role: "manager" },
    ]);
  });
});
