use crate::{BuiltMutation, MutationParamKind, MutationParamSpec, MutationRow, QueryError, Value};

fn required_row_value(row: &MutationRow, column: &str) -> Result<Value, QueryError> {
    match row.get(column) {
        None | Some(Value::Null) => Err(QueryError::MissingRowValue {
            column: column.to_owned(),
        }),
        Some(value) => Ok(value.clone()),
    }
}

fn bind_spec(spec: &MutationParamSpec, rows: &[MutationRow]) -> Result<Value, QueryError> {
    match spec.kind {
        MutationParamKind::Value => Ok(Value::Array(
            rows.iter()
                .map(|row| row.get(&spec.column).cloned().unwrap_or(Value::Null))
                .collect(),
        )),
        MutationParamKind::Present => Ok(Value::Array(
            rows.iter()
                .map(|row| Value::Bool(row.contains_key(&spec.column)))
                .collect(),
        )),
        MutationParamKind::Required => Ok(Value::Array(
            rows.iter()
                .map(|row| required_row_value(row, &spec.column))
                .collect::<Result<Vec<_>, _>>()?,
        )),
    }
}

/// Bind mutation rows to the ordered parameter specification.
pub fn bind_mutation_rows(
    mutation: &BuiltMutation,
    rows: &[MutationRow],
) -> Result<Vec<Value>, QueryError> {
    for row in rows {
        for key in row.keys() {
            if !mutation.columns.contains(key) {
                return Err(QueryError::UnknownColumn {
                    column: key.clone(),
                });
            }
        }
    }

    mutation
        .param_specs
        .iter()
        .map(|spec| bind_spec(spec, rows))
        .collect()
}
