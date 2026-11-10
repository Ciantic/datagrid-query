import { QueryError, quoteIdentifier, validateSqlType } from "./shared.js";
import type {
  MutationColumn,
  MutationOptions,
  MutationParamKind,
  MutationParamSpec,
} from "./types.js";

/** @internal */
export interface MutationContext {
  table: string;
  tableName: string;
  columns: Map<string, MutationColumn>;
  columnNames: string[];
  primaryKey: string[];
  versionColumn: string | undefined;
  returning: string[];
}

/** @internal */
export function mutationContext(options: MutationOptions): MutationContext {
  const tableName = quoteIdentifier(options.table, "table");
  const table =
    options.schema === undefined
      ? tableName
      : `${quoteIdentifier(options.schema, "schema")}.${tableName}`;

  const columns = new Map<string, MutationColumn>();
  for (const column of options.columns) {
    quoteIdentifier(column.name, "column");
    validateSqlType(column.type);
    if (columns.has(column.name)) {
      throw new QueryError(`Duplicate column: ${JSON.stringify(column.name)}`);
    }

    if (column.insertDefaultSql !== undefined && column.insertDefaultSql.trim().length === 0) {
      throw new QueryError(
        `Insert default SQL must not be empty for column: ${JSON.stringify(column.name)}`,
      );
    }

    if (column.updateDefaultSql !== undefined && column.updateDefaultSql.trim().length === 0) {
      throw new QueryError(
        `Update default SQL must not be empty for column: ${JSON.stringify(column.name)}`,
      );
    }

    columns.set(column.name, column);
  }

  if (options.primaryKey.length === 0) {
    throw new QueryError("Primary key must contain at least one column");
  }

  const seenPrimaryKey = new Set<string>();
  for (const key of options.primaryKey) {
    quoteIdentifier(key, "primary key column");
    if (seenPrimaryKey.has(key)) {
      throw new QueryError(`Duplicate primary key column: ${JSON.stringify(key)}`);
    }

    if (!columns.has(key)) {
      throw new QueryError(`Unknown primary key column: ${JSON.stringify(key)}`);
    }

    seenPrimaryKey.add(key);
  }

  if (options.versionColumn !== undefined) {
    quoteIdentifier(options.versionColumn, "version column");
    if (!columns.has(options.versionColumn)) {
      throw new QueryError(`Unknown version column: ${JSON.stringify(options.versionColumn)}`);
    }

    if (seenPrimaryKey.has(options.versionColumn)) {
      throw new QueryError(
        `Version column must not be part of the primary key: ${JSON.stringify(options.versionColumn)}`,
      );
    }
  }

  const returning = [...(options.returning ?? options.primaryKey)];
  for (const name of returning) {
    quoteIdentifier(name, "returning column");
    if (!columns.has(name)) {
      throw new QueryError(`Unknown returning column: ${JSON.stringify(name)}`);
    }
  }

  return {
    table,
    tableName,
    columns,
    columnNames: options.columns.map((column) => column.name),
    primaryKey: [...options.primaryKey],
    versionColumn: options.versionColumn,
    returning,
  };
}

/** @internal */
export function mutationColumn(context: MutationContext, name: string): MutationColumn {
  const column = context.columns.get(name);
  if (column === undefined) {
    throw new QueryError(`Unknown column: ${JSON.stringify(name)}`);
  }

  return column;
}

/** @internal */
export function pushParamSpec(
  specs: MutationParamSpec[],
  kind: MutationParamKind,
  column: string,
  type: string,
): string {
  specs.push({ kind, column });
  return `$${specs.length}::${type}[]`;
}

/** @internal */
export function renderMutationReturning(context: MutationContext, qualifier: string): string {
  if (context.returning.length === 0) {
    return "";
  }

  const columns = context.returning.map(
    (name) => `${qualifier}.${quoteIdentifier(name, "returning column")}`,
  );
  return ` RETURNING ${columns.join(", ")}`;
}

/** @internal */
export function renderInsertSelect(
  options: MutationOptions,
  context: MutationContext,
  paramSpecs: MutationParamSpec[],
  alias: string | undefined,
): string {
  const columns = options.columns.filter((column) => column.insertable !== false);
  if (columns.length === 0) {
    throw new QueryError("Insert requires at least one insertable column");
  }

  const columnNames: string[] = [];
  const selectExpressions: string[] = [];
  const aliases: string[] = [];
  const placeholders: string[] = [];

  for (const column of columns) {
    const quoted = quoteIdentifier(column.name, "column");
    placeholders.push(pushParamSpec(paramSpecs, "value", column.name, column.type));
    columnNames.push(quoted);
    aliases.push(quoted);

    if (column.insertDefaultSql !== undefined) {
      const presentAlias = `"${column.name}#present"`;
      placeholders.push(pushParamSpec(paramSpecs, "present", column.name, "bool"));
      aliases.push(presentAlias);
      selectExpressions.push(
        `CASE WHEN v.${presentAlias} THEN v.${quoted} ELSE ${column.insertDefaultSql} END`,
      );
    } else {
      selectExpressions.push(`v.${quoted}`);
    }
  }

  const target = alias === undefined ? context.table : `${context.table} AS ${alias}`;
  return `INSERT INTO ${target} (${columnNames.join(", ")}) SELECT ${selectExpressions.join(", ")} FROM unnest(${placeholders.join(", ")}) AS v(${aliases.join(", ")})`;
}
