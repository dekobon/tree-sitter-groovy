---
name: batch-fix
description: Fix multiple GitHub issues on an integration branch. Issues touching different areas (grammar, scanner, queries, bindings, tests) run in parallel worktrees; issues sharing an area or affecting cross-binding code run sequentially. Each goes through fix, simplify, review, and remediation before merging. Use when asked to fix several issues at once.
---

# Batch Fix GitHub Issues

Fix multiple GitHub issues on a single integration branch. Issues are
classified by affected area(s) and triaged for quick-win priority and
cross-issue dependencies, then scheduled into waves where issues touching
different areas run in parallel. Quick wins are front-loaded for fast
feedback. Issues sharing an area, or any issue that touches `bindings/`
(per-language bindings deliberately mirror each other), are serialized to
avoid merge conflicts. Each issue goes through the full pipeline:
investigate, fix, simplify, review, remediate, validate, commit. Successful
fixes are merged to the integration branch. Failures are logged and skipped.

## Arguments

Parse `$ARGUMENTS` as a space-separated list of issue references and flags:
`#42 #57 #63` or `42 57 63` (with or without `#` prefix).

Optional flags:

- `--sequential`: force single-issue waves (no parallel processing). Use
  when issues have cross-area dependencies that would conflict on merge.

Extract the numeric issue numbers. If no issues are provided, abort with:
"Error: provide at least one issue number. Usage: /batch-fix #42 #57 #63"

---

## Step 0: Validate

### 0a: Validate issues exist

For each issue number, run:

```bash
gh issue view <number> --json number,title,state,labels,body,comments --jq '{number, title, state, labels: [.labels[].name], body, comments}'
```

If any issue does not exist or is already closed, warn the user and
remove it from the list. If no valid open issues remain, abort.

Record each issue's number, title, body, labels, and comments for later
steps. This data is reused in Step 2 (classification) and Step 4
(worktree agent prompts) -- do not re-fetch.

### 0b: Ensure clean working tree

```bash
git status --porcelain
```

If there are uncommitted changes, abort with:
"Error: working tree is dirty. Please commit or stash your changes before
running /batch-fix."

### 0c: Detect isolation mode

```bash
PROJECT_ROOT="$(git rev-parse --show-toplevel)"
if [[ "$PROJECT_ROOT" == *".claude/worktrees/"* ]]; then
  ISOLATION_MODE="worktree"
else
  ISOLATION_MODE="branch"
fi
```

- **Worktree mode**: Agents are launched with `isolation: "worktree"` and
  run in parallel (existing behavior).
- **Branch mode**: Agents are launched WITHOUT `isolation: "worktree"`
  and run sequentially using feature branches. All agents in a wave MUST
  be processed one at a time (they share the working directory).

Record `ISOLATION_MODE` for use in Step 4.

---

## Step 1: Create integration branch

Determine a unique branch name. Try `fix/batch-YYYY-MM-DD` first, then
append a sequence number if it already exists:

```bash
DATE=$(date +%Y-%m-%d)
BRANCH="fix/batch-${DATE}"
SEQ=2
while git rev-parse --verify "$BRANCH" >/dev/null 2>&1; do
  BRANCH="fix/batch-${DATE}-${SEQ}"
  SEQ=$((SEQ + 1))
done
git checkout -b "$BRANCH" main
```

Record the branch name as `INTEGRATION_BRANCH`.

---

## Step 2: Classify and triage issues

For each issue, determine which area(s) it affects and assess complexity.

### 2a: Area classification

The project areas are:

- `grammar` — `grammar.js`. Generated outputs `src/parser.c`,
  `src/grammar.json`, `src/node-types.json` are touched whenever this
  area changes.
- `scanner` — `src/scanner.c`, the hand-written external scanner for
  slashy-vs-division, GString interpolation, automatic semicolons,
  label colons, block comments / Groovydoc.
- `queries` — `queries/groovy/*.scm` (highlights, folds, indents,
  injections, locals).
- `bindings` — per-language wrappers under
  `bindings/{c,go,node,python,rust,swift}/`. **All bindings expose
  `tree_sitter_groovy()` and are siblings — a bug in one binding often
  exists in others.**
- `tests` — `test/corpus/*.txt` and `test/highlight/*.groovy`.
- `ci` — `.github/workflows/**`.
- `docs` — `README.md`, `AGENTS.md`, `CLAUDE.md`, `CHANGELOG.md`,
  `SPECIFICATION.md`, `docs/`.
- `deps` — `package.json`, `Cargo.toml`, `pyproject.toml`,
  `tree-sitter.json`, `go.mod`, `Package.swift`, `binding.gyp`,
  `setup.py`, `Makefile`.

Signals (in priority order):

1. **Labels**: GitHub labels matching area names map directly to areas.
2. **Title/body keywords**:
   - "grammar", "rule", "precedence", "ambiguity", "node type", "field",
     "AST", `grammar.js`, "elvis_expression", "trait_declaration" →
     `grammar`
   - "scanner", "external scanner", `scanner.c`, "slashy", "GString",
     "automatic semicolon", "tokenizer" → `scanner`
   - "highlight", "capture", "fold", "indent", `.scm`, `#match?`,
     "injection" → `queries`
   - "binding", "node binding", "rust binding", "python binding",
     "go binding", "swift binding", "c binding", `tree_sitter_groovy()`
     → `bindings` with `cross_lang: true` (see special case below)
   - "corpus test", "highlight test", `test/corpus/`, `test/highlight/`,
     "regression test" → `tests`
   - "workflow", "CI", "GitHub Actions", `.github/workflows/` → `ci`
   - "README", "AGENTS.md", "CLAUDE.md", "CHANGELOG", "doc",
     "documentation", `SPECIFICATION.md` → `docs`
   - "version bump", "tree-sitter-cli version", "package metadata",
     "Makefile", "package-lock.json", "Cargo.lock" → `deps`
3. **Ambiguous**: If the area cannot be determined from labels or
   keywords, classify as `unknown`.

**Special case — `bindings/` and cross-binding code**: The per-language
bindings under `bindings/<lang>/` deliberately mirror each other.
Flag `cross_lang: true` for any issue touching this directory other than
a single, language-specific binding bug clearly scoped to one wrapper.

**Special case — `grammar` ripple effects**: Grammar changes regenerate
`src/parser.c`, `src/grammar.json`, and `src/node-types.json` and
commonly require updates to `queries/groovy/*.scm` and `test/corpus/`.
Flag a grammar issue `cross_area: true` when the body explicitly
mentions renaming a node, removing a field, or changing AST shape.

### 2b: Quick-win detection

Flag issues as `quick_win: true` if they match **two or more** positive
indicators AND **zero** disqualifiers.

**Positive indicators**:

- References a single specific file path
- Contains a clear failing-test name, parser error, or capture-name
  typo
- Mentions a specific grammar rule, query capture, or scanner branch
- Has a "good first issue" or "bug" label
- Body is short (< 500 characters) with a clear reproduction case
- Fix is described in the issue itself

**Disqualifiers** (any one prevents quick-win):

- Requires AST-shape change
- Spans multiple areas explicitly
- Needs external input or design decision
- References missing Apache Groovy language coverage
- Requires a `tree-sitter-cli` version bump
- Has `cross_area: true` or `cross_lang: true`
- Requires editing `SPECIFICATION.md`

### 2c: Cross-issue dependency detection

Scan each issue for `depends on #<N>`, `blocked by #<N>`, `after #<N>`,
`requires #<N>`. Bare `#<N>` references do NOT imply dependency. Only
consider references to issues in the current batch.

**Cycle detection**: If dependencies form a cycle, log a warning and
drop all edges in the cycle.

### 2d: Print classification

```
## Issue Classification
| Issue | Title | Area | Cross-area | Cross-lang | Quick-win | Depends on |
|-------|-------|------|------------|------------|-----------|------------|
```

---

## Step 3: Schedule waves

Group issues into processing waves. The algorithm is identical to the
generic tree-sitter `batch-fix` workflow:

1. Two issues can run in the same wave only if they affect **different
   areas** (neither is `unknown`, neither is `cross_area`, neither is
   `cross_lang`).
2. `unknown` / `cross_area` / `cross_lang` issues are placed in their
   own wave (one at a time) after all classified issues.
3. `--sequential` forces every issue into its own wave.
4. **Dependency ordering** takes precedence over quick-win priority.
5. **Quick-win priority**: Within each area group, quick-win issues are
   scheduled first.
6. User-specified order is the tiebreaker.

Print the wave plan:

```
## Processing Plan
Isolation: <worktree (parallel) | branch (sequential)>
Wave 1 (parallel): #42 (grammar, quick-win), #57 (queries)
Wave 2 (parallel): #63 (scanner), #71 (tests, quick-win)
Wave 3 (sequential): #80 (cross-lang bindings, depends on #42)
```

---

## Step 4: Process waves

For each wave:

### 4a: Spawn agents

Use the issue data cached from Step 0a to populate each agent's prompt.
Substitute `<ISSUE_NUMBER>`, `<ISSUE_TITLE>`, and `<ISSUE_BODY>` into
the agent prompt (see below).

#### Worktree mode (`ISOLATION_MODE=worktree`)

Every agent MUST be launched with `isolation: "worktree"` and
`model: "opus"`. Multi-issue waves: launch ALL agents in a single
message block (parallel tool calls). Do NOT use `run_in_background`.

#### Branch mode (`ISOLATION_MODE=branch`)

Agents are processed **sequentially** (one at a time). For each issue:

```bash
BRANCH="fix/issue-${ISSUE_NUMBER}"
if git rev-parse --verify "$BRANCH" >/dev/null 2>&1; then
  git branch -D "$BRANCH"
fi
git checkout -b "$BRANCH" "$INTEGRATION_BRANCH"
```

Launch ONE Agent with `model: "opus"` (no `isolation`).

**On SUCCESS**:

```bash
git checkout "$INTEGRATION_BRANCH"
git merge "fix/issue-${ISSUE_NUMBER}" --no-edit
git branch -d "fix/issue-${ISSUE_NUMBER}"
```

**On FAILED / SKIPPED**:

```bash
git checkout -- .
git reset HEAD
git checkout "$INTEGRATION_BRANCH"
git branch -D "fix/issue-${ISSUE_NUMBER}"
```

### 4b: Process results (worktree mode only)

For each agent result:

**On SUCCESS**:

```bash
git checkout <INTEGRATION_BRANCH>
git merge <worktree-branch> --no-edit
```

If conflict: `git merge --abort`, log FAILED.

**On SKIPPED / FAILED**: log and continue.

### 4c: Wave checkpoint

```bash
git checkout <INTEGRATION_BRANCH>
npx tree-sitter generate
npx tree-sitter test
```

If `generate` or `test` fails after a multi-issue merge, bisect by
re-merging each wave branch one at a time.

---

## Step 5: Consolidate CHANGELOG

After all waves, collect CHANGELOG entries from successful agents and
apply them in a single commit on the integration branch.

```bash
git add CHANGELOG.md
git commit -m "$(cat <<'EOF'
docs(changelog): consolidate entries from batch fix

Update CHANGELOG.md with entries from all successfully merged issue
fixes in this batch.
EOF
)"
```

Collect non-empty `LESSON:` values for Step 7.

---

## Step 6: Final validation

```bash
git checkout <INTEGRATION_BRANCH>
npx tree-sitter generate
npx tree-sitter test
npm run lint
```

If `lint` fails on a fixable issue (trailing whitespace, missing
semicolon), fix and re-run. If `test` or `generate` fails, bisect the
merge commits to find the culprit, reset before the bad merge, and
re-apply subsequent good merges.

---

## Step 7: Summary

```
## Batch Fix Results
Branch: <INTEGRATION_BRANCH>
Isolation: <worktree | branch>

### Processing Plan
<wave plan from Step 3>

### Succeeded
| # | Issue | Title | Area | Quick-win | Wave | Commit | Files Changed |

### Skipped
| # | Issue | Title | Reason |

### Failed
| # | Issue | Title | Reason |

### Statistics
- Issues attempted: N
- Succeeded: N
- Skipped: N
- Failed: N
- Waves executed: N

### Proposed lessons
| # | From issue | Lesson |
```

If any lessons were proposed, end with: "Run `/lessons-learned` to
review."

---

## Agent Prompt

**BEGIN AGENT PROMPT**

You are fixing GitHub issue #<ISSUE_NUMBER>: <ISSUE_TITLE>

Issue body:

```
<ISSUE_BODY>
```

You must complete the full fix lifecycle: investigate, implement,
simplify, review, remediate, validate, commit. Do NOT close the GitHub
issue — only annotate it. The `Fixes #N` commit trailer will close it
on merge.

### Setup — Environment Verification (MANDATORY)

```bash
PROJECT_ROOT="$(git rev-parse --show-toplevel)"
if [[ "$PROJECT_ROOT" == *".claude/worktrees/"* ]]; then
  ISOLATION_MODE="worktree"
else
  ISOLATION_MODE="branch"
fi
AGENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
```

**HARD GATE**: If `AGENT_BRANCH` is `main`, `master`, or `HEAD`
(detached), abort and return `STATUS: FAILED`.

**BRANCH SAFETY**: Do NOT switch branches. All commits land on this
branch.

### Phase 1: Investigate and Fix

1. Re-read `CLAUDE.md`, `AGENTS.md`, and the relevant section of
   `SPECIFICATION.md`.
2. Read `docs/lessons_learned.md` if it exists.
3. Investigate root cause across grammar / scanner / queries / bindings
   / tests. For cross-binding fixes, check whether the same fix is
   needed in every `bindings/<lang>/`.
4. Plan with `sequential-thinking:sequentialthinking`. Cover root cause,
   approaches, edge cases (empty input, GString boundaries, multi-catch,
   command chains, contextual keywords), tests, and regeneration.
5. Implement. After every `grammar.js` edit, run
   `npx tree-sitter generate`.
6. Self-review: correctness, AST-shape stability (per
   `SPECIFICATION.md` §3.2 every Groovy-specific operator gets a
   distinct named node), simplicity, completeness, tests, conventions.
7. Fix any issues found.
8. Write tests: corpus test with field names; highlight test if
   capture-related; regression test.
9. Update agent-local documentation. Do NOT update `CHANGELOG.md` — the
   orchestrator consolidates entries in Step 5.

### Phase 2: Simplify

Review `git diff HEAD` for reuse, clarity, efficiency. Apply fixes
directly. Run `npx tree-sitter generate && npx tree-sitter test`.

### Phase 3: Review and Remediate

Audit the diff per the `/review` skill's checklist. Fix actionable
findings.

### Phase 4: Validate

```bash
npx tree-sitter generate
npx tree-sitter test
npm run lint
```

### Phase 5: Commit

Verify branch hasn't drifted. Stage only intentional files (NOT
`git add -A`). When `grammar.js` changed, stage `src/parser.c`,
`src/grammar.json`, `src/node-types.json` together.

Conventional Commits message with `Fixes #<N>` in body.

### Phase 6: Annotate GitHub Issue

Update body AND add comment via `gh issue edit --body-file` and
`gh issue comment --body-file`. Do NOT close.

### Phase 7: Report Result

```
STATUS: SUCCESS
BRANCH: <branch-name>
COMMIT: <short-hash>
FILES: <number of files changed>
SUMMARY: <one-line description>
CHANGELOG: <changelog entry text>
LESSON: <hard-won lesson or "none">
```

Or `STATUS: SKIPPED` / `STATUS: FAILED` with reason.

On FAILED: `git checkout -- . && git reset HEAD` (do NOT `git clean
-fd`).

**END AGENT PROMPT**

---

## Guardrails

- Do NOT merge the integration branch into `main` — leave for the user.
- Do NOT close GitHub issues — `Fixes #N` handles it on merge.
- Do NOT `git push --force` or any destructive operations.
- Do NOT delete worktrees.
- Do NOT skip the review phase.
- Do NOT bump `tree-sitter-cli` or grammar dependency versions as part
  of an issue fix.
- Do NOT hand-edit `src/parser.c`, `src/grammar.json`,
  `src/node-types.json`.
- Parallel agents MUST touch different areas — same-area, cross-binding,
  and cross-area issues are always serialized.
