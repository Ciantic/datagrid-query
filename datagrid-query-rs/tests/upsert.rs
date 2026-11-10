use datagrid_query_rs::{MutationOptions, Value, build_upsert};
mod support;
use support::{
    PgliteServer, create_mutation_tables, membership_options, mutation_row as row, run, text,
    widget_options,
};

fn array(values: Vec<Value>) -> Value {
    Value::Array(values)
}

#[test]
fn builds_upsert_with_conflict_update_and_version_locking() {
    let options = widget_options(vec![row(&[
        ("id", Value::Int(1)),
        ("name", text("a")),
        ("version", Value::Int(1)),
    ])]);

    let query = build_upsert(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"INSERT INTO "widgets" AS u ("id", "name", "note", "created_at", "updated_at", "version") SELECT CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END FROM unnest($1::bigint[], $2::bool[], $3::text[], $4::text[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::bigint[], $10::bool[]) AS v("id", "id#present", "name", "note", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") ON CONFLICT ("id") DO UPDATE SET "name" = excluded."name", "note" = excluded."note", "updated_at" = excluded."updated_at", "version" = u."version" + 1 WHERE u."version" = excluded."version" RETURNING u."id""#
    );
    assert_eq!(
        query.params,
        vec![
            array(vec![Value::Int(1)]),
            array(vec![Value::Bool(true)]),
            array(vec![text("a")]),
            array(vec![Value::Null]),
            array(vec![Value::Null]),
            array(vec![Value::Bool(false)]),
            array(vec![Value::Null]),
            array(vec![Value::Bool(false)]),
            array(vec![Value::Int(1)]),
            array(vec![Value::Bool(true)]),
        ]
    );
}

#[test]
fn accepts_typescript_upsert_json() {
    let json = include_str!("../../fixtures/upsert-options.json");
    let fixture: serde_json::Value = serde_json::from_str(json).unwrap();
    let options: MutationOptions = serde_json::from_value(fixture.clone()).unwrap();

    assert_eq!(serde_json::to_value(&options).unwrap(), fixture);

    let query = build_upsert(&options).unwrap();
    assert!(query.sql.starts_with(r#"INSERT INTO "widgets" AS u"#));
    assert!(query.sql.contains(r#"ON CONFLICT ("id") DO UPDATE SET"#));
    assert!(
        query
            .sql
            .contains(r#"WHERE u."version" = excluded."version""#)
    );
    assert!(query.sql.contains(r#"RETURNING u."id""#));
    assert_eq!(query.params.len(), 10);
}

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

    let upsert = build_upsert(&widget_options(vec![
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
    ]))
    .unwrap();
    run(&mut client, &upsert);

    let rows = client
        .query(
            r#"SELECT "id", "name", "note", "created_at", "version" FROM "widgets" ORDER BY "id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        rows.iter()
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

    let upsert = build_upsert(&membership_options(vec![
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
    ]))
    .unwrap();
    run(&mut client, &upsert);

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
            (1, 10, "super".to_owned()),
            (1, 20, "user".to_owned()),
            (2, 30, "owner".to_owned()),
        ]
    );
}

#[test]
fn builds_composite_primary_key_upsert() {
    let options = MutationOptions {
        table: "memberships".to_owned(),
        schema: None,
        columns: vec![
            support::mutation_column("tenant_id", "bigint"),
            support::mutation_column("user_id", "bigint"),
            support::mutation_column("role", "text"),
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

    let upsert = build_upsert(&options).unwrap();
    assert!(
        upsert
            .sql
            .contains(r#"ON CONFLICT ("tenant_id", "user_id") DO UPDATE SET"#)
    );
    assert!(!upsert.sql.contains("WHERE u."));
}
