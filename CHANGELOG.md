# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2026-05-24

Maintenance release. No grammar, scanner, or query changes; this
ships the regenerated parser source from a newer `tree-sitter-cli`
plus dependency bumps. AST shape, query captures, and binding
public surface are all identical to `0.2.0`.

### Changed

- Bumped `tree-sitter-cli` from 0.25.10 to 0.26.9 (dev dependency).
  `src/parser.c` and `src/tree_sitter/array.h` have been regenerated
  with the new CLI; the committed parser source matches what CI
  expects.
- Bumped the Rust `tree-sitter` runtime from 0.26.8 to 0.26.9
  (patch).
- Bumped `node-addon-api` from 8.7.0 to 8.8.0 (minor).
- Pinned several GitHub Actions to current major versions
  (`actions/upload-artifact` v7, `actions/download-artifact` v8,
  `pypa/cibuildwheel` 3.4.1, and the actions-minor-and-patch
  group).
- CI: aligned the regenerate workflow with the upstream
  `tree-sitter-grammars/template`, added GitHub Actions to the
  CodeQL language matrix, and closed the release-pipeline known
  gaps surfaced after `v0.2.0`. Dependabot is now allowed to bump
  `tree-sitter-cli`.

## [0.2.0] - 2026-05-19

First version published through the automated release pipeline to
all three registries (`@dekobon/tree-sitter-groovy` on npm,
`dekobon-tree-sitter-groovy` on crates.io and PyPI). Grammar,
scanner, queries, and bindings are unchanged from `0.1.0`.

`0.1.0` was a manual bootstrap upload to crates.io, required
because crates.io does not yet support pending Trusted
Publishers — the crate had to exist before the publisher entry
could be registered. It has been yanked. npm and PyPI never saw
`0.1.0`.

### Changed

- **BREAKING: Python binding minimum is now 3.9** (previously 3.8).
  setuptools >= 82.0.1 — required by the build — dropped Python
  3.8 support, so the cibuildwheel matrix and `requires-python`
  both move to 3.9. abi3 wheels at the 3.9 floor still cover 3.9
  through 3.13+. No Python 3.8 user could have consumed this
  package via PyPI (the registry never had `0.1.0`), but Python
  3.8 users building from sdist need to either pin to a pre-`0.2.0`
  source release or upgrade their Python.
- **Release pipeline now uses Trusted Publishing on all three
  registries.** crates.io, npm, and PyPI all authenticate via
  short-lived OIDC tokens minted by GitHub Actions; no long-lived
  `NPM_TOKEN` or `CARGO_REGISTRY_TOKEN` secret on the repo.
- **`bindings/rust` packaging hardened**: the `[package].include`
  patterns in `Cargo.toml` are now anchored to the package root,
  so bare basenames like `LICENSE-MIT` no longer match
  same-named files under `node_modules/`. Published tarball
  contents are unchanged (34 files, 5.7 MiB).

## [0.1.0] - 2026-05-18

Initial release. Purpose-built Groovy 2.x--4.x parser with complete
operator coverage, six language bindings, and full editor query support.

### Added

#### Grammar

- **Complete operator coverage** -- every Groovy operator emits a
  distinct named node (no MISSING-child inspection or tree-shape
  heuristics required). Operators:
  - Arithmetic (`+` `-` `*` `/` `%` `**` `++` `--`)
  - Relational / equality (`<` `<=` `>` `>=` `==` `!=` `<=>`)
  - Identity (`===` `!==`)
  - Regex (`=~` find, `==~` match)
  - Logical (`&&` `||` `!` `==>` implication)
  - Bitwise (`&` `|` `^` `~` `<<` `>>` `>>>`)
  - Conditional (`? :` ternary, `?:` Elvis)
  - Assignment (`=` `+=` `-=` `*=` `/=` `%=` `**=` `<<=` `>>=`
    `>>>=` `&=` `^=` `|=` `?=`)
  - Range (`..` `..<` `<..` `<..<`)
  - Member access (`.` `?.` `??.` `*.` `.&` `.@` `::`)
  - Subscript (`[]` `?[]`)
  - Spread (`*args` `*:`)
  - Membership (`in` `!in` `instanceof` `!instanceof`)
  - Coercion (`as`)
  - Power (`**`, right-associative, Apache precedence)
- **Declarations** -- `class`, `trait`, `interface`, `enum`, `record`
  (Groovy 4), `sealed` / `non-sealed` / `permits`, `@interface`,
  method, constructor, field, `static` initializer, formal parameters
  (typed, default-valued, varargs), throws clauses, generics
  (`type_parameters`, `type_arguments`, bounded wildcards),
  inheritance (`extends`, `implements`), modifiers, annotations,
  package, import (static, wildcard, aliased).
- **Statements** -- `if` / `else`, `while`, `do-while`, `for`
  (C-style and for-in), `switch` (classic and Groovy 4 arrow form),
  `try` / `catch` / `finally` (multi-catch, try-with-resources),
  `return`, `break`, `continue`, `throw`, `assert`, `yield`,
  `labeled_statement`, `pipeline` (Jenkins).
- **Expressions** -- numeric literals (all bases, underscores, type
  suffixes), string literals (single-quoted, triple-single, double-
  quoted, triple-double, slashy regex, dollar-slashy), GString
  interpolation (`$name` and `${expr}` as parse-tree children),
  boolean / null literals, closures (typed and untyped parameters),
  list / map literals (trailing commas), object creation (`new`),
  parenthesized type cast, command chains (multi-argument),
  method invocation (named arguments, spread arguments), quoted
  identifiers, parenthesized expressions.
- **Types** -- `type_identifier`, `generic_type` (nested, bounded
  wildcards), `qualified_type`, `array_type`.
- **External scanner** (`src/scanner.c`) -- handles slashy-string vs.
  division disambiguation, GString interpolation boundaries, line /
  block / Groovydoc comment dispatch, and scaffolding for automatic
  semicolons and label colons.
- **Shebang** support (`#!` at file start).

#### Queries

- `highlights.scm` -- full syntax highlighting with Groovy 4
  contextual keywords (`var`, `record`, `sealed`, `non-sealed`,
  `permits`, `yield`).
- `locals.scm` -- scope, definition, and reference captures.
- `tags.scm` -- ctags-style definition / reference tagging.
- `indents.scm` -- indentation rules.
- `folds.scm` -- code folding regions.
- `injections.scm` -- slashy and dollar-slashy bodies inject as
  `regex`; Groovydoc injects as `javadoc`; SQL DSL pattern for
  `sql.execute """..."""`.

#### Bindings

- C, Go, Node.js, Python, Rust, and Swift bindings, all calling
  `tree_sitter_groovy()`.
- Node binding exposes `HIGHLIGHTS_QUERY` as a lazy property that
  reads `queries/groovy/highlights.scm` on first access, matching the
  Rust binding's `dekobon_tree_sitter_groovy::HIGHLIGHTS_QUERY`.
- Node binding no longer exports the deprecated `name` field
  (`require('@dekobon/tree-sitter-groovy').name` returns `undefined`).
  TypeScript declarations refreshed to match the current upstream
  `tree-sitter` CLI template.

#### Editor integration

- `ftdetect/groovy.lua` for Neovim filetype detection of extensions
  that Neovim's built-in detection misses (`*.gvy`, `*.gy`,
  `*.jenkinsfile`) plus the `Jenkinsfile.*` filename pattern common
  in repos with multiple pipelines.

#### Tests

- 63 corpus test files covering every operator family, statement
  type, and declaration form.
- 8 highlight assertion test files.
- 10 stress test files including real-world Jenkins pipeline and
  Gradle buildscript samples.
- Rust integration test (`parse_stress.rs`) asserting zero ERROR
  and zero MISSING nodes across all stress files.
- Dedicated regression test file (`regressions.txt`) pinning fixes
  for murtaza64/tree-sitter-groovy #16, #22, #36, #37, #39 and
  dekobon/big-code-analysis #246, #247.

#### Infrastructure

- CI workflow with multi-platform testing (Linux, Windows, macOS),
  ESLint, Clippy, cargo-deny, typos, taplo, markdownlint, and
  scanner-diff-gated fuzzing. Third-party actions are SHA-pinned.
  CI workflow runs with read-only `GITHUB_TOKEN` permissions.
- CodeQL analysis workflow for C, JavaScript, Python, Go, and Rust.
- Dependabot configuration for all six package ecosystems.
- `SPECIFICATION.md` -- authoritative grammar design document.
- `docs/divergences-from-spec.md` -- documented deviations from the
  specification with rationale.

### Known limitations

See [`docs/divergences-from-spec.md`](docs/divergences-from-spec.md)
for full details.

- Groovy 5 contextual keywords (`val`, `async`, `await`, `defer`)
  parse as plain identifiers (matches Groovy 2.x--4.x behavior).
- Typed local declarations require an initializer (`String x = ...`
  works; bare `String x` does not).
- `a\n+ b` joins as one expression rather than splitting into two
  statements.

[Unreleased]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/dekobon/tree-sitter-groovy/releases/tag/v0.1.0
