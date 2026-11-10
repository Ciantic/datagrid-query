import type { PGlite } from "@electric-sql/pglite";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { bindMutationRows, buildUpsert } from "../src/index.js";
import {
  createDatabase,
  membershipOptions,
  upsertFixture,
  upsertFixtureRows,
  widgetOptions,
} from "./support.js";

let db: PGlite;

beforeAll(async () => {
  db = await createDatabase();
});

afterAll(async () => {
  await db.close();
});

describe("buildUpsert", () => {
  it("builds an upsert with conflict update and version locking", () => {
    const query = buildUpsert(widgetOptions());
    const rows = [{ id: 1, name: "a", version: 1 }];
    const params = bindMutationRows(query, rows);

    expect(query.sql).toBe(
      'INSERT INTO "widgets" AS u ("id", "name", "note", "created_at", "updated_at", "version") SELECT CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END FROM unnest($1::bigint[], $2::bool[], $3::text[], $4::text[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::bigint[], $10::bool[]) AS v("id", "id#present", "name", "note", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") ON CONFLICT ("id") DO UPDATE SET "name" = excluded."name", "note" = excluded."note", "updated_at" = excluded."updated_at", "version" = u."version" + 1 WHERE u."version" = excluded."version" RETURNING u."id"',
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
      [1],
      [true],
    ]);
  });

  it("accepts JSON-serialized upsert options", () => {
    const serialized = JSON.parse(JSON.stringify(upsertFixture)) as typeof upsertFixture;
    const fromJson = buildUpsert(serialized);
    const fromObject = buildUpsert(upsertFixture);
    const params = bindMutationRows(fromJson, upsertFixtureRows);

    expect(fromJson).toEqual(fromObject);
    expect(fromJson.sql).toContain('INSERT INTO "widgets" AS u');
    expect(fromJson.sql).toContain('ON CONFLICT ("id") DO UPDATE SET');
    expect(fromJson.sql).toContain('WHERE u."version" = excluded."version"');
    expect(fromJson.sql).toContain('RETURNING u."id"');
    expect(params).toHaveLength(10);
  });

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
