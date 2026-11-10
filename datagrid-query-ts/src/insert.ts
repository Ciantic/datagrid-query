import {
  mutationContext,
  renderInsertSelect,
  renderMutationReturning,
} from "./mutation.js";
import type { BuiltQuery, MutationOptions } from "./types.js";

export function buildInsert(options: MutationOptions): BuiltQuery {
  const context = mutationContext(options);
  const params: unknown[] = [];
  const sql = `${renderInsertSelect(options, context, params, undefined)}${renderMutationReturning(context, context.tableName)}`;

  return { sql, params };
}
