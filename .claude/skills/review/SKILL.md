---
name: review
description: Audit grammar / query / binding changes for correctness, performance, and quality. Use when asked to review changes, diffs, or pull requests.
---

# Review Changes

Audit the current change set for correctness, performance, security, and
quality problems. Produce concrete, actionable findings.

## Scope

Determine what to review based on `$ARGUMENTS`:

| Argument | Scope |
|----------|-------|
| *(empty)* | Unstaged + staged changes (`git diff HEAD`) |
| `staged` | Staged changes only (`git diff --cached`) |
| `branch` | All commits on current branch vs `main` (`git diff main...HEAD`) |
| `pr <N>` | Pull request diff (`gh pr diff <N>`) |
| `<commit>` | Single commit (`git show <commit>`) |
| `<commit>..<commit>` | Commit range |
| `<path or glob>` | Full-file review (no diff) |

---

## Step 1: Gather diff and context

1. Obtain the diff for the determined scope.
2. List every file touched. Read **full files** — not just hunks. Findings need
   surrounding context.
3. For grammar changes: regenerate (`npx tree-sitter generate`) before
   reasoning about effects. Stale `src/parser.c` / `src/grammar.json` is a
   source of spurious findings. The hand-written `src/scanner.c` is
   independent of regeneration.
4. Re-read the relevant `SPECIFICATION.md` section. Grammar shape must
   match the spec; if it doesn't, the *spec* may be wrong rather than the
   code — flag the divergence either way.
5. If `docs/lessons_learned.md` exists, read it and check whether any lesson
   applies.

---

## Step 2: Audit checklist

Apply every applicable question. Record each finding:

```
FINDING: <short title>
FILE: <path>:<line range>
EVIDENCE: <what is wrong and why>
SEVERITY: bug | performance | security | code-smell | test-gap
EFFORT: trivial | small | medium
```

### Grammar correctness (`grammar.js`)

1. Does the new rule conflict with an existing rule? `tree-sitter generate`
   prints conflicts — verify the diff produces zero conflicts. Conflicts
   that are intentional must be listed in the grammar's `conflicts` array
   with a comment pointing to the relevant `SPECIFICATION.md` section.
2. Are precedences correct for new operators? The precedence table in
   `SPECIFICATION.md` §3.1 mirrors the Apache reference grammar's
   `expression` rule; a Groovy operator at the wrong `PREC` level silently
   parses but produces a wrong tree (e.g., putting `..` above shifts when
   Apache puts them at the same level).
3. Is `token.immediate` used where the grammar requires no whitespace
   between adjacent tokens (e.g. `?.`, `??.`, `*.`, `.&`, `.@`, `::`, the
   `as` keyword's trailing-context, `!instanceof`, `!in`)?
4. Does the rule introduce ambiguity? Look for: rules whose alternatives
   can match the same prefix without precedence; uses of `repeat` that
   could match zero or many tokens. The known-tricky Groovy ambiguities
   are documented in `SPECIFICATION.md` §5.3 (label / map / ternary),
   §5.4 (slashy vs division), §5.5 (command chain ambiguities).
5. For new top-level constructs (e.g. `trait_declaration`,
   `enum_declaration`): is the rule wired into `source_file` (or another
   producer of `_statement` / `_declaration`)? An unwired rule will never
   match.
6. Do field names (`field('object', ...)`, `field('property', ...)`) match
   what queries expect and what `SPECIFICATION.md` §3.2 promises?
7. Does the rule preserve the contract in `SPECIFICATION.md` §3.2 that
   each Groovy-specific operator emits a distinct named node?
   `binary_expression` with an `operator` field is correct for symmetric
   operators (`+`, `-`, `==`, etc.) but **wrong** for Elvis, safe-nav,
   spaceship, regex, identity, etc. — those must be their own node kinds.

### Scanner correctness (`src/scanner.c`)

8. Is the scanner state correctly serialised in
   `tree_sitter_groovy_external_scanner_serialize` and deserialised in
   `…_deserialize`? Anything used only within a single `scan` call may be
   a local; anything that must persist across calls (paren depth, GString
   nesting stack) **must** be in the serialised state.
9. Slashy-string-start (§6.2): is the previous-non-trivia-token context
   correctly tracked? Emitting `_slashy_string_start` after an
   identifier produces wrong parses (`a / b` becomes `a / b /`).
10. GString interpolation (§6.3): does the scanner correctly track brace
    depth inside `${ … }`? An unbalanced `{` inside the interpolation must
    not close the GString.
11. Automatic semicolon (§6.4): the scanner must NOT emit a terminator if
    the next token is one that continues the expression (dot, ternary `?`,
    binary operator, `else`, etc.).

### Query correctness (`queries/groovy/*.scm`)

12. Do all node types referenced exist in the regenerated
    `src/node-types.json`? `Impossible pattern` at runtime means the
    query references a non-existent node.
13. Are field selectors correct? `(method_invocation name: (identifier))`
    must match the actual field name the grammar uses.
14. Are `#match?` regex anchors correct? An unanchored regex over names
    will over-match.
15. Are highlight tags consistent with neovim/helix conventions
    (`@function.builtin`, `@constant`, `@operator`, `@keyword`)?
16. Are Groovy 4 contextual keywords (`async`, `await`, `defer`, `var`,
    `record`, `sealed`, `non-sealed`, `permits`, `yield`, `val`) only
    highlighted as `@keyword` in their keyword positions, not as
    identifiers used elsewhere?

### Binding correctness (`bindings/`)

17. Does every binding call `tree_sitter_groovy()` (not e.g.
    `tree_sitter_java()`)?
18. Are scanner symbols in `src/scanner.c` named
    `tree_sitter_groovy_external_scanner_*` (matching the grammar's name)?
    The scanner is hand-written and not touched by `tree-sitter generate`,
    so any drift must come from a manual edit.
19. Are external scanner symbols listed in the build inputs of every
    binding (`binding.gyp`, `setup.py`, `Package.swift`, Go `binding.go`,
    `bindings/rust/build.rs`)? Missing `scanner.c` at link time →
    undefined symbol at load time, which `node-gyp` does NOT catch.
20. Do package metadata files (`package.json`, `Cargo.toml`,
    `pyproject.toml`, `tree-sitter.json`, `go.mod`) all use the same name,
    version, and license?

### Test coverage

21. Does every new grammar rule have a corpus test under `test/corpus/`?
22. Do the corpus tests pin the exact AST shape — including field names
    — not just rule presence?
23. For new highlight captures: is there an entry under `test/highlight/`
    that asserts the capture by name?
24. Are negative tests present where Groovy diverges from Java? E.g.
    `trait T { … }` should not be confused with a juxt-call followed by
    a closure; `0..<n` should not parse as `0 .. < n`.
25. Are precedence tie-break tests present for cases like
    `a ? b ?: c : d`, `0..n<<1`, `a ?: b ?: c` (see
    `SPECIFICATION.md` §8.1 corpus list)?

### Performance

26. Are new regex tokens unbounded? `identifier: /[A-Za-z_$][A-Za-z0-9_$]*/`
    is fine; a new rule using `/.+/` is not.
27. Does the grammar still pass `npx tree-sitter test` in well under a
    second? A new rule that explodes parser table size shows up in
    regeneration time and `parser.c` size.

### Security

28. Does any change to bindings expose a path that loads an arbitrary
    `.so`, or trusts user-supplied paths without canonicalisation?
29. Do scanner edits introduce unbounded loops or unbounded memory
    growth on pathological input (e.g. unterminated slashy string,
    unterminated triple-quoted GString)?

### Code quality

30. Stale comments referencing prior Groovy grammars (`amaanq`,
    `murtaza64`) in newly added code?
31. Inconsistent naming (`Java` vs `Groovy`, `java` vs `groovy`)
    introduced by the diff?

---

## Step 3: Validate findings

For each finding: re-read the evidence, confirm file and line range,
discard anything speculative. Collapse findings sharing a root cause.

If a finding is pre-existing and the diff does not make it worse, mark
"pre-existing" but still report it.

---

## Step 4: Report

```
## Review: <scope description>

### Bugs / Grammar
| # | Finding | File | Effort | Evidence |

### Test gaps
| # | Finding | File | Effort | Evidence |

### Code quality
| # | Finding | File | Effort | Evidence |

### Summary
- Files reviewed: N
- Findings: N
- Verdict: APPROVE | APPROVE WITH COMMENTS | REQUEST CHANGES
```

If zero findings, say so explicitly and state APPROVE.

---

## Guardrails

- Do NOT implement fixes. Review-only.
- Do NOT report findings without concrete evidence (file + line + reasoning).
- Read full files, not just hunks.
- Verify regeneration was run before assessing grammar changes — stale
  `src/parser.c` will mislead you.
