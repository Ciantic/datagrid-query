use datagrid_query_rs::{MutationOptions, Value, build_upsert};
mod support;
use support::{bind, mutation_column as column, mutation_row as row, text, widget_options};

fn array(values: Vec<Value>) -> Value {
    Value::Array(values)
}

#[test]
fn builds_upsert_with_conflict_update_and_version_locking() {
    let mutation = build_upsert(&widget_options()).unwrap();
    let params = bind(
        &mutation,
        &[row(&[
            ("id", Value::Int(1)),
            ("name", text("a")),
            ("version", Value::Int(1)),
        ])],
    );

    assert_eq!(
        mutation.sql,
        r#"INSERT INTO "widgets" AS u ("id", "name", "note", "created_at", "updated_at", "version") SELECT CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END FROM unnest($1::bigint[], $2::bool[], $3::text[], $4::text[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::bigint[], $10::bool[]) AS v("id", "id#present", "name", "note", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") ON CONFLICT ("id") DO UPDATE SET "name" = excluded."name", "note" = excluded."note", "updated_at" = excluded."updated_at", "version" = u."version" + 1 WHERE u."version" = excluded."version" RETURNING u."id""#
    );
    assert_eq!(
        params,
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

    let mutation = build_upsert(&options).unwrap();
    let rows = vec![
        row(&[
            ("id", Value::Int(100)),
            ("name", text("alpha")),
            ("version", Value::Int(2)),
        ]),
        row(&[("name", text("beta"))]),
    ];
    let params = bind(&mutation, &rows);

    assert!(mutation.sql.starts_with(r#"INSERT INTO "widgets" AS u"#));
    assert!(mutation.sql.contains(r#"ON CONFLICT ("id") DO UPDATE SET"#));
    assert!(
        mutation
            .sql
            .contains(r#"WHERE u."version" = excluded."version""#)
    );
    assert!(mutation.sql.contains(r#"RETURNING u."id""#));
    assert_eq!(params.len(), 10);
}

#[test]
fn builds_composite_primary_key_upsert() {
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
    };
    let mutation = build_upsert(&options).unwrap();

    assert!(
        mutation
            .sql
            .contains(r#"ON CONFLICT ("tenant_id", "user_id") DO UPDATE SET"#)
    );
    assert!(!mutation.sql.contains("WHERE u."));
}
