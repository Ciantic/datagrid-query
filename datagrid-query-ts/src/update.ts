import {
  mutationColumn,
  mutationContext,
  pushParamSpec,
  renderMutationReturning,
} from "./mutation.js";
import { QueryError, quoteIdentifier } from "./shared.js";
import type { BuiltMutation, MutationOptions, MutationParamSpec } from "./types.js";

export function buildUpdate(options: MutationOptions): BuiltMutation {
  const context = mutationContext(options);
  const paramSpecs: MutationParamSpec[] = [];
  const aliases: string[] = [];
  const placeholders: string[] = [];

  for (const key of context.primaryKey) {
    const column = mutationColumn(context, key);
    const quoted = quoteIdentifier(key, "primary key column");
    placeholders.push(pushParamSpec(paramSpecs, "required", key, column.type));
    aliases.push(quoted);
  }

  if (context.versionColumn !== undefined) {
    const column = mutationColumn(context, context.versionColumn);
    const quoted = quoteIdentifier(context.versionColumn, "version column");
    placeholders.push(
      pushParamSpec(paramSpecs, "required", context.versionColumn, column.type),
    );
    aliases.push(quoted);
  }

  const setAssignments: string[] = [];
  for (const column of options.columns) {
    if (column.updatable === false) {
      continue;
    }

    if (context.primaryKey.includes(column.name)) {
      continue;
    }

    if (context.versionColumn === column.name) {
      continue;
    }

    const quoted = quoteIdentifier(column.name, "column");
    placeholders.push(pushParamSpec(paramSpecs, "value", column.name, column.type));
    aliases.push(quoted);

    const presentAlias = `"${column.name}#present"`;
    placeholders.push(pushParamSpec(paramSpecs, "present", column.name, "bool"));
    aliases.push(presentAlias);

    const fallback = column.updateDefaultSql ?? `u.${quoted}`;
    setAssignments.push(
      `${quoted} = CASE WHEN v.${presentAlias} THEN v.${quoted} ELSE ${fallback} END`,
    );
  }

  if (context.versionColumn !== undefined) {
    const quoted = quoteIdentifier(context.versionColumn, "version column");
    setAssignments.push(`${quoted} = u.${quoted} + 1`);
  }

  if (setAssignments.length === 0) {
    throw new QueryError("Update requires at least one updatable column or a version column");
  }

  const where: string[] = [];
  for (const key of context.primaryKey) {
    const quoted = quoteIdentifier(key, "primary key column");
    where.push(`u.${quoted} = v.${quoted}`);
  }

  if (context.versionColumn !== undefined) {
    const quoted = quoteIdentifier(context.versionColumn, "version column");
    where.push(`u.${quoted} = v.${quoted}`);
  }

  const sql = `UPDATE ${context.table} AS u SET ${setAssignments.join(", ")} FROM unnest(${placeholders.join(", ")}) AS v(${aliases.join(", ")}) WHERE ${where.join(" AND ")}${renderMutationReturning(context, "u")}`;
  return { sql, columns: context.columnNames, paramSpecs };
}
