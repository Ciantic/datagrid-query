import {
  mutationContext,
  presentArray,
  pushTypedArray,
  renderMutationReturning,
  valueArray,
} from "./mutation.js";
import { QueryError, quoteIdentifier } from "./shared.js";
import type { BuiltQuery, MutationOptions } from "./types.js";

function renderMergeSource(
  options: MutationOptions,
  context: ReturnType<typeof mutationContext>,
  params: unknown[],
): { placeholders: string[]; aliases: string[] } {
  const placeholders: string[] = [];
  const aliases: string[] = [];

  for (const column of options.columns) {
    const primaryKey = context.primaryKey.includes(column.name);
    const versionColumn = context.versionColumn === column.name;
    const insertable = column.insertable !== false;
    const updatable = column.updatable !== false && !primaryKey && !versionColumn;

    if (!insertable && !updatable && !primaryKey && !versionColumn) {
      continue;
    }

    const quoted = quoteIdentifier(column.name, "column");
    placeholders.push(pushTypedArray(params, valueArray(options.rows, column.name), column.type));
    aliases.push(quoted);

    if (updatable || (insertable && column.insertDefaultSql !== undefined)) {
      placeholders.push(pushTypedArray(params, presentArray(options.rows, column.name), "bool"));
      aliases.push(`"${column.name}#present"`);
    }
  }

  return { placeholders, aliases };
}

export function buildMerge(options: MutationOptions): BuiltQuery {
  const context = mutationContext(options);
  const params: unknown[] = [];
  const source = renderMergeSource(options, context, params);

  const insertColumns = options.columns.filter((column) => column.insertable !== false);
  if (insertColumns.length === 0) {
    throw new QueryError("Merge requires at least one insertable column");
  }

  const insertNames = insertColumns.map((column) => quoteIdentifier(column.name, "column"));
  const insertValues = insertColumns.map((column) => {
    const quoted = quoteIdentifier(column.name, "column");
    if (column.insertDefaultSql !== undefined) {
      return `CASE WHEN v."${column.name}#present" THEN v.${quoted} ELSE ${column.insertDefaultSql} END`;
    }

    return `v.${quoted}`;
  });

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
    const presentAlias = `"${column.name}#present"`;
    const fallback = column.updateDefaultSql ?? `u.${quoted}`;
    assignments.push(
      `${quoted} = CASE WHEN v.${presentAlias} THEN v.${quoted} ELSE ${fallback} END`,
    );
  }

  let matchedCondition = "";
  if (context.versionColumn !== undefined) {
    const quoted = quoteIdentifier(context.versionColumn, "version column");
    matchedCondition = ` AND u.${quoted} = v.${quoted}`;
    assignments.push(`${quoted} = u.${quoted} + 1`);
  }

  const matchedSql =
    assignments.length === 0
      ? "WHEN MATCHED THEN DO NOTHING"
      : `WHEN MATCHED${matchedCondition} THEN UPDATE SET ${assignments.join(", ")}`;

  const on = context.primaryKey
    .map((name) => {
      const quoted = quoteIdentifier(name, "primary key column");
      return `u.${quoted} = v.${quoted}`;
    })
    .join(" AND ");

  const sql = `MERGE INTO ${context.table} AS u USING unnest(${source.placeholders.join(", ")}) AS v(${source.aliases.join(", ")}) ON ${on} ${matchedSql} WHEN NOT MATCHED THEN INSERT (${insertNames.join(", ")}) VALUES (${insertValues.join(", ")})${renderMutationReturning(context, "u")}`;

  return { sql, params };
}
