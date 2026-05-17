---
name: lessons-learned
description: Review project activity (issues, commits, changelog) and draft entries for docs/lessons_learned.md.
---

# Lessons Learned

Review recent project activity, identify hard-won lessons, evaluate them
against a strict quality bar, draft entries for `docs/lessons_learned.md`.

If `docs/lessons_learned.md` does not yet exist, create the directory and
seed with a short header before the first entry — do not append into a
missing file.

**Argument**: `$ARGUMENTS` — empty for full workflow, or a hint to narrow
search (e.g. `"scanner"`, `"slashy strings"`).

---

## Step 1: Establish boundary

```bash
git log -1 --format=%aI -- docs/lessons_learned.md
```

If the file has no substantive history, fall back to:

```bash
git log --reverse --format=%aI | head -1
```

Record as `$BOUNDARY`.

---

## Step 2: Read current coverage

Read `docs/lessons_learned.md` in full. Record:

1. Highest lesson number — new entries continue at N+1.
2. Each lesson's title and domain — for overlap detection.
3. Issue numbers already cited.

Mandatory step. If the file has no lessons, record highest as 0.

---

## Step 3: Gather evidence

Four sources. If `$ARGUMENTS` provides a hint, use as an additional
keyword.

### 3a: Closed issues since boundary

```bash
gh issue list --state closed --search "closed:>$BOUNDARY" --limit 100 \
  --json number,title,body,labels,closedAt
```

Scan for hard-lesson signals: "root cause", "turns out", "subtle",
"silent", "regression", "broke", "workaround", "misunderstood".

### 3b: Git commits since boundary

```bash
git log --since="$BOUNDARY" --format="%H %s" -- grammar.js src/scanner.c queries/ bindings/ test/ SPECIFICATION.md
```

Look for: substantial fix commits, refactors that changed approach,
recurring scanner / regeneration fallout, changes that reverted earlier
changes, spec changes that flipped a precedence decision.

### 3c: CHANGELOG entries since boundary

If `CHANGELOG.md` exists, focus on Fixed / Changed sections.

### 3d: Documentation changes (skip when hint is given)

```bash
git log --since="$BOUNDARY" --name-only --format="" -- docs/ README.md CLAUDE.md SPECIFICATION.md
```

---

## Step 4: Deep investigation

For items showing hard-lesson signals: read full issue threads, examine
diffs (`git show <commit>`), look for pattern repetition (same mistake
more than once).

Record each candidate:
- Source reference (issue, commit)
- One-line summary
- Evidence strength: strong / moderate / weak

---

## Step 5: Quality gate

> **Genuinely hard (cost real debugging time or caused real bugs) AND
> important (likely to recur).**

Present candidates as a ranked batch:

```
### Candidate N: <summary>
- Source: #<issue>, <commit>
- Quality: QUALIFIES / DOES NOT QUALIFY
- Overlap: None / Related to lesson #N
- Reasoning: <why it meets or fails the bar>
```

For non-qualifying candidates, suggest an alternative home:

| Signal | Alternative |
|--------|-------------|
| One-off debug trick | Code comment at the site |
| Architectural decision | `SPECIFICATION.md` section or design note |
| Project convention | `CLAUDE.md` / `AGENTS.md` |
| Already covered | Merge into existing lesson |
| Too specific | Issue comment or PR description |

**"No candidates qualify" is a valid success state.** Do not lower the
bar to produce output.

Wait for user selection before drafting.

---

## Step 6: Draft entries

For each selected candidate, draft per the existing format in
`docs/lessons_learned.md`:

1. `## N. <Pithy Principle Name>` — next sequential number.
2. Opening paragraph: general lesson (not issue-specific).
3. Bold sub-examples with issue/commit references (`**Description** (#42, abc1234).`).
4. Closing `**Lesson:**` paragraph.
5. Horizontal rule (`---`) after the entry.

### Overlap

- Merge into existing if very close.
- Cross-reference if related but distinct.
- Skip if redundant.
- Do NOT modify existing lessons without explicit user approval.

Show the complete draft. Wait for approval.

---

## Step 7: Apply and stage

After approval:

1. Append to `docs/lessons_learned.md`.
2. Stage: `git add docs/lessons_learned.md`.
3. Do NOT commit — staging only.

---

## Guardrails

- Quality bar is non-negotiable.
- No automatic commits — stage only.
- Preserve existing lessons.
- Append by default; warn on insertion (other skills reference lesson
  numbers).
- Every drafted lesson cites at least one issue or commit.
- "No candidates qualify" is a valid outcome.
