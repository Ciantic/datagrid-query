use datagrid_query_rs::{Value, build_insert};

mod support;
use support::{
    PgliteServer, create_mutation_tables, membership_options, mutation_row as row, run_mutation,
    text, widget_options,
};

#[test]
fn executes_insert_against_pglite() {
    let (_server, mut client) = PgliteServer::start();
    create_mutation_tables(&mut client);

    let mutation = build_insert(&widget_options()).unwrap();
    let rows = vec![
        row(&[
            ("id", Value::Int(100)),
            ("name", text("alpha")),
            ("note", text("n0")),
            ("created_at", text("c1")),
            ("updated_at", text("u1")),
            ("version", Value::Int(1)),
        ]),
        row(&[("name", text("beta"))]),
    ];
    run_mutation(&mut client, &mutation, &rows);

    let inserted = client
        .query(
            r#"SELECT "id", "name", "version" FROM "widgets" ORDER BY "id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        inserted
            .iter()
            .map(|row| {
                (
                    row.get::<_, i64>("id"),
                    row.get::<_, String>("name"),
                    row.get::<_, i64>("version"),
                )
            })
            .collect::<Vec<_>>(),
        vec![(100, "alpha".to_owned(), 1), (9000, "beta".to_owned(), 1)]
    );
}

#[test]
fn executes_composite_insert_against_pglite() {
    let (_server, mut client) = PgliteServer::start();
    create_mutation_tables(&mut client);

    let mutation = build_insert(&membership_options()).unwrap();
    let rows = vec![
        row(&[
            ("tenant_id", Value::Int(1)),
            ("user_id", Value::Int(10)),
            ("role", text("admin")),
        ]),
        row(&[
            ("tenant_id", Value::Int(1)),
            ("user_id", Value::Int(20)),
            ("role", text("user")),
        ]),
    ];
    run_mutation(&mut client, &mutation, &rows);

    let inserted = client
        .query(
            r#"SELECT "tenant_id", "user_id", "role" FROM "memberships" ORDER BY "tenant_id", "user_id""#,
            &[],
        )
        .unwrap();
    assert_eq!(
        inserted
            .iter()
            .map(|row| {
                (
                    row.get::<_, i64>("tenant_id"),
                    row.get::<_, i64>("user_id"),
                    row.get::<_, String>("role"),
                )
            })
            .collect::<Vec<_>>(),
        vec![(1, 10, "admin".to_owned()), (1, 20, "user".to_owned())]
    );
}
