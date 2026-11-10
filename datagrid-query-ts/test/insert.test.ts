import { describe, expect, it } from "vitest";
import { bindMutationRows, buildInsert } from "../src/index.js";
import { widgetOptions } from "./support.js";

describe("buildInsert", () => {
  it("builds a batch insert with defaults and presence flags", () => {
    const query = buildInsert(widgetOptions());
    const params = bindMutationRows(query, [{ id: 1, name: "a" }]);

    expect(query.sql).toBe(
      'INSERT INTO "widgets" ("id", "name", "note", "created_at", "updated_at", "version") SELECT CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END FROM unnest($1::bigint[], $2::bool[], $3::text[], $4::text[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::bigint[], $10::bool[]) AS v("id", "id#present", "name", "note", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") RETURNING "widgets"."id"',
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
      [null],
      [false],
    ]);
  });
});
