use datagrid_query_rs::{
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

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
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
        condition("name", FilterOperator::ILike, Some(text("a%"))),
        Filter::Group(FilterGroup {
            combinator: Combinator::Or,
            filters: vec![
                condition(
                    "status",
                    FilterOperator::In,
                    Some(Value::Array(vec![text("active"), text("pending")])),
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
        vec![text("a%"), text("active"), text("pending")]
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
    options.filters = vec![condition("name", FilterOperator::Like, Some(text("C%")))];
    options.sort = vec![sort("id", SortDirection::Asc)];
    options.limit = Some(5);

    let query = build_query(&options).unwrap();

    assert_eq!(
        query.sql,
        r#"SELECT * FROM (SELECT "id", "name", "age" FROM "people" WHERE "age" >= $1) AS "adults" WHERE "name" LIKE $2 ORDER BY "id" ASC LIMIT 5"#
    );
    assert_eq!(query.params, vec![Value::Int(18), text("C%")]);
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
