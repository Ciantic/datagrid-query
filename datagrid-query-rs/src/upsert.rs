use crate::mutation::{mutation_context, render_insert_select, render_mutation_returning};
use crate::shared::quote_identifier;
use crate::{BuiltMutation, MutationOptions, MutationParamSpec, QueryError};

/// Build a parameterised batch `INSERT ... ON CONFLICT` without executing it.
pub fn build_upsert(options: &MutationOptions) -> Result<BuiltMutation, QueryError> {
    let context = mutation_context(options)?;
    let mut param_specs: Vec<MutationParamSpec> = Vec::new();
    let insert = render_insert_select(options, &context, &mut param_specs, Some("u"))?;
    let conflict_target = context
        .primary_key
        .iter()
        .map(|name| quote_identifier(name, "primary key column"))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");

    let mut assignments = Vec::new();
    for column in &options.columns {
        if !column.updatable
            || context.primary_key.contains(&column.name)
            || context.version_column.as_ref() == Some(&column.name)
        {
            continue;
        }

        let quoted = quote_identifier(&column.name, "column")?;
        assignments.push(format!("{quoted} = excluded.{quoted}"));
    }

    if let Some(version) = &context.version_column {
        let quoted = quote_identifier(version, "version column")?;
        assignments.push(format!("{quoted} = u.{quoted} + 1"));
    }

    let conflict_sql = if assignments.is_empty() {
        format!(" ON CONFLICT ({conflict_target}) DO NOTHING")
    } else {
        let where_clause = match &context.version_column {
            Some(version) => {
                let quoted = quote_identifier(version, "version column")?;
                format!(" WHERE u.{quoted} = excluded.{quoted}")
            }
            None => String::new(),
        };

        format!(
            " ON CONFLICT ({conflict_target}) DO UPDATE SET {}{where_clause}",
            assignments.join(", ")
        )
    };

    let sql = format!(
        "{insert}{conflict_sql}{}",
        render_mutation_returning(&context, "u")?
    );
    Ok(BuiltMutation {
        sql,
        columns: context.column_names,
        param_specs,
    })
}
