import { QueryError, pushParam, quoteIdentifier } from "./shared.js";
import type {
  BuiltQuery,
  FilterCondition,
  FilterGroup,
  FilterNode,
  QueryOptions,
} from "./types.js";

function requireValue(value: unknown, operator: string): unknown {
  if (value === undefined || value === null) {
    throw new QueryError(`Operator "${operator}" requires a value`);
  }

  return value;
}

function requireArray(value: unknown, operator: string): readonly unknown[] {
  if (!Array.isArray(value)) {
    throw new QueryError(`Operator "${operator}" requires an array value`);
  }

  return value;
}

function isFilterGroup(node: FilterNode): node is FilterGroup {
  return "combinator" in node;
}

function renderFilter(node: FilterNode, params: unknown[]): string {
  if (isFilterGroup(node)) {
    if (node.combinator !== "and" && node.combinator !== "or") {
      throw new QueryError(`Invalid filter combinator: ${JSON.stringify(node.combinator)}`);
    }

    if (node.filters.length === 0) {
      return node.combinator === "and" ? "TRUE" : "FALSE";
    }

    const joiner = node.combinator === "and" ? " AND " : " OR ";
    const rendered = node.filters.map((child) => renderFilter(child, params));
    return `(${rendered.join(joiner)})`;
  }

  return renderCondition(node, params);
}

function renderCondition(condition: FilterCondition, params: unknown[]): string {
  const column = quoteIdentifier(condition.column, "filter column");
  const operator = condition.operator;

  switch (operator) {
    case "eq":
    case "neq": {
      if (condition.value === undefined || condition.value === null) {
        return `${column} ${operator === "eq" ? "IS NULL" : "IS NOT NULL"}`;
      }

      const sqlOperator = operator === "eq" ? "=" : "<>";
      return `${column} ${sqlOperator} ${pushParam(params, condition.value)}`;
    }
    case "gt":
      return `${column} > ${pushParam(params, requireValue(condition.value, operator))}`;
    case "gte":
      return `${column} >= ${pushParam(params, requireValue(condition.value, operator))}`;
    case "lt":
      return `${column} < ${pushParam(params, requireValue(condition.value, operator))}`;
    case "lte":
      return `${column} <= ${pushParam(params, requireValue(condition.value, operator))}`;
    case "like":
      return `${column} LIKE ${pushParam(params, requireValue(condition.value, operator))}`;
    case "ilike":
      return `${column} ILIKE ${pushParam(params, requireValue(condition.value, operator))}`;
    case "notLike":
      return `${column} NOT LIKE ${pushParam(params, requireValue(condition.value, operator))}`;
    case "notIlike":
      return `${column} NOT ILIKE ${pushParam(params, requireValue(condition.value, operator))}`;
    case "in":
    case "notIn": {
      const values = requireArray(condition.value, operator);
      if (values.length === 0) {
        return operator === "in" ? "FALSE" : "TRUE";
      }

      const placeholders = values.map((value) => pushParam(params, value));
      return `${column} ${operator === "in" ? "IN" : "NOT IN"} (${placeholders.join(", ")})`;
    }
    case "any":
    case "notAny": {
      const values = requireArray(condition.value, operator);
      const placeholder = pushParam(params, values);
      return `${column} ${operator === "any" ? "= ANY" : "<> ALL"}(${placeholder})`;
    }
    case "isNull":
      return `${column} IS NULL`;
    case "isNotNull":
      return `${column} IS NOT NULL`;
    case "between":
    case "notBetween": {
      const values = requireArray(condition.value, operator);
      if (values.length !== 2) {
        throw new QueryError(`Operator "${operator}" requires exactly two values`);
      }

      const [lower, upper] = values as [unknown, unknown];
      const sqlOperator = operator === "between" ? "BETWEEN" : "NOT BETWEEN";
      return `${column} ${sqlOperator} ${pushParam(params, lower)} AND ${pushParam(params, upper)}`;
    }
    default:
      throw new QueryError(`Unsupported filter operator: ${String(operator)}`);
  }
}

function renderSource(options: QueryOptions, params: unknown[]): string {
  if ("subquery" in options && options.subquery !== undefined) {
    if (options.table !== undefined) {
      throw new QueryError("Exactly one of table or subquery must be provided");
    }

    const { sql, alias } = options.subquery;
    if (sql.trim().length === 0) {
      throw new QueryError("Subquery SQL must not be empty");
    }

    const quotedAlias = quoteIdentifier(alias, "subquery alias");
    if (options.subquery.params !== undefined) {
      params.push(...options.subquery.params);
    }

    return `(${sql}) AS ${quotedAlias}`;
  }

  if (options.table !== undefined) {
    const table = quoteIdentifier(options.table, "table");
    if (options.schema === undefined) {
      return table;
    }

    return `${quoteIdentifier(options.schema, "schema")}.${table}`;
  }

  throw new QueryError("Exactly one of table or subquery must be provided");
}

function integerLiteral(value: number | undefined, name: string): string | undefined {
  if (value === undefined) {
    return undefined;
  }

  if (!Number.isSafeInteger(value) || value < 0) {
    throw new QueryError(`${name} must be a non-negative safe integer`);
  }

  return String(value);
}

export function buildQuery(options: QueryOptions): BuiltQuery {
  const params: unknown[] = [];
  const from = renderSource(options, params);

  const columns = options.columns ?? [];
  const selectList =
    columns.length === 0
      ? "*"
      : columns
          .map((column) => {
            if (typeof column === "string") {
              return quoteIdentifier(column, "column");
            }

            const name = quoteIdentifier(column.name, "column");
            if (column.alias === undefined) {
              return name;
            }

            return `${name} AS ${quoteIdentifier(column.alias, "column alias")}`;
          })
          .join(", ");

  const parts = [`SELECT ${selectList}`, `FROM ${from}`];

  const filters = options.filters ?? [];
  if (filters.length > 0) {
    const where = filters.map((filter) => renderFilter(filter, params)).join(" AND ");
    parts.push(`WHERE ${where}`);
  }

  const sort = options.sort ?? [];
  if (sort.length > 0) {
    const orderBy = sort
      .map((spec) => {
        const column = quoteIdentifier(spec.column, "sort column");
        const direction = spec.direction ?? "asc";
        if (direction !== "asc" && direction !== "desc") {
          throw new QueryError(`Invalid sort direction: ${JSON.stringify(direction)}`);
        }

        let clause = `${column} ${direction.toUpperCase()}`;
        if (spec.nulls !== undefined) {
          if (spec.nulls !== "first" && spec.nulls !== "last") {
            throw new QueryError(`Invalid nulls order: ${JSON.stringify(spec.nulls)}`);
          }

          clause += ` NULLS ${spec.nulls.toUpperCase()}`;
        }

        return clause;
      })
      .join(", ");
    parts.push(`ORDER BY ${orderBy}`);
  }

  const limit = integerLiteral(options.limit, "limit");
  if (limit !== undefined) {
    parts.push(`LIMIT ${limit}`);
  }

  const offset = integerLiteral(options.offset, "offset");
  if (offset !== undefined) {
    parts.push(`OFFSET ${offset}`);
  }

  return { sql: parts.join(" "), params };
}
