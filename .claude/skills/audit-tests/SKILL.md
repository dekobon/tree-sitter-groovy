---
name: audit-tests
description: Audit corpus and highlight tests for tests that pass for the wrong reason — designed to pass rather than designed to catch regressions.
---

# Audit Tests

Audit test code for tests that **pass for the wrong reason**. Goal: find
tests that look like coverage but would not catch a real regression.

## Scope

Determine what to audit based on `$ARGUMENTS`:

| Argument | Scope |
|----------|-------|
| *(empty)* | Tests in unstaged + staged changes (`git diff HEAD`) |
| `staged` | Tests in staged changes only |
| `branch` | Tests in all branch commits vs `main` |
| *file path* | Specific test file (`test/corpus/*.txt`, `test/highlight/*.groovy`) |
| *directory path* | All test files under the directory |

---

## Step 1: Collect test code

1. For diff-based scopes: extract only the corpus/highlight tests added or
   modified. Pre-existing unchanged tests are out of scope.
2. For path scopes: include all corpus tests under `test/corpus/` and
   highlight tests under `test/highlight/`.
3. Read each test file in full — surrounding tests and shared fixtures
   matter.

---

## Step 2: Understand what each test claims to verify

For every test in scope, answer three questions:

1. **What does the test name say?** Parse the heading as a specification.
2. **What does the body actually verify?** For corpus tests, read both the
   input source and the expected AST. For highlight tests, read the input
   and the inline `; ^^^^ @capture` markers.
3. **What would have to break for this test to fail?** That is the test's
   real coverage claim.

If 1 and 3 disagree, that is a finding.

---

## Step 3: Audit checklist

```
FINDING: <short title>
FILE: <path>:<line range>
EVIDENCE: <what is wrong and why>
SEVERITY: false-pass | weak-assertion | wrong-target | incidental
EFFORT: trivial | small | medium
```

### Corpus tests (`test/corpus/`)

1. **Empty / wildcard expected tree**: does the expected `(source_file)`
   have no children, or only opaque parents? A test that asserts "parses
   to *some* source_file" catches almost nothing.
2. **Trivial input**: does the input not actually exercise the rule the
   test name implies? A test named `Elvis chain` whose input is `a ?: b`
   (single Elvis, not a chain) is false-pass.
3. **No field names where the rule defines them**: `elvis_expression`
   defines `field('value', ...)` and `field('default', ...)`. If the
   expected tree omits the field labels, a regression that drops the
   field wiring still passes.
4. **Wrong rule asserted**: does the expected tree show the input parsing
   as a generic `(binary_expression)` when the new rule
   (`elvis_expression`) was supposed to handle it? That is a wrong-target
   test masquerading as coverage of the new rule. This is the exact
   anti-pattern `SPECIFICATION.md` §3.2 promises to avoid.
5. **Coupling to incidental whitespace / formatting**: corpus tests
   should not be sensitive to comment placement or trailing newlines
   unless the grammar specifically pins those.
6. **Single-input test for a multi-form rule**: an `if_statement` rule
   should be tested with `if`, `if/else`, `if/else if/else`, nested `if`,
   and `if` followed by a closure — not just one canonical example.
7. **Precedence tie-break absence**: tests for `?:` / `? :` / `..` /
   `<<` / `==>` must include the tie-break cases enumerated in
   `SPECIFICATION.md` §8.1 (`operators-precedence-tiebreaks.txt`).
   Without those, a precedence regression silently changes the tree
   shape for valid inputs.

### Highlight tests (`test/highlight/`)

8. **Capture marker on the wrong span**: `; ^^^^^ @keyword` aligned to
   fewer characters than the keyword spans is a silent partial check.
9. **Missing negative coverage**: if the grammar conditionally tags
   `MyClass::new` as `@function`, the highlight tests must include a
   non-qualified call (`new MyClass()`) to confirm the regex did not
   over-match. Same applies to the Groovy 4 contextual keywords
   (`async`, `await`, etc.) which are identifiers outside their keyword
   positions.

### Tests that test the framework

10. Does a new test reduce to "tree-sitter parses anything" rather than
    the specific construct? `(source_file)` with no children is a smoke
    test, not a regression test.

---

## Step 4: Verify findings by execution

For every finding, run:

```bash
npx tree-sitter test --include "<test-name regex>"
```

Confirm the test currently passes. For false-pass findings, the
strongest verification is mutation: temporarily perturb `grammar.js`
(e.g. delete the `elvis_expression` rule, or replace it with a generic
`binary_expression`), regenerate (`npx tree-sitter generate`), and re-run
the test. If the test still passes, the false-pass is confirmed.
**Revert the grammar change after.**

This step is mandatory. Do not report findings based solely on reading.

---

## Step 5: Report and fix

```
## Test Audit: <scope>

### False-pass
| # | Finding | File:Line | Effort | Evidence |

### Weak assertions
| # | Finding | File:Line | Effort | Evidence |

### Wrong target
| # | Finding | File:Line | Effort | Evidence |

### Summary
- Tests audited: N
- Findings: N
- Verdict: PASS | FINDINGS TO FIX
```

For each finding, fix the test directly:

- **False-pass**: redesign the input so the rule under test is actually
  exercised.
- **Weak assertion**: add the missing field labels or child nodes to the
  expected tree.
- **Wrong target**: rename or rewrite to match the name.

After fixing, re-run `npx tree-sitter test` to verify everything still
passes.

---

## Guardrails

- Do NOT report findings without verifying by execution.
- Do NOT touch grammar / queries / bindings — only test files.
- Never rewrite an entire corpus file to fix one test; modify only what
  needs changing.
- Run `npx tree-sitter test` after fixes to confirm nothing else broke.
