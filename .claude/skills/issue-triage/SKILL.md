---
name: issue-triage
description: Fetch open GitHub issues and produce a read-only triage report with quick wins and recommended groupings.
---

# Issue Triage

Generate a triage report for open issues. Read-only — no creation,
modification, or closing.

**Argument**: `$ARGUMENTS`

- Empty → all qualifying open issues
- `<component>` → filter by title prefix matching a component
  (`grammar`, `scanner`, `queries`, `bindings`, `tests`, `spec`)

---

## Step 1: Fetch issues

### 1a: Ensure labels exist

```bash
ensure_label() {
  local name="$1" color="$2" desc="$3"
  if ! gh label list --limit 200 --json name --jq '.[].name' | grep -qx "$name"; then
    gh label create "$name" --color "$color" --description "$desc"
  fi
}
ensure_label grammar  "1d76db" "Touches grammar.js"
ensure_label scanner  "1d76db" "Touches src/scanner.c"
ensure_label queries  "1d76db" "Touches queries/"
ensure_label bindings "1d76db" "Touches bindings/"
ensure_label spec     "1d76db" "Touches SPECIFICATION.md"
ensure_label upstream-apache-groovy "fbca04" "Depends on an Apache Groovy language change"
ensure_label refactor "fbca04" "Refactor without behavior change"
ensure_label security "ee0701" "Security-relevant finding"
```

`bug`, `enhancement`, `documentation` already exist on most repos.

### 1b: Query and merge

```bash
(
  gh issue list --state open --label bug           --limit 200 --json number,title,labels,body
  gh issue list --state open --label enhancement   --limit 200 --json number,title,labels,body
  gh issue list --state open --label refactor      --limit 200 --json number,title,labels,body
  gh issue list --state open --label documentation --limit 200 --json number,title,labels,body
  gh issue list --state open --label security      --limit 200 --json number,title,labels,body
  gh issue list --state open --label upstream-apache-groovy --limit 200 --json number,title,labels,body
) | jq -s '
  add
  | unique_by(.number)
  | [ .[] | select([ .labels[].name ] | any(. == "low-priority") | not) ]
'
```

Filter further if `$ARGUMENTS` names a component. Save the result.

---

## Step 2: Read every issue body

Titles mislead. For each issue, read the full body. Note: scope,
complexity signals (cross-binding work, AST shape change, scanner
involvement), dependencies on other issues, root cause hints.

---

## Step 3: Classify quick wins

### Positive indicators (need 3+ to qualify)

1. Single component (grammar OR queries OR one binding)
2. Narrow scope — one rule, one query, one test
3. Fix is obvious from the issue
4. < 50 lines changed
5. No new abstractions
6. No AST shape change (so no corpus-test churn)
7. No scanner change (scanner edits are high-risk)

### Disqualifiers (any one eliminates)

1. Requires AST shape change (causes corpus test churn)
2. Touches `src/scanner.c`
3. Touches multiple bindings
4. Bundles multiple distinct problems
5. Has the `upstream-apache-groovy` label (depends on an Apache Groovy
   language change before it can be fixed locally)
6. Requires changing `SPECIFICATION.md` (design decisions need
   discussion before code)

**Err on the side of NOT classifying as a quick win.**

---

## Step 4: Identify groupings

A group requires 2+ issues.

### By component

`grammar`, `scanner`, `queries`, `bindings/<lang>`, `tests`, `spec`.

### By theme

- **Same Groovy operator family**: multiple issues touching Elvis /
  safe-nav / spread / regex / identity / spaceship / ranges / method
  references.
- **Same statement form**: multiple issues in shared machinery
  (closures, classes, switch, try-catch).
- **Cross-binding parity**: same defect reported in multiple bindings
  (Node, Python, Rust, etc.) — fixing one is a template for fixing all.
- **Apache-blocked**: issues blocked on the same Apache Groovy language
  change.
- **Same `SPECIFICATION.md` section**: multiple issues against §3
  (operators), §4 (statements), §5 (edge cases), §6 (scanner).

Do NOT force groupings.

---

## Step 5: Produce the report

Every fetched issue must appear in exactly one section.

```markdown
## Issue Triage Report

**Scope**: all | <component>
**Issues analyzed**: N
**Date**: YYYY-MM-DD

### Quick Wins

| # | Title | Why it's a quick win |
|---|-------|----------------------|

### Recommended Groupings

| Group | Issues | Rationale |
|-------|--------|-----------|

### Remaining Issues

| # | Title | Notes |
|---|-------|-------|

---

**Quick wins**: #X, #Y, #Z
**Grouped**: [#A, #B], [#C, #D], #E
```

If a section is empty, keep the header and write "None".

---

## Guardrails

- **Read-only**: do not create, modify, close, or comment on issues.
- **Read every body**: never classify on title alone.
- **No forced groupings**: ungrouped issues belong in Remaining.
- **Complete coverage**: every fetched issue appears in exactly one
  section.
- **Excluded**: `low-priority`-labeled issues never appear.
