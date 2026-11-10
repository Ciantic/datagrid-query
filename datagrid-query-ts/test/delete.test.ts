import type { PGlite } from "@electric-sql/pglite";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { bindMutationRows, buildDelete } from "../src/index.js";
import { createDatabase, membershipOptions, widgetOptions } from "./support.js";

let db: PGlite;

beforeAll(async () => {
  db = await createDatabase();
});

afterAll(async () => {
  await db.close();
});

describe("buildDelete", () => {
  it("builds a batch delete with version locking", () => {
    const query = buildDelete(widgetOptions());
    const params = bindMutationRows(query, [{ id: 1, version: 1 }]);

    expect(query.sql).toBe(
      'DELETE FROM "widgets" AS u USING unnest($1::bigint[], $2::bigint[]) AS v("id", "version") WHERE u."id" = v."id" AND u."version" = v."version" RETURNING u."id"',
    );
    expect(params).toEqual([[1], [1]]);
  });

  it("deletes only rows matching the primary key and version", async () => {
    await db.exec(`
      DELETE FROM widgets;
      INSERT INTO widgets (id, name, note, created_at, updated_at, version) VALUES
        (100, 'alpha', NULL, 'c1', 'u1', 2),
        (200, 'beta', NULL, 'c2', 'u2', 1);
    `);

    const stale = buildDelete(widgetOptions());
    await db.query(stale.sql, bindMutationRows(stale, [{ id: 100, version: 1 }]));

    const afterStale = await db.query<{ id: number }>('SELECT "id" FROM "widgets" ORDER BY "id"');
    expect(afterStale.rows).toEqual([{ id: 100 }, { id: 200 }]);

    const remove = buildDelete(widgetOptions());
    await db.query(remove.sql, bindMutationRows(remove, [{ id: 200, version: 1 }]));

    const remaining = await db.query<{ id: number }>('SELECT "id" FROM "widgets" ORDER BY "id"');
    expect(remaining.rows).toEqual([{ id: 100 }]);
  });

  it("executes composite primary key deletes without a version column", async () => {
    await db.exec(`
      DELETE FROM memberships;
      INSERT INTO memberships (tenant_id, user_id, role) VALUES
        (1, 10, 'admin'),
        (1, 20, 'user');
    `);

    const remove = buildDelete(membershipOptions());
    await db.query(remove.sql, bindMutationRows(remove, [{ tenant_id: 1, user_id: 10 }]));

    const remaining = await db.query<{
      tenant_id: number;
      user_id: number;
      role: string;
    }>(
      'SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id"',
    );
    expect(remaining.rows).toEqual([{ tenant_id: 1, user_id: 20, role: "user" }]);
  });
});
