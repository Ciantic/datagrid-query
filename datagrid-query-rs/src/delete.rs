use crate::mutation::{
    mutation_column, mutation_context, push_param_spec, render_mutation_returning,
};
use crate::shared::quote_identifier;
use crate::{BuiltMutation, MutationOptions, MutationParamKind, MutationParamSpec, QueryError};

/// Build a parameterised batch `DELETE` without executing it.
pub fn build_delete(options: &MutationOptions) -> Result<BuiltMutation, QueryError> {
    let context = mutation_context(options)?;
    let mut param_specs: Vec<MutationParamSpec> = Vec::new();
    let mut aliases = Vec::new();
    let mut placeholders = Vec::new();
    let mut where_parts = Vec::new();

    for key in &context.primary_key {
        let column = mutation_column(&context, key)?;
        let quoted = quote_identifier(key, "primary key column")?;
        placeholders.push(push_param_spec(
            &mut param_specs,
            MutationParamKind::Required,
            key,
            &column.data_type,
        ));
        aliases.push(quoted.clone());
        where_parts.push(format!("u.{quoted} = v.{quoted}"));
    }

    if let Some(version) = &context.version_column {
        let column = mutation_column(&context, version)?;
        let quoted = quote_identifier(version, "version column")?;
        placeholders.push(push_param_spec(
            &mut param_specs,
            MutationParamKind::Required,
            version,
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
    Ok(BuiltMutation {
        sql,
        columns: context.column_names,
        param_specs,
    })
}
