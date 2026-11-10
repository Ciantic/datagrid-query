use datagrid_query::{MutationOptions, Value, bind_mutation_rows, build_merge};
mod helpers;
use helpers::{mutation_row as row, widget_options};

#[test]
fn builds_merge_with_per_field_patch_semantics() {
    let mutation = build_merge(&MutationOptions {
        table: "merge_widgets".to_owned(),
        ..widget_options()
    })
    .unwrap();
    let params = bind_mutation_rows(
        &mutation,
        &[row(&[
            ("id", Value::Int(100)),
            ("version", Value::Int(3)),
            ("name", Value::Text("patched".to_owned())),
        ])],
    )
    .unwrap();

    assert_eq!(
        mutation.sql,
        r#"MERGE INTO "merge_widgets" AS u USING unnest($1::bigint[], $2::bool[], $3::text[], $4::bool[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::text[], $10::bool[], $11::bigint[], $12::bool[]) AS v("id", "id#present", "name", "name#present", "note", "note#present", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") ON u."id" = v."id" WHEN MATCHED AND u."version" = v."version" THEN UPDATE SET "name" = CASE WHEN v."name#present" THEN v."name" ELSE u."name" END, "note" = CASE WHEN v."note#present" THEN v."note" ELSE u."note" END, "updated_at" = CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, "version" = u."version" + 1 WHEN NOT MATCHED THEN INSERT ("id", "name", "note", "created_at", "updated_at", "version") VALUES (CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END) RETURNING u."id""#
    );
    assert_eq!(
        params,
        vec![
            Value::Array(vec![Value::Int(100)]),
            Value::Array(vec![Value::Bool(true)]),
            Value::Array(vec![Value::Text("patched".to_owned())]),
            Value::Array(vec![Value::Bool(true)]),
            Value::Array(vec![Value::Null]),
            Value::Array(vec![Value::Bool(false)]),
            Value::Array(vec![Value::Null]),
            Value::Array(vec![Value::Bool(false)]),
            Value::Array(vec![Value::Null]),
            Value::Array(vec![Value::Bool(false)]),
            Value::Array(vec![Value::Int(3)]),
            Value::Array(vec![Value::Bool(true)]),
        ]
    );
}

#[test]
fn accepts_typescript_merge_json() {
    let json = include_str!("../../fixtures/upsert-options.json");
    let options: MutationOptions = serde_json::from_str(json).unwrap();
    let mutation = build_merge(&options).unwrap();
    let rows = vec![
        row(&[
            ("id", Value::Int(100)),
            ("name", Value::Text("alpha".to_owned())),
            ("version", Value::Int(2)),
        ]),
        row(&[("name", Value::Text("beta".to_owned()))]),
    ];
    bind_mutation_rows(&mutation, &rows).unwrap();

    assert!(mutation.sql.starts_with(r#"MERGE INTO "widgets" AS u"#));
    assert!(
        mutation
            .sql
            .contains(r#""name" = CASE WHEN v."name#present" THEN v."name" ELSE u."name" END"#)
    );
}
