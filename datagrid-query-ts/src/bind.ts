import { QueryError } from "./shared.js";
import type { BuiltMutation, MutationParamSpec, MutationRow } from "./types.js";

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

function bindSpec(spec: MutationParamSpec, rows: readonly MutationRow[]): unknown[] {
  switch (spec.kind) {
    case "value":
      return rows.map((row) => rowValue(row, spec.column));
    case "present":
      return rows.map((row) => rowHasValue(row, spec.column));
    case "required":
      return rows.map((row) => requiredRowValue(row, spec.column));
    default:
      throw new QueryError(`Unsupported parameter kind: ${String(spec.kind)}`);
  }
}

export function bindMutationRows(
  mutation: BuiltMutation,
  rows: readonly MutationRow[],
): unknown[] {
  const allowed = new Set(mutation.columns);
  for (const row of rows) {
    for (const key of Object.keys(row)) {
      if (!allowed.has(key)) {
        throw new QueryError(`Unknown row column: ${JSON.stringify(key)}`);
      }
    }
  }

  return mutation.paramSpecs.map((spec) => bindSpec(spec, rows));
}
