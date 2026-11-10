#![allow(dead_code)]

use datagrid_query::{
    BuiltMutation, MutationColumn, MutationOptions, MutationRow, Value, bind_mutation_rows,
};
use std::collections::BTreeMap;

pub(crate) fn bind(mutation: &BuiltMutation, rows: &[MutationRow]) -> Vec<Value> {
    bind_mutation_rows(mutation, rows).expect("mutation rows should bind")
}

pub(crate) fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

pub(crate) fn mutation_column(name: &str, data_type: &str) -> MutationColumn {
    MutationColumn {
        name: name.to_owned(),
        data_type: data_type.to_owned(),
        ..Default::default()
    }
}

pub(crate) fn widget_columns() -> Vec<MutationColumn> {
    vec![
        MutationColumn {
            insert_default_sql: Some("9000".to_owned()),
            ..mutation_column("id", "bigint")
        },
        mutation_column("name", "text"),
        mutation_column("note", "text"),
        MutationColumn {
            insert_default_sql: Some("now()::text".to_owned()),
            updatable: false,
            ..mutation_column("created_at", "text")
        },
        MutationColumn {
            insert_default_sql: Some("now()::text".to_owned()),
            update_default_sql: Some("now()::text".to_owned()),
            ..mutation_column("updated_at", "text")
        },
        MutationColumn {
            insert_default_sql: Some("1".to_owned()),
            ..mutation_column("version", "bigint")
        },
    ]
}

pub(crate) fn widget_options() -> MutationOptions {
    MutationOptions {
        table: "widgets".to_owned(),
        schema: None,
        columns: widget_columns(),
        primary_key: vec!["id".to_owned()],
        version_column: Some("version".to_owned()),
        returning: None,
    }
}

pub(crate) fn merge_widget_options() -> MutationOptions {
    MutationOptions {
        table: "merge_widgets".to_owned(),
        ..widget_options()
    }
}

pub(crate) fn membership_options() -> MutationOptions {
    MutationOptions {
        table: "memberships".to_owned(),
        schema: None,
        columns: vec![
            mutation_column("tenant_id", "bigint"),
            mutation_column("user_id", "bigint"),
            mutation_column("role", "text"),
        ],
        primary_key: vec!["tenant_id".to_owned(), "user_id".to_owned()],
        version_column: None,
        returning: None,
    }
}

pub(crate) fn mutation_row(entries: &[(&str, Value)]) -> MutationRow {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect::<BTreeMap<_, _>>()
}
