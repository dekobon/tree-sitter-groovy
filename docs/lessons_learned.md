# Lessons learned

Long-form lessons from non-obvious bugs encountered while building
`tree-sitter-groovy`. Format borrowed from `tree-sitter-irules`.

Each lesson is numbered and immutable once added — append rather than
edit, so old references stay valid.

---

## 1. External scanner must own everything that starts with `/` (and the whitespace next to it)

**Date**: 2026-05-17.
**Context**: Wiring up `block_comment`, `groovydoc_comment`, and
`line_comment` in `src/scanner.c` for SPECIFICATION.md §6.6.

**Symptom**: A file like

```groovy
foo
// hi
bar
```

parsed as `(source_file (identifier) (ERROR (UNEXPECTED '/')) (identifier) (identifier))`
even though the external scanner had a working line-comment branch
that succeeded for `//x` at start-of-file. The first `/` of the
comment was being consumed before the scanner saw it; the scanner
was being invoked at column 1 (the second `/`) instead of column 0.

**Root cause**: With `extras: $ => [/\s/, $.line_comment, ...]`,
tree-sitter's generated lexer matched `\n` via the `\s` rule and
called `SKIP(0)`, which advances past the newline and re-enters the
internal lex state machine — *without* re-invoking the external
scanner. The internal lexer then tried to match `/` at the new
position. No internal rule matches `/`, so the lexer failed; error
recovery consumed `/` as part of an ERROR node and re-attempted
lexing from the next character. By the time the external scanner
ran, the first `/` had already been eaten.

**Fix**: Have the external scanner skip its own leading whitespace
via `lexer->advance(lexer, true)` before checking for `/`. Then,
when tree-sitter calls the scanner *before* the internal lexer (the
documented behaviour at the start of each lex attempt), the
scanner gets the chance to consume both the whitespace and the
following comment as one combined skip+token.

**Generalisation**: Any time an external token shares a *prefix
class* (here, "what comes right after whitespace") with what
in-grammar `extras` would otherwise SKIP, the external scanner
must do the leading-whitespace skip itself. Otherwise tree-sitter
falls through to error recovery between the SKIP and the next
external attempt. This is independent of whether `\s` stays in
`extras` — the scanner needs to be defensive about its lookahead
either way.

**Where this matters next**:

- Slashy strings (`/pat/`) share the leading `/` with comments and
  with division. The same skip-then-dispatch pattern applies.
- `AUTOMATIC_SEMICOLON` (§6.4) wants to *fire* on a newline. That
  scanner branch is at odds with skipping whitespace; we'll need
  a separate code path that runs *before* the whitespace skip so
  the auto-semi check sees the `\n` instead of swallowing it.

## 2. Tree-sitter query `_` wildcard only matches *named* nodes

**Date**: 2026-05-17.
**Context**: Iter 5 added a highlight query for unary operators:

```scm
(unary_expression
  operator: _ @operator)
```

The corresponding `test/highlight/literals.groovy` assertions
(`-x // <- operator`, `!flag // <- operator`) passed in iter 5
through iter 7. In iter 8, adding `binary_expression` to the
grammar caused the same assertions to start failing with
`expected highlight 'operator', actual highlights: none`.

**Root cause**: The `_` wildcard in tree-sitter queries matches
*named* nodes only. Anonymous tokens (`+`, `-`, `!`, `~` and
similar single-character operators, parens, etc.) are NOT matched
by `_`. The iter-5 query was already silently broken — it
asserted nothing — but the highlight-test runner happens to pass
when an assertion has no matching capture for some scanner
configurations and fails when grammar changes shift the scanner
state. The new failure is the symptom; the latent bug existed
the whole time.

**Fix**: Capture anonymous-token operators by listing them in a
top-level alternation:

```scm
[
  "+" "-" "*" "/" "%" "!" "~"
] @operator
```

This matches the literal anonymous tokens wherever they appear,
covering both unary and binary uses with a single rule.

**Generalisation**: Whenever you want to highlight a punctuation
character or a single-keyword operator, list it literally rather
than relying on `_` in a field position. Reserve `_` for cases
where the field can hold a *named* sub-node (e.g.
`field: (some_named_node) @cap`).

## 3. Corpus test titles cannot contain `==` runs

**Date**: 2026-05-17.
**Context**: Iter 10 added `test/corpus/operators-regex.txt` with
test titles like `==~ is one token (not == ~)` and `== ~ with
spaces parses as binary plus unary`.

**Symptom**: A single test in the middle of the file failed
spectacularly — the expected tree was one regex node, but the
actual parse was a much larger fragment containing several
following tests' inputs merged together, with `MISSING ")"` and
`ERROR` nodes everywhere.

**Root cause**: The tree-sitter corpus test format delimits tests
with lines made entirely of `=` characters. The test-file parser
scans for these delimiter runs to slice the file into test
cases. When a title *contains* `==` (or longer), the parser
mis-identifies the title row as a partial delimiter and merges
adjacent tests, throwing off the boundary detection for every
subsequent test in the file.

**Fix**: Rename titles to avoid `=` characters entirely. For
operator names, write them out longhand: `==~` becomes "Regex
match" or "Regex match is one token, not equality plus complement."

**Generalisation**: Corpus test titles should be plain prose with
no syntactically significant characters (`=`, possibly `-`, `<`).
Embed example tokens in the body, not in the title. This also
keeps the test list output readable.
