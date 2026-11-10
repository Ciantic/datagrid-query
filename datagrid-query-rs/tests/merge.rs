use datagrid_query_rs::{MutationOptions, Value, build_merge};
mod support;
use support::{
    PgliteServer, create_mutation_tables, membership_options, merge_widget_options,
    mutation_row as row, run, text, widget_options,
};

fn array(values: Vec<Value>) -> Value {
    Value::Array(values)
}

#[test]
fn builds_merge_with_per_field_patch_semantics() {
    let options = widget_options(vec![row(&[
        ("id", Value::Int(100)),
        ("version", Value::Int(3)),
        ("name", text("patched")),
    ])]);

    let query = build_merge(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"MERGE INTO "widgets" AS u USING unnest($1::bigint[], $2::bool[], $3::text[], $4::bool[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::text[], $10::bool[], $11::bigint[], $12::bool[]) AS v("id", "id#present", "name", "name#present", "note", "note#present", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") ON u."id" = v."id" WHEN MATCHED AND u."version" = v."version" THEN UPDATE SET "name" = CASE WHEN v."name#present" THEN v."name" ELSE u."name" END, "note" = CASE WHEN v."note#present" THEN v."note" ELSE u."note" END, "updated_at" = CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, "version" = u."version" + 1 WHEN NOT MATCHED THEN INSERT ("id", "name", "note", "created_at", "updated_at", "version") VALUES (CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END) RETURNING u."id""#
    );
    assert_eq!(
        query.params,
        vec![
            array(vec![Value::Int(100)]),
            array(vec![Value::Bool(true)]),
            array(vec![text("patched")]),
            array(vec![Value::Bool(true)]),
            array(vec![Value::Null]),
            array(vec![Value::Bool(false)]),
            array(vec![Value::Null]),
            array(vec![Value::Bool(false)]),
            array(vec![Value::Null]),
            array(vec![Value::Bool(false)]),
            array(vec![Value::Int(3)]),
            array(vec![Value::Bool(true)]),
        ]
    );
}

#[test]
fn accepts_typescript_merge_json() {
    let json = include_str!("../../fixtures/upsert-options.json");
    let options: MutationOptions = serde_json::from_str(json).unwrap();
    let query = build_merge(&options).unwrap();

    assert!(query.sql.starts_with(r#"MERGE INTO "widgets" AS u"#));
    assert!(
        query
            .sql
            .contains(r#""name" = CASE WHEN v."name#present" THEN v."name" ELSE u."name" END"#)
    );
}

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

    let merge = build_merge(&merge_widget_options(vec![
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
    ]))
    .unwrap();
    run(&mut client, &merge);

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

    let second = build_merge(&merge_widget_options(vec![
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
    ]))
    .unwrap();
    run(&mut client, &second);

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

    let merge = build_merge(&membership_options(vec![
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
    ]))
    .unwrap();
    run(&mut client, &merge);

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
        vec![
            (1, 10, "admin".to_owned()),
            (1, 20, "manager".to_owned()),
            (2, 30, "owner".to_owned()),
        ]
    );
}
