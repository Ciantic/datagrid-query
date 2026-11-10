use datagrid_query_rs::{Value, build_upsert};

mod support;
use support::{
    PgliteServer, create_mutation_tables, membership_options, mutation_row as row, run_mutation,
    text, widget_options,
};

#[test]
fn executes_upsert_against_pglite() {
    let (_server, mut client) = PgliteServer::start();
    create_mutation_tables(&mut client);
    client
        .batch_execute(
            r#"
            INSERT INTO widgets (id, name, note, created_at, updated_at, version) VALUES
                (100, 'alpha', 'old-note', 'c1', 'u1', 2),
                (9000, 'beta', NULL, 'c9', 'u9', 2);
            "#,
        )
        .unwrap();

    let mutation = build_upsert(&widget_options()).unwrap();
    let rows = vec![
        row(&[
            ("id", Value::Int(100)),
            ("name", text("alpha3")),
            ("note", text("n2")),
            ("created_at", text("ignored")),
            ("updated_at", text("u3")),
            ("version", Value::Int(2)),
        ]),
        row(&[
            ("id", Value::Int(200)),
            ("name", text("gamma")),
            ("created_at", text("c3")),
            ("updated_at", text("u3")),
            ("version", Value::Int(1)),
        ]),
        row(&[
            ("id", Value::Int(9000)),
            ("name", text("stale")),
            ("version", Value::Int(1)),
        ]),
    ];
    run_mutation(&mut client, &mutation, &rows);

    let upserted = client
        .query(
            r#"SELECT "id", "name", "note", "created_at", "version" FROM "widgets" ORDER BY "id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        upserted
            .iter()
            .map(|row| {
                (
                    row.get::<_, i64>("id"),
                    row.get::<_, String>("name"),
                    row.get::<_, Option<String>>("note"),
                    row.get::<_, String>("created_at"),
                    row.get::<_, i64>("version"),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                100,
                "alpha3".to_owned(),
                Some("n2".to_owned()),
                "c1".to_owned(),
                3
            ),
            (200, "gamma".to_owned(), None, "c3".to_owned(), 1),
            (9000, "beta".to_owned(), None, "c9".to_owned(), 2),
        ]
    );
}

#[test]
fn executes_composite_upsert_against_pglite() {
    let (_server, mut client) = PgliteServer::start();
    create_mutation_tables(&mut client);
    client
        .batch_execute(
            r#"
            INSERT INTO memberships (tenant_id, user_id, role) VALUES
                (1, 10, 'admin'),
                (1, 20, 'user');
            "#,
        )
        .unwrap();

    let mutation = build_upsert(&membership_options()).unwrap();
    let rows = vec![
        row(&[
            ("tenant_id", Value::Int(1)),
            ("user_id", Value::Int(10)),
            ("role", text("super")),
        ]),
        row(&[
            ("tenant_id", Value::Int(2)),
            ("user_id", Value::Int(30)),
            ("role", text("owner")),
        ]),
    ];
    run_mutation(&mut client, &mutation, &rows);

    let upserted = client
        .query(
            r#"SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        upserted
            .iter()
            .map(|row| {
                (
                    row.get::<_, i64>("tenant_id"),
                    row.get::<_, i64>("user_id"),
                    row.get::<_, String>("role"),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (1, 10, "super".to_owned()),
            (1, 20, "user".to_owned()),
            (2, 30, "owner".to_owned()),
        ]
    );
}
