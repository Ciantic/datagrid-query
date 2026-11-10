import { QueryError, pushParam, quoteIdentifier, validateSqlType } from "./shared.js";
import type { MutationColumn, MutationOptions, MutationRow } from "./types.js";

export interface MutationContext {
  table: string;
  tableName: string;
  columns: Map<string, MutationColumn>;
  primaryKey: string[];
  versionColumn: string | undefined;
  returning: string[];
}

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

  for (const row of options.rows) {
    for (const key of Object.keys(row)) {
      if (!columns.has(key)) {
        throw new QueryError(`Unknown row column: ${JSON.stringify(key)}`);
      }
    }
  }

  return {
    table,
    tableName,
    columns,
    primaryKey: [...options.primaryKey],
    versionColumn: options.versionColumn,
    returning,
  };
}

export function mutationColumn(context: MutationContext, name: string): MutationColumn {
  const column = context.columns.get(name);
  if (column === undefined) {
    throw new QueryError(`Unknown column: ${JSON.stringify(name)}`);
  }

  return column;
}

function rowHasValue(row: MutationRow, column: string): boolean {
  return Object.prototype.hasOwnProperty.call(row, column) && row[column] !== undefined;
}

function rowValue(row: MutationRow, column: string): unknown {
  return rowHasValue(row, column) ? row[column] : null;
}

function requiredRowValue(row: MutationRow, column: string): unknown {
  if (!rowHasValue(row, column) || row[column] === null) {
    throw new QueryError(`Row is missing required value for column: ${JSON.stringify(column)}`);
  }

  return row[column];
}

export function valueArray(rows: readonly MutationRow[], column: string): unknown[] {
  return rows.map((row) => rowValue(row, column));
}

export function presentArray(rows: readonly MutationRow[], column: string): boolean[] {
  return rows.map((row) => rowHasValue(row, column));
}

export function requiredArray(rows: readonly MutationRow[], column: string): unknown[] {
  return rows.map((row) => requiredRowValue(row, column));
}

export function pushTypedArray(params: unknown[], values: unknown[], type: string): string {
  return `${pushParam(params, values)}::${type}[]`;
}

export function renderMutationReturning(context: MutationContext, qualifier: string): string {
  if (context.returning.length === 0) {
    return "";
  }

  const columns = context.returning.map(
    (name) => `${qualifier}.${quoteIdentifier(name, "returning column")}`,
  );
  return ` RETURNING ${columns.join(", ")}`;
}

export function renderInsertSelect(
  options: MutationOptions,
  context: MutationContext,
  params: unknown[],
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
    placeholders.push(pushTypedArray(params, valueArray(options.rows, column.name), column.type));
    columnNames.push(quoted);
    aliases.push(quoted);

    if (column.insertDefaultSql !== undefined) {
      const presentAlias = `"${column.name}#present"`;
      placeholders.push(pushTypedArray(params, presentArray(options.rows, column.name), "bool"));
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
