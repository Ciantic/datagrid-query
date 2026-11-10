use postgres::{Client, NoTls};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const SERVER_SCRIPT: &str = r#"
import { PGlite } from "@electric-sql/pglite@0.5.16";
import { PGLiteSocketServer } from "@electric-sql/pglite-socket@0.2.19";

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
"#;

pub(crate) struct PgliteServer {
    child: Child,
}

impl PgliteServer {
    pub(crate) fn start() -> (Self, Client) {
        let port = free_port();

        let child = Command::new("bun")
            .args(["--install=fallback", "--eval", SERVER_SCRIPT])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env("PGLITE_PORT", port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|error| {
                panic!("failed to start `bun`: {error}. Install Bun from https://bun.sh first.")
            });

        let mut server = Self { child };
        let client = server.connect(port);
        (server, client)
    }

    fn connect(&mut self, port: u16) -> Client {
        let connection = format!(
            "host=127.0.0.1 port={port} user=postgres password=postgres dbname=postgres connect_timeout=1"
        );
        let deadline = Instant::now() + Duration::from_secs(60);

        loop {
            if let Some(status) = self.child.try_wait().expect("failed to poll PGlite server") {
                panic!("PGlite server exited during startup: {status}");
            }

            match Client::connect(&connection, NoTls) {
                Ok(client) => return client,
                Err(error) => {
                    if Instant::now() >= deadline {
                        panic!("timed out waiting for PGlite server: {error}");
                    }
                }
            }

            thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for PgliteServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("failed to reserve a local port");
    listener
        .local_addr()
        .expect("failed to read the reserved port")
        .port()
}
