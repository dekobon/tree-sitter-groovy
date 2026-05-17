# tree-sitter-groovy

[![CI](https://github.com/dekobon/tree-sitter-groovy/actions/workflows/ci.yml/badge.svg)](https://github.com/dekobon/tree-sitter-groovy/actions/workflows/ci.yml)

Tree-sitter grammar for [Apache Groovy](https://groovy-lang.org/) —
the JVM scripting language used in Jenkins pipelines, Gradle build
files, and Spock test specifications.

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

Early development. Repository skeleton is in place; `grammar.js` and
`src/scanner.c` are being implemented against the specification.

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
tree-sitter-groovy = "0.1"
tree-sitter = "0.25"
```

```rust
let mut parser = tree_sitter::Parser::new();
let language = tree_sitter_groovy::LANGUAGE;
parser.set_language(&language.into()).expect("Error loading Groovy parser");
let tree = parser.parse(source, None).unwrap();
```

## Contributing

See [`AGENTS.md`](AGENTS.md) for project conventions (commit messages,
versioning, validation gates) and
[`SPECIFICATION.md`](SPECIFICATION.md) for grammar design decisions.

When fixing a bug, add a regression test under `test/corpus/` that
would catch it if reintroduced.

## License

Dual-licensed under [Apache License 2.0](LICENSE-APACHE) or
[MIT](LICENSE-MIT), at your option. Copyright Elijah Zupancic.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project by you, as defined in the
Apache-2.0 license, shall be dual-licensed as above, without any
additional terms or conditions.
