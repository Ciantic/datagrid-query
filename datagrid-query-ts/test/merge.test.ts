import type { PGlite } from "@electric-sql/pglite";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { bindMutationRows, buildMerge } from "../src/index.js";
import {
  createDatabase,
  membershipOptions,
  mergeWidgetOptions,
  upsertFixture,
  upsertFixtureRows,
} from "./support.js";

let db: PGlite;

beforeAll(async () => {
  db = await createDatabase();
});

afterAll(async () => {
  await db.close();
});

describe("buildMerge", () => {
  it("builds a MERGE with per-field patch semantics", () => {
    const query = buildMerge(mergeWidgetOptions());
    const params = bindMutationRows(query, [
      { id: 100, version: 3, name: "patched" },
    ]);

    expect(query.sql).toBe(
      'MERGE INTO "merge_widgets" AS u USING unnest($1::bigint[], $2::bool[], $3::text[], $4::bool[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::text[], $10::bool[], $11::bigint[], $12::bool[]) AS v("id", "id#present", "name", "name#present", "note", "note#present", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") ON u."id" = v."id" WHEN MATCHED AND u."version" = v."version" THEN UPDATE SET "name" = CASE WHEN v."name#present" THEN v."name" ELSE u."name" END, "note" = CASE WHEN v."note#present" THEN v."note" ELSE u."note" END, "updated_at" = CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, "version" = u."version" + 1 WHEN NOT MATCHED THEN INSERT ("id", "name", "note", "created_at", "updated_at", "version") VALUES (CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END) RETURNING u."id"',
    );
    expect(params).toEqual([
      [100],
      [true],
      ["patched"],
      [true],
      [null],
      [false],
      [null],
      [false],
      [null],
      [false],
      [3],
      [true],
    ]);
  });

  it("accepts JSON-serialized merge options", () => {
    const serialized = JSON.parse(JSON.stringify(upsertFixture)) as typeof upsertFixture;
    const query = buildMerge(serialized);
    const params = bindMutationRows(query, upsertFixtureRows);

    expect(query.sql).toContain('MERGE INTO "widgets" AS u');
    expect(params).toHaveLength(12);
  });

  it("executes MERGE as a patch-style upsert", async () => {
    await db.exec(`
      DELETE FROM merge_widgets;
      INSERT INTO merge_widgets (id, name, note, created_at, updated_at, version)
      VALUES (100, 'alpha', 'old-note', 'c0', 'u0', 3);
    `);

    const first = buildMerge(mergeWidgetOptions());
    const firstRows = [
      { id: 100, version: 3, name: "patched" },
      { id: 200, name: "new", note: "new-note", version: 1 },
      { id: 100, version: 2, name: "stale" },
    ];
    await db.query(first.sql, bindMutationRows(first, firstRows));

    const merged = await db.query<{
      id: number;
      name: string;
      note: string | null;
      version: number;
    }>('SELECT "id", "name", "note", "version" FROM "merge_widgets" ORDER BY "id"');
    expect(merged.rows).toEqual([
      { id: 100, name: "patched", note: "old-note", version: 4 },
      { id: 200, name: "new", note: "new-note", version: 1 },
    ]);

    const timestamps = await db.query<{
      id: number;
      created_at: string;
      updated_at: string;
    }>(
      'SELECT "id", "created_at", "updated_at" FROM "merge_widgets" ORDER BY "id"',
    );
    expect(timestamps.rows[0]?.created_at).toBe("c0");
    expect(timestamps.rows[0]?.updated_at).not.toBe("u0");
    expect(timestamps.rows[1]?.created_at).toBeTruthy();
    expect(timestamps.rows[1]?.updated_at).toBeTruthy();

    const second = buildMerge(mergeWidgetOptions());
    const secondRows = [{ id: 200, version: 1, note: null }];
    await db.query(second.sql, bindMutationRows(second, secondRows));

    const patched = await db.query<{
      name: string;
      note: string | null;
      version: number;
    }>('SELECT "name", "note", "version" FROM "merge_widgets" WHERE "id" = 200');
    expect(patched.rows).toEqual([{ name: "new", note: null, version: 2 }]);
  });

  it("executes composite primary key merges without a version column", async () => {
    await db.exec(`
      DELETE FROM memberships;
      INSERT INTO memberships (tenant_id, user_id, role) VALUES
        (1, 10, 'admin'),
        (1, 20, 'user');
    `);

    const merge = buildMerge(membershipOptions());
    const rows = [
      { tenant_id: 1, user_id: 20, role: "manager" },
      { tenant_id: 2, user_id: 30, role: "owner" },
    ];
    await db.query(merge.sql, bindMutationRows(merge, rows));

    const merged = await db.query<{
      tenant_id: number;
      user_id: number;
      role: string;
    }>(
      'SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id"',
    );
    expect(merged.rows).toEqual([
      { tenant_id: 1, user_id: 10, role: "admin" },
      { tenant_id: 1, user_id: 20, role: "manager" },
      { tenant_id: 2, user_id: 30, role: "owner" },
    ]);
  });
});
