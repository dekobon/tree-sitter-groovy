# Contributing to tree-sitter-groovy

Thank you for your interest in contributing. This document covers the
essentials; [`AGENTS.md`](AGENTS.md) has the full project conventions
(commit messages, versioning, validation gates) and
[`SPECIFICATION.md`](SPECIFICATION.md) is the authoritative grammar
design document.

## Getting started

```bash
npm install
npx tree-sitter generate
npx tree-sitter test
```

## Making changes

### Grammar (`grammar.js`)

1. Read the relevant section of `SPECIFICATION.md` before changing a
   rule.
2. After every edit, regenerate: `npx tree-sitter generate`.
3. Do not hand-edit `src/parser.c`, `src/grammar.json`, or
   `src/node-types.json` — they are build outputs.

### External scanner (`src/scanner.c`)

The scanner is hand-written and survives regeneration. It handles
context-sensitive lexing (slashy vs. division, GString interpolation,
comments). Edit it directly when the change is in its domain.

### Queries (`queries/groovy/*.scm`)

Highlight, fold, indent, injection, locals, and tag queries live under
`queries/groovy/`. When adding a new grammar rule, add the
corresponding query captures at the same time.

### Tests

- **Corpus tests** (`test/corpus/*.txt`) — input + expected S-expression.
  One file per operator or statement family.
- **Highlight tests** (`test/highlight/*.groovy`) — assertion-based
  highlight checks.
- **Regression tests** — when fixing a bug, add a test that would catch
  it if reintroduced.

## Validation

Before submitting a pull request, run:

```bash
npx tree-sitter generate
npx tree-sitter test
npm run lint
```

If you changed Rust bindings:

```bash
cargo clippy --all-targets -- -D warnings
cargo test
```

If you changed repo-level config (TOML, Markdown):

```bash
typos
taplo format --check
markdownlint-cli2
```

## Commit messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).
Format: `<type>(<scope>): <subject>`.

| Type | Use for |
|------|---------|
| `feat` | New grammar rule, query capture, or binding feature |
| `fix` | Bug fix in grammar, scanner, queries, bindings, or tests |
| `refactor` | Restructure without behavior change |
| `docs` | Documentation only |
| `test` | Add or correct tests only |
| `build` | Build/packaging files |
| `ci` | CI workflow changes |
| `chore` | Release tags, regenerated parser, version bumps |

Subject line: imperative, lowercase, no trailing period, max 72 chars.
Issue references (`Fixes #NN`) go in the body, not the subject.

## Changelog

If your change affects user-visible behavior, add an entry under
`## [Unreleased]` in [`CHANGELOG.md`](CHANGELOG.md) in the appropriate
section (Added / Changed / Fixed / Removed). Reference the issue or PR
number.

## Reporting bugs

Open a [GitHub issue](https://github.com/dekobon/tree-sitter-groovy/issues)
with:

1. A minimal Groovy snippet that triggers the bug.
2. The expected parse tree (or highlight result).
3. The actual parse tree you see.

## Code of conduct

This project follows the
[Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).
