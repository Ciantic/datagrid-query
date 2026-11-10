import type { MutationColumn, MutationOptions, MutationRow } from "../src/index.js";
import { readFileSync } from "node:fs";

export const widgetColumns: MutationColumn[] = [
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

export function widgetOptions(table = "widgets"): MutationOptions {
  return {
    table,
    columns: widgetColumns,
    primaryKey: ["id"],
    versionColumn: "version",
  };
}

export function mergeWidgetOptions(): MutationOptions {
  return widgetOptions("merge_widgets");
}

const membershipColumns: MutationColumn[] = [
  { name: "tenant_id", type: "bigint" },
  { name: "user_id", type: "bigint" },
  { name: "role", type: "text" },
];

export function membershipOptions(): MutationOptions {
  return {
    table: "memberships",
    columns: membershipColumns,
    primaryKey: ["tenant_id", "user_id"],
  };
}

export const upsertFixture = JSON.parse(
  readFileSync(new URL("../../fixtures/upsert-options.json", import.meta.url), "utf8"),
) as MutationOptions;

export const upsertFixtureRows: MutationRow[] = [
  { id: 100, name: "alpha", version: 2 },
  { name: "beta" },
];
