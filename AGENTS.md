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
- `src/scanner.c` — hand-written external scanner. Handles the four
  context-sensitive lexing situations from `SPECIFICATION.md` §6:
  slashy-vs-division, GString interpolation, automatic semicolons
  (newline-as-terminator), labelled-statement colons, and the
  block-comment / Groovydoc single-token emit. Symbols are
  `tree_sitter_groovy_external_scanner_*`. `tree-sitter generate` does
  NOT rewrite `scanner.c`, so these symbols are stable across
  regenerations.
- `src/tree_sitter/*.h` — vendored tree-sitter runtime headers; do not
  edit.
- `queries/groovy/highlights.scm` — Treesitter highlight captures.
  Includes Groovy 4 contextual keywords (`async`, `await`, `defer`,
  `var`, `record`, `sealed`, `non-sealed`, `permits`, `yield`,
  `val`).
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
Keep-a-Changelog-formatted `CHANGELOG.md` at the repo root.

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

```
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

The published artifacts (`tree-sitter-groovy` on crates.io, npm, PyPI;
Go module; Swift package) follow [SemVer 2.0.0](https://semver.org/).
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
