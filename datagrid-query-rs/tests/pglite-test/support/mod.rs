#![allow(dead_code)]

#[path = "../../support/mod.rs"]
mod shared;
pub(crate) use shared::*;

use datagrid_query::{BuiltMutation, BuiltQuery, MutationRow, QueryOptions, Value, build_query};
use postgres::types::ToSql;
use postgres::{Client, NoTls, Row};
use std::collections::BTreeMap;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const PGLITE_SERVER_SCRIPT: &str = r#"
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
            .args(["--install=fallback", "--eval", PGLITE_SERVER_SCRIPT])
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

pub(crate) fn execute(client: &mut Client, options: &QueryOptions) -> Vec<Row> {
    let query = build_query(options).expect("query should build");
    run(client, &query)
}

pub(crate) fn run(client: &mut Client, query: &BuiltQuery) -> Vec<Row> {
    run_sql(client, &query.sql, &query.params)
}

pub(crate) fn run_mutation(
    client: &mut Client,
    mutation: &BuiltMutation,
    rows: &[MutationRow],
) -> Vec<Row> {
    let params = bind(mutation, rows);
    run_sql(client, &mutation.sql, &params)
}

fn run_sql(client: &mut Client, sql: &str, params: &[Value]) -> Vec<Row> {
    let params = to_sql_params(sql, params);
    let refs: Vec<&(dyn ToSql + Sync)> = params.iter().map(|param| param.as_ref()).collect();

    client
        .query(sql, &refs)
        .unwrap_or_else(|error| panic!("query failed: {error:?}\n{sql}"))
}

fn to_sql_params(sql: &str, values: &[Value]) -> Vec<Box<dyn ToSql + Sync>> {
    let types = parameter_types(sql);

    values
        .iter()
        .enumerate()
        .map(|(index, value)| match value {
            Value::Null => boxed(Option::<String>::None),
            Value::Bool(value) => boxed(*value),
            Value::Int(value) => boxed(*value),
            Value::Float(value) => boxed(*value),
            Value::Text(value) => boxed(value.clone()),
            Value::Bytes(value) => boxed(value.clone()),
            Value::Array(values) => array_param(values, types.get(&(index + 1))),
            Value::Json(value) => boxed(value.clone()),
        })
        .collect()
}

fn boxed<T>(value: T) -> Box<dyn ToSql + Sync>
where
    T: ToSql + Sync + 'static,
{
    Box::new(value)
}

fn parameter_types(sql: &str) -> BTreeMap<usize, String> {
    let bytes = sql.as_bytes();
    let mut types = BTreeMap::new();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'$' || index + 1 >= bytes.len() || !bytes[index + 1].is_ascii_digit() {
            index += 1;
            continue;
        }

        let start = index + 1;
        let mut end = start;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }

        let Ok(number) = sql[start..end].parse::<usize>() else {
            index = end;
            continue;
        };

        if !sql[end..].starts_with("::") {
            index = end;
            continue;
        }

        let type_start = end + 2;
        let mut type_end = type_start;
        while type_end < bytes.len() && bytes[type_end] != b'[' {
            type_end += 1;
        }

        if type_end < bytes.len() {
            types.insert(number, sql[type_start..type_end].trim().to_owned());
            index = type_end + 1;
        } else {
            index = end;
        }
    }

    types
}

fn array_param(values: &[Value], data_type: Option<&String>) -> Box<dyn ToSql + Sync> {
    if let Some(data_type) = data_type {
        if data_type.ends_with("bool") {
            return boxed(
                values
                    .iter()
                    .map(|value| match value {
                        Value::Bool(value) => Some(*value),
                        Value::Null => None,
                        _ => panic!("expected boolean array parameter"),
                    })
                    .collect::<Vec<Option<bool>>>(),
            );
        }

        if data_type.contains("int") {
            return boxed(
                values
                    .iter()
                    .map(|value| match value {
                        Value::Int(value) => Some(*value),
                        Value::Null => None,
                        _ => panic!("expected integer array parameter"),
                    })
                    .collect::<Vec<Option<i64>>>(),
            );
        }

        return boxed(
            values
                .iter()
                .map(|value| match value {
                    Value::Text(value) => Some(value.clone()),
                    Value::Null => None,
                    _ => panic!("expected text array parameter"),
                })
                .collect::<Vec<Option<String>>>(),
        );
    }

    if values
        .iter()
        .all(|value| matches!(value, Value::Int(_) | Value::Null))
    {
        return boxed(
            values
                .iter()
                .map(|value| match value {
                    Value::Int(value) => Some(*value),
                    Value::Null => None,
                    _ => unreachable!(),
                })
                .collect::<Vec<Option<i64>>>(),
        );
    }

    if values
        .iter()
        .all(|value| matches!(value, Value::Text(_) | Value::Null))
    {
        return boxed(
            values
                .iter()
                .map(|value| match value {
                    Value::Text(value) => Some(value.clone()),
                    Value::Null => None,
                    _ => unreachable!(),
                })
                .collect::<Vec<Option<String>>>(),
        );
    }

    if values
        .iter()
        .all(|value| matches!(value, Value::Bool(_) | Value::Null))
    {
        return boxed(
            values
                .iter()
                .map(|value| match value {
                    Value::Bool(value) => Some(*value),
                    Value::Null => None,
                    _ => unreachable!(),
                })
                .collect::<Vec<Option<bool>>>(),
        );
    }

    panic!("PGlite test helper only supports integer, text, and boolean array parameters");
}

pub(crate) fn id_names(rows: &[Row]) -> Vec<(i64, String)> {
    rows.iter()
        .map(|row| (row.get("id"), row.get("name")))
        .collect()
}

pub(crate) fn ids(rows: &[Row]) -> Vec<i64> {
    rows.iter().map(|row| row.get("id")).collect()
}

pub(crate) fn create_mutation_tables(client: &mut Client) {
    client
        .batch_execute(
            r#"
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
            "#,
        )
        .expect("failed to create mutation test tables");
}
