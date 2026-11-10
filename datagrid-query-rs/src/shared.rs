use crate::{QueryError, Value};

pub(crate) fn push_param(params: &mut Vec<Value>, value: &Value) -> String {
    params.push(value.clone());
    format!("${}", params.len())
}

pub(crate) fn quote_identifier(value: &str, kind: &'static str) -> Result<String, QueryError> {
    if !is_valid_identifier(value) {
        return Err(QueryError::InvalidIdentifier {
            kind,
            value: value.to_owned(),
        });
    }

    Ok(format!("\"{value}\""))
}

fn is_valid_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    match bytes.next() {
        Some(first) if first == b'_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }

    bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
}

pub(crate) fn is_valid_sql_type(value: &str) -> bool {
    let mut bytes = value.bytes();
    match bytes.next() {
        Some(first) if first == b'_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }

    bytes.all(|byte| {
        byte == b'_'
            || byte == b' '
            || byte == b'.'
            || byte == b'('
            || byte == b')'
            || byte == b','
            || byte.is_ascii_alphanumeric()
    })
}
