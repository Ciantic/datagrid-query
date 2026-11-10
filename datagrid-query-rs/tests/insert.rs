use datagrid_query_rs::{
    BuiltMutation, MutationOptions, QueryError, Value, bind_mutation_rows, build_insert,
};
mod support;
use support::{bind, mutation_row as row, text, widget_options};

fn array(values: Vec<Value>) -> Value {
    Value::Array(values)
}

#[test]
fn builds_batch_insert_with_defaults_and_presence_flags() {
    let mutation = build_insert(&widget_options()).unwrap();
    let params = bind(
        &mutation,
        &[row(&[("id", Value::Int(1)), ("name", text("a"))])],
    );

    assert_eq!(
        mutation.sql,
        r#"INSERT INTO "widgets" ("id", "name", "note", "created_at", "updated_at", "version") SELECT CASE WHEN v."id#present" THEN v."id" ELSE 9000 END, v."name", v."note", CASE WHEN v."created_at#present" THEN v."created_at" ELSE now()::text END, CASE WHEN v."updated_at#present" THEN v."updated_at" ELSE now()::text END, CASE WHEN v."version#present" THEN v."version" ELSE 1 END FROM unnest($1::bigint[], $2::bool[], $3::text[], $4::text[], $5::text[], $6::bool[], $7::text[], $8::bool[], $9::bigint[], $10::bool[]) AS v("id", "id#present", "name", "note", "created_at", "created_at#present", "updated_at", "updated_at#present", "version", "version#present") RETURNING "widgets"."id""#
    );
    assert_eq!(
        params,
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
      "versionColumn": "version"
    }
    "#;

    let options: MutationOptions = serde_json::from_str(json).unwrap();

    assert_eq!(
        options.columns[0].insert_default_sql.as_deref(),
        Some("9000")
    );
    assert_eq!(options.primary_key, vec!["id"]);

    let mutation = build_insert(&options).unwrap();
    assert!(mutation.sql.contains(r#""id#present""#));
    assert_eq!(mutation.param_specs.len(), 5);
}

#[test]
fn rejects_invalid_mutation_options() {
    let mut options = widget_options();
    options.primary_key.clear();
    assert_eq!(build_insert(&options), Err(QueryError::EmptyPrimaryKey));

    let mut options = widget_options();
    options.columns[0].data_type = "uuid; DROP TABLE widgets".to_owned();
    assert!(matches!(
        build_insert(&options),
        Err(QueryError::InvalidType { .. })
    ));

    let mut options = widget_options();
    options.version_column = Some("id".to_owned());
    assert_eq!(
        build_insert(&options),
        Err(QueryError::VersionColumnInPrimaryKey {
            column: "id".to_owned(),
        })
    );

    let mut options = widget_options();
    options.columns[1].name = "id".to_owned();
    assert_eq!(
        build_insert(&options),
        Err(QueryError::DuplicateColumn {
            column: "id".to_owned(),
        })
    );
}

#[test]
fn serializes_built_mutation_parameter_specs() {
    let mutation = build_insert(&widget_options()).unwrap();
    let json = serde_json::to_value(&mutation).unwrap();

    assert_eq!(json["columns"][0], "id");
    assert_eq!(json["paramSpecs"][0]["kind"], "value");
    assert_eq!(json["paramSpecs"][0]["column"], "id");
    assert_eq!(json["paramSpecs"][1]["kind"], "present");

    let round_trip: BuiltMutation = serde_json::from_value(json).unwrap();
    assert_eq!(round_trip, mutation);
}

#[test]
fn rejects_unknown_row_columns() {
    let mutation = build_insert(&widget_options()).unwrap();
    assert_eq!(
        bind_mutation_rows(&mutation, &[row(&[("unknown", Value::Int(1))])]),
        Err(QueryError::UnknownColumn {
            column: "unknown".to_owned(),
        })
    );
}
