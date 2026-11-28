# AGENTS.md

Universal project instructions for AI coding assistants.

## Project overview

`tree-sitter-groovy` is a [tree-sitter](https://tree-sitter.github.io/tree-sitter/)
grammar for [Apache Groovy](https://groovy-lang.org/) — the JVM scripting
language widely used in Jenkins pipelines, Gradle build files, and Spock
test specifications.

This grammar is a **standalone** Groovy parser (it does *not* extend
`tree-sitter-java`). It synthesises the best ideas from the two existing
community grammars while fixing every operator and statement gap they
left open:

- [`amaanq/tree-sitter-groovy`](https://github.com/amaanq/tree-sitter-groovy)
  — extends `tree-sitter-java`, has a clean precedence table and shebang
  handling, but mis-shapes Groovy-specific syntax (Elvis, safe-nav,
  spread, regex ops, traits, exclusive ranges, etc.) as Java-shaped
  parse trees.
- [`murtaza64/tree-sitter-groovy`](https://github.com/murtaza64/tree-sitter-groovy)
  — purpose-built for Groovy, broad operator coverage and slashy
  strings, but has open bugs around block comments, multi-catch,
  multi-assignment, enum / switch-arrow, and a `pipeline`-at-EOF
  constraint.

The authoritative design document is `SPECIFICATION.md`. Every grammar
and binding decision in this repo must map back to a section there.

The default branch is **`main`**.

## Project layout

- `grammar.js` — grammar definition. Implements every operator and
  statement listed in `SPECIFICATION.md` §3 and §4.
- `src/parser.c`, `src/grammar.json`, `src/node-types.json` — generated
  by `tree-sitter generate`. Treat as build outputs; do not hand-edit.
- `src/scanner.c` — hand-written external scanner for the
  context-sensitive lexing from `SPECIFICATION.md` §6 that the grammar
  cannot express. The `externals` list in `grammar.js` and the scanner's
  `TokenType` enum must stay in the same order. It emits:
  - the slashy-string opener, which tells a regex from division
    (§6.2), and line, block and Groovydoc comments as single tokens
    (§6.6);
  - the contextual keywords `record` (§6.7), `trait` and `sealed`
    (§6.13), each only before the declaration it opens, and `var`,
    `yield`, `in` and `as` (§6.16), only before a declared name or a
    multi-assignment's names, an expression (for `yield`, one that
    starts on its line), or a type; and such a word as a named
    argument's key;
  - the key of a named argument in a parenthesis-free call, on the
    call's line and before a single `:` that only spaces, tabs, line
    escapes and block comments (which may span lines) separate from it
    (§6.8);
  - the start of such a call that is a value (of a declaration or an
    assignment, or in parentheses or an interpolation), of a statement
    whose receiver has a member access (`obj.method 1`), or of a
    statement whose first argument is a `new`, an array type, such a
    call in parentheses or a name (`foo new Bar()`, `println x`), each
    with its first argument on the receiver's line; a name is a first
    argument only where Apache's isInvalidLocalVariableDeclaration
    makes the statement no declaration (`foo bar`, not `String s`),
    and a literal only where the argument goes on past it
    (`println "a" + b`, not `foo 1` then `-x` on the next line)
    (§6.14); where the value is an assignment's that is a list element
    or a call's argument, the same decision emits a twin of the value
    start, which opens a call that no map entry or named argument takes
    as its key, so `[x = foo 'a': 1]` passes `'a': 1` to `foo` (§6.27);
  - a subscript `[` on the same line as the expression before it
    (§6.9), a `(` that opens a line after an expression where a
    statement may start and the parentheses can open one, so that it
    opens that statement rather than a call's arguments (§6.18), and
    the start of an array type that is a value (`String[].class`),
    which needs `[]` after the name (§6.15);
  - a `)` with something after it on its line, which ranks a cast,
    one before a name there, which ranks it above a member that the
    name would start, and a `)` that closes no cast (§6.11): before a
    relational operator that Groovy applies to the parentheses
    (`(x) instanceof Foo`, `(x) as Foo`), a division (`(x) / 2`) or a
    keyword that starts no operand (`if (a) (b) else c`), also where
    only a cast's type would take the `)`, which ends that reading, and
    at the end of a line before a line that opens with `(`, unless,
    read on Groovy's tokens for at most 256 characters
    (`ENCLOSING_BRACKET_LOOKAHEAD_CHARACTERS`), a `)` or `]` closes
    brackets around both lines or `->` follows the first parentheses.
    Outside brackets Groovy ends the statement at that line break and
    the `(` opens the next one (§6.18); the reading of a cast whose
    operand is on the later line, kept alive there, made a run of `(x)`
    lines quadratic;
  - the `<` of a cast type's type arguments where they close, so that
    `x as a.B<c` compares, and of another generic type's where they
    close or run past the read, so that `(i<n)` and `{ it<2 }` compare
    and `(List<String>) x` casts (§3.2.2), and the `!in` operator,
    which needs whitespace or a bracket after it (§6.12);
  - the start of a lambda, before its parameters in parentheses or its
    one name, when `->` follows (§6.17);
  - an empty token after the `}` of a lambda's brace body that no
    expression continues, which ends the body there (§6.19), another
    after the `}` of a block that opens a statement, decided the same
    way, so that braces an expression continues (`{ foo() }()`) stay a
    closure (§6.22), and `_lambda_body_reread`, which no rule takes, so
    that the parser breaks down a reused block and lexes its `}` again
    (§6.19, §6.22);
  - the first `,` of `(a, b) = …`, turned down where the parentheses go
    on as a method's parameters, which ends the reading of
    `def f(a, b) { }` as `def f` and a multi-assignment there (§6.20);
  - an empty token before the type of a typed local declaration
    (`String s`, `int a, b`, `String s = 'a'`), emitted when the
    declaration ends on its line, the next line does not go on with it,
    or a list of names or an initializer follows, which the same reading
    of a statement's first name decides, so `foo bar` stays a call
    (§6.21);
  - an empty token after `return`, `break` or `continue` where a line
    break follows on the keyword's line, which ends the statement there,
    as Groovy takes the value or the label only on that line (§6.23);
  - an empty token before a statement's first name whose closure closes
    on the name's line, read on Groovy's tokens for at most 128
    characters (`CLOSURE_CALL_LOOKAHEAD_CHARACTERS`), and that a member
    access follows, so that the call is the access's object
    (`foo { }.bar()`), and another where a first argument follows the
    accesses, which makes them a call's receiver (`foo { }.bar 1`)
    (§6.24);
  - an empty token before `this`, `void`, a primitive type, `goto`,
    `const` or `threadsafe` where the name of a method with no `def` and
    no return type may come and `(` follows on the line, which makes
    that name an error, as Groovy's `methodName` takes none of these
    words and they are identifiers here, so no reserved set can hold
    them (§6.25);
  - the `@` of `@interface` where an annotation type may start, when the
    word `interface` follows, and the `(` of a multi-assignment's names
    after modifiers alone, after `final` and annotations, or after `def`
    and annotations or modifiers, when the parentheses hold two or more
    names, or names and then `=` (`private @B(a, b) = l`), and with no
    look-ahead where no annotation's arguments may follow, after a
    modifier keyword or an annotation's arguments (`private (a)` while
    its `=` is not typed yet). With the two tokens, the `@` of an
    annotation and the `(` of its arguments do not continue a complete
    list of modifiers, so the arguments of an annotation after a
    modifier in a type body do not fork, and a reparse reads an
    annotation inserted after a modifier into that list (§6.26).

  Nine externals are never emitted. `_unreachable` ends
  `_stray_case`, the error production for a `case` label where a
  statement starts, so that error recovery inserts it as a missing
  `switch` and each such line recovers on its own (§5.2); the scanner
  never reads it. Rules make the others valid where the
  scanner needs to know where it is: `_string_body` inside a string
  body, where the scanner emits nothing outside error recovery
  (§6.10); `_type_start` where a type may start (§6.15);
  `_closure_parameters_start` where closure parameters may start and
  after a closure parameter's annotations, the one place a `->` after
  an initializer ends a parameter list (`{ String a = 1 -> }`) rather
  than belonging to a lambda in the value (§6.21); `_type_only_close`
  before the `)` of a cast of an
  array or generic type, which no expression is, so that the rule above
  keeps that cast across the line break (`(List<String>)` then `(x)`)
  (§6.11); `_body_block_end` after the `}` of the block that is the
  body of `if`, `else`, a loop or an arrow case, where the scanner
  reads a `(` as at a statement's start, so that a lambda statement
  after it (`if (c) { a() }` then `(x) -> x`) parses the same from
  scratch and after a reparse (§6.17); `_slash_divides` after the `)`
  of the condition of `if` or a loop and of a method's parameters, and
  after the `}` of a block or a type's body, where no expression ends
  but Groovy's lexer reads a `/` on that line as division, so that the
  slashy opener keeps the one-line rule there (`if (x) / 2`,
  `def f() { a } / 2`) (§6.2); `_annotation_arguments_may_follow`
  right after an annotation's name, where its arguments may follow,
  the one place the `(` of a multi-assignment after modifiers needs
  the look-ahead above (§6.26); and `_error_sentinel`, valid
  only during error recovery, where the scanner emits only the
  comments, the marker of an unterminated `/*`, the slashy opener and
  the contextual keywords.

  Its state has four bits. The first records "no `*/` from here on";
  the second records "the body goes on past the token that follows";
  the third records "a jump statement goes on past the block comment
  that follows". An empty hidden token carries these bits, emitted
  at the first unterminated `/*` (§6.6), after a lambda's body or a
  statement's block that an expression continues (§6.19, §6.22),
  or after `return`, `break` or `continue` before a block comment
  followed by something else on the line (§6.23). The next call
  clears the second and third bits. The fourth records "a slashy
  string may follow a comment after a line break"; a block or
  Groovydoc comment carries it when a slashy string can start a
  statement on that line (§6.2). GString interpolation, automatic
  semicolons and label colons are in-grammar
  (`docs/divergences-from-spec.md`). Symbols are
  `tree_sitter_groovy_external_scanner_*`. `tree-sitter generate` does
  NOT rewrite `scanner.c`, so these symbols are stable across
  regenerations.
- `src/tree_sitter/*.h` — vendored tree-sitter runtime headers; do not
  edit.
- `queries/groovy/highlights.scm` — Treesitter highlight captures.
  Includes the Groovy 4 contextual keywords presently recognised by
  the grammar (`var`, `record`, `sealed`, `non-sealed`, `permits`,
  `yield`). The Groovy 6 contextual keywords `async`, `await`, `defer`,
  and `val` are not yet in the grammar — when they land, add highlight
  rules and assertions at the same time.
- `queries/groovy/folds.scm`, `queries/groovy/indents.scm` — fold and
  indent.
- `queries/groovy/injections.scm` — language injection rules; slashy
  and dollar-slashy string bodies inject as `regex`.
- `bindings/{c,go,node,python,rust,swift}/` — language bindings. All
  call `tree_sitter_groovy()`.
- `test/corpus/*.txt` — corpus tests (input + expected AST). Per-operator
  files anchor `SPECIFICATION.md` §8.1; `regressions.txt` anchors §8.2
  (every closed murtaza64 issue + dekobon issues #246/#247).
- `test/highlight/*.groovy` — highlight assertion tests.
- `ftplugin/groovy.lua` — neovim filetype hook.
- `package.json`, `Cargo.toml`, `pyproject.toml`, `tree-sitter.json`,
  `go.mod`, `Package.swift`, `binding.gyp`, `setup.py`, `Makefile` —
  multi-language packaging metadata.
- `.taplo.toml`, `.typos.toml`, `deny.toml`,
  `.markdownlint-cli2.jsonc` — repo-level lint/format tooling configs
  (TOML formatter, spell-check, license/advisory checks, Markdown
  lint). All scoped to first-party files.
- `SPECIFICATION.md` — authoritative grammar design document. **Read
  before editing `grammar.js`.**

## Editing principles

- Treat `src/parser.c`, `src/grammar.json`, `src/node-types.json` as
  build outputs. Edit `grammar.js` and regenerate.
- `src/scanner.c` is hand-written and survives regeneration unchanged.
  Its symbols are already named `tree_sitter_groovy_external_scanner_*`
  and stay in sync with the generated `parser.c` automatically.
- For non-code files (Markdown, TOML, YAML, JSON): use targeted edits
  with scoped `old_string` / `new_string` pairs.
- Never rewrite an entire corpus test file to fix one test. Modify only
  the specific test that needs changing.
- When fixing a bug, add a regression test (corpus or highlight) that
  would catch it if reintroduced.
- Default to no comments. Only add one when the *why* is non-obvious
  (e.g. why a precedence level was chosen, why a regex looks the way
  it does, why a token is in the external scanner rather than the
  grammar).
- The Apache reference grammar
  ([`apache/groovy:src/antlr/GroovyParser.g4`](https://github.com/apache/groovy/blob/master/src/antlr/GroovyParser.g4)
  and `GroovyLexer.g4`) is the tie-break authority on precedence and
  syntax disputes. `SPECIFICATION.md` §3.1 notes where we diverge and
  why.

## Tool choice

- **Code search**: `rg` (ripgrep). Never `grep` via Bash.
- **File search**: `fd` (or `fdfind` on Debian/Ubuntu). Never `find`
  via Bash.
- **External docs**: prefer the
  [tree-sitter docs](https://tree-sitter.github.io/tree-sitter/),
  the [Apache Groovy language reference](https://groovy-lang.org/),
  and the Apache Groovy ANTLR grammar
  ([`GroovyParser.g4`](https://github.com/apache/groovy/blob/master/src/antlr/GroovyParser.g4),
  [`GroovyLexer.g4`](https://github.com/apache/groovy/blob/master/src/antlr/GroovyLexer.g4))
  over generic web search.

## Validation gates

Before considering a change done:

```bash
npx tree-sitter generate
npx tree-sitter test
npm run lint
```

If the Rust binding (`bindings/rust/**`, `Cargo.toml`) changed, also run:

```bash
cargo clippy --all-targets -- -D warnings
cargo test
```

If repo-level configuration changed (TOML files, `CHANGELOG.md`, other
Markdown docs), also run the applicable tooling check:

```bash
taplo format --check    # TOML formatting (config: .taplo.toml)
typos                   # spell-check (config: .typos.toml)
cargo deny check        # licenses, advisories, sources (config: deny.toml)
markdownlint-cli2 "**/*.md"  # Markdown lint (config: .markdownlint-cli2.jsonc)
```

If grammar / scanner changed, run `npx tree-sitter test` until clean.

The committed `src/parser.c` must be generated by the same
`tree-sitter-cli` version that CI uses. CI resolves the version from
`package-lock.json` (`packages["node_modules/tree-sitter-cli"].version`)
and pins `tree-sitter/setup-action/cli@v2` to that exact tag. Always
regenerate `src/parser.c` with the locally-installed CLI (`npx
tree-sitter generate`) — never with a globally-installed one that may
be a different version — and commit both the lockfile and `parser.c`
together when bumping `tree-sitter-cli`.

## Versioning, commits, and changelog

This project uses Conventional Commits, Semantic Versioning, and a
Keep-a-Changelog-formatted `CHANGELOG.md` at the repo root. The
step-by-step release walkthrough — version bump, changelog move,
tag-and-push, monitoring, and recovery — lives in
[`docs/cutting-a-release.md`](docs/cutting-a-release.md).

### Conventional Commits

Every commit message follows
[Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/):
`<type>(<scope>): <subject>`. Subject is imperative, lowercase, no
trailing period, ≤ 72 chars. `Fixes #NN` / `Closes #NN` go in the body,
not the subject.

Allowed `<type>` values:

| type | use for |
|------|---------|
| `feat` | new grammar rule, new query capture, new binding feature |
| `fix` | bug fix in grammar / scanner / queries / bindings / tests |
| `refactor` | restructure without behavior change |
| `docs` | README, AGENTS.md, CLAUDE.md, SPECIFICATION.md, skills, doc comments |
| `test` | add or correct corpus / highlight tests only |
| `build` | binding build files (`binding.gyp`, `setup.py`, `Cargo.toml`, `Package.swift`, `Makefile`, `package.json` deps) |
| `ci` | `.github/workflows/**` |
| `chore` | release tags, regenerated parser source, version bumps |

Allowed `<scope>` values mirror the project layout:
`grammar`, `scanner`, `queries`, `bindings`, `bindings/<lang>` (e.g.
`bindings/rust`), `tests`, `ci`, `docs`, `spec`, `deps`. Use the most
specific scope that fits.

Examples:

```text
feat(grammar): add elvis_expression as distinct node kind
fix(scanner): emit slashy_string_start only after operator context
fix(queries): highlight async/await/defer contextual keywords
refactor(grammar): hoist precedence levels to match Apache reference
docs(spec): note ternary and elvis share Apache precedence level
test(corpus): pin Elvis chain S-expression for dekobon #246
build(bindings/rust): include scanner.c in build.rs sources
ci: gate fuzz on src/scanner.c diff
chore: regenerate parser after grammar.js edit
```

### Semantic Versioning

The published artifacts follow [SemVer 2.0.0](https://semver.org/).
Distribution names differ because the unscoped `tree-sitter-groovy`
name is held by `amaanq/tree-sitter-groovy` on every flat registry:

- **npm**: `@dekobon/tree-sitter-groovy` (scoped)
- **crates.io**: `dekobon-tree-sitter-groovy` (owner-prefixed)
- **PyPI**: `dekobon-tree-sitter-groovy` (owner-prefixed)
- **Go module**: `github.com/dekobon/tree-sitter-groovy` (URL-namespaced)
- **Swift package**: identified by Git URL
- **C library**: `libtree-sitter-groovy` (no naming conflict; unchanged)

The single source of truth for the version is `tree-sitter.json` →
`metadata.version`; `package.json`, `Cargo.toml`, and `pyproject.toml`
must be kept in sync. `npx tree-sitter version <X.Y.Z>` (wrapped by
`make version`) updates them all.

A change is **breaking** (major bump) if it:

- changes the AST shape of an existing rule (renames a node, removes a
  field, changes which children appear),
- removes or renames a query capture, or
- changes a binding's public symbol, module name, or function signature.

A change is **additive** (minor bump) if it adds a new rule, a new
optional field, a new query capture, or a new binding without touching
existing public surface.

A change is a **patch** if it fixes a parsing bug, query bug, or
binding bug without changing AST shape, query captures, or public
symbols.

### CHANGELOG.md

`CHANGELOG.md` follows
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/). Every
PR that affects user-visible behavior MUST add an entry under
`## [Unreleased]` in one of these sections:

- `### Added` — new rules / captures / bindings / public functionality.
- `### Changed` — behavior changes to existing functionality (note
  breaking ones explicitly with **BREAKING:**).
- `### Deprecated` — soon-to-be-removed features.
- `### Removed` — removals.
- `### Fixed` — bug fixes.
- `### Security` — security-relevant fixes.

Each entry references the issue or PR (`(#42)`). On release, rename
`## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD`, create a fresh empty
`## [Unreleased]` above it, tag, and update the `tree-sitter.json` /
package metadata version.

PRs that don't touch user-visible behavior (refactors, internal docs,
CI tweaks, regenerated parser source from a no-op grammar edit) do
**not** require a changelog entry — but should say so explicitly in
the PR description so the omission is intentional.

## GitHub workflow

- For non-trivial `gh issue` / `gh pr` bodies, write to a temp file
  and pass via `--body-file` to avoid shell quoting issues.
- Do not push or open PRs without explicit user instruction.
- Only close an issue when ALL items are resolved.
- When updating issues, update BOTH the body AND add a comment.

## Tone

Criticism is welcome — point out mistakes, suggest better approaches,
cite relevant standards. Be skeptical and concise.
