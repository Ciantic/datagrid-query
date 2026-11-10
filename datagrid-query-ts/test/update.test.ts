import { describe, expect, it } from "vitest";
import { bindMutationRows, buildUpdate } from "../src/index.js";
import { widgetOptions } from "./support.js";

describe("buildUpdate", () => {
  it("builds a batch update with presence flags and version locking", () => {
    const query = buildUpdate(widgetOptions());
    const params = bindMutationRows(query, [{ id: 1, version: 1, name: "b" }]);

    expect(query.sql).toBe(
      'UPDATE "widgets" AS u SET "name" = CASE WHEN v."name#present" THEN v."name" ELSE u."name" END, "note" = CASE WHEN v."note#present" THEN v."note" ELSE u."note" END, "updated_at" = CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, "version" = u."version" + 1 FROM unnest($1::bigint[], $2::bigint[], $3::text[], $4::bool[], $5::text[], $6::bool[], $7::text[], $8::bool[]) AS v("id", "version", "name", "name#present", "note", "note#present", "updated_at", "updated_at#present") WHERE u."id" = v."id" AND u."version" = v."version" RETURNING u."id"',
    );
    expect(params).toEqual([[1], [1], ["b"], [true], [null], [false], [null], [false]]);
  });
});
