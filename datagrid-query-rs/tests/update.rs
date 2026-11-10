use datagrid_query_rs::{MutationOptions, QueryError, Value, build_update};
mod support;
use support::{
    PgliteServer, create_mutation_tables, membership_options, mutation_column as column,
    mutation_row as row, run, text, widget_options,
};

fn array(values: Vec<Value>) -> Value {
    Value::Array(values)
}

#[test]
fn builds_batch_update_with_presence_flags_and_version_locking() {
    let options = widget_options(vec![row(&[
        ("id", Value::Int(1)),
        ("version", Value::Int(1)),
        ("name", text("b")),
    ])]);

    let query = build_update(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"UPDATE "widgets" AS u SET "name" = CASE WHEN v."name#present" THEN v."name" ELSE u."name" END, "note" = CASE WHEN v."note#present" THEN v."note" ELSE u."note" END, "updated_at" = CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, "version" = u."version" + 1 FROM unnest($1::bigint[], $2::bigint[], $3::text[], $4::bool[], $5::text[], $6::bool[], $7::text[], $8::bool[]) AS v("id", "version", "name", "name#present", "note", "note#present", "updated_at", "updated_at#present") WHERE u."id" = v."id" AND u."version" = v."version" RETURNING u."id""#
    );
    assert_eq!(
        query.params,
        vec![
            array(vec![Value::Int(1)]),
            array(vec![Value::Int(1)]),
            array(vec![text("b")]),
            array(vec![Value::Bool(true)]),
            array(vec![Value::Null]),
            array(vec![Value::Bool(false)]),
            array(vec![Value::Null]),
            array(vec![Value::Bool(false)]),
        ]
    );
}

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

    let update = build_update(&widget_options(vec![
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
    ]))
    .unwrap();
    run(&mut client, &update);

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

    let clear_note = build_update(&widget_options(vec![row(&[
        ("id", Value::Int(100)),
        ("version", Value::Int(2)),
        ("note", Value::Null),
    ])]))
    .unwrap();
    run(&mut client, &clear_note);

    let cleared = client
        .query(
            r#"SELECT "name", "note", "version" FROM "widgets" WHERE "id" = 100"#,
            &[],
        )
        .unwrap();
    assert_eq!(cleared[0].get::<_, String>("name"), "alpha2");
    assert_eq!(cleared[0].get::<_, Option<String>>("note"), None);
    assert_eq!(cleared[0].get::<_, i64>("version"), 3);

    let stale = build_update(&widget_options(vec![row(&[
        ("id", Value::Int(9000)),
        ("version", Value::Int(1)),
        ("name", text("stale")),
    ])]))
    .unwrap();
    run(&mut client, &stale);

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

    let update = build_update(&membership_options(vec![row(&[
        ("tenant_id", Value::Int(1)),
        ("user_id", Value::Int(20)),
        ("role", text("manager")),
    ])]))
    .unwrap();
    run(&mut client, &update);

    let rows = client
        .query(
            r#"SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        rows.iter()
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

#[test]
fn rejects_missing_update_version() {
    let options = widget_options(vec![row(&[("id", Value::Int(1))])]);
    assert_eq!(
        build_update(&options),
        Err(QueryError::MissingRowValue {
            column: "version".to_owned(),
        })
    );
}

#[test]
fn builds_composite_primary_key_update() {
    let options = MutationOptions {
        table: "memberships".to_owned(),
        schema: None,
        columns: vec![
            column("tenant_id", "bigint"),
            column("user_id", "bigint"),
            column("role", "text"),
        ],
        primary_key: vec!["tenant_id".to_owned(), "user_id".to_owned()],
        version_column: None,
        returning: None,
        rows: vec![row(&[
            ("tenant_id", Value::Int(1)),
            ("user_id", Value::Int(10)),
            ("role", text("admin")),
        ])],
    };

    let update = build_update(&options).unwrap();
    assert_eq!(
        update.sql,
        r#"UPDATE "memberships" AS u SET "role" = CASE WHEN v."role#present" THEN v."role" ELSE u."role" END FROM unnest($1::bigint[], $2::bigint[], $3::text[], $4::bool[]) AS v("tenant_id", "user_id", "role", "role#present") WHERE u."tenant_id" = v."tenant_id" AND u."user_id" = v."user_id" RETURNING u."tenant_id", u."user_id""#
    );
}
