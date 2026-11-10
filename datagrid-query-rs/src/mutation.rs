use crate::shared::{is_valid_sql_type, quote_identifier};
use crate::{MutationColumn, MutationOptions, MutationRow, QueryError, Value};
use std::collections::BTreeMap;

pub(crate) struct MutationContext {
    pub(crate) table: String,
    pub(crate) table_name: String,
    pub(crate) columns: BTreeMap<String, MutationColumn>,
    pub(crate) primary_key: Vec<String>,
    pub(crate) version_column: Option<String>,
    pub(crate) returning: Vec<String>,
}

pub(crate) fn mutation_context(options: &MutationOptions) -> Result<MutationContext, QueryError> {
    let table_name = quote_identifier(&options.table, "table")?;
    let table = match &options.schema {
        Some(schema) => format!("{}.{table_name}", quote_identifier(schema, "schema")?),
        None => table_name.clone(),
    };

    let mut columns = BTreeMap::new();
    for column in &options.columns {
        quote_identifier(&column.name, "column")?;
        if !is_valid_sql_type(&column.data_type) {
            return Err(QueryError::InvalidType {
                value: column.data_type.clone(),
            });
        }

        if let Some(default_sql) = &column.insert_default_sql {
            if default_sql.trim().is_empty() {
                return Err(QueryError::EmptyDefaultSql {
                    column: column.name.clone(),
                    field: "insert",
                });
            }
        }

        if let Some(default_sql) = &column.update_default_sql {
            if default_sql.trim().is_empty() {
                return Err(QueryError::EmptyDefaultSql {
                    column: column.name.clone(),
                    field: "update",
                });
            }
        }

        if columns
            .insert(column.name.clone(), column.clone())
            .is_some()
        {
            return Err(QueryError::DuplicateColumn {
                column: column.name.clone(),
            });
        }
    }

    if options.primary_key.is_empty() {
        return Err(QueryError::EmptyPrimaryKey);
    }

    let mut seen_primary_key = BTreeMap::new();
    for key in &options.primary_key {
        quote_identifier(key, "primary key column")?;
        if seen_primary_key.insert(key, ()).is_some() {
            return Err(QueryError::DuplicatePrimaryKey {
                column: key.clone(),
            });
        }

        if !columns.contains_key(key) {
            return Err(QueryError::UnknownColumn {
                column: key.clone(),
            });
        }
    }

    if let Some(version) = &options.version_column {
        quote_identifier(version, "version column")?;
        if !columns.contains_key(version) {
            return Err(QueryError::UnknownColumn {
                column: version.clone(),
            });
        }

        if seen_primary_key.contains_key(version) {
            return Err(QueryError::VersionColumnInPrimaryKey {
                column: version.clone(),
            });
        }
    }

    let returning = options
        .returning
        .clone()
        .unwrap_or_else(|| options.primary_key.clone());
    for name in &returning {
        quote_identifier(name, "returning column")?;
        if !columns.contains_key(name) {
            return Err(QueryError::UnknownColumn {
                column: name.clone(),
            });
        }
    }

    for row in &options.rows {
        for key in row.keys() {
            if !columns.contains_key(key) {
                return Err(QueryError::UnknownColumn {
                    column: key.clone(),
                });
            }
        }
    }

    Ok(MutationContext {
        table,
        table_name,
        columns,
        primary_key: options.primary_key.clone(),
        version_column: options.version_column.clone(),
        returning,
    })
}

fn row_value(row: &MutationRow, column: &str) -> Value {
    row.get(column).cloned().unwrap_or(Value::Null)
}

fn required_row_value(row: &MutationRow, column: &str) -> Result<Value, QueryError> {
    match row.get(column) {
        None | Some(Value::Null) => Err(QueryError::MissingRowValue {
            column: column.to_owned(),
        }),
        Some(value) => Ok(value.clone()),
    }
}

pub(crate) fn value_array(rows: &[MutationRow], column: &str) -> Value {
    Value::Array(rows.iter().map(|row| row_value(row, column)).collect())
}

pub(crate) fn present_array(rows: &[MutationRow], column: &str) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| Value::Bool(row.contains_key(column)))
            .collect(),
    )
}

pub(crate) fn required_array(rows: &[MutationRow], column: &str) -> Result<Value, QueryError> {
    let values = rows
        .iter()
        .map(|row| required_row_value(row, column))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Value::Array(values))
}

pub(crate) fn push_array_param(params: &mut Vec<Value>, value: Value, data_type: &str) -> String {
    params.push(value);
    format!("${}::{data_type}[]", params.len())
}

pub(crate) fn render_mutation_returning(
    context: &MutationContext,
    qualifier: &str,
) -> Result<String, QueryError> {
    if context.returning.is_empty() {
        return Ok(String::new());
    }

    let mut columns = Vec::with_capacity(context.returning.len());
    for name in &context.returning {
        columns.push(format!(
            "{qualifier}.{}",
            quote_identifier(name, "returning column")?
        ));
    }

    Ok(format!(" RETURNING {}", columns.join(", ")))
}

pub(crate) fn render_insert_select(
    options: &MutationOptions,
    context: &MutationContext,
    params: &mut Vec<Value>,
    alias: Option<&str>,
) -> Result<String, QueryError> {
    let columns = options
        .columns
        .iter()
        .filter(|column| column.insertable)
        .collect::<Vec<_>>();
    if columns.is_empty() {
        return Err(QueryError::NoInsertableColumns);
    }

    let mut column_names = Vec::with_capacity(columns.len());
    let mut select_expressions = Vec::with_capacity(columns.len());
    let mut aliases = Vec::new();
    let mut placeholders = Vec::new();

    for column in columns {
        let quoted = quote_identifier(&column.name, "column")?;
        placeholders.push(push_array_param(
            params,
            value_array(&options.rows, &column.name),
            &column.data_type,
        ));
        column_names.push(quoted.clone());
        aliases.push(quoted.clone());

        if let Some(default_sql) = &column.insert_default_sql {
            let present_alias = format!("\"{}#present\"", column.name);
            placeholders.push(push_array_param(
                params,
                present_array(&options.rows, &column.name),
                "bool",
            ));
            aliases.push(present_alias.clone());
            select_expressions.push(format!(
                "CASE WHEN v.{present_alias} THEN v.{quoted} ELSE {default_sql} END"
            ));
        } else {
            select_expressions.push(format!("v.{quoted}"));
        }
    }

    let target = match alias {
        Some(alias) => format!("{} AS {alias}", context.table),
        None => context.table.clone(),
    };

    Ok(format!(
        "INSERT INTO {target} ({}) SELECT {} FROM unnest({}) AS v({})",
        column_names.join(", "),
        select_expressions.join(", "),
        placeholders.join(", "),
        aliases.join(", ")
    ))
}
