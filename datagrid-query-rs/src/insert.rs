use crate::mutation::{mutation_context, render_insert_select, render_mutation_returning};
use crate::{BuiltMutation, MutationOptions, MutationParamSpec, QueryError};

/// Build a parameterised batch `INSERT` without executing it.
pub fn build_insert(options: &MutationOptions) -> Result<BuiltMutation, QueryError> {
    let context = mutation_context(options)?;
    let mut param_specs: Vec<MutationParamSpec> = Vec::new();
    let mut sql = render_insert_select(options, &context, &mut param_specs, None)?;
    sql.push_str(&render_mutation_returning(&context, &context.table_name)?);

    Ok(BuiltMutation {
        sql,
        columns: context.column_names,
        param_specs,
    })
}
