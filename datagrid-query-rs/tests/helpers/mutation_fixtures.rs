use datagrid_query::{MutationColumn, MutationOptions, MutationRow, Value};
use std::collections::BTreeMap;

pub(crate) fn widget_options() -> MutationOptions {
    MutationOptions {
        table: "widgets".to_owned(),
        schema: None,
        columns: vec![
            MutationColumn {
                name: "id".to_owned(),
                data_type: "bigint".to_owned(),
                insert_default_sql: Some("9000".to_owned()),
                ..Default::default()
            },
            MutationColumn {
                name: "name".to_owned(),
                data_type: "text".to_owned(),
                ..Default::default()
            },
            MutationColumn {
                name: "note".to_owned(),
                data_type: "text".to_owned(),
                ..Default::default()
            },
            MutationColumn {
                name: "created_at".to_owned(),
                data_type: "text".to_owned(),
                insert_default_sql: Some("now()::text".to_owned()),
                updatable: false,
                ..Default::default()
            },
            MutationColumn {
                name: "updated_at".to_owned(),
                data_type: "text".to_owned(),
                insert_default_sql: Some("now()::text".to_owned()),
                update_default_sql: Some("now()::text".to_owned()),
                ..Default::default()
            },
            MutationColumn {
                name: "version".to_owned(),
                data_type: "bigint".to_owned(),
                insert_default_sql: Some("1".to_owned()),
                ..Default::default()
            },
        ],
        primary_key: vec!["id".to_owned()],
        version_column: Some("version".to_owned()),
        returning: None,
    }
}

pub(crate) fn membership_options() -> MutationOptions {
    MutationOptions {
        table: "memberships".to_owned(),
        schema: None,
        columns: vec![
            MutationColumn {
                name: "tenant_id".to_owned(),
                data_type: "bigint".to_owned(),
                ..Default::default()
            },
            MutationColumn {
                name: "user_id".to_owned(),
                data_type: "bigint".to_owned(),
                ..Default::default()
            },
            MutationColumn {
                name: "role".to_owned(),
                data_type: "text".to_owned(),
                ..Default::default()
            },
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
