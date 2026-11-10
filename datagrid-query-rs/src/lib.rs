use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::fmt;

mod delete;
mod insert;
mod merge;
mod mutation;
mod query;
mod shared;
mod update;
mod upsert;

pub use delete::build_delete;
pub use insert::build_insert;
pub use merge::build_merge;
pub use query::build_query;
pub use update::build_update;
pub use upsert::build_upsert;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuiltQuery {
    pub sql: String,
    pub params: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub insertable: bool,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub updatable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insert_default_sql: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_default_sql: Option<String>,
}

pub type MutationRow = BTreeMap<String, Value>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationOptions {
    pub table: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub columns: Vec<MutationColumn>,
    pub primary_key: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_column: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returning: Option<Vec<String>>,
    pub rows: Vec<MutationRow>,
}

impl Default for MutationColumn {
    fn default() -> Self {
        Self {
            name: String::new(),
            data_type: String::new(),
            insertable: true,
            updatable: true,
            insert_default_sql: None,
            update_default_sql: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryOptions {
    #[serde(flatten)]
    pub source: QuerySource,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<SelectColumn>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<Filter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sort: Vec<Sort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QuerySource {
    Table {
        #[serde(rename = "table")]
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        schema: Option<String>,
    },
    Subquery {
        subquery: SubquerySource,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubquerySource {
    pub sql: String,
    pub alias: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SelectColumn {
    Name(String),
    Aliased {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alias: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Filter {
    Condition(FilterCondition),
    Group(FilterGroup),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterCondition {
    pub column: String,
    pub operator: FilterOperator,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterGroup {
    pub combinator: Combinator,
    pub filters: Vec<Filter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Combinator {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FilterOperator {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    #[serde(rename = "ilike")]
    ILike,
    NotLike,
    #[serde(rename = "notIlike")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NullsOrder {
    First,
    Last,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sort {
    pub column: String,
    #[serde(default)]
    pub direction: SortDirection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    Json(JsonValue),
}

impl Value {
    fn from_json(value: JsonValue) -> Self {
        match value {
            JsonValue::Null => Self::Null,
            JsonValue::Bool(value) => Self::Bool(value),
            JsonValue::Number(value) => match value.as_i64() {
                Some(value) => Self::Int(value),
                None => Self::Float(value.as_f64().unwrap_or(f64::NAN)),
            },
            JsonValue::String(value) => Self::Text(value),
            JsonValue::Array(values) => {
                Self::Array(values.into_iter().map(Self::from_json).collect())
            }
            JsonValue::Object(_) => Self::Json(value),
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Int(value) => serializer.serialize_i64(*value),
            Self::Float(value) => serializer.serialize_f64(*value),
            Self::Text(value) => serializer.serialize_str(value),
            Self::Bytes(value) => serializer.serialize_bytes(value),
            Self::Array(values) => values.serialize(serializer),
            Self::Json(value) => value.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = JsonValue::deserialize(deserializer)?;
        Ok(Self::from_json(value))
    }
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
    DuplicateColumn {
        column: String,
    },
    UnknownColumn {
        column: String,
    },
    EmptyPrimaryKey,
    DuplicatePrimaryKey {
        column: String,
    },
    InvalidType {
        value: String,
    },
    VersionColumnInPrimaryKey {
        column: String,
    },
    NoInsertableColumns,
    NoUpdatableColumns,
    MissingRowValue {
        column: String,
    },
    EmptyDefaultSql {
        column: String,
        field: &'static str,
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
            Self::DuplicateColumn { column } => write!(f, "duplicate column: {column:?}"),
            Self::UnknownColumn { column } => write!(f, "unknown column: {column:?}"),
            Self::EmptyPrimaryKey => f.write_str("primary key must contain at least one column"),
            Self::DuplicatePrimaryKey { column } => {
                write!(f, "duplicate primary key column: {column:?}")
            }
            Self::InvalidType { value } => write!(f, "invalid SQL type: {value:?}"),
            Self::VersionColumnInPrimaryKey { column } => {
                write!(
                    f,
                    "version column must not be part of the primary key: {column:?}"
                )
            }
            Self::NoInsertableColumns => {
                f.write_str("insert requires at least one insertable column")
            }
            Self::NoUpdatableColumns => {
                f.write_str("update requires at least one updatable column or a version column")
            }
            Self::MissingRowValue { column } => {
                write!(f, "row is missing required value for column: {column:?}")
            }
            Self::EmptyDefaultSql { column, field } => {
                write!(
                    f,
                    "{field} default SQL must not be empty for column: {column:?}"
                )
            }
        }
    }
}

impl std::error::Error for QueryError {}

fn default_true() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}
