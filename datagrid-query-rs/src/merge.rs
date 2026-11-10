use crate::mutation::{
    mutation_context, present_array, push_array_param, render_mutation_returning, value_array,
};
use crate::shared::quote_identifier;
use crate::{BuiltQuery, MutationOptions, QueryError};

/// Build a parameterised `MERGE` with per-field patch semantics.
pub fn build_merge(options: &MutationOptions) -> Result<BuiltQuery, QueryError> {
    let context = mutation_context(options)?;
    let mut params = Vec::new();
    let mut placeholders = Vec::new();
    let mut aliases = Vec::new();

    for column in &options.columns {
        let primary_key = context.primary_key.contains(&column.name);
        let version_column = context.version_column.as_ref() == Some(&column.name);
        let insertable = column.insertable;
        let updatable = column.updatable && !primary_key && !version_column;

        if !insertable && !updatable && !primary_key && !version_column {
            continue;
        }

        let quoted = quote_identifier(&column.name, "column")?;
        placeholders.push(push_array_param(
            &mut params,
            value_array(&options.rows, &column.name),
            &column.data_type,
        ));
        aliases.push(quoted);

        if updatable || (insertable && column.insert_default_sql.is_some()) {
            placeholders.push(push_array_param(
                &mut params,
                present_array(&options.rows, &column.name),
                "bool",
            ));
            aliases.push(format!("\"{}#present\"", column.name));
        }
    }

    let insert_columns = options
        .columns
        .iter()
        .filter(|column| column.insertable)
        .collect::<Vec<_>>();
    if insert_columns.is_empty() {
        return Err(QueryError::NoInsertableColumns);
    }

    let mut insert_names = Vec::with_capacity(insert_columns.len());
    let mut insert_values = Vec::with_capacity(insert_columns.len());
    for column in insert_columns {
        let quoted = quote_identifier(&column.name, "column")?;
        insert_names.push(quoted.clone());
        match &column.insert_default_sql {
            Some(default_sql) => insert_values.push(format!(
                "CASE WHEN v.\"{}#present\" THEN v.{quoted} ELSE {default_sql} END",
                column.name
            )),
            None => insert_values.push(format!("v.{quoted}")),
        }
    }

    let mut assignments = Vec::new();
    for column in &options.columns {
        if !column.updatable
            || context.primary_key.contains(&column.name)
            || context.version_column.as_ref() == Some(&column.name)
        {
            continue;
        }

        let quoted = quote_identifier(&column.name, "column")?;
        let present_alias = format!("\"{}#present\"", column.name);
        let fallback = match &column.update_default_sql {
            Some(default_sql) => default_sql.clone(),
            None => format!("u.{quoted}"),
        };
        assignments.push(format!(
            "{quoted} = CASE WHEN v.{present_alias} THEN v.{quoted} ELSE {fallback} END"
        ));
    }

    let mut matched_condition = String::new();
    if let Some(version) = &context.version_column {
        let quoted = quote_identifier(version, "version column")?;
        matched_condition = format!(" AND u.{quoted} = v.{quoted}");
        assignments.push(format!("{quoted} = u.{quoted} + 1"));
    }

    let matched_sql = if assignments.is_empty() {
        "WHEN MATCHED THEN DO NOTHING".to_owned()
    } else {
        format!(
            "WHEN MATCHED{matched_condition} THEN UPDATE SET {}",
            assignments.join(", ")
        )
    };

    let on = context
        .primary_key
        .iter()
        .map(|name| {
            let quoted = quote_identifier(name, "primary key column")?;
            Ok(format!("u.{quoted} = v.{quoted}"))
        })
        .collect::<Result<Vec<_>, QueryError>>()?
        .join(" AND ");

    let sql = format!(
        "MERGE INTO {} AS u USING unnest({}) AS v({}) ON {} {} WHEN NOT MATCHED THEN INSERT ({}) VALUES ({}){}",
        context.table,
        placeholders.join(", "),
        aliases.join(", "),
        on,
        matched_sql,
        insert_names.join(", "),
        insert_values.join(", "),
        render_mutation_returning(&context, "u")?
    );

    Ok(BuiltQuery { sql, params })
}
