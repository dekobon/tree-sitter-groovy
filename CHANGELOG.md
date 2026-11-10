# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Class, interface, trait, record, and `@interface` bodies now accept
  instance initializer blocks — `class C { { init() } }`. The block is
  a bare `block` child of `class_body`, next to the existing
  `static_initializer`; previously it closed the class early on a
  `MISSING "}"`. Enum bodies are unchanged. A block on the line after
  a field *with* an initialiser (`int x = 5\n{ … }`) is not an
  initializer but that expression's trailing closure, the reading
  Apache's grammar gives it too — end the field with `;` to get the
  initializer; see `docs/divergences-from-spec.md` §11.

  The block also follows a field with no initialiser on the same
  line, which costs an error report: a nested `class`, `interface` or
  `trait` declaration, unsupported before and after, used to get an
  `ERROR` at its `{`. `class Inner { … }` inside a class body now
  parses as a field `Inner` of type `class` followed by an
  initializer block, often with no `ERROR` at all.

  No node kind is added, but the generated parser gains one auxiliary
  symbol, so the numeric ids of the three alias kinds
  (`named_argument`, `quoted_identifier`, `type_identifier`) each move
  up by one (308–310, now 309–311). So do the fourteen hidden
  auxiliary symbols numbered after the new one (294–307, now
  295–308), which never appear in a tree. Symbol ids below 294 and
  all field ids are unchanged. (#68)

### Fixed

- Anonymous inner classes now parse. `new Runnable() { … }` had no
  production. At v0.3.0 the body was cut loose from the creation: in
  the reported field initialiser it spilled into the enclosing class
  behind an `ERROR`, in a script or method body it became a separate
  closure statement with no `ERROR` at all. Since v0.4.0 the
  trailing-closure call of #49 claimed it, yielding a
  `method_invocation` whose `closure` held the class members — a tree
  that looked like a valid call and carried an `ERROR` only when a
  member was not also a valid statement.
  `object_creation_expression` gains an optional `body` field holding
  a `class_body`, and every `{` that follows the arguments of
  `new T(...)` is read as that body. Together with the instance
  initializer above this covers double-brace initialisation
  (`new HashMap() {{ put(1, 2) }}`).

  Trees change for input that already parsed without an error:
  `new Foo() { }`, and any body made only of members that are also
  valid statements, move from `method_invocation` / `closure` to
  `object_creation_expression` / `class_body`. Calls such as
  `new Foo().bar { }` and `foo() { }` keep their trailing closure.
  Groovy is narrower about the body than this grammar: it calls a
  closure on the new object when the `{` starts a later line (outside
  parentheses and brackets) or holds something that cannot be a class
  body. `new ArrayList()\n{ list.add(1) }` and `new T() { x -> x }`
  therefore move from a `method_invocation` with a `closure` to a
  `class_body` with an `ERROR` in it; see
  `docs/divergences-from-spec.md` §11. Still unsupported, as before:
  enum constants with an anonymous class body (`A { … }`), qualified
  creation (`outer.new Inner(...)`), and a cast written directly in
  front of `new T()` in a field initialiser that follows another
  member (`def f = (Foo) new T() { … }`). Symbol and field ids are
  unchanged by this fix. (#68)

## [0.4.0] - 2026-11-09

### Added

- `method_invocation` now accepts a trailing closure as the call's last
  argument, with or without a parenthesised argument list —
  `list.find { it > 5 }` and `list.inject(0) { acc, x -> acc + x }` both
  reduce to a single `method_invocation` node (`closure` field alongside
  the optional `arguments` field). Previously any call written this way
  outside of a bare statement-level command chain (`§5.5`) required
  parentheses around the closure or failed to parse. `arguments` is
  no longer a required field in `node-types.json`, so code that
  assumes every `method_invocation` has one must handle its absence.
  (#49)

### Changed

- **BREAKING:** raised the Rust crate's minimum supported Rust version
  (`rust-version`) from 1.85 to 1.90, the floor that `tree-sitter`
  0.27 and `tree-sitter-language` 0.1.8+ declare. Consumers on 1.85–1.89
  must stay on an earlier release of this crate. The `tree-sitter`
  dev dependency moves from 0.26 to 0.27; the `tree-sitter-language`
  requirement is unchanged at `0.1.7`. (#66)
- Bumped `tree-sitter-cli` from 0.26.13 to 0.27.2 and raised the
  declared range in `package.json` to `^0.27.2` so it matches the CLI
  the committed parser source is generated with. It is a dev
  dependency, so nothing changes for downstream consumers; regenerating
  under 0.27.2 leaves `src/parser.c` and `src/tree_sitter/*.h`
  byte-identical to the committed sources (still ABI 15). (#66)

### Fixed

- A nested `enum` inside a class body (`static enum EventType { ... }`,
  a common Groovy domain-model idiom for entity fields backed by a
  fixed set of values) now parses as a class member. Previously
  `_class_member` had no enum_declaration alternative at all, so `enum`
  silently misparsed as a field_declaration's type name and everything
  after it in the class body fell apart. (#49)
- `catch_formal_parameter`'s type is now optional — Groovy allows a
  bare `catch (e) { … }`, implicitly typed as Exception. Previously a
  type was mandatory. (#49)
- Constructor calls now accept the diamond operator (`new ArrayList<>()`,
  `new HashMap<>()`). Ordinary types and nested type arguments still
  require non-empty argument lists, so `List<> xs` and
  `new HashMap<String, List<>>()` remain errors. (#49)
- `field_declaration` now accepts a modifier-only field with no `def`
  and no type (`static MEDIUM_TTL = 900`), inferring the field's type as
  Object — a common static-constant idiom. Previously a field required
  `def` or an explicit type even when a modifier alone already
  distinguishes it from a bare expression statement. (#49)
- Typed fields retain their type when followed by a constructor rather
  than being split into a modifier-only field and a method declaration
  (`private final int value` followed by `C() {}`). (#49)
- `enum_declaration` now accepts modifiers (`public enum`, `static enum` —
  the latter for enums nested inside a class). Previously any modifier
  before `enum` was rejected. (#49)
- `for_in_statement` now also accepts the Java-compatible enhanced-for
  form Groovy allows side by side with its own `for (x in xs)`: `for
  (Type x : xs)` and `for (def x : xs)`. Only the `in` keyword form
  parsed before. (#49)
- `formal_parameter` now accepts annotations (`@PathVariable("id") Long
  id`, `@RequestBody Map body`). Spring/JPA-style annotated parameters
  are ubiquitous on framework-facing methods and previously misparsed
  the whole parameter list. (#49)
- Accept enum class members directly after the constants with no `;`
  between, as Groovy's own parser does (`classBody` in
  `GroovyParser.g4`, GROOVY-7773 / GROOVY-9306). Previously an enum
  such as

  ```groovy
  enum E {
      A(1), B, C(2)
      E(int v) { }
  }
  ```

  closed on a zero-width `MISSING "}"` after the last constant and the
  constructor was re-parsed as a top-level method invocation followed
  by a closure. The `;` and `,;` spellings keep parsing as before, a
  trailing `,` directly before a member stays an error as in Groovy,
  and an enum whose members carry a modifier or `def` no longer needs
  a leading `;` when it has no constants.

  Newlines are whitespace to this grammar, so where Groovy reads the
  line break the parser has to choose. After a bare first constant,
  `enum E { A\n E() { } }` is a constant followed by a constructor,
  not a method `E` returning type `A`. The same preference applies
  when there are no constants at all: `enum E { int f() { } }` reads
  as a constant `int` and a constructor `f`. Spell the member with a
  modifier (`private E() { }`), a `def`, or a leading `;` to make it a
  member — all three were already required at v0.3.0, which could not
  parse the member either way. Constants missing their comma
  (`enum E { A\n B }`) stay an `ERROR`, as in Groovy; at v0.3.0 they
  silently parsed as a field `A B`. Enum constants with an anonymous
  class body (`A { … }`) stay unsupported.

  Node kinds, symbol ids, and field ids are unchanged. (#55)

## [0.3.0] - 2026-08-30

### Changed

- **BREAKING:** the minimum supported Python is now 3.11. Wheels are
  tagged `cp311-abi3` (was `cp38-abi3`), the extension is compiled
  against the 3.11 limited API (`Py_LIMITED_API` 0x030B0000), and the
  `core` extra requires `tree-sitter~=0.26`.

  The previously declared 3.9 floor never worked. The committed parser
  is ABI 15, which needs the `tree-sitter` runtime at 0.25 or later,
  and every such release requires Python 3.10+. The newest runtime
  installable on 3.9 is 0.23.2, which rejects ABI 15 outright --
  `ValueError: Incompatible Language version 15. Must be between 13
  and 14` -- so `Parser` could never be constructed there, and
  `pip install dekobon-tree-sitter-groovy[core]` did not even resolve.

  Python 3.10 is a genuine removal rather than a correction: it works
  today with `tree-sitter` 0.26. It is dropped because it reaches end
  of life on 2026-10-31, which would make it a floor with two months
  of runway. (#42, #44)
- The Python build no longer requires the `wheel` package. `setup.py`
  takes `bdist_wheel` from `setuptools.command.bdist_wheel`, which
  setuptools has owned since 70.1; the `wheel` copy emitted a
  `FutureWarning` announcing its removal, and `wheel` is dropped from
  `build-system.requires`. (#42)
- Bumped `tree-sitter-cli` from 0.26.12 to 0.26.13 and the
  `tree-sitter` crate from 0.26.11 to 0.26.13, and raised the declared
  `tree-sitter-cli` range in `package.json` from `^0.26.9` to
  `^0.26.13` so it matches the CLI the committed parser source is
  generated with. Both are dev dependencies, so neither is installed
  for downstream consumers and no runtime version changes. The
  generated sources (`src/parser.c`, `src/tree_sitter/array.h`) are
  unchanged from the 0.26.12 output already committed. (#41)

### Fixed

- The vendored `src/tree_sitter/array.h` now compiles inside a C++
  translation unit. Its array macros cast their result through
  `decltype`, where previously the bare `void *` produced by its
  allocation helpers and swap temporary failed to convert to the
  array's element pointer type. C builds are unaffected — under C the
  cast expands to a no-op. This arrived as regenerated source in the
  `tree-sitter-cli` 0.26.12 bump. (#40)

## [0.2.2] - 2026-06-04

### Fixed

- Accept `;` as a terminator, separator, and empty statement / member
  in statement-list contexts (`source_file`, `block`, `closure`,
  `switch` `case`/`default` bodies) and member-list contexts (class /
  interface / trait / enum bodies). This covers the common
  abstract-method (`interface Foo { void bar(); }`), field
  (`int x = 1;`), and semicolon-terminated statement
  (`void f() { x(); }`) forms that previously produced `ERROR` nodes;
  at script top level the last also cascaded into a misparse of the
  whole method. (#20)
- Accept a `;` terminating a braceless single-statement body: keep the
  `else` attached after an `if` consequence ends in `;`
  (`if (x) a(); else b()`), allow a `do`-`while` body to end in `;`
  (`do a(); while (c)`), and accept a trailing `;` on a `switch`
  arrow-case body (`case 1 -> foo();`). (#20)
- Accept a typed or `def`/`var` variable declaration in the C-style
  `for` init clause (`for (int i = 0; …)`, `for (def i = 0; …)`), which
  previously errored. (#20)

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

[Unreleased]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/dekobon/tree-sitter-groovy/releases/tag/v0.1.0
