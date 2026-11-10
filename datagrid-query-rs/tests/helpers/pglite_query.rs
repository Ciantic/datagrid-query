use datagrid_query::Value;
use postgres::types::ToSql;
use postgres::{Client, Row};
use std::collections::BTreeMap;

pub(crate) fn run_sql(client: &mut Client, sql: &str, params: &[Value]) -> Vec<Row> {
    let params = to_sql_params(sql, params);
    let refs: Vec<&(dyn ToSql + Sync)> = params.iter().map(|param| param.as_ref()).collect();

    client
        .query(sql, &refs)
        .unwrap_or_else(|error| panic!("query failed: {error:?}\n{sql}"))
}

fn to_sql_params(sql: &str, values: &[Value]) -> Vec<Box<dyn ToSql + Sync>> {
    let types = parameter_types(sql);

    values
        .iter()
        .enumerate()
        .map(|(index, value)| -> Box<dyn ToSql + Sync> {
            match value {
                Value::Null => Box::new(Option::<String>::None),
                Value::Bool(value) => Box::new(*value),
                Value::Int(value) => Box::new(*value),
                Value::Float(value) => Box::new(*value),
                Value::Text(value) => Box::new(value.clone()),
                Value::Bytes(value) => Box::new(value.clone()),
                Value::Array(values) => array_param(values, types.get(&(index + 1))),
                Value::Json(value) => Box::new(value.clone()),
            }
        })
        .collect()
}

fn parameter_types(sql: &str) -> BTreeMap<usize, String> {
    let bytes = sql.as_bytes();
    let mut types = BTreeMap::new();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'$' || index + 1 >= bytes.len() || !bytes[index + 1].is_ascii_digit() {
            index += 1;
            continue;
        }

        let start = index + 1;
        let mut end = start;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }

        let Ok(number) = sql[start..end].parse::<usize>() else {
            index = end;
            continue;
        };

        if !sql[end..].starts_with("::") {
            index = end;
            continue;
        }

        let type_start = end + 2;
        let mut type_end = type_start;
        while type_end < bytes.len() && bytes[type_end] != b'[' {
            type_end += 1;
        }

        if type_end < bytes.len() {
            types.insert(number, sql[type_start..type_end].trim().to_owned());
            index = type_end + 1;
        } else {
            index = end;
        }
    }

    types
}

fn array_param(values: &[Value], data_type: Option<&String>) -> Box<dyn ToSql + Sync> {
    if let Some(data_type) = data_type {
        if data_type.ends_with("bool") {
            return Box::new(
                values
                    .iter()
                    .map(|value| match value {
                        Value::Bool(value) => Some(*value),
                        Value::Null => None,
                        _ => panic!("expected boolean array parameter"),
                    })
                    .collect::<Vec<Option<bool>>>(),
            );
        }

        if data_type.contains("int") {
            return Box::new(
                values
                    .iter()
                    .map(|value| match value {
                        Value::Int(value) => Some(*value),
                        Value::Null => None,
                        _ => panic!("expected integer array parameter"),
                    })
                    .collect::<Vec<Option<i64>>>(),
            );
        }

        return Box::new(
            values
                .iter()
                .map(|value| match value {
                    Value::Text(value) => Some(value.clone()),
                    Value::Null => None,
                    _ => panic!("expected text array parameter"),
                })
                .collect::<Vec<Option<String>>>(),
        );
    }

    if values
        .iter()
        .all(|value| matches!(value, Value::Int(_) | Value::Null))
    {
        return Box::new(
            values
                .iter()
                .map(|value| match value {
                    Value::Int(value) => Some(*value),
                    Value::Null => None,
                    _ => unreachable!(),
                })
                .collect::<Vec<Option<i64>>>(),
        );
    }

    if values
        .iter()
        .all(|value| matches!(value, Value::Text(_) | Value::Null))
    {
        return Box::new(
            values
                .iter()
                .map(|value| match value {
                    Value::Text(value) => Some(value.clone()),
                    Value::Null => None,
                    _ => unreachable!(),
                })
                .collect::<Vec<Option<String>>>(),
        );
    }

    if values
        .iter()
        .all(|value| matches!(value, Value::Bool(_) | Value::Null))
    {
        return Box::new(
            values
                .iter()
                .map(|value| match value {
                    Value::Bool(value) => Some(*value),
                    Value::Null => None,
                    _ => unreachable!(),
                })
                .collect::<Vec<Option<bool>>>(),
        );
    }

    panic!("PGlite test helper only supports integer, text, and boolean array parameters");
}
