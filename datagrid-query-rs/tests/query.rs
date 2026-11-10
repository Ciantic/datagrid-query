use datagrid_query::{
    Combinator, Filter, FilterCondition, FilterGroup, FilterOperator, NullsOrder, QueryError,
    QueryOptions, QuerySource, SelectColumn, Sort, SortDirection, SubquerySource, Value,
    build_query,
};

fn blank(source: QuerySource) -> QueryOptions {
    QueryOptions {
        source,
        columns: Vec::new(),
        filters: Vec::new(),
        sort: Vec::new(),
        limit: None,
        offset: None,
    }
}

fn table(name: &str) -> QuerySource {
    QuerySource::Table {
        schema: None,
        name: name.to_owned(),
    }
}

fn condition(column: &str, operator: FilterOperator, value: Option<Value>) -> Filter {
    Filter::Condition(FilterCondition {
        column: column.to_owned(),
        operator,
        value,
    })
}

fn sort(column: &str, direction: SortDirection) -> Sort {
    Sort {
        column: column.to_owned(),
        direction,
        nulls: None,
    }
}

#[test]
fn builds_filtered_sorted_paginated_query() {
    let mut options = blank(table("people"));
    options.columns = vec![
        SelectColumn::Name("id".to_owned()),
        SelectColumn::Name("name".to_owned()),
    ];
    options.filters = vec![condition("age", FilterOperator::Gte, Some(Value::Int(18)))];
    options.sort = vec![sort("name", SortDirection::Asc)];
    options.limit = Some(2);
    options.offset = Some(1);

    let query = build_query(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"SELECT "id", "name" FROM "people" WHERE "age" >= $1 ORDER BY "name" ASC LIMIT 2 OFFSET 1"#
    );
    assert_eq!(query.params, vec![Value::Int(18)]);
}

#[test]
fn combines_top_level_and_with_nested_or_groups() {
    let mut options = blank(table("people"));
    options.filters = vec![
        condition(
            "name",
            FilterOperator::ILike,
            Some(Value::Text("a%".to_owned())),
        ),
        Filter::Group(FilterGroup {
            combinator: Combinator::Or,
            filters: vec![
                condition(
                    "status",
                    FilterOperator::In,
                    Some(Value::Array(vec![
                        Value::Text("active".to_owned()),
                        Value::Text("pending".to_owned()),
                    ])),
                ),
                condition("deleted_at", FilterOperator::IsNull, None),
            ],
        }),
    ];
    options.sort = vec![sort("id", SortDirection::Asc)];

    let query = build_query(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"SELECT * FROM "people" WHERE "name" ILIKE $1 AND ("status" IN ($2, $3) OR "deleted_at" IS NULL) ORDER BY "id" ASC"#
    );
    assert_eq!(
        query.params,
        vec![
            Value::Text("a%".to_owned()),
            Value::Text("active".to_owned()),
            Value::Text("pending".to_owned())
        ]
    );
}

#[test]
fn treats_eq_and_neq_null_as_is_null_and_is_not_null() {
    let mut options = blank(table("people"));
    options.columns = vec![SelectColumn::Name("id".to_owned())];
    options.filters = vec![condition("deleted_at", FilterOperator::Eq, None)];
    options.sort = vec![sort("id", SortDirection::Asc)];

    let query = build_query(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"SELECT "id" FROM "people" WHERE "deleted_at" IS NULL ORDER BY "id" ASC"#
    );
    assert!(query.params.is_empty());

    options.filters = vec![condition(
        "deleted_at",
        FilterOperator::Neq,
        Some(Value::Null),
    )];
    let query = build_query(&options).unwrap();
    assert_eq!(
        query.sql,
        r#"SELECT "id" FROM "people" WHERE "deleted_at" IS NOT NULL ORDER BY "id" ASC"#
    );
}

#[test]
fn builds_between_filters() {
    let mut options = blank(table("people"));
    options.columns = vec![
        SelectColumn::Name("id".to_owned()),
        SelectColumn::Name("score".to_owned()),
    ];
    options.filters = vec![condition(
        "score",
        FilterOperator::Between,
        Some(Value::Array(vec![Value::Int(70), Value::Int(90)])),
    )];
    options.sort = vec![sort("score", SortDirection::Desc)];

    let query = build_query(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"SELECT "id", "score" FROM "people" WHERE "score" BETWEEN $1 AND $2 ORDER BY "score" DESC"#
    );
    assert_eq!(query.params, vec![Value::Int(70), Value::Int(90)]);
}

#[test]
fn uses_constants_for_empty_in_lists() {
    let mut options = blank(table("people"));
    options.filters = vec![condition(
        "id",
        FilterOperator::In,
        Some(Value::Array(Vec::new())),
    )];

    let query = build_query(&options).unwrap();
    assert_eq!(query.sql, r#"SELECT * FROM "people" WHERE FALSE"#);
    assert!(query.params.is_empty());

    options.filters = vec![condition(
        "id",
        FilterOperator::NotIn,
        Some(Value::Array(Vec::new())),
    )];

    let query = build_query(&options).unwrap();
    assert_eq!(query.sql, r#"SELECT * FROM "people" WHERE TRUE"#);
    assert!(query.params.is_empty());
}

#[test]
fn passes_id_array_as_one_any_parameter() {
    let ids = Value::Array(vec![Value::Int(1), Value::Int(3)]);

    let mut options = blank(table("people"));
    options.columns = vec![SelectColumn::Name("id".to_owned())];
    options.filters = vec![condition("id", FilterOperator::Any, Some(ids.clone()))];
    options.sort = vec![sort("id", SortDirection::Asc)];

    let query = build_query(&options).unwrap();
    assert_eq!(
        query.sql,
        r#"SELECT "id" FROM "people" WHERE "id" = ANY($1) ORDER BY "id" ASC"#
    );
    assert_eq!(query.params, vec![ids.clone()]);

    options.filters = vec![condition("id", FilterOperator::NotAny, Some(ids.clone()))];
    let query = build_query(&options).unwrap();
    assert_eq!(
        query.sql,
        r#"SELECT "id" FROM "people" WHERE "id" <> ALL($1) ORDER BY "id" ASC"#
    );
    assert_eq!(query.params, vec![ids]);
}

#[test]
fn offsets_outer_parameters_after_parameterised_subquery() {
    let mut options = blank(QuerySource::Subquery {
        subquery: SubquerySource {
            sql: r#"SELECT "id", "name", "age" FROM "people" WHERE "age" >= $1"#.to_owned(),
            alias: "adults".to_owned(),
            params: vec![Value::Int(18)],
        },
    });
    options.filters = vec![condition(
        "name",
        FilterOperator::Like,
        Some(Value::Text("C%".to_owned())),
    )];
    options.sort = vec![sort("id", SortDirection::Asc)];
    options.limit = Some(5);

    let query = build_query(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"SELECT * FROM (SELECT "id", "name", "age" FROM "people" WHERE "age" >= $1) AS "adults" WHERE "name" LIKE $2 ORDER BY "id" ASC LIMIT 5"#
    );
    assert_eq!(
        query.params,
        vec![Value::Int(18), Value::Text("C%".to_owned())]
    );
}

#[test]
fn supports_schema_qualified_tables_column_aliases_and_nulls_ordering() {
    let mut options = blank(QuerySource::Table {
        schema: Some("public".to_owned()),
        name: "people".to_owned(),
    });
    options.columns = vec![SelectColumn::Aliased {
        name: "first_name".to_owned(),
        alias: Some("firstName".to_owned()),
    }];
    options.filters = vec![condition("id", FilterOperator::Eq, Some(Value::Int(1)))];
    options.sort = vec![Sort {
        column: "name".to_owned(),
        direction: SortDirection::Desc,
        nulls: Some(NullsOrder::Last),
    }];

    let query = build_query(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"SELECT "first_name" AS "firstName" FROM "public"."people" WHERE "id" = $1 ORDER BY "name" DESC NULLS LAST"#
    );
    assert_eq!(query.params, vec![Value::Int(1)]);
}

#[test]
fn uses_constants_for_empty_groups() {
    let mut options = blank(table("people"));
    options.filters = vec![Filter::Group(FilterGroup {
        combinator: Combinator::And,
        filters: Vec::new(),
    })];

    let query = build_query(&options).unwrap();
    assert_eq!(query.sql, r#"SELECT * FROM "people" WHERE TRUE"#);

    options.filters = vec![Filter::Group(FilterGroup {
        combinator: Combinator::Or,
        filters: Vec::new(),
    })];

    let query = build_query(&options).unwrap();
    assert_eq!(query.sql, r#"SELECT * FROM "people" WHERE FALSE"#);
}

#[test]
fn rejects_identifiers_that_are_not_plain_sql_names() {
    let options = blank(QuerySource::Table {
        schema: None,
        name: "people; DROP TABLE people".to_owned(),
    });
    assert!(matches!(
        build_query(&options),
        Err(QueryError::InvalidIdentifier { kind: "table", .. })
    ));

    let mut options = blank(table("people"));
    options.columns = vec![SelectColumn::Name("name\" OR 1=1 --".to_owned())];
    assert!(matches!(
        build_query(&options),
        Err(QueryError::InvalidIdentifier { kind: "column", .. })
    ));

    let mut options = blank(table("people"));
    options.sort = vec![sort("bad-name", SortDirection::Asc)];
    assert!(matches!(
        build_query(&options),
        Err(QueryError::InvalidIdentifier {
            kind: "sort column",
            ..
        })
    ));

    let options = blank(QuerySource::Subquery {
        subquery: SubquerySource {
            sql: r#"SELECT * FROM "people""#.to_owned(),
            alias: "bad alias".to_owned(),
            params: Vec::new(),
        },
    });
    assert!(matches!(
        build_query(&options),
        Err(QueryError::InvalidIdentifier {
            kind: "subquery alias",
            ..
        })
    ));
}

#[test]
fn rejects_invalid_filter_values() {
    let mut options = blank(table("people"));
    options.filters = vec![condition("age", FilterOperator::Gt, None)];
    assert!(matches!(
        build_query(&options),
        Err(QueryError::InvalidFilterValue { .. })
    ));

    options.filters = vec![condition("age", FilterOperator::In, Some(Value::Int(18)))];
    assert!(matches!(
        build_query(&options),
        Err(QueryError::InvalidFilterValue { .. })
    ));

    options.filters = vec![condition(
        "age",
        FilterOperator::Between,
        Some(Value::Array(vec![Value::Int(18)])),
    )];
    assert!(matches!(
        build_query(&options),
        Err(QueryError::InvalidFilterValue { .. })
    ));
}

#[test]
fn rejects_an_empty_subquery() {
    let options = blank(QuerySource::Subquery {
        subquery: SubquerySource {
            sql: "   ".to_owned(),
            alias: "source".to_owned(),
            params: Vec::new(),
        },
    });

    assert_eq!(build_query(&options), Err(QueryError::EmptySubquery));
}

#[test]
fn deserializes_typescript_table_options() {
    let json = r#"
    {
      "table": "people",
      "columns": ["id", {"name": "first_name", "alias": "firstName"}],
      "filters": [
        {"column": "age", "operator": "gte", "value": 18},
        {
          "combinator": "or",
          "filters": [
            {"column": "status", "operator": "in", "value": ["active", "pending"]},
            {"column": "deleted_at", "operator": "isNull"}
          ]
        }
      ],
      "sort": [{"column": "name"}],
      "limit": 50,
      "offset": 0
    }
    "#;

    let options: QueryOptions = serde_json::from_str(json).unwrap();

    assert_eq!(
        options.source,
        QuerySource::Table {
            schema: None,
            name: "people".to_owned(),
        }
    );
    assert_eq!(
        options.columns,
        vec![
            SelectColumn::Name("id".to_owned()),
            SelectColumn::Aliased {
                name: "first_name".to_owned(),
                alias: Some("firstName".to_owned()),
            },
        ]
    );
    assert_eq!(
        options.sort,
        vec![Sort {
            column: "name".to_owned(),
            direction: SortDirection::Asc,
            nulls: None,
        }]
    );
    assert_eq!(options.limit, Some(50));
    assert_eq!(options.offset, Some(0));

    let query = build_query(&options).unwrap();
    assert_eq!(
        query.sql,
        r#"SELECT "id", "first_name" AS "firstName" FROM "people" WHERE "age" >= $1 AND ("status" IN ($2, $3) OR "deleted_at" IS NULL) ORDER BY "name" ASC LIMIT 50 OFFSET 0"#
    );
    assert_eq!(
        query.params,
        vec![
            Value::Int(18),
            Value::Text("active".to_owned()),
            Value::Text("pending".to_owned()),
        ]
    );
}

#[test]
fn deserializes_typescript_subquery_options() {
    let json = r#"
    {
      "subquery": {
        "sql": "SELECT \"id\", \"name\" FROM \"people\" WHERE \"age\" >= $1",
        "alias": "adults",
        "params": [18]
      },
      "filters": [{"column": "id", "operator": "any", "value": [1, 3]}],
      "limit": 5
    }
    "#;

    let options: QueryOptions = serde_json::from_str(json).unwrap();

    assert_eq!(
        options.source,
        QuerySource::Subquery {
            subquery: SubquerySource {
                sql: r#"SELECT "id", "name" FROM "people" WHERE "age" >= $1"#.to_owned(),
                alias: "adults".to_owned(),
                params: vec![Value::Int(18)],
            },
        }
    );
    assert!(options.columns.is_empty());
    assert!(options.sort.is_empty());
    assert_eq!(options.offset, None);

    let query = build_query(&options).unwrap();
    assert_eq!(
        query.sql,
        r#"SELECT * FROM (SELECT "id", "name" FROM "people" WHERE "age" >= $1) AS "adults" WHERE "id" = ANY($2) LIMIT 5"#
    );
    assert_eq!(
        query.params,
        vec![
            Value::Int(18),
            Value::Array(vec![Value::Int(1), Value::Int(3)]),
        ]
    );
}

#[test]
fn serializes_typescript_compatible_shape() {
    let options = QueryOptions {
        source: QuerySource::Table {
            schema: Some("public".to_owned()),
            name: "people".to_owned(),
        },
        columns: vec![
            SelectColumn::Name("id".to_owned()),
            SelectColumn::Aliased {
                name: "first_name".to_owned(),
                alias: Some("firstName".to_owned()),
            },
            SelectColumn::Aliased {
                name: "last_name".to_owned(),
                alias: None,
            },
        ],
        filters: vec![
            Filter::Condition(FilterCondition {
                column: "age".to_owned(),
                operator: FilterOperator::Gte,
                value: Some(Value::Int(18)),
            }),
            Filter::Group(FilterGroup {
                combinator: Combinator::Or,
                filters: vec![Filter::Condition(FilterCondition {
                    column: "status".to_owned(),
                    operator: FilterOperator::NotILike,
                    value: Some(Value::Text("archived%".to_owned())),
                })],
            }),
        ],
        sort: vec![Sort {
            column: "name".to_owned(),
            direction: SortDirection::Desc,
            nulls: Some(NullsOrder::Last),
        }],
        limit: Some(25),
        offset: None,
    };

    let json = serde_json::to_value(&options).unwrap();

    assert_eq!(json["table"], "people");
    assert_eq!(json["schema"], "public");
    assert!(json.get("subquery").is_none());
    assert_eq!(json["columns"][0], "id");
    assert_eq!(
        json["columns"][1],
        serde_json::json!({"name": "first_name", "alias": "firstName"})
    );
    assert_eq!(json["columns"][2], serde_json::json!({"name": "last_name"}));
    assert_eq!(json["filters"][0]["operator"], "gte");
    assert_eq!(json["filters"][1]["combinator"], "or");
    assert_eq!(json["filters"][1]["filters"][0]["operator"], "notIlike");
    assert_eq!(json["sort"][0]["direction"], "desc");
    assert_eq!(json["sort"][0]["nulls"], "last");
    assert!(json.get("offset").is_none());

    let round_trip: QueryOptions = serde_json::from_value(json).unwrap();
    assert_eq!(round_trip, options);
}

#[test]
fn serializes_subquery_shape() {
    let options = QueryOptions {
        source: QuerySource::Subquery {
            subquery: SubquerySource {
                sql: "SELECT 1".to_owned(),
                alias: "source".to_owned(),
                params: vec![Value::Bool(true)],
            },
        },
        columns: Vec::new(),
        filters: Vec::new(),
        sort: Vec::new(),
        limit: None,
        offset: None,
    };

    let json = serde_json::to_value(&options).unwrap();

    assert!(json.get("table").is_none());
    assert_eq!(json["subquery"]["sql"], "SELECT 1");
    assert_eq!(json["subquery"]["alias"], "source");
    assert_eq!(json["subquery"]["params"][0], true);

    let round_trip: QueryOptions = serde_json::from_value(json).unwrap();
    assert_eq!(round_trip, options);
}

#[test]
fn deserializes_arbitrary_json_filter_values() {
    let json = r#"
    {
      "table": "events",
      "filters": [
        {"column": "score", "operator": "eq", "value": 1.5},
        {"column": "payload", "operator": "eq", "value": {"active": true, "tags": [1, null]}}
      ]
    }
    "#;

    let options: QueryOptions = serde_json::from_str(json).unwrap();

    assert_eq!(
        options.filters[0],
        Filter::Condition(FilterCondition {
            column: "score".to_owned(),
            operator: FilterOperator::Eq,
            value: Some(Value::Float(1.5)),
        })
    );
    assert_eq!(
        options.filters[1],
        Filter::Condition(FilterCondition {
            column: "payload".to_owned(),
            operator: FilterOperator::Eq,
            value: Some(Value::Json(serde_json::json!({
                "active": true,
                "tags": [1, null],
            }))),
        })
    );
}

#[test]
fn serializes_operator_names_like_typescript() {
    let cases = [
        (FilterOperator::Eq, "eq"),
        (FilterOperator::Neq, "neq"),
        (FilterOperator::Gt, "gt"),
        (FilterOperator::Gte, "gte"),
        (FilterOperator::Lt, "lt"),
        (FilterOperator::Lte, "lte"),
        (FilterOperator::Like, "like"),
        (FilterOperator::ILike, "ilike"),
        (FilterOperator::NotLike, "notLike"),
        (FilterOperator::NotILike, "notIlike"),
        (FilterOperator::In, "in"),
        (FilterOperator::NotIn, "notIn"),
        (FilterOperator::Any, "any"),
        (FilterOperator::NotAny, "notAny"),
        (FilterOperator::IsNull, "isNull"),
        (FilterOperator::IsNotNull, "isNotNull"),
        (FilterOperator::Between, "between"),
        (FilterOperator::NotBetween, "notBetween"),
    ];

    for (operator, expected) in cases {
        assert_eq!(
            serde_json::to_value(operator).unwrap(),
            serde_json::Value::String(expected.to_owned())
        );
        let round_trip: FilterOperator =
            serde_json::from_value(serde_json::json!(expected)).unwrap();
        assert_eq!(round_trip, operator);
    }
}
