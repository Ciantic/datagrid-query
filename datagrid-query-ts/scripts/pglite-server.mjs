import { PGlite } from "@electric-sql/pglite";
import { PGLiteSocketServer } from "@electric-sql/pglite-socket";

const host = process.env.PGLITE_HOST ?? "127.0.0.1";
const port = Number(process.env.PGLITE_PORT);

if (!Number.isInteger(port) || port <= 0) {
  console.error("PGLITE_PORT must be a positive integer");
  process.exit(1);
}

const db = await PGlite.create();
const server = new PGLiteSocketServer({ db, host, port });

try {
  await server.start();
} catch (error) {
  console.error(error);
  await db.close();
  process.exit(1);
}

async function shutdown() {
  try {
    await server.stop();
  } finally {
    await db.close();
  }
  process.exit(0);
}

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
