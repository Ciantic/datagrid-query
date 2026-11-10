use crate::mutation::{mutation_context, render_insert_select, render_mutation_returning};
use crate::{BuiltQuery, MutationOptions, QueryError};

/// Build a parameterised batch `INSERT` without executing it.
pub fn build_insert(options: &MutationOptions) -> Result<BuiltQuery, QueryError> {
    let context = mutation_context(options)?;
    let mut params = Vec::new();
    let mut sql = render_insert_select(options, &context, &mut params, None)?;
    sql.push_str(&render_mutation_returning(&context, &context.table_name)?);

    Ok(BuiltQuery { sql, params })
}
