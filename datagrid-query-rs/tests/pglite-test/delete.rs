use datagrid_query_rs::{Value, build_delete};

mod support;
use support::{
    PgliteServer, create_mutation_tables, membership_options, mutation_row as row, run_mutation,
    widget_options,
};

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
