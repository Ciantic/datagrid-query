use datagrid_query_rs::{MutationOptions, QueryError, Value, bind_mutation_rows, build_delete};
mod support;
use support::{
    PgliteServer, bind, create_mutation_tables, membership_options, mutation_column as column,
    mutation_row as row, run_mutation, widget_options,
};

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
fn executes_delete_against_pglite() {
    let (_server, mut client) = PgliteServer::start();
    create_mutation_tables(&mut client);
    client
        .batch_execute(
            r#"
            INSERT INTO widgets (id, name, note, created_at, updated_at, version) VALUES
                (100, 'alpha', NULL, 'c1', 'u1', 2),
                (200, 'beta', NULL, 'c2', 'u2', 1);
            "#,
        )
        .unwrap();

    let stale = build_delete(&widget_options()).unwrap();
    run_mutation(
        &mut client,
        &stale,
        &[row(&[("id", Value::Int(100)), ("version", Value::Int(1))])],
    );

    let after_stale = client
        .query(r#"SELECT "id" FROM "widgets" ORDER BY "id""#, &[])
        .unwrap();
    assert_eq!(support::ids(&after_stale), vec![100, 200]);

    let delete = build_delete(&widget_options()).unwrap();
    run_mutation(
        &mut client,
        &delete,
        &[row(&[("id", Value::Int(200)), ("version", Value::Int(1))])],
    );

    let remaining = client
        .query(r#"SELECT "id" FROM "widgets" ORDER BY "id""#, &[])
        .unwrap();
    assert_eq!(support::ids(&remaining), vec![100]);
}

#[test]
fn executes_composite_delete_against_pglite() {
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

    let mutation = build_delete(&membership_options()).unwrap();
    let rows = vec![row(&[
        ("tenant_id", Value::Int(1)),
        ("user_id", Value::Int(10)),
    ])];
    run_mutation(&mut client, &mutation, &rows);

    let remaining = client
        .query(
            r#"SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        remaining
            .iter()
            .map(|row| {
                (
                    row.get::<_, i64>("tenant_id"),
                    row.get::<_, i64>("user_id"),
                    row.get::<_, String>("role"),
                )
            })
            .collect::<Vec<_>>(),
        vec![(1, 20, "user".to_owned())]
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
