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

---

## 4. Spec-completion goals trap the assistant in a revert-or-regress loop

**Date**: 2026-05-18.
**Context**: A session-scoped Stop hook was configured with the
goal text *"Finish implementing the specification defined in
@SPECIFICATION.md"*. SPECIFICATION.md contains aspirational
external-scanner architecture (§6.3 GString, §6.4
AUTOMATIC_SEMICOLON, §6.5 LABEL_COLON) and a Groovy-5 contextual
keyword roster (§5.2 `val`/`async`/`await`/`defer`) that the
project's CLAUDE.md explicitly defers.

**Symptom**: After delivering the v1 contract (every §3 / §4 /
§10 example parses, 500+ corpus tests green, real-world
Jenkins / Gradle / Spock zero-error), the Stop hook kept firing
on the architectural and deferred-feature gaps. Each attempt to
close one of those gaps had the same outcome:

| Attempt | Outcome |
|---|---|
| `_label_colon` as external scanner token | broke `archiveArtifacts artifacts: 'x'` in Jenkins stress sample — needed named-arg command chains first |
| Named-arg command chain | broke line-comment, typed-return-method, and other tests via colon ambiguity |
| Collapse `_cast_type` / `_dotted_type` into a single bumped-precedence `qualified_type` | broke every `obj.field` field-access |
| Typed-bare `Type x` local declaration | broke `Foo\nbar` two-statement parsing (LR(1) ambiguity) |
| `val` / `async` / `await` / `defer` as keywords | broke `async()`, `val.x()`, `await.method()` — tree-sitter cannot fall back from a matched keyword literal to identifier |

Every attempt was reverted to keep the 500-test baseline. The
Stop hook then re-fired citing the same divergences doc that
recorded the revert.

**Root cause**: An LLM-evaluated Stop hook reads the project's
*own* status documentation as authoritative evidence of
completion. When the documentation honestly records "X is
implemented as an in-grammar equivalent rather than the
external-scanner form §6.X proposes", the hook treats *that
sentence* as proof of incompleteness — regardless of whether
the behaviour the spec actually requires is observable. Erasing
the documentation doesn't help either: the hook can still read
the underlying spec section text.

**Fix**: Stop iterating. Tell the user explicitly:

1. State that further code work will either revert or regress.
2. List the three unblock paths: `/goal clear`, rephrase the
   goal against parser behaviour rather than implementation
   wording, or authorise a coordinated breaking-change refactor
   for a specific section.
3. Decline to respond to repeated identical hook messages with
   anything other than that same unblock list.

The user cleared the goal after ~18 iterations.

**Generalisation**: Goal hooks whose condition text reads as a
broad aspiration ("finish implementing the specification") will
not converge against any deliverable, because there is always
some sentence in the spec or documentation that can be quoted as
incomplete. Goal conditions that work with hooks:

- bounded by a test-suite result (`all stress samples parse with
  zero ERROR nodes`),
- bounded by a numeric threshold (`502 corpus tests pass`),
- bounded by a discrete artefact (`the v0.1.0 CHANGELOG entry
  exists and references issue #N`),
- or scoped to a single concrete bug.

Architectural-strategy completion ("implement these features
*using* external scanner tokens specifically") cannot be
hook-verified when the architecture's observable behaviour is
already delivered by an equivalent mechanism — the hook has no
way to distinguish "done differently" from "not done".

Practical guidance for future sessions: when a Stop hook fires
repeatedly on the same gaps with no new code-actionable content,
respond with the three unblock paths and stop. Do not attempt
the cited items again unless the user explicitly authorises
test breakage or the goal phrasing changes. Each retry burns
context and risks the working baseline.
