use datagrid_query_rs::{Value, build_update};

mod support;
use support::{
    PgliteServer, create_mutation_tables, membership_options, mutation_row as row, run_mutation,
    text, widget_options,
};

#[test]
fn executes_update_against_pglite() {
    let (_server, mut client) = PgliteServer::start();
    create_mutation_tables(&mut client);
    client
        .batch_execute(
            r#"
            INSERT INTO widgets (id, name, note, created_at, updated_at, version) VALUES
                (100, 'alpha', 'old-note', 'c1', 'u1', 1),
                (9000, 'beta', NULL, 'c2', 'u2', 1);
            "#,
        )
        .unwrap();

    let mutation = build_update(&widget_options()).unwrap();
    let rows = vec![
        row(&[
            ("id", Value::Int(100)),
            ("version", Value::Int(1)),
            ("name", text("alpha2")),
            ("note", text("n1")),
            ("updated_at", text("u2")),
        ]),
        row(&[
            ("id", Value::Int(9000)),
            ("version", Value::Int(1)),
            ("name", text("beta2")),
        ]),
    ];
    run_mutation(&mut client, &mutation, &rows);

    let updated = client
        .query(
            r#"SELECT "id", "name", "note", "version" FROM "widgets" ORDER BY "id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        updated
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
            (100, "alpha2".to_owned(), Some("n1".to_owned()), 2),
            (9000, "beta2".to_owned(), None, 2),
        ]
    );

    let clear_note = build_update(&widget_options()).unwrap();
    let clear_rows = vec![row(&[
        ("id", Value::Int(100)),
        ("version", Value::Int(2)),
        ("note", Value::Null),
    ])];
    run_mutation(&mut client, &clear_note, &clear_rows);

    let cleared = client
        .query(
            r#"SELECT "name", "note", "version" FROM "widgets" WHERE "id" = 100"#,
            &[],
        )
        .unwrap();
    assert_eq!(cleared[0].get::<_, String>("name"), "alpha2");
    assert_eq!(cleared[0].get::<_, Option<String>>("note"), None);
    assert_eq!(cleared[0].get::<_, i64>("version"), 3);

    let stale = build_update(&widget_options()).unwrap();
    let stale_rows = vec![row(&[
        ("id", Value::Int(9000)),
        ("version", Value::Int(1)),
        ("name", text("stale")),
    ])];
    run_mutation(&mut client, &stale, &stale_rows);

    let unchanged = client
        .query(
            r#"SELECT "name", "version" FROM "widgets" WHERE "id" = 9000"#,
            &[],
        )
        .unwrap();
    assert_eq!(unchanged[0].get::<_, String>("name"), "beta2");
    assert_eq!(unchanged[0].get::<_, i64>("version"), 2);
}

#[test]
fn executes_composite_update_against_pglite() {
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

    let mutation = build_update(&membership_options()).unwrap();
    let rows = vec![row(&[
        ("tenant_id", Value::Int(1)),
        ("user_id", Value::Int(20)),
        ("role", text("manager")),
    ])];
    run_mutation(&mut client, &mutation, &rows);

    let updated = client
        .query(
            r#"SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        updated
            .iter()
            .map(|row| {
                (
                    row.get::<_, i64>("tenant_id"),
                    row.get::<_, i64>("user_id"),
                    row.get::<_, String>("role"),
                )
            })
            .collect::<Vec<_>>(),
        vec![(1, 10, "admin".to_owned()), (1, 20, "manager".to_owned())]
    );
}
