import { PGlite } from "@electric-sql/pglite";
import { readFileSync } from "node:fs";
import type { MutationColumn, MutationOptions, MutationRow } from "../src/index.js";

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

export function widgetOptions(rows: MutationRow[], table = "widgets"): MutationOptions {
  return {
    table,
    columns: widgetColumns,
    primaryKey: ["id"],
    versionColumn: "version",
    rows,
  };
}

export function mergeWidgetOptions(rows: MutationRow[]): MutationOptions {
  return widgetOptions(rows, "merge_widgets");
}

const membershipColumns: MutationColumn[] = [
  { name: "tenant_id", type: "bigint" },
  { name: "user_id", type: "bigint" },
  { name: "role", type: "text" },
];

export function membershipOptions(rows: MutationRow[]): MutationOptions {
  return {
    table: "memberships",
    columns: membershipColumns,
    primaryKey: ["tenant_id", "user_id"],
    rows,
  };
}

export const upsertFixture = JSON.parse(
  readFileSync(new URL("../../fixtures/upsert-options.json", import.meta.url), "utf8"),
) as MutationOptions;

export async function createDatabase(): Promise<PGlite> {
  const db = new PGlite();
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

    CREATE TABLE merge_widgets (
      id bigint PRIMARY KEY,
      name text NOT NULL,
      note text,
      created_at text NOT NULL,
      updated_at text NOT NULL,
      version bigint NOT NULL
    );
  `);

  return db;
}
