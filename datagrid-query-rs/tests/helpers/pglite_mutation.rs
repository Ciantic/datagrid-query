use datagrid_query::{BuiltMutation, MutationRow, bind_mutation_rows};
use postgres::{Client, Row};

use super::pglite_query::run_sql;

pub(crate) fn run_mutation(
    client: &mut Client,
    mutation: &BuiltMutation,
    rows: &[MutationRow],
) -> Vec<Row> {
    let params = bind_mutation_rows(mutation, rows).expect("mutation rows should bind");
    run_sql(client, &mutation.sql, &params)
}

pub(crate) fn create_mutation_tables(client: &mut Client) {
    client
        .batch_execute(
            r#"
            CREATE TABLE widgets (
                id bigint PRIMARY KEY,
                name text NOT NULL,
                note text,
                created_at text NOT NULL,
                updated_at text NOT NULL,
                version bigint NOT NULL
            );

            CREATE TABLE memberships (
                tenant_id bigint NOT NULL,
                user_id bigint NOT NULL,
                role text NOT NULL,
                PRIMARY KEY (tenant_id, user_id)
            );

            CREATE TABLE merge_widgets (
                id bigint PRIMARY KEY,
                name text NOT NULL,
                note text,
                created_at text NOT NULL,
                updated_at text NOT NULL,
                version bigint NOT NULL
            );
            "#,
        )
        .expect("failed to create mutation test tables");
}
