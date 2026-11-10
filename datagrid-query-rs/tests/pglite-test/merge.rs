use datagrid_query_rs::{Value, build_merge};

mod support;
use support::{
    PgliteServer, create_mutation_tables, membership_options, merge_widget_options,
    mutation_row as row, run_mutation, text,
};

#[test]
fn executes_merge_against_pglite() {
    let (_server, mut client) = PgliteServer::start();
    create_mutation_tables(&mut client);
    client
        .batch_execute(
            r#"
            INSERT INTO merge_widgets (id, name, note, created_at, updated_at, version)
            VALUES (100, 'alpha', 'old-note', 'c0', 'u0', 3);
            "#,
        )
        .unwrap();

    let mutation = build_merge(&merge_widget_options()).unwrap();
    let rows = vec![
        row(&[
            ("id", Value::Int(100)),
            ("version", Value::Int(3)),
            ("name", text("patched")),
            ("created_at", text("ignored")),
        ]),
        row(&[
            ("id", Value::Int(200)),
            ("name", text("new")),
            ("note", text("new-note")),
            ("updated_at", text("u2")),
            ("version", Value::Int(1)),
        ]),
        row(&[
            ("id", Value::Int(100)),
            ("version", Value::Int(2)),
            ("name", text("stale")),
            ("created_at", text("ignored2")),
        ]),
    ];
    run_mutation(&mut client, &mutation, &rows);

    let merged = client
        .query(
            r#"SELECT "id", "name", "note", "version" FROM "merge_widgets" ORDER BY "id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        merged
            .iter()
            .map(|row| {
                (
                    row.get::<_, i64>("id"),
                    row.get::<_, String>("name"),
                    row.get::<_, Option<String>>("note"),
                    row.get::<_, i64>("version"),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (100, "patched".to_owned(), Some("old-note".to_owned()), 4),
            (200, "new".to_owned(), Some("new-note".to_owned()), 1),
        ]
    );

    let timestamps = client
        .query(
            r#"SELECT "id", "created_at", "updated_at" FROM "merge_widgets" ORDER BY "id""#,
            &[],
        )
        .unwrap();
    assert_eq!(timestamps[0].get::<_, String>("created_at"), "c0");
    assert_ne!(timestamps[0].get::<_, String>("updated_at"), "u0");
    assert_eq!(timestamps[1].get::<_, String>("updated_at"), "u2");

    let second = build_merge(&merge_widget_options()).unwrap();
    let second_rows = vec![
        row(&[
            ("id", Value::Int(200)),
            ("version", Value::Int(1)),
            ("note", Value::Null),
        ]),
        row(&[
            ("id", Value::Int(300)),
            ("name", text("new3")),
            ("note", text("n3")),
            ("created_at", text("c3")),
            ("updated_at", text("u3")),
            ("version", Value::Int(1)),
        ]),
    ];
    run_mutation(&mut client, &second, &second_rows);

    let patched = client
        .query(
            r#"SELECT "id", "name", "note", "version" FROM "merge_widgets" ORDER BY "id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        patched
            .iter()
            .map(|row| {
                (
                    row.get::<_, i64>("id"),
                    row.get::<_, String>("name"),
                    row.get::<_, Option<String>>("note"),
                    row.get::<_, i64>("version"),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (100, "patched".to_owned(), Some("old-note".to_owned()), 4),
            (200, "new".to_owned(), None, 2),
            (300, "new3".to_owned(), Some("n3".to_owned()), 1),
        ]
    );
}

#[test]
fn executes_composite_merge_against_pglite() {
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

    let mutation = build_merge(&membership_options()).unwrap();
    let rows = vec![
        row(&[
            ("tenant_id", Value::Int(1)),
            ("user_id", Value::Int(20)),
            ("role", text("manager")),
        ]),
        row(&[
            ("tenant_id", Value::Int(2)),
            ("user_id", Value::Int(30)),
            ("role", text("owner")),
        ]),
    ];
    run_mutation(&mut client, &mutation, &rows);

    let merged = client
        .query(
            r#"SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        merged
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
            (1, 10, "admin".to_owned()),
            (1, 20, "manager".to_owned()),
            (2, 30, "owner".to_owned()),
        ]
    );
}
