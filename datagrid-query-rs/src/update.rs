use crate::mutation::{
    mutation_context, present_array, push_array_param, render_mutation_returning, required_array,
    value_array,
};
use crate::shared::quote_identifier;
use crate::{BuiltQuery, MutationOptions, QueryError};

/// Build a parameterised batch `UPDATE` without executing it.
pub fn build_update(options: &MutationOptions) -> Result<BuiltQuery, QueryError> {
    let context = mutation_context(options)?;
    let mut params = Vec::new();
    let mut aliases = Vec::new();
    let mut placeholders = Vec::new();

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
        aliases.push(quoted);
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
        aliases.push(quoted);
    }

    let mut set_assignments = Vec::new();
    for column in &options.columns {
        if !column.updatable
            || context.primary_key.contains(&column.name)
            || context.version_column.as_ref() == Some(&column.name)
        {
            continue;
        }

        let quoted = quote_identifier(&column.name, "column")?;
        placeholders.push(push_array_param(
            &mut params,
            value_array(&options.rows, &column.name),
            &column.data_type,
        ));
        aliases.push(quoted.clone());

        let present_alias = format!("\"{}#present\"", column.name);
        placeholders.push(push_array_param(
            &mut params,
            present_array(&options.rows, &column.name),
            "bool",
        ));
        aliases.push(present_alias.clone());

        let fallback = match &column.update_default_sql {
            Some(default_sql) => default_sql.clone(),
            None => format!("u.{quoted}"),
        };
        set_assignments.push(format!(
            "{quoted} = CASE WHEN v.{present_alias} THEN v.{quoted} ELSE {fallback} END"
        ));
    }

    if let Some(version) = &context.version_column {
        let quoted = quote_identifier(version, "version column")?;
        set_assignments.push(format!("{quoted} = u.{quoted} + 1"));
    }

    if set_assignments.is_empty() {
        return Err(QueryError::NoUpdatableColumns);
    }

    let mut where_parts = Vec::new();
    for key in &context.primary_key {
        let quoted = quote_identifier(key, "primary key column")?;
        where_parts.push(format!("u.{quoted} = v.{quoted}"));
    }

    if let Some(version) = &context.version_column {
        let quoted = quote_identifier(version, "version column")?;
        where_parts.push(format!("u.{quoted} = v.{quoted}"));
    }

    let sql = format!(
        "UPDATE {} AS u SET {} FROM unnest({}) AS v({}) WHERE {}{}",
        context.table,
        set_assignments.join(", "),
        placeholders.join(", "),
        aliases.join(", "),
        where_parts.join(" AND "),
        render_mutation_returning(&context, "u")?
    );
    Ok(BuiltQuery { sql, params })
}
