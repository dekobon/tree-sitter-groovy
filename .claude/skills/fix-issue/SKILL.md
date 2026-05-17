---
name: fix-issue
description: Complete workflow for fixing GitHub issues — investigation, implementation, review, testing, documentation.
---

# Fix GitHub Issue Workflow

1. Read the issue: `gh issue view <number>` plus all comments.
2. Re-read project conventions: `CLAUDE.md`, `AGENTS.md`, `README.md`,
   and the relevant section of `SPECIFICATION.md`. Note any rule
   directly relevant.
3. Investigate. The bug may live in:
   - **`grammar.js`** — wrong rule, wrong precedence, ambiguous
     alternatives.
   - **`src/scanner.c`** — external scanner mishandling of an edge case
     (slashy-vs-division, GString interpolation, automatic semicolon,
     labelled-statement colon, block comment / Groovydoc).
   - **`queries/groovy/*.scm`** — wrong capture, wrong node type, broken
     `#match?` regex.
   - **`bindings/`** — symbol mismatch, missing scanner in build inputs.
   - **`test/corpus/`** — false-pass test that masked the real bug.
   - **`SPECIFICATION.md`** — design document inconsistency.
   Locate the root cause before changing anything.
4. **Plan with sequential thinking.** Use the
   `sequential-thinking:sequentialthinking` MCP tool to reason step-by-step.
   The plan MUST cover:
   - Root cause (not just symptom).
   - Approaches and trade-offs.
   - Edge cases: empty input, deeply-nested braces, escaped braces,
     comments inside class bodies, GString interpolation in slashy
     strings, multi-catch with three or more types, multi-line strings
     containing operators, command chains with named arguments and
     trailing closures, mixed line endings, non-ASCII identifiers.
   - Anti-patterns to avoid (silent ambiguity, regex over-match, fields
     defined but not used by queries, generic `binary_expression` for
     Groovy-specific operators).
   - Tests that will be added.
   - Documentation to update.
5. **Implement the fix.** For grammar / scanner edits, regenerate after
   every change:

   ```bash
   npx tree-sitter generate
   ```

   `tree-sitter generate` rewrites `src/parser.c`, `src/grammar.json`,
   and `src/node-types.json`. It does not touch `src/scanner.c`; the
   scanner's `tree_sitter_groovy_external_scanner_*` symbols stay in
   sync automatically.

6. **Write tests first or alongside.** At minimum:
   - **Corpus test** under `test/corpus/` reproducing the bug input and
     pinning the correct AST. Include field names.
   - **Highlight test** under `test/highlight/` if the bug was
     capture-related.
   - **Regression test** that would catch the exact bug if reintroduced.

7. **Validation gates** before committing:

   ```bash
   npx tree-sitter generate
   npx tree-sitter test     # corpus + highlight
   npm run lint             # eslint over grammar.js
   ```

   If any check fails, fix and re-run.

8. **Review the change** for: correctness (root cause vs symptom),
   completeness (other rules affected by the same change?), simplicity,
   test coverage, conventions (no comments unless the *why* is
   non-obvious), distinct named nodes for Groovy-specific operators
   (per `SPECIFICATION.md` §3.2).

9. Update documentation:
   - `README.md` — if user-facing behavior changed.
   - `SPECIFICATION.md` — if a grammar shape decision changed.
   - `CHANGELOG.md` — add an entry under `## [Unreleased]` in the
     matching section (Added / Changed / Fixed / Removed / Security).
     Reference the issue with `(#NN)`. Skip only when the change has no
     user-visible effect (pure refactor, internal docs, CI tweak) — and
     say so explicitly in the PR description.
   - `docs/lessons_learned.md` — only if the bug cost real debugging
     time and is likely to recur. High bar.

10. If a hard-won lesson came out of the fix, run `/lessons-learned` to
    draft it and prompt the user for approval.

11. Commit with Conventional Commits (`fix(grammar): ...`,
    `fix(scanner): ...`, `fix(queries): ...`, `fix(bindings): ...`).
    Subject imperative, lowercase, no trailing period, ≤ 72 chars.
    `Fixes #NN` in the body, not the subject. Allowed types and scopes
    are listed in `AGENTS.md`.

12. Update the issue body with results AND add a comment with research
    and findings. Use `--body-file` for non-trivial bodies.

13. Close the issue with `gh issue close <number>` only when ALL items
    are resolved.

## Worktree safety reminder

If running inside a worktree (`git rev-parse --show-toplevel` returns a
path under `.claude/worktrees/`): never delete worktrees, never `cd` to
the main repo, never check out a different branch, never write outside
your worktree.
