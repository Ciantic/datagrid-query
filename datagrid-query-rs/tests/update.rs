use datagrid_query_rs::{MutationOptions, QueryError, Value, bind_mutation_rows, build_update};
mod support;
use support::{bind, mutation_column as column, mutation_row as row, text, widget_options};

fn array(values: Vec<Value>) -> Value {
    Value::Array(values)
}

#[test]
fn builds_batch_update_with_presence_flags_and_version_locking() {
    let mutation = build_update(&widget_options()).unwrap();
    let params = bind(
        &mutation,
        &[row(&[
            ("id", Value::Int(1)),
            ("version", Value::Int(1)),
            ("name", text("b")),
        ])],
    );

    assert_eq!(
        mutation.sql,
        r#"UPDATE "widgets" AS u SET "name" = CASE WHEN v."name#present" THEN v."name" ELSE u."name" END, "note" = CASE WHEN v."note#present" THEN v."note" ELSE u."note" END, "updated_at" = CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, "version" = u."version" + 1 FROM unnest($1::bigint[], $2::bigint[], $3::text[], $4::bool[], $5::text[], $6::bool[], $7::text[], $8::bool[]) AS v("id", "version", "name", "name#present", "note", "note#present", "updated_at", "updated_at#present") WHERE u."id" = v."id" AND u."version" = v."version" RETURNING u."id""#
    );
    assert_eq!(
        params,
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
fn rejects_missing_update_version() {
    let mutation = build_update(&widget_options()).unwrap();
    assert_eq!(
        bind_mutation_rows(&mutation, &[row(&[("id", Value::Int(1))])]),
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
    };
    let mutation = build_update(&options).unwrap();

    assert_eq!(
        mutation.sql,
        r#"UPDATE "memberships" AS u SET "role" = CASE WHEN v."role#present" THEN v."role" ELSE u."role" END FROM unnest($1::bigint[], $2::bigint[], $3::text[], $4::bool[]) AS v("tenant_id", "user_id", "role", "role#present") WHERE u."tenant_id" = v."tenant_id" AND u."user_id" = v."user_id" RETURNING u."tenant_id", u."user_id""#
    );
}
