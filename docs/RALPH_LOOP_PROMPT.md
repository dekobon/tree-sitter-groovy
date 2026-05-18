# Ralph Loop Task — Implement `tree-sitter-groovy` to spec

This document is the **standing prompt** for the Ralph Loop session
that is iteratively implementing `tree-sitter-groovy`. The Ralph
state file (`.claude/ralph-loop.local.md`) refers back to this file
on every iteration, so this is where the real task definition
lives.

---

## 0. Mission

Drive `tree-sitter-groovy` from its current bootstrap state to a
complete, tested implementation of the contract in
`SPECIFICATION.md`. The grammar must parse the Groovy constructs
listed in `SPECIFICATION.md` §3 and §4 without `ERROR` nodes, the
external scanner must implement all five tokens in §6, the queries
must cover §9, and the test corpus must cover §8.

When — and only when — **every** item on the
`docs/IMPLEMENTATION_PROGRESS.md` checklist is checked off **and**
all three validation gates below pass on a clean tree, output the
exact completion promise (see §6) so the loop terminates. Otherwise
end the iteration (no promise) so the loop continues with the next
iteration.

---

## 1. Per-iteration workflow

Every Ralph iteration begins with a fresh context. The agent only
sees what's on disk — there is no memory of prior iterations beyond
file contents and `git log`. **Do these steps in this order, every
time:**

1. **Orient yourself.** Read in this order:
   - This file (`docs/RALPH_LOOP_PROMPT.md`) — the contract.
   - `docs/IMPLEMENTATION_PROGRESS.md` — what's done, what's next,
     known blockers.
   - `git log --oneline -20` — what previous iterations did.
   - `CLAUDE.md`, `AGENTS.md` — project conventions.
   - Skim the most recently modified files referenced in the
     progress tracker.
   - Read the specific `SPECIFICATION.md` section(s) relevant to
     this iteration's chunk. **Do not read the whole spec each
     time** — use the section headings and the progress tracker to
     find the right window.

2. **Pick the next chunk.** Look at `docs/IMPLEMENTATION_PROGRESS.md`
   and pick the first unchecked item under "Active iteration plan."
   If that section is empty or stale, derive the next chunk from the
   first unchecked checklist item in §4 of that file. Keep the chunk
   small — one operator family, one statement form, one scanner
   token, one corpus file. Larger chunks fail more often and are
   harder to land.

3. **Check the build is green before touching it.** Run:

   ```bash
   npx tree-sitter generate
   npx tree-sitter test
   npm run lint
   ```

   If any of these fail on a clean checkout, **fix that first** —
   leaving a broken build for the next iteration is the worst thing
   you can do in a Ralph loop. Stop everything else and stabilise.

4. **Implement the chunk.** Edit `grammar.js`, `src/scanner.c`,
   `queries/groovy/*.scm`, and/or `test/corpus/*.txt` as needed.
   Follow the editing rules in §3 below. After every grammar edit,
   regenerate (`npx tree-sitter generate`).

5. **Write or update tests.** Every grammar rule added or changed
   must have at least one corpus test pinning its S-expression. If
   the chunk closes a bug from `SPECIFICATION.md` §10, add a
   regression test in `test/corpus/regressions.txt` referencing the
   issue number.

6. **Validate.** Re-run all three gates from step 3. If any fail,
   debug and fix. Do not commit a red build.

7. **Update the progress tracker.** Tick off the items completed
   this iteration. Add new items if the work surfaced sub-tasks.
   Note any decisions that diverge from the spec in
   `docs/divergences-from-spec.md`.

8. **Commit.** Use Conventional Commits per `AGENTS.md`. One commit
   per logical chunk. Reference the SPECIFICATION.md section in the
   body (`See SPECIFICATION.md §3.2 row for elvis_expression.`). Do
   NOT use `--no-verify` or amend prior commits. Do NOT push.

9. **Add a CHANGELOG.md entry** under `## [Unreleased]` if the
   change is user-visible (a new node kind, a query capture, a
   binding-surface change). Skip the entry only for purely-internal
   changes; the PR description we'll write at release time will
   document the omission.

10. **Decide whether to stop or continue.** If `docs/IMPLEMENTATION_PROGRESS.md`
    still has unchecked items, or any gate is red, **end the
    iteration** (do not output the completion promise). Ralph will
    feed this prompt back and the next iteration will pick up. Only
    output the completion promise when §5 is fully satisfied.

---

## 2. Order of work (suggested)

This is a recommendation, not a hard sequence — feel free to
re-order based on what unblocks the most progress. But each layer
generally needs the previous one solid before it can be exercised.

1. **External scanner** (`src/scanner.c`): implement the token
   types from `SPECIFICATION.md` §6 one at a time, starting with
   the ones that block the most grammar work:
   - `BLOCK_COMMENT` / `GROOVYDOC_COMMENT` (§6.6) — no grammar
     dependency, unblocks empty-comment regression test.
   - `AUTOMATIC_SEMICOLON` (§6.4) — unblocks statement rules.
   - `GSTRING_BODY` / `GSTRING_INTERPOLATION_START` (§6.3) —
     unblocks string literals.
   - `SLASHY_STRING_*` (§6.2) — unblocks regex operators.
   - `LABEL_COLON` (§6.5) — unblocks labelled statements.

2. **Primary expressions** in `grammar.js`: literals (number,
   string, boolean, null), identifiers, parenthesised expressions,
   list literals, map literals, closures.

3. **Postfix and access tier**: subscript `[]`, safe subscript
   `?[]`, call `()`, dot family (`.`, `?.`, `??.`, `*.`, `.&`,
   `.@`, `::`).

4. **Unary tier**: prefix/postfix `++` `--`, unary `+` `-` `!` `~`,
   `new`.

5. **Binary tiers, top-down by precedence** from `SPECIFICATION.md`
   §3.1 — power, multiplicative, additive, shift/range,
   relational (including `as`, `in`, `!in`, `instanceof`,
   `!instanceof`), equality (including identity and regex match),
   bitwise, logical, implication, ternary/Elvis, assignment. Each
   tier brings new node kinds from §3.2; pin one corpus test per
   distinct node kind.

6. **Statements and declarations** from §4: imports, package,
   class/interface/trait/enum/record/annotation-type, method,
   variable (including multi-assignment), control flow, switch
   (classic and arrow), try (including multi-catch and
   try-with-resources), `pipeline`, labelled statements, command
   chains.

7. **Queries** (`queries/groovy/*.scm`) per §9 — `highlights.scm`
   first (testable via `test/highlight/`), then `locals.scm`,
   `injections.scm`, `folds.scm`, `indents.scm`.

8. **Regression corpus** (`test/corpus/regressions.txt`) per §8.2 —
   add tests as fixes land, so each tracked bug becomes a guarded
   regression.

9. **Stress corpus** (`test/stress/` + a Rust integration test
   under `bindings/rust/tests/`) per §8.3 — last, once the grammar
   is broadly stable.

---

## 3. Editing rules

- **`SPECIFICATION.md` is the contract.** If a rule's shape on the
  page disagrees with what's easy to implement, prefer the spec.
  If the spec is genuinely wrong, fix the spec in the same commit
  (or earlier in the same PR-equivalent of iterations) and note
  the change in `docs/divergences-from-spec.md`.

- **`src/parser.c`, `src/grammar.json`, `src/node-types.json` are
  build outputs.** Edit `grammar.js`, regenerate. Never hand-edit
  the generated parser.

- **`src/scanner.c` is hand-written and survives regeneration.**
  Its symbols are already `tree_sitter_groovy_external_scanner_*`
  and stay in sync with the generated parser automatically.

- **Use the dedicated tools.** Edit/Write for files. `rg` for
  text search, never `grep`. `fd` for file search, never `find`.

- **No comments unless the WHY is non-obvious** — see CLAUDE.md.
  Precedence-level choices, regex shapes, and external-scanner
  state transitions are good candidates for short why-comments
  pointing at the spec section.

- **Never rewrite a whole corpus file to fix one test.** Use
  targeted edits.

- **When fixing a bug surfaced by testing**, add a corpus or
  highlight test that would catch it if reintroduced.

- **Respect worktree safety** from CLAUDE.md. If you are inside a
  `.claude/worktrees/...` directory, never `cd` out, never run
  `git worktree remove`, never delete the worktree. (Ralph
  normally runs in the main checkout, but the rule still applies.)

---

## 4. Validation gates (must pass before commit)

```bash
npx tree-sitter generate
npx tree-sitter test
npm run lint
```

- `tree-sitter generate` must complete with no errors and no new
  parser conflicts beyond those declared in `grammar.js` `conflicts:`.
- `tree-sitter test` must show 100% pass for corpus and highlight
  tests. (Once tests exist — early iterations may legitimately have
  zero tests; that's fine. As soon as a corpus file is added it must
  pass.)
- `npm run lint` must pass cleanly (no warnings on the files you
  touched).

For the Rust binding, optionally run `cargo build` in
`bindings/rust/` once that target is in scope per §2 step 9. Not
required earlier.

---

## 5. Completion criteria

The completion promise may be output **only** when **all** of the
following are simultaneously true on the main checkout:

1. Every section header in `SPECIFICATION.md` §3 and §4 has at
   least one passing corpus test and no `ERROR`-emitting case in
   the corpus.
2. `src/scanner.c` implements all token types in §6 (not stub
   returns).
3. `queries/groovy/highlights.scm`, `folds.scm`, `indents.scm`,
   `injections.scm`, `locals.scm` cover §9.1 / §9.2 / §9.3 — i.e.
   keywords, operators, strings, types, functions, comments,
   regex-injection, scope rules.
4. `test/corpus/` includes the operator files enumerated in §8.1
   and the per-issue regressions in §8.2. Every file passes.
5. `test/highlight/` has at least one assertion file per major
   highlight category (keyword, operator, string, type, function,
   comment). All pass.
6. The three validation gates (§4 of this prompt) all pass on
   `main` HEAD.
7. `docs/IMPLEMENTATION_PROGRESS.md` has every checklist item
   ticked off, with no "open blocker" sections remaining.
8. `CHANGELOG.md` under `## [Unreleased]` lists every user-visible
   addition.

If any one of these is false, do NOT output the promise — end the
iteration so Ralph continues.

---

## 6. Completion promise

The promise string is:

```
<promise>GROOVY_GRAMMAR_FULLY_IMPLEMENTED</promise>
```

The Ralph stop hook checks the **last assistant text block** of
each iteration for this exact tag. Output it only when §5 above is
truly satisfied. Outputting a false promise wastes the user's time
and undoes the point of the loop — see the "CRITICAL RULE" in
`commands/ralph-loop.md`.

---

## 7. Things to avoid

- **Do not delete `.claude/worktrees/` or any worktree** — see
  CLAUDE.md "Worktree safety."
- **Do not push to remote** unless explicitly asked.
- **Do not open PRs or close issues** from inside the loop.
- **Do not skip pre-commit hooks** (`--no-verify`) or bypass
  signing.
- **Do not commit if a gate is red.** Fix forward; never `git
  reset --hard` to escape a broken state — diagnose root cause.
- **Do not hand-edit `src/parser.c`, `src/grammar.json`, or
  `src/node-types.json`**.
- **Do not summarise the whole spec each iteration** — that
  burns context. Use the progress tracker as the index.
- **Do not lie to escape the loop.** If you genuinely cannot make
  progress, document the blocker in
  `docs/IMPLEMENTATION_PROGRESS.md` under "Open blockers" and end
  the iteration. The user can intervene if needed.

---

## 8. If you get stuck

- Re-read the relevant `SPECIFICATION.md` section. The contract is
  surprisingly complete; most "stuck" moments are misremembering
  the spec.
- Consult Apache's reference grammar at
  `https://github.com/apache/groovy/blob/master/src/antlr/GroovyParser.g4`
  and `GroovyLexer.g4` for tie-breaks on syntax disputes.
- Cross-check against `amaanq/tree-sitter-groovy` and
  `murtaza64/tree-sitter-groovy` for prior art — but remember
  this grammar is **standalone** and intentionally diverges from
  both. Their patterns inform, they do not bind.
- If a tree-sitter conflict refuses to resolve, document it in
  `docs/divergences-from-spec.md` with the parser state and the
  tokens involved, and try a different precedence assignment in
  the next iteration.
- Smaller chunks. If a 200-line grammar change cannot be made to
  pass, split it into 20-line chunks and land them one at a time.
