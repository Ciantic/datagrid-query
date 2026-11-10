import {
  mutationContext,
  renderInsertSelect,
  renderMutationReturning,
} from "./mutation.js";
import type { BuiltMutation, MutationOptions, MutationParamSpec } from "./types.js";

export function buildInsert(options: MutationOptions): BuiltMutation {
  const context = mutationContext(options);
  const paramSpecs: MutationParamSpec[] = [];
  const sql = `${renderInsertSelect(options, context, paramSpecs, undefined)}${renderMutationReturning(context, context.tableName)}`;

  return { sql, columns: context.columnNames, paramSpecs };
}
