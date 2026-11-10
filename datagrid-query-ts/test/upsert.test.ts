import { describe, expect, it } from "vitest";
import { bindMutationRows, buildUpsert } from "../src/index.js";
import { upsertFixture, upsertFixtureRows, widgetOptions } from "./support.js";

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
});
