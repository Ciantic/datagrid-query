use crate::mutation::{
    mutation_context, push_array_param, render_mutation_returning, required_array,
};
use crate::shared::quote_identifier;
use crate::{BuiltQuery, MutationOptions, QueryError};

/// Build a parameterised batch `DELETE` without executing it.
pub fn build_delete(options: &MutationOptions) -> Result<BuiltQuery, QueryError> {
    let context = mutation_context(options)?;
    let mut params = Vec::new();
    let mut aliases = Vec::new();
    let mut placeholders = Vec::new();
    let mut where_parts = Vec::new();

    for key in &context.primary_key {
        let column = context
            .columns
            .get(key)
            .ok_or_else(|| QueryError::UnknownColumn {
                column: key.clone(),
            })?;
        let quoted = quote_identifier(key, "primary key column")?;
        placeholders.push(push_array_param(
            &mut params,
            required_array(&options.rows, key)?,
            &column.data_type,
        ));
        aliases.push(quoted.clone());
        where_parts.push(format!("u.{quoted} = v.{quoted}"));
    }

    if let Some(version) = &context.version_column {
        let column = context
            .columns
            .get(version)
            .ok_or_else(|| QueryError::UnknownColumn {
                column: version.clone(),
            })?;
        let quoted = quote_identifier(version, "version column")?;
        placeholders.push(push_array_param(
            &mut params,
            required_array(&options.rows, version)?,
            &column.data_type,
        ));
        aliases.push(quoted.clone());
        where_parts.push(format!("u.{quoted} = v.{quoted}"));
    }

    let sql = format!(
        "DELETE FROM {} AS u USING unnest({}) AS v({}) WHERE {}{}",
        context.table,
        placeholders.join(", "),
        aliases.join(", "),
        where_parts.join(" AND "),
        render_mutation_returning(&context, "u")?
    );
    Ok(BuiltQuery { sql, params })
}
