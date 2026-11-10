import { describe, expect, it } from "vitest";
import { bindMutationRows, buildDelete } from "../src/index.js";
import { widgetOptions } from "./support.js";

describe("buildDelete", () => {
  it("builds a batch delete with version locking", () => {
    const query = buildDelete(widgetOptions());
    const params = bindMutationRows(query, [{ id: 1, version: 1 }]);

    expect(query.sql).toBe(
      'DELETE FROM "widgets" AS u USING unnest($1::bigint[], $2::bigint[]) AS v("id", "version") WHERE u."id" = v."id" AND u."version" = v."version" RETURNING u."id"',
    );
    expect(params).toEqual([[1], [1]]);
  });
});
