use datagrid_query_rs::{MutationOptions, QueryError, Value, bind_mutation_rows, build_delete};
mod support;
use support::{bind, mutation_column as column, mutation_row as row, widget_options};

fn array(values: Vec<Value>) -> Value {
    Value::Array(values)
}

#[test]
fn builds_batch_delete_with_version_locking() {
    let mutation = build_delete(&widget_options()).unwrap();
    let params = bind(
        &mutation,
        &[row(&[("id", Value::Int(1)), ("version", Value::Int(1))])],
    );

    assert_eq!(
        mutation.sql,
        r#"DELETE FROM "widgets" AS u USING unnest($1::bigint[], $2::bigint[]) AS v("id", "version") WHERE u."id" = v."id" AND u."version" = v."version" RETURNING u."id""#
    );
    assert_eq!(
        params,
        vec![array(vec![Value::Int(1)]), array(vec![Value::Int(1)]),]
    );
}

#[test]
fn rejects_missing_delete_key() {
    let mutation = build_delete(&widget_options()).unwrap();
    assert_eq!(
        bind_mutation_rows(&mutation, &[row(&[("version", Value::Int(1))])]),
        Err(QueryError::MissingRowValue {
            column: "id".to_owned(),
        })
    );
}

#[test]
fn builds_composite_primary_key_delete() {
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
    let mutation = build_delete(&options).unwrap();

    assert_eq!(
        mutation.sql,
        r#"DELETE FROM "memberships" AS u USING unnest($1::bigint[], $2::bigint[]) AS v("tenant_id", "user_id") WHERE u."tenant_id" = v."tenant_id" AND u."user_id" = v."user_id" RETURNING u."tenant_id", u."user_id""#
    );
}
