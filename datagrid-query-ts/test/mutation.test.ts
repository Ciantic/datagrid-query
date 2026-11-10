import { PGlite } from "@electric-sql/pglite";
import { readFileSync } from "node:fs";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  buildDelete,
  buildInsert,
  buildUpdate,
  buildUpsert,
} from "../src/index.js";
import type { MutationColumn, MutationOptions, MutationRow } from "../src/index.js";

const widgetColumns: MutationColumn[] = [
  { name: "id", type: "bigint", insertDefaultSql: "9000" },
  { name: "name", type: "text" },
  { name: "note", type: "text" },
  {
    name: "created_at",
    type: "text",
    insertDefaultSql: "now()::text",
    updatable: false,
  },
  {
    name: "updated_at",
    type: "text",
    insertDefaultSql: "now()::text",
    updateDefaultSql: "now()::text",
  },
  { name: "version", type: "bigint", insertDefaultSql: "1" },
];

function widgetOptions(rows: MutationRow[]): MutationOptions {
  return {
    table: "widgets",
    columns: widgetColumns,
    primaryKey: ["id"],
    versionColumn: "version",
    rows,
  };
}

const membershipColumns: MutationColumn[] = [
  { name: "tenant_id", type: "bigint" },
  { name: "user_id", type: "bigint" },
  { name: "role", type: "text" },
];

function membershipOptions(rows: MutationRow[]): MutationOptions {
  return {
    table: "memberships",
    columns: membershipColumns,
    primaryKey: ["tenant_id", "user_id"],
    rows,
  };
}

const upsertFixture = JSON.parse(
  readFileSync(new URL("../../fixtures/upsert-options.json", import.meta.url), "utf8"),
) as MutationOptions;

let db: PGlite;

beforeAll(async () => {
  db = new PGlite();
  await db.exec(`
    CREATE TABLE widgets (
      id bigint PRIMARY KEY,
      name text NOT NULL,
      note text,
      created_at text NOT NULL,
      updated_at text NOT NULL,
      version bigint NOT NULL
    );

    CREATE TABLE memberships (
      tenant_id bigint NOT NULL,
      user_id bigint NOT NULL,
      role text NOT NULL,
      PRIMARY KEY (tenant_id, user_id)
    );
  `);
});

afterAll(async () => {
  await db.close();
});

describe("mutation builder SQL", () => {
  it("builds a batch insert with defaults and presence flags", () => {
    const query = buildInsert(widgetOptions([{ id: 1, name: "a" }]));

    expect(query.sql).toBe(
      'INSERT INTO "widgets" ("id", "name", "note", "created_at", "updated_at", "version") SELECT CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END FROM unnest($1::bigint[], $2::bool[], $3::text[], $4::text[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::bigint[], $10::bool[]) AS v("id", "id#present", "name", "note", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") RETURNING "widgets"."id"',
    );
    expect(query.params).toEqual([
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

  it("builds a batch update with presence flags and version locking", () => {
    const query = buildUpdate(widgetOptions([{ id: 1, version: 1, name: "b" }]));

    expect(query.sql).toBe(
      'UPDATE "widgets" AS u SET "name" = CASE WHEN v."name#present" THEN v."name" ELSE u."name" END, "note" = CASE WHEN v."note#present" THEN v."note" ELSE u."note" END, "updated_at" = CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, "version" = u."version" + 1 FROM unnest($1::bigint[], $2::bigint[], $3::text[], $4::bool[], $5::text[], $6::bool[], $7::text[], $8::bool[]) AS v("id", "version", "name", "name#present", "note", "note#present", "updated_at", "updated_at#present") WHERE u."id" = v."id" AND u."version" = v."version" RETURNING u."id"',
    );
    expect(query.params).toEqual([[1], [1], ["b"], [true], [null], [false], [null], [false]]);
  });

  it("builds an upsert with conflict update and version locking", () => {
    const query = buildUpsert(widgetOptions([{ id: 1, name: "a", version: 1 }]));

    expect(query.sql).toBe(
      'INSERT INTO "widgets" AS u ("id", "name", "note", "created_at", "updated_at", "version") SELECT CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END FROM unnest($1::bigint[], $2::bool[], $3::text[], $4::text[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::bigint[], $10::bool[]) AS v("id", "id#present", "name", "note", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") ON CONFLICT ("id") DO UPDATE SET "name" = excluded."name", "note" = excluded."note", "updated_at" = excluded."updated_at", "version" = u."version" + 1 WHERE u."version" = excluded."version" RETURNING u."id"',
    );
    expect(query.params).toEqual([
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
    const serialized = JSON.parse(JSON.stringify(upsertFixture)) as MutationOptions;
    const fromJson = buildUpsert(serialized);
    const fromObject = buildUpsert(upsertFixture);

    expect(fromJson).toEqual(fromObject);
    expect(fromJson.sql).toContain('INSERT INTO "widgets" AS u');
    expect(fromJson.sql).toContain('ON CONFLICT ("id") DO UPDATE SET');
    expect(fromJson.sql).toContain('WHERE u."version" = excluded."version"');
    expect(fromJson.sql).toContain('RETURNING u."id"');
    expect(fromJson.params).toHaveLength(10);
  });

  it("builds a batch delete with version locking", () => {
    const query = buildDelete(widgetOptions([{ id: 1, version: 1 }]));

    expect(query.sql).toBe(
      'DELETE FROM "widgets" AS u USING unnest($1::bigint[], $2::bigint[]) AS v("id", "version") WHERE u."id" = v."id" AND u."version" = v."version" RETURNING u."id"',
    );
    expect(query.params).toEqual([[1], [1]]);
  });
});

describe("mutation execution", () => {
  it("executes insert, update, upsert, and delete", async () => {
    const insert = buildInsert(
      widgetOptions([
        {
          id: 100,
          name: "alpha",
          note: null,
          created_at: "c1",
          updated_at: "u1",
          version: 1,
        },
        { name: "beta" },
      ]),
    );
    await db.query(insert.sql, insert.params);

    const inserted = await db.query<{ id: number; name: string; version: number }>(
      'SELECT "id", "name", "version" FROM "widgets" ORDER BY "id"',
    );
    expect(inserted.rows).toEqual([
      { id: 100, name: "alpha", version: 1 },
      { id: 9000, name: "beta", version: 1 },
    ]);

    const update = buildUpdate(
      widgetOptions([
        { id: 100, version: 1, name: "alpha2", note: "n1" },
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

    const upsert = buildUpsert(
      widgetOptions([
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
      ]),
    );
    await db.query(upsert.sql, upsert.params);

    const upserted = await db.query<{
      id: number;
      name: string;
      note: string | null;
      version: number;
    }>('SELECT "id", "name", "note", "version" FROM "widgets" ORDER BY "id"');
    expect(upserted.rows).toEqual([
      { id: 100, name: "alpha3", note: "n2", version: 3 },
      { id: 200, name: "gamma", note: null, version: 1 },
      { id: 9000, name: "beta2", note: null, version: 2 },
    ]);

    const created = await db.query<{ id: number; created_at: string }>(
      'SELECT "id", "created_at" FROM "widgets" WHERE "id" IN (100, 200) ORDER BY "id"',
    );
    expect(created.rows).toEqual([
      { id: 100, created_at: "c1" },
      { id: 200, created_at: "c3" },
    ]);

    const staleUpsert = buildUpsert(widgetOptions([{ id: 9000, name: "stale", version: 1 }]));
    await db.query(staleUpsert.sql, staleUpsert.params);

    const stale = await db.query<{ name: string }>(
      'SELECT "name" FROM "widgets" WHERE "id" = 9000',
    );
    expect(stale.rows).toEqual([{ name: "beta2" }]);

    const deleteQuery = buildDelete(widgetOptions([{ id: 200, version: 1 }]));
    await db.query(deleteQuery.sql, deleteQuery.params);

    const remaining = await db.query<{ id: number }>('SELECT "id" FROM "widgets" ORDER BY "id"');
    expect(remaining.rows).toEqual([{ id: 100 }, { id: 9000 }]);
  });

  it("executes composite primary keys without a version column", async () => {
    const insert = buildInsert(
      membershipOptions([
        { tenant_id: 1, user_id: 10, role: "admin" },
        { tenant_id: 1, user_id: 20, role: "user" },
      ]),
    );
    await db.query(insert.sql, insert.params);

    const update = buildUpdate(
      membershipOptions([{ tenant_id: 1, user_id: 20, role: "manager" }]),
    );
    await db.query(update.sql, update.params);

    const upsert = buildUpsert(
      membershipOptions([
        { tenant_id: 1, user_id: 10, role: "super" },
        { tenant_id: 2, user_id: 30, role: "owner" },
      ]),
    );
    await db.query(upsert.sql, upsert.params);

    const upserted = await db.query<{
      tenant_id: number;
      user_id: number;
      role: string;
    }>(
      'SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id"',
    );
    expect(upserted.rows).toEqual([
      { tenant_id: 1, user_id: 10, role: "super" },
      { tenant_id: 1, user_id: 20, role: "manager" },
      { tenant_id: 2, user_id: 30, role: "owner" },
    ]);

    const deleteQuery = buildDelete(membershipOptions([{ tenant_id: 1, user_id: 10 }]));
    await db.query(deleteQuery.sql, deleteQuery.params);

    const remaining = await db.query<{
      tenant_id: number;
      user_id: number;
      role: string;
    }>(
      'SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id"',
    );
    expect(remaining.rows).toEqual([
      { tenant_id: 1, user_id: 20, role: "manager" },
      { tenant_id: 2, user_id: 30, role: "owner" },
    ]);
  });
});
