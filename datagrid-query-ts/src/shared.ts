const IDENTIFIER_PATTERN = /^[A-Za-z_][A-Za-z0-9_]*$/;
const SQL_TYPE_PATTERN = /^[A-Za-z_][A-Za-z0-9_ .(),]*$/;

export class QueryError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "QueryError";
  }
}

export function quoteIdentifier(value: string, kind: string): string {
  if (!IDENTIFIER_PATTERN.test(value)) {
    throw new QueryError(`Invalid ${kind} identifier: ${JSON.stringify(value)}`);
  }

  return `"${value}"`;
}

export function pushParam(params: unknown[], value: unknown): string {
  params.push(value);
  return `$${params.length}`;
}

export function validateSqlType(value: string): void {
  if (!SQL_TYPE_PATTERN.test(value)) {
    throw new QueryError(`Invalid SQL type: ${JSON.stringify(value)}`);
  }
}
