import { PGlite } from "@electric-sql/pglite";

export * from "../support.js";

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
