import {
  mutationColumn,
  mutationContext,
  pushTypedArray,
  renderMutationReturning,
  requiredArray,
} from "./mutation.js";
import { quoteIdentifier } from "./shared.js";
import type { BuiltQuery, MutationOptions } from "./types.js";

export function buildDelete(options: MutationOptions): BuiltQuery {
  const context = mutationContext(options);
  const params: unknown[] = [];
  const aliases: string[] = [];
  const placeholders: string[] = [];
  const where: string[] = [];

  for (const key of context.primaryKey) {
    const column = mutationColumn(context, key);
    const quoted = quoteIdentifier(key, "primary key column");
    placeholders.push(pushTypedArray(params, requiredArray(options.rows, key), column.type));
    aliases.push(quoted);
    where.push(`u.${quoted} = v.${quoted}`);
  }

  if (context.versionColumn !== undefined) {
    const column = mutationColumn(context, context.versionColumn);
    const quoted = quoteIdentifier(context.versionColumn, "version column");
    placeholders.push(
      pushTypedArray(params, requiredArray(options.rows, context.versionColumn), column.type),
    );
    aliases.push(quoted);
    where.push(`u.${quoted} = v.${quoted}`);
  }

  const sql = `DELETE FROM ${context.table} AS u USING unnest(${placeholders.join(", ")}) AS v(${aliases.join(", ")}) WHERE ${where.join(" AND ")}${renderMutationReturning(context, "u")}`;
  return { sql, params };
}
