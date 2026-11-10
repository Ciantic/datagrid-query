use datagrid_query_rs::{
    MutationColumn, MutationOptions, MutationRow, QueryError, Value, build_delete, build_insert,
    build_update, build_upsert,
};
use std::collections::BTreeMap;

fn column(name: &str, data_type: &str) -> MutationColumn {
    MutationColumn {
        name: name.to_owned(),
        data_type: data_type.to_owned(),
        ..Default::default()
    }
}

fn widget_columns() -> Vec<MutationColumn> {
    vec![
        MutationColumn {
            insert_default_sql: Some("9000".to_owned()),
            ..column("id", "bigint")
        },
        column("name", "text"),
        column("note", "text"),
        MutationColumn {
            insert_default_sql: Some("now()::text".to_owned()),
            updatable: false,
            ..column("created_at", "text")
        },
        MutationColumn {
            insert_default_sql: Some("now()::text".to_owned()),
            update_default_sql: Some("now()::text".to_owned()),
            ..column("updated_at", "text")
        },
        MutationColumn {
            insert_default_sql: Some("1".to_owned()),
            ..column("version", "bigint")
        },
    ]
}

fn widget_options(rows: Vec<MutationRow>) -> MutationOptions {
    MutationOptions {
        table: "widgets".to_owned(),
        schema: None,
        columns: widget_columns(),
        primary_key: vec!["id".to_owned()],
        version_column: Some("version".to_owned()),
        returning: None,
        rows,
    }
}

fn row(entries: &[(&str, Value)]) -> MutationRow {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect::<BTreeMap<_, _>>()
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

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
fn builds_batch_update_with_presence_flags_and_version_locking() {
    let options = widget_options(vec![row(&[
        ("id", Value::Int(1)),
        ("version", Value::Int(1)),
        ("name", text("b")),
    ])]);

    let query = build_update(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"UPDATE "widgets" AS u SET "name" = CASE WHEN v."name#present" THEN v."name" ELSE u."name" END, "note" = CASE WHEN v."note#present" THEN v."note" ELSE u."note" END, "updated_at" = CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, "version" = u."version" + 1 FROM unnest($1::bigint[], $2::bigint[], $3::text[], $4::bool[], $5::text[], $6::bool[], $7::text[], $8::bool[]) AS v("id", "version", "name", "name#present", "note", "note#present", "updated_at", "updated_at#present") WHERE u."id" = v."id" AND u."version" = v."version" RETURNING u."id""#
    );
    assert_eq!(
        query.params,
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
fn builds_upsert_with_conflict_update_and_version_locking() {
    let options = widget_options(vec![row(&[
        ("id", Value::Int(1)),
        ("name", text("a")),
        ("version", Value::Int(1)),
    ])]);

    let query = build_upsert(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"INSERT INTO "widgets" AS u ("id", "name", "note", "created_at", "updated_at", "version") SELECT CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END FROM unnest($1::bigint[], $2::bool[], $3::text[], $4::text[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::bigint[], $10::bool[]) AS v("id", "id#present", "name", "note", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") ON CONFLICT ("id") DO UPDATE SET "name" = excluded."name", "note" = excluded."note", "updated_at" = excluded."updated_at", "version" = u."version" + 1 WHERE u."version" = excluded."version" RETURNING u."id""#
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
            array(vec![Value::Int(1)]),
            array(vec![Value::Bool(true)]),
        ]
    );
}

#[test]
fn builds_batch_delete_with_version_locking() {
    let options = widget_options(vec![row(&[
        ("id", Value::Int(1)),
        ("version", Value::Int(1)),
    ])]);

    let query = build_delete(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"DELETE FROM "widgets" AS u USING unnest($1::bigint[], $2::bigint[]) AS v("id", "version") WHERE u."id" = v."id" AND u."version" = v."version" RETURNING u."id""#
    );
    assert_eq!(
        query.params,
        vec![array(vec![Value::Int(1)]), array(vec![Value::Int(1)]),]
    );
}

#[test]
fn supports_composite_primary_keys_without_version_column() {
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
        rows: vec![row(&[
            ("tenant_id", Value::Int(1)),
            ("user_id", Value::Int(10)),
            ("role", text("admin")),
        ])],
    };

    let insert = build_insert(&options).unwrap();
    assert_eq!(
        insert.sql,
        r#"INSERT INTO "memberships" ("tenant_id", "user_id", "role") SELECT v."tenant_id", v."user_id", v."role" FROM unnest($1::bigint[], $2::bigint[], $3::text[]) AS v("tenant_id", "user_id", "role") RETURNING "memberships"."tenant_id", "memberships"."user_id""#
    );

    let update = build_update(&options).unwrap();
    assert_eq!(
        update.sql,
        r#"UPDATE "memberships" AS u SET "role" = CASE WHEN v."role#present" THEN v."role" ELSE u."role" END FROM unnest($1::bigint[], $2::bigint[], $3::text[], $4::bool[]) AS v("tenant_id", "user_id", "role", "role#present") WHERE u."tenant_id" = v."tenant_id" AND u."user_id" = v."user_id" RETURNING u."tenant_id", u."user_id""#
    );

    let upsert = build_upsert(&options).unwrap();
    assert!(
        upsert
            .sql
            .contains(r#"ON CONFLICT ("tenant_id", "user_id") DO UPDATE SET"#)
    );
    assert!(!upsert.sql.contains("WHERE u."));

    let delete = build_delete(&options).unwrap();
    assert_eq!(
        delete.sql,
        r#"DELETE FROM "memberships" AS u USING unnest($1::bigint[], $2::bigint[]) AS v("tenant_id", "user_id") WHERE u."tenant_id" = v."tenant_id" AND u."user_id" = v."user_id" RETURNING u."tenant_id", u."user_id""#
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
fn upsert_options_are_interchangeable_with_typescript_json() {
    let json = include_str!("../../fixtures/upsert-options.json");
    let fixture: serde_json::Value = serde_json::from_str(json).unwrap();
    let options: MutationOptions = serde_json::from_value(fixture.clone()).unwrap();

    assert_eq!(serde_json::to_value(&options).unwrap(), fixture);

    let query = build_upsert(&options).unwrap();

    assert!(query.sql.starts_with(r#"INSERT INTO "widgets" AS u"#));
    assert!(query.sql.contains(r#"ON CONFLICT ("id") DO UPDATE SET"#));
    assert!(
        query
            .sql
            .contains(r#"WHERE u."version" = excluded."version""#)
    );
    assert!(query.sql.contains(r#"RETURNING u."id""#));
    assert!(
        !query
            .sql
            .contains(r#""created_at" = excluded."created_at""#)
    );
    assert_eq!(query.params.len(), 10);
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
fn rejects_missing_update_keys_and_versions() {
    let options = widget_options(vec![row(&[("id", Value::Int(1))])]);
    assert_eq!(
        build_update(&options),
        Err(QueryError::MissingRowValue {
            column: "version".to_owned(),
        })
    );

    let options = widget_options(vec![row(&[("version", Value::Int(1))])]);
    assert_eq!(
        build_delete(&options),
        Err(QueryError::MissingRowValue {
            column: "id".to_owned(),
        })
    );
}
