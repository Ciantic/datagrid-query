use datagrid_query::{Value, build_insert};

#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::{
    PgliteServer, create_mutation_tables, membership_options, mutation_row as row, run_mutation,
    widget_options,
};

#[test]
fn executes_insert_against_pglite() {
    let (_server, mut client) = PgliteServer::start();
    create_mutation_tables(&mut client);

    let mutation = build_insert(&widget_options()).unwrap();
    let rows = vec![
        row(&[
            ("id", Value::Int(100)),
            ("name", Value::Text("alpha".to_owned())),
            ("note", Value::Text("n0".to_owned())),
            ("created_at", Value::Text("c1".to_owned())),
            ("updated_at", Value::Text("u1".to_owned())),
            ("version", Value::Int(1)),
        ]),
        row(&[("name", Value::Text("beta".to_owned()))]),
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
            ("role", Value::Text("admin".to_owned())),
        ]),
        row(&[
            ("tenant_id", Value::Int(1)),
            ("user_id", Value::Int(20)),
            ("role", Value::Text("user".to_owned())),
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
