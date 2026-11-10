use datagrid_query_rs::{MutationOptions, QueryError, Value, build_insert};
mod support;
use support::{
    PgliteServer, create_mutation_tables, membership_options, mutation_row as row, run, text,
    widget_options,
};

fn array(values: Vec<Value>) -> Value {
    Value::Array(values)
}

#[test]
fn builds_batch_insert_with_defaults_and_presence_flags() {
    let options = widget_options(vec![row(&[("id", Value::Int(1)), ("name", text("a"))])]);

    let query = build_insert(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"INSERT INTO "widgets" ("id", "name", "note", "created_at", "updated_at", "version") SELECT CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END FROM unnest($1::bigint[], $2::bool[], $3::text[], $4::text[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::bigint[], $10::bool[]) AS v("id", "id#present", "name", "note", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") RETURNING "widgets"."id""#
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
            array(vec![Value::Null]),
            array(vec![Value::Bool(false)]),
        ]
    );
}

#[test]
fn deserializes_typescript_mutation_options() {
    let json = r#"
    {
      "table": "widgets",
      "columns": [
        {"name": "id", "type": "bigint", "insertDefaultSql": "9000"},
        {"name": "name", "type": "text"},
        {"name": "version", "type": "bigint", "insertDefaultSql": "1"}
      ],
      "primaryKey": ["id"],
      "versionColumn": "version",
      "rows": [
        {"id": 1, "name": "a", "version": 1},
        {"name": "b"}
      ]
    }
    "#;

    let options: MutationOptions = serde_json::from_str(json).unwrap();

    assert_eq!(
        options.columns[0].insert_default_sql.as_deref(),
        Some("9000")
    );
    assert_eq!(options.primary_key, vec!["id"]);
    assert_eq!(options.rows[0].get("id"), Some(&Value::Int(1)));
    assert_eq!(options.rows[1].get("id"), None);

    let query = build_insert(&options).unwrap();
    assert!(query.sql.contains(r#""id#present""#));
    assert_eq!(query.params.len(), 5);
}

#[test]
fn rejects_invalid_mutation_options() {
    let mut options = widget_options(vec![row(&[("id", Value::Int(1))])]);
    options.primary_key.clear();
    assert_eq!(build_insert(&options), Err(QueryError::EmptyPrimaryKey));

    let mut options = widget_options(vec![row(&[("id", Value::Int(1))])]);
    options.columns[0].data_type = "uuid; DROP TABLE widgets".to_owned();
    assert!(matches!(
        build_insert(&options),
        Err(QueryError::InvalidType { .. })
    ));

    let options = widget_options(vec![row(&[("unknown", Value::Int(1))])]);
    assert_eq!(
        build_insert(&options),
        Err(QueryError::UnknownColumn {
            column: "unknown".to_owned(),
        })
    );

    let mut options = widget_options(vec![row(&[("id", Value::Int(1))])]);
    options.version_column = Some("id".to_owned());
    assert_eq!(
        build_insert(&options),
        Err(QueryError::VersionColumnInPrimaryKey {
            column: "id".to_owned(),
        })
    );

    let mut options = widget_options(vec![row(&[("id", Value::Int(1))])]);
    options.columns[1].name = "id".to_owned();
    assert_eq!(
        build_insert(&options),
        Err(QueryError::DuplicateColumn {
            column: "id".to_owned(),
        })
    );
}

#[test]
fn executes_insert_against_pglite() {
    let (_server, mut client) = PgliteServer::start();
    create_mutation_tables(&mut client);

    let insert = build_insert(&widget_options(vec![
        row(&[
            ("id", Value::Int(100)),
            ("name", text("alpha")),
            ("note", text("n0")),
            ("created_at", text("c1")),
            ("updated_at", text("u1")),
            ("version", Value::Int(1)),
        ]),
        row(&[("name", text("beta"))]),
    ]))
    .unwrap();
    run(&mut client, &insert);

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

    let insert = build_insert(&membership_options(vec![
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
    ]))
    .unwrap();
    run(&mut client, &insert);

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
        vec![(1, 10, "admin".to_owned()), (1, 20, "user".to_owned())]
    );
}
