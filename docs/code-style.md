# Code style

## Comments stay on one line

Every code comment must fit on a single line. That includes `//`, Rust doc
comments (`///`, `//!`), and JavaScript/TypeScript doc comments (`/** ... */`)
when they open and close on the same line.

Multi-line comment blocks and consecutive line-comment paragraphs are not
allowed. When a comment needs more than one line, move the explanation into
`docs/` and keep a short one-line pointer in the code.

### Good

```rust
/// Validate `name` and wrap it in double quotes.
pub fn quote_identifier(name: &str) -> Result<String, InvalidIdentifier> {
```

```ts
/** Ignored for `isNull` / `isNotNull`; an array for `in`. */
value?: unknown;
```

```ts
// An empty IN list can never match.
return "FALSE";
```

### Bad

```rust
/// Validate `name` and wrap it in double quotes.
///
/// Accepts `^[A-Za-z_][A-Za-z0-9_]*$`.
pub fn quote_identifier(name: &str) -> Result<String, InvalidIdentifier> {
```

```ts
/**
 * Build a parameterised `SELECT` statement from grid state.
 */
export function buildQuery(options: QueryOptions): BuiltQuery {
```

## Mark TypeScript internal exports

The package entry point (`src/index.ts`) defines the public TypeScript API. A
declaration exported from another module only so that package code can use it
must be marked with a one-line `@internal` JSDoc tag.

```ts
/** @internal */
export function quoteIdentifier(value: string, kind: string): string {
```

Public exports re-exported from `src/index.ts` do not need the tag. Rust does
not need this annotation because `pub`, `pub(crate)`, and module privacy express
visibility directly.

## Enforcement

No automated check is in place yet. When implementation work resumes, add a
source scan to each project's test suite so multi-line comments fail the build.
