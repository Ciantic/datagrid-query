import { describe, expect, it } from "vitest";
import { bindMutationRows, buildMerge } from "../src/index.js";
import { mergeWidgetOptions, upsertFixture, upsertFixtureRows } from "./support.js";

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
});
