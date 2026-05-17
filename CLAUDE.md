# CLAUDE.md

## Shared project instructions

@AGENTS.md

## Claude Code-specific configuration

### Worktree safety (ABSOLUTE PRIORITY)

If you are running inside a worktree (check:
`git rev-parse --show-toplevel` returns a path under `.claude/worktrees/`),
the following are hard bans — violating them destroys other agents'
in-progress work:

- Never run `git worktree remove`, `git worktree prune`, or `rm -rf` on
  any worktree directory.
- Never `cd` to the main repository, check out a different branch, or
  write files outside your worktree.
- Never use `/clean_gone` or any command that removes worktrees.
- The only entity that may remove a worktree is the Claude Code runtime
  that created it (automatic cleanup on session end).
- If you see stale worktrees, leave them alone — another agent may be
  using them, or the user will clean them up manually.

### Tool choice

- **Text search**: built-in `Grep`, or `rg` via Bash. Never `grep`.
- **File search**: built-in `Glob`, or `fd` (or `fdfind` on
  Debian/Ubuntu) via Bash. Never `find`.
- **External docs**: prefer Context7, the
  [tree-sitter docs](https://tree-sitter.github.io/tree-sitter/), the
  [Apache Groovy language reference](https://groovy-lang.org/), and
  the [Apache Groovy ANTLR grammar](https://github.com/apache/groovy/tree/master/src/antlr)
  over generic web search.

### Editing

- For `grammar.js`, `queries/*.scm`, and binding source files: prefer
  targeted `Edit` tool calls with scoped `old_string` / `new_string`
  pairs.
- For non-code files (Markdown, TOML, YAML, JSON): use targeted `Edit`
  tool calls.
- After any `grammar.js` change, regenerate with `npx tree-sitter
  generate`. The hand-written `src/scanner.c` is independent and does
  not need renaming.
- Read `SPECIFICATION.md` before making non-trivial grammar changes —
  it is the contract every rule must honor.

### Skills available under `.claude/skills/`

| Skill | Use when… |
|-------|-----------|
| `review` | Read-only review of a diff, branch, PR, or commit range |
| `audit-tests` | Finding corpus/highlight tests that pass for the wrong reason |
| `fix-issue` | End-to-end workflow for fixing a GitHub issue |
| `batch-fix` | Fixing multiple GitHub issues on a single integration branch, with parallel worktrees per area |
| `issue-plan` | Reading an issue, building a sequential-thinking plan, rating it, applying `low-priority` |
| `issue-triage` | Producing a read-only triage report (quick wins + groupings) over open issues |
| `lessons-learned` | Drafting entries for `docs/lessons_learned.md` |

The `review` and `issue-triage` skills are read-only and must not
modify the working tree; the others may edit files as part of their
workflow.
