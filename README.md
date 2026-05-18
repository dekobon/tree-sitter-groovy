# tree-sitter-groovy

[![CI](https://github.com/dekobon/tree-sitter-groovy/actions/workflows/ci.yml/badge.svg)](https://github.com/dekobon/tree-sitter-groovy/actions/workflows/ci.yml)
[![CodeQL](https://github.com/dekobon/tree-sitter-groovy/actions/workflows/codeql.yml/badge.svg)](https://github.com/dekobon/tree-sitter-groovy/actions/workflows/codeql.yml)
[![npm](https://img.shields.io/npm/v/@dekobon/tree-sitter-groovy)](https://www.npmjs.com/package/@dekobon/tree-sitter-groovy)
[![crates.io](https://img.shields.io/crates/v/dekobon-tree-sitter-groovy)](https://crates.io/crates/dekobon-tree-sitter-groovy)
[![PyPI](https://img.shields.io/pypi/v/dekobon-tree-sitter-groovy)](https://pypi.org/project/dekobon-tree-sitter-groovy/)

Tree-sitter grammar for [Apache Groovy](https://groovy-lang.org/) —
the JVM scripting language used in Jenkins pipelines, Gradle build
files, and Spock test specifications.

**Published as**: `@dekobon/tree-sitter-groovy` on npm,
`dekobon-tree-sitter-groovy` on crates.io and PyPI,
`github.com/dekobon/tree-sitter-groovy` as a Go module / Swift package.
The unscoped name belongs to the upstream comparison grammar linked
below; the C library (no naming conflict) still ships as
`libtree-sitter-groovy`.

This grammar is **purpose-built** for Groovy (it does not extend the
`tree-sitter-java` grammar). It synthesises the best ideas from the
two existing community grammars while closing every operator and
statement gap they left open:

- [`amaanq/tree-sitter-groovy`](https://github.com/amaanq/tree-sitter-groovy)
  — clean precedence table and shebang handling, but mis-shapes
  Groovy-specific syntax (Elvis `?:`, safe navigation `?.`, spread
  operators, regex `=~` / `==~`, traits, exclusive ranges, identity
  `===` / `!==`, spaceship `<=>`, etc.) as Java-shaped parse trees.
- [`murtaza64/tree-sitter-groovy`](https://github.com/murtaza64/tree-sitter-groovy)
  — purpose-built for Groovy, broad operator coverage and slashy
  strings, but has open bugs around empty block comments, multi-catch,
  multi-assignment, enums, switch-arrow, and a `pipeline`-at-EOF
  constraint.

The full design is in [`SPECIFICATION.md`](SPECIFICATION.md).

## Status

Fully functional Groovy 2.x--4.x parser with complete operator
coverage, GString interpolation, generics, closures, command chains,
and six language bindings. See [Known limitations](#known-limitations)
for deferred items.

## Operator coverage

Every Groovy operator emits a **distinct named node** so downstream
tooling can identify each by node kind alone — no `MISSING`-child
inspection, no tree-shape heuristics. This is the contract that
closes
[`dekobon/big-code-analysis#246`](https://github.com/dekobon/big-code-analysis/issues/246)
(Elvis short-circuit counting) and
[`dekobon/big-code-analysis#247`](https://github.com/dekobon/big-code-analysis/issues/247)
(all Groovy-specific operator gaps).

| Category | Operators |
|---|---|
| Arithmetic | `+` `-` `*` `/` `%` `**` `++` `--` |
| Relational | `<` `<=` `>` `>=` `==` `!=` `<=>` (spaceship) |
| Identity | `===` `!==` |
| Regex | `=~` (find) `==~` (match) |
| Logical | `&&` `\|\|` `!` `==>` (implication) |
| Bitwise | `&` `\|` `^` `~` `<<` `>>` `>>>` |
| Conditional | `? :` (ternary) `?:` (Elvis) |
| Assignment | `=` `+=` `-=` `*=` `/=` `%=` `**=` `<<=` `>>=` `>>>=` `&=` `^=` `\|=` `?=` (Elvis assign) |
| Range | `..` `..<` `<..` `<..<` |
| Member access | `.` `?.` `??.` (safe chain dot) `*.` (spread-dot) `.&` (method pointer) `.@` (direct field) `::` (method reference) |
| Subscript | `[]` `?[]` (safe index) |
| Spread | `*args` `*:` (spread-map) |
| Membership | `in` `!in` `instanceof` `!instanceof` |
| Coercion | `as` |

## Editor support

Tree-sitter queries (`queries/groovy/highlights.scm`,
`folds.scm`, `indents.scm`, `injections.scm`) are provided for editor
integration. Slashy and dollar-slashy string bodies inject as `regex`
for nested regex highlighting.

Recognized file extensions: `.groovy`, `.gradle`, `.gvy`, `.gy`, and
`Jenkinsfile`.

### Filetype detection

The repo ships `ftdetect/groovy.lua` and `ftplugin/groovy.lua` for
Neovim. Neovim's built-in filetype detection already maps `*.groovy`,
`*.gradle`, and the bare filename `Jenkinsfile` to filetype `groovy`;
the shipped `ftdetect/groovy.lua` fills the gaps by mapping:

- `*.gvy` and `*.gy` — declared in `tree-sitter.json` but not in
  Neovim's defaults.
- `*.jenkinsfile` — uncommon (the canonical name is `Jenkinsfile`) but
  declared in `tree-sitter.json` and used by some teams as an
  editor-detection hook.
- `Jenkinsfile.*` variants (`Jenkinsfile.ci`, `Jenkinsfile.release`,
  …) — common in repos with multiple pipelines, not handled by
  Neovim's built-in detection.

The existing `ftplugin/groovy.lua` then calls `vim.treesitter.start()`.
Highlighting only renders once the parser binary and
`queries/groovy/highlights.scm` are registered with `nvim-treesitter`
(or equivalent) — without that, the `start()` call is a no-op and the
buffer falls back to non-treesitter highlighting.

For files that don't match any of those (e.g. a hand-named
`build-pipeline` with no extension), opt in with one of:

- **Modeline** at the top of the file:
  `// vim: set filetype=groovy :`
- **Per-project autocmd** in `.nvim.lua` (sourced after `:cd` into the
  project via Neovim's `'exrc'`). Note `vim.fn.getcwd()` is evaluated
  when the autocmd is *defined*, so this snippet belongs in a
  per-project config, not a global `init.lua`:

  ```lua
  vim.api.nvim_create_autocmd({ "BufRead", "BufNewFile" }, {
    pattern = vim.fn.getcwd() .. "/jenkins/*",
    callback = function() vim.bo.filetype = "groovy" end,
  })
  ```

Other editors follow the same pattern: rely on built-in detection for
the common extensions and override per-directory or via a header
comment for the rest.

## Building

```bash
npm install
make build
make test
```

## Bindings

Available bindings: C, Go, Node.js, Python, Rust, Swift. See
`bindings/` for binding-specific READMEs and tests.

For Rust consumers:

```toml
[dependencies]
dekobon-tree-sitter-groovy = "0.1"
tree-sitter = "0.25"
```

```rust
let mut parser = tree_sitter::Parser::new();
let language = dekobon_tree_sitter_groovy::LANGUAGE;
parser.set_language(&language.into()).expect("Error loading Groovy parser");
let tree = parser.parse(source, None).unwrap();
```

## Known limitations

These are documented in detail in
[`docs/divergences-from-spec.md`](docs/divergences-from-spec.md).

- **Groovy 5 contextual keywords** (`val`, `async`, `await`, `defer`)
  are not yet exposed as keyword tokens. They parse as plain
  identifiers, matching Groovy 2.x--4.x compiler behavior.
- **Typed local declarations require an initializer** -- `String x`
  is ambiguous with `String(x)` (method call). Use `String x = ...`
  or `def x` instead.
- **Leading-operator line continuation** -- `a\n+ b` greedily joins
  as `a + b` rather than splitting into two statements. Idiomatic
  Groovy places continuation operators at the end of the previous
  line.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for how to build, test, and
submit changes. [`AGENTS.md`](AGENTS.md) covers project conventions
(commit messages, versioning, validation gates) and
[`SPECIFICATION.md`](SPECIFICATION.md) is the authoritative grammar
design document.

## Acknowledgements

We are grateful for the work done in the [murtaza64/tree-sitter-groovy](https://github.com/murtaza64/tree-sitter-groovy)
and [amaanq/tree-sitter-groovy](https://github.com/amaanq/tree-sitter-groovy). These projects served as inspiration
for some of the approaches in this project

## License

Dual-licensed under [Apache License 2.0](LICENSE-APACHE) or
[MIT](LICENSE-MIT).
