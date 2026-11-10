import {
  mutationContext,
  renderInsertSelect,
  renderMutationReturning,
} from "./mutation.js";
import { quoteIdentifier } from "./shared.js";
import type { BuiltMutation, MutationOptions, MutationParamSpec } from "./types.js";

export function buildUpsert(options: MutationOptions): BuiltMutation {
  const context = mutationContext(options);
  const paramSpecs: MutationParamSpec[] = [];
  const insert = renderInsertSelect(options, context, paramSpecs, "u");
  const conflictTarget = context.primaryKey
    .map((name) => quoteIdentifier(name, "primary key column"))
    .join(", ");

  const assignments: string[] = [];
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
    assignments.push(`${quoted} = excluded.${quoted}`);
  }

  if (context.versionColumn !== undefined) {
    const quoted = quoteIdentifier(context.versionColumn, "version column");
    assignments.push(`${quoted} = u.${quoted} + 1`);
  }

  let conflictSql: string;
  if (assignments.length === 0) {
    conflictSql = ` ON CONFLICT (${conflictTarget}) DO NOTHING`;
  } else {
    let where = "";
    if (context.versionColumn !== undefined) {
      const quoted = quoteIdentifier(context.versionColumn, "version column");
      where = ` WHERE u.${quoted} = excluded.${quoted}`;
    }

    conflictSql = ` ON CONFLICT (${conflictTarget}) DO UPDATE SET ${assignments.join(", ")}${where}`;
  }

  const sql = `${insert}${conflictSql}${renderMutationReturning(context, "u")}`;
  return { sql, columns: context.columnNames, paramSpecs };
}
