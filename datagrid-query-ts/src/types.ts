export type FilterOperator =
  | "eq"
  | "neq"
  | "gt"
  | "gte"
  | "lt"
  | "lte"
  | "like"
  | "ilike"
  | "notLike"
  | "notIlike"
  | "in"
  | "notIn"
  | "any"
  | "notAny"
  | "isNull"
  | "isNotNull"
  | "between"
  | "notBetween";

export interface FilterCondition {
  column: string;
  operator: FilterOperator;
  value?: unknown;
}

export interface FilterGroup {
  combinator: "and" | "or";
  filters: readonly FilterNode[];
}

export type FilterNode = FilterCondition | FilterGroup;

export type SortDirection = "asc" | "desc";
export type NullsOrder = "first" | "last";

export interface SortSpec {
  column: string;
  direction?: SortDirection;
  nulls?: NullsOrder;
}

export type SelectColumn = string | { name: string; alias?: string };

export interface SubquerySource {
  sql: string;
  alias: string;
  params?: readonly unknown[];
}

interface CommonQueryOptions {
  columns?: readonly SelectColumn[];
  filters?: readonly FilterNode[];
  sort?: readonly SortSpec[];
  limit?: number;
  offset?: number;
}

export type QueryOptions = CommonQueryOptions &
  (
    | { table: string; schema?: string; subquery?: never }
    | { table?: never; schema?: never; subquery: SubquerySource }
  );

export interface BuiltQuery {
  sql: string;
  params: unknown[];
}

export interface MutationColumn {
  name: string;
  type: string;
  insertable?: boolean;
  updatable?: boolean;
  insertDefaultSql?: string;
  updateDefaultSql?: string;
}

export type MutationRow = Record<string, unknown>;

export interface MutationOptions {
  table: string;
  schema?: string;
  columns: readonly MutationColumn[];
  primaryKey: readonly string[];
  versionColumn?: string;
  returning?: readonly string[];
  rows: readonly MutationRow[];
}
