use std::fmt;

/// Build a parameterised `SELECT` statement without executing it.
pub fn build_query(options: &QueryOptions) -> Result<BuiltQuery, QueryError> {
    let mut params = Vec::new();
    let from = render_source(&options.source, &mut params)?;
    let select_list = render_columns(&options.columns)?;

    let mut parts = vec![format!("SELECT {select_list}"), format!("FROM {from}")];

    let where_clause = render_filters(&options.filters, &mut params)?;
    if !where_clause.is_empty() {
        parts.push(format!("WHERE {where_clause}"));
    }

    let order_by = render_sort(&options.sort)?;
    if !order_by.is_empty() {
        parts.push(format!("ORDER BY {order_by}"));
    }

    if let Some(limit) = options.limit {
        parts.push(format!("LIMIT {limit}"));
    }

    if let Some(offset) = options.offset {
        parts.push(format!("OFFSET {offset}"));
    }

    Ok(BuiltQuery {
        sql: parts.join(" "),
        params,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuiltQuery {
    pub sql: String,
    pub params: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryOptions {
    pub source: QuerySource,
    pub columns: Vec<SelectColumn>,
    pub filters: Vec<Filter>,
    pub sort: Vec<Sort>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuerySource {
    Table {
        schema: Option<String>,
        name: String,
    },
    Subquery {
        sql: String,
        alias: String,
        params: Vec<Value>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectColumn {
    Name(String),
    Aliased { name: String, alias: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    Condition(FilterCondition),
    Group(FilterGroup),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterCondition {
    pub column: String,
    pub operator: FilterOperator,
    pub value: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterGroup {
    pub combinator: Combinator,
    pub filters: Vec<Filter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOperator {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    ILike,
    NotLike,
    NotILike,
    In,
    NotIn,
    Any,
    NotAny,
    IsNull,
    IsNotNull,
    Between,
    NotBetween,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullsOrder {
    First,
    Last,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sort {
    pub column: String,
    pub direction: SortDirection,
    pub nulls: Option<NullsOrder>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    InvalidIdentifier {
        kind: &'static str,
        value: String,
    },
    EmptySubquery,
    InvalidFilterValue {
        operator: &'static str,
        expected: &'static str,
    },
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { kind, value } => {
                write!(f, "invalid {kind} identifier: {value:?}")
            }
            Self::EmptySubquery => f.write_str("subquery SQL must not be empty"),
            Self::InvalidFilterValue { operator, expected } => {
                write!(f, "operator {operator} requires {expected}")
            }
        }
    }
}

impl std::error::Error for QueryError {}

impl FilterOperator {
    fn as_str(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Neq => "neq",
            Self::Gt => "gt",
            Self::Gte => "gte",
            Self::Lt => "lt",
            Self::Lte => "lte",
            Self::Like => "like",
            Self::ILike => "ilike",
            Self::NotLike => "notLike",
            Self::NotILike => "notIlike",
            Self::In => "in",
            Self::NotIn => "notIn",
            Self::Any => "any",
            Self::NotAny => "notAny",
            Self::IsNull => "isNull",
            Self::IsNotNull => "isNotNull",
            Self::Between => "between",
            Self::NotBetween => "notBetween",
        }
    }
}

fn render_source(source: &QuerySource, params: &mut Vec<Value>) -> Result<String, QueryError> {
    match source {
        QuerySource::Table { schema, name } => {
            let table = quote_identifier(name, "table")?;
            match schema {
                Some(schema) => {
                    let schema = quote_identifier(schema, "schema")?;
                    Ok(format!("{schema}.{table}"))
                }
                None => Ok(table),
            }
        }
        QuerySource::Subquery {
            sql,
            alias,
            params: subquery_params,
        } => {
            if sql.trim().is_empty() {
                return Err(QueryError::EmptySubquery);
            }

            let alias = quote_identifier(alias, "subquery alias")?;
            params.extend(subquery_params.iter().cloned());
            Ok(format!("({sql}) AS {alias}"))
        }
    }
}

fn render_columns(columns: &[SelectColumn]) -> Result<String, QueryError> {
    if columns.is_empty() {
        return Ok("*".to_owned());
    }

    let mut rendered = Vec::with_capacity(columns.len());
    for column in columns {
        match column {
            SelectColumn::Name(name) => rendered.push(quote_identifier(name, "column")?),
            SelectColumn::Aliased { name, alias } => rendered.push(format!(
                "{} AS {}",
                quote_identifier(name, "column")?,
                quote_identifier(alias, "column alias")?
            )),
        }
    }

    Ok(rendered.join(", "))
}

fn render_filters(filters: &[Filter], params: &mut Vec<Value>) -> Result<String, QueryError> {
    let mut rendered = Vec::with_capacity(filters.len());
    for filter in filters {
        rendered.push(render_filter(filter, params)?);
    }

    Ok(rendered.join(" AND "))
}

fn render_filter(filter: &Filter, params: &mut Vec<Value>) -> Result<String, QueryError> {
    match filter {
        Filter::Condition(condition) => render_condition(condition, params),
        Filter::Group(group) => {
            if group.filters.is_empty() {
                return Ok(match group.combinator {
                    Combinator::And => "TRUE",
                    Combinator::Or => "FALSE",
                }
                .to_owned());
            }

            let joiner = match group.combinator {
                Combinator::And => " AND ",
                Combinator::Or => " OR ",
            };

            let mut rendered = Vec::with_capacity(group.filters.len());
            for child in &group.filters {
                rendered.push(render_filter(child, params)?);
            }

            Ok(format!("({})", rendered.join(joiner)))
        }
    }
}

fn render_condition(
    condition: &FilterCondition,
    params: &mut Vec<Value>,
) -> Result<String, QueryError> {
    let column = quote_identifier(&condition.column, "filter column")?;
    let operator = condition.operator;

    match operator {
        FilterOperator::Eq => match &condition.value {
            None | Some(Value::Null) => Ok(format!("{column} IS NULL")),
            Some(value) => Ok(format!("{column} = {}", push_param(params, value))),
        },
        FilterOperator::Neq => match &condition.value {
            None | Some(Value::Null) => Ok(format!("{column} IS NOT NULL")),
            Some(value) => Ok(format!("{column} <> {}", push_param(params, value))),
        },
        FilterOperator::Gt => render_comparison(&column, ">", condition, operator, params),
        FilterOperator::Gte => render_comparison(&column, ">=", condition, operator, params),
        FilterOperator::Lt => render_comparison(&column, "<", condition, operator, params),
        FilterOperator::Lte => render_comparison(&column, "<=", condition, operator, params),
        FilterOperator::Like => render_comparison(&column, "LIKE", condition, operator, params),
        FilterOperator::ILike => render_comparison(&column, "ILIKE", condition, operator, params),
        FilterOperator::NotLike => {
            render_comparison(&column, "NOT LIKE", condition, operator, params)
        }
        FilterOperator::NotILike => {
            render_comparison(&column, "NOT ILIKE", condition, operator, params)
        }
        FilterOperator::In | FilterOperator::NotIn => {
            let values = require_array(&condition.value, operator)?;
            if values.is_empty() {
                return Ok(match operator {
                    FilterOperator::In => "FALSE",
                    _ => "TRUE",
                }
                .to_owned());
            }

            let mut placeholders = Vec::with_capacity(values.len());
            for value in values {
                placeholders.push(push_param(params, value));
            }

            let sql_operator = match operator {
                FilterOperator::In => "IN",
                _ => "NOT IN",
            };

            Ok(format!(
                "{column} {sql_operator} ({})",
                placeholders.join(", ")
            ))
        }
        FilterOperator::Any | FilterOperator::NotAny => {
            let values = require_array(&condition.value, operator)?;
            let value = Value::Array(values.to_vec());
            let sql_operator = match operator {
                FilterOperator::Any => "= ANY",
                _ => "<> ALL",
            };

            Ok(format!(
                "{column} {sql_operator}({})",
                push_param(params, &value)
            ))
        }
        FilterOperator::IsNull => Ok(format!("{column} IS NULL")),
        FilterOperator::IsNotNull => Ok(format!("{column} IS NOT NULL")),
        FilterOperator::Between | FilterOperator::NotBetween => {
            let values = require_array(&condition.value, operator)?;
            if values.len() != 2 {
                return Err(QueryError::InvalidFilterValue {
                    operator: operator.as_str(),
                    expected: "exactly two values",
                });
            }

            let sql_operator = match operator {
                FilterOperator::Between => "BETWEEN",
                _ => "NOT BETWEEN",
            };

            Ok(format!(
                "{column} {sql_operator} {} AND {}",
                push_param(params, &values[0]),
                push_param(params, &values[1])
            ))
        }
    }
}

fn render_comparison(
    column: &str,
    sql_operator: &str,
    condition: &FilterCondition,
    operator: FilterOperator,
    params: &mut Vec<Value>,
) -> Result<String, QueryError> {
    let value = require_value(&condition.value, operator)?;
    Ok(format!(
        "{column} {sql_operator} {}",
        push_param(params, value)
    ))
}

fn require_value(value: &Option<Value>, operator: FilterOperator) -> Result<&Value, QueryError> {
    match value {
        None | Some(Value::Null) => Err(QueryError::InvalidFilterValue {
            operator: operator.as_str(),
            expected: "a non-null value",
        }),
        Some(value) => Ok(value),
    }
}

fn require_array(value: &Option<Value>, operator: FilterOperator) -> Result<&[Value], QueryError> {
    match value {
        Some(Value::Array(values)) => Ok(values),
        _ => Err(QueryError::InvalidFilterValue {
            operator: operator.as_str(),
            expected: "an array value",
        }),
    }
}

fn render_sort(sort: &[Sort]) -> Result<String, QueryError> {
    let mut rendered = Vec::with_capacity(sort.len());
    for spec in sort {
        let column = quote_identifier(&spec.column, "sort column")?;
        let direction = match spec.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };

        let mut clause = format!("{column} {direction}");
        if let Some(nulls) = spec.nulls {
            clause.push_str(match nulls {
                NullsOrder::First => " NULLS FIRST",
                NullsOrder::Last => " NULLS LAST",
            });
        }

        rendered.push(clause);
    }

    Ok(rendered.join(", "))
}

fn push_param(params: &mut Vec<Value>, value: &Value) -> String {
    params.push(value.clone());
    format!("${}", params.len())
}

fn quote_identifier(value: &str, kind: &'static str) -> Result<String, QueryError> {
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
