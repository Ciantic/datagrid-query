use datagrid_query::{MutationOptions, Value, bind_mutation_rows, build_upsert};
mod helpers;
use helpers::{membership_options, mutation_row as row, widget_options};

#[test]
fn builds_upsert_with_conflict_update_and_version_locking() {
    let mutation = build_upsert(&widget_options()).unwrap();
    let params = bind_mutation_rows(
        &mutation,
        &[row(&[
            ("id", Value::Int(1)),
            ("name", Value::Text("a".to_owned())),
            ("version", Value::Int(1)),
        ])],
    )
    .unwrap();

    assert_eq!(
        mutation.sql,
        r#"INSERT INTO "widgets" AS u ("id", "name", "note", "created_at", "updated_at", "version") SELECT CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END FROM unnest($1::bigint[], $2::bool[], $3::text[], $4::text[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::bigint[], $10::bool[]) AS v("id", "id#present", "name", "note", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") ON CONFLICT ("id") DO UPDATE SET "name" = excluded."name", "note" = excluded."note", "updated_at" = excluded."updated_at", "version" = u."version" + 1 WHERE u."version" = excluded."version" RETURNING u."id""#
    );
    assert_eq!(
        params,
        vec![
            Value::Array(vec![Value::Int(1)]),
            Value::Array(vec![Value::Bool(true)]),
            Value::Array(vec![Value::Text("a".to_owned())]),
            Value::Array(vec![Value::Null]),
            Value::Array(vec![Value::Null]),
            Value::Array(vec![Value::Bool(false)]),
            Value::Array(vec![Value::Null]),
            Value::Array(vec![Value::Bool(false)]),
            Value::Array(vec![Value::Int(1)]),
            Value::Array(vec![Value::Bool(true)]),
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
            ("name", Value::Text("alpha".to_owned())),
            ("version", Value::Int(2)),
        ]),
        row(&[("name", Value::Text("beta".to_owned()))]),
    ];
    let params = bind_mutation_rows(&mutation, &rows).unwrap();

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
    let options = membership_options();
    let mutation = build_upsert(&options).unwrap();

    assert!(
        mutation
            .sql
            .contains(r#"ON CONFLICT ("tenant_id", "user_id") DO UPDATE SET"#)
    );
    assert!(!mutation.sql.contains("WHERE u."));
}
