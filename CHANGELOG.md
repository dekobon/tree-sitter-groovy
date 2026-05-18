# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Node binding.** `bindings/node/index.js` now exposes
  `HIGHLIGHTS_QUERY` as a lazy property that reads
  `queries/groovy/highlights.scm` on first access (cached thereafter
  by replacing the getter with the string value, matching the upstream
  `tree-sitter` CLI template). Brings the Node binding to parity with
  the Rust binding, which already re-exports the same query as
  `tree_sitter_groovy::HIGHLIGHTS_QUERY`.
- **Editor integration.** `ftdetect/groovy.lua` ships with the grammar
  so Neovim auto-detects filetype `groovy` for the extensions
  declared in `tree-sitter.json` that Neovim's built-in detection
  misses (`*.gvy`, `*.gy`, `*.jenkinsfile`) plus the `Jenkinsfile.*`
  filename pattern (`Jenkinsfile.ci`, `Jenkinsfile.release`, etc.)
  common in repos with multiple pipelines. The existing
  `ftplugin/groovy.lua` then calls `vim.treesitter.start()`. README
  gains a "Filetype detection" section covering what is auto-detected
  plus modeline and per-project autocmd opt-in recipes for files that
  don't match any pattern.

### Changed

- **BREAKING.** Node binding `bindings/node/binding.cc` no longer
  exports the `name` field (`require('tree-sitter-groovy').name`
  returns `undefined`). The matching `name: string` declaration is
  removed from `bindings/node/index.d.ts`. Upstream
  `Parser.Language.name` has been deprecated since the NAPI migration
  and no other binding in this repo (Rust, Python, Go, Swift) exposes
  it; consumers needing the package identifier should read it from
  `package.json`. The rest of the TypeScript declarations are
  refreshed to match the current upstream `tree-sitter` CLI template:
  `language` is documented `@private` and typed `unknown` (fixing a
  pre-existing self-referential `language: Language` declaration), and
  the optional `HIGHLIGHTS_QUERY?: string` field is declared.

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
  scanner-diff-gated fuzzing.
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
