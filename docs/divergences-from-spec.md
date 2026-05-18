# Divergences from SPECIFICATION.md

This document records places where `grammar.js` deliberately differs
from `SPECIFICATION.md`, with rationale. Add an entry each time a
grammar implementation choice forces a spec-level decision to be
revisited or relaxed.

If a divergence is intentional and the spec should be updated to match,
update both this file and `SPECIFICATION.md`. If a divergence is
intentional but the spec stays the way it is (e.g., the spec describes
the ideal and the implementation is a pragmatic stopgap), record the
reasoning here.

---

## 1. Conflicts list differs from the spec's §7 prediction

**Spec §7** predicted this `conflicts:` list:

```
[$.label, $.identifier]
[$.map_entry, $.label]
[$._type_arguments, $.binary_expression]
[$.parenthesized_type_cast, $.parenthesized_expression]
[$.closure, $.map_literal]
[$.command_chain, $.expression_statement]
[$.method_reference_expression, $.ternary_expression]
[$.trait_declaration, $.class_declaration]
```

**Actual `grammar.js` conflicts:**

```
[$._expression, $._type]
[$.for_in_statement, $._expression]
[$.variable_declarator, $._property_name]
```

The spec's list was an educated guess written before any rule was
implemented; once the grammar landed, none of the predicted
conflicts actually fired. Two reasons:

- Several entries (`command_chain`, `_type_arguments`) belong to
  features that haven't landed yet — generics and juxt-function
  calls. They will appear with those features.
- Other entries (`closure` vs `map_literal`, `trait_declaration` vs
  `class_declaration`, `label` vs `identifier`, `method_reference`
  vs `ternary`) were avoided by using `prec` and structural
  differences instead of explicit conflict declarations. For
  example, `closure` and `map_literal` use different brackets
  (`{}` vs `[]`) so they never compete at the same point;
  `labeled_statement` uses `prec(1)` to win at statement-start
  position because map-entries live inside `[…]` so cannot
  collide there.

The actual three conflicts are forced by features the spec did
not have direct prediction-rules for:

- `[$._expression, $._type]` — `(Foo)` inside parens could be the
  head of a `parenthesized_type_cast` or a `parenthesized_expression`;
  resolved by what follows `)`.
- `[$.for_in_statement, $._expression]` — `for ( identifier …` is
  either the head of a `for_statement` init expression or the
  variable of a `for_in_statement`; resolved by `;` vs `in`.
- `[$.variable_declarator, $._property_name]` — `def identifier` is
  either a no-init variable or the start of a method name;
  resolved by `(` (method) vs `=` / `,` / end (variable).

**Action**: keep the spec text as historical record; rely on this
file plus `grammar.js` comments for the authoritative conflict
list.

---

## 2. GString interpolation: not yet implemented

**Spec §5.7 / §6.3** require double-quoted and triple-double
strings to expose interpolation structure (`$identifier` and
`${expr}`) as parse-tree children rather than as flat text.

**Current implementation**: double-quoted (`"…"`) and triple-double
(`"""…"""`) strings remain flat single-token regex matches.

The earlier groundwork for scanner branches (`_gstring_body`,
`_gstring_interpolation_start`) was removed when the review caught
that listing unreferenced externals lets the dormant branches
mis-fire in tree-sitter's error-recovery state. When a contributor
implements structured GString parsing, both the scanner branches
**and** the externals listing must be re-added in the same change
— a half-wired token (in externals but not consumed by any rule)
is worse than nothing.

---

## 3. AUTOMATIC_SEMICOLON: not yet wired

**Spec §6.4** wants the external scanner to emit an automatic
semicolon when a newline follows a statement-ending token and the
next token doesn't continue the expression.

**Current implementation**: `\n` is in the `extras` set, so the
in-grammar lexer swallows it before any scanner branch can fire.
Statements parse cleanly because `repeat($._statement)` greedily
matches one statement at a time. The `return foo\nbar` greedy-
extension case is the known v1 limitation; workaround is an
explicit `;`.

The earlier scaffolding (an `AUTOMATIC_SEMICOLON` external token
plus `paren_depth` / `bracket_depth` counters on the `Scanner`
struct) was removed when the review caught that the counters were
never updated and the external listing made the dormant branch
reachable in error-recovery states. Re-wiring requires three
coordinated changes:

1. Add `_automatic_semicolon` back to the `externals` array.
2. Have the scanner track paren/bracket depth (this requires the
   scanner to observe `(`, `)`, `[`, `]` as it scans — tree-sitter
   does not surface those to the external scanner automatically).
3. Remove `\n` from `extras` and consume `_automatic_semicolon`
   from the affected statement rules.

---

## 4. `non-sealed` is a single `token()` rather than three tokens

**Spec §4** lists `non-sealed` alongside `sealed` as a modifier;
it does not specify the lexer treatment.

**Current implementation** uses `token('non-sealed')` so the
hyphen never lexes as a subtraction operator. As a side effect,
the bare identifier `non-sealed_var` (legal Groovy: `non`, `-`,
`sealed_var`) would mis-lex as `non-sealed` + `_var` if it ever
appeared. In practice nobody writes identifiers shaped this way;
the spec §5.2 lists `non-sealed` as a contextual keyword
specifically so it can be used in modifier position only.

**Action**: accept the edge case. If real-world code ever hits
this, the fix is to move `non-sealed` recognition into the
external scanner with a class-keyword trailing-context predicate.

---

## 5. `_type` covers identifier / array / qualified — but the last one is context-sensitive

**Spec §4** allows full Java-style type expressions in `_type`
positions: generics (`List<String>`), array dimensions
(`int[]`), and qualified names (`java.util.List`).

**Current implementation**:
- Unqualified identifier types ✓
- `array_type` (`int[]`, `String[][]`) ✓ (iter 45)
- `qualified_type` (`java.util.List`) — works in **most** type
  positions (`new`, `parenthesized_type_cast`, formal parameters)
  but NOT in `cast_expression` (`x as java.util.Map`) or
  `instanceof_expression` (`x instanceof java.lang.Number`). In
  those slots, tree-sitter's LR(1) chooses `field_access` for the
  trailing `.Map` over extending the qualified_type, producing
  `(field_access (cast x as java.util) Map)` rather than
  `(cast x as java.util.Map)`. The structural ambiguity is real:
  qualified_type and field_access have identical token sequences
  and only context distinguishes them, and `cast_expression` at
  `PREC.RELATIONAL` is looser than `field_access` at
  `PREC.ACCESS`.
- Generics (`List<String>`) — not yet implemented.

**Action**: real-world fix is one of
(a) move `as Type` and `instanceof Type` recognition into an
    external scanner so the type slot consumes the full dotted
    name greedily, or
(b) inline the cast/instanceof RHS to require a `_type` that's
    explicitly NOT a single identifier followed by a `.`
    expression continuation.
Both are larger refactors. For v1 users can parenthesise:
`x as (java.util.Map)` parses cleanly (the parens force the type
context).

---

## 6. Typed local variable declarations require an initializer

**Spec §4 / §5.12** show typed declarations of all shapes,
including the no-initializer form (`String x`).

**Current implementation** accepts `Type x = expr` and
`Type x = expr, y = expr2` (via `_initialized_declarator`) but
not the bare `Type x`. Reason: `Type x` is grammatically
indistinguishable from `Type(x)` (a method invocation where the
identifier `Type` is treated as a callee and `x` is the single
positional argument) without a 2-token lookahead.

Workaround: users can use `def x` or `var x` for the no-init form;
the typed form requires `= value`. Real-world Groovy code almost
universally uses `=` initializers on local declarations anyway.

**Action**: lift this restriction once we have a sound
disambiguation strategy. Likely path: a conflict declaration on
`[$.local_variable_declaration, $.method_invocation]` plus a
priority weighting that prefers the declaration when the next
token is `;` or end-of-statement.

---

## 7. Slashy-regex injection relies on the raw `/` prefix

**Spec §9.2** wants `queries/groovy/injections.scm` to inject
slashy and dollar-slashy strings as `regex`.

**Current implementation** in `injections.scm` uses
`(#match? @injection.content "^/[^/]")` and `"^\\$/"` against the
string's source text to distinguish slashy from quoted strings.
This works only because slashy and quoted strings happen to share
a single `string_literal` node kind in v1: there is no
`slashy_string_literal` to query against directly.

The fragility: if a future grammar refactor allows a quoted string
to start with `/` in some construction (e.g. a future
escape-sequence change), the match-anchor heuristic could
mis-inject. The robust fix is to split slashy and dollar-slashy
into distinct node kinds (`slashy_string_literal`,
`dollar_slashy_string_literal`), wire highlights / injections to
them by node-kind rather than by source-text regex, and update
the corpus.

**Action**: deferred until the GString restructure (#2) lands,
since both touch `string_literal` and a shared refactor is cheaper
than two separate AST shape changes.

---

## 8. Operator-only-distinguished rules cannot pin the operator in corpus tests

**Spec §8** wants every Groovy operator family to have a corpus
test that pins the operator in the expected AST. For rules with
distinct node kinds per operator (Elvis, safe-nav, range,
spaceship, regex find/match, identity, membership, instanceof,
spread-dot, safe-chain-dot, method-pointer, method-reference,
direct-field-access, logical-implication, power) this works: the
node kind itself names the operator.

For families that fold many operators into a single node kind via
a `field('operator', op)` member — `binary_expression`,
`assignment_expression`, `unary_expression`,
`unary_update_expression`, `update_expression` — the operator is
an anonymous token. **tree-sitter v0.25.10 strips anonymous tokens
from default test output**, so the corpus tests for these
families cannot distinguish e.g. `+=` from `-=`, `..` from `..<`,
`<<` from `>>`, or `+` from `-` at the AST-comparison layer. A
regression that swapped `>>>=` for `>>=` in `grammar.js` would
pass every corpus test in `operators-assignment.txt`.

Highlight tests can fingerprint operator tokens directly via the
`@operator` capture, but doing so for the `/` family is blocked
by the slashy-vs-division scanner ambiguity (§6.2). Until that
disambiguation lands, the `/`-adjacent operators (`/`, `/=`,
`*/`, etc.) cannot be reliably highlight-tested either, because
the scanner mis-tokenises the `//` marker comment line as a
slashy-string body that spans into the assertion comment.

**Action**: two follow-ups, both outside the test-only fix scope:

1. Tighten the slashy-string scanner branch to require a
   non-expression-end previous token (per §6.2's
   "context-sensitive emit"). This makes `a / b` unambiguously a
   binary division regardless of GLR exploration.
2. After that, add `test/highlight/operators.groovy` with `// ^
   operator` markers covering each operator-only-distinguished
   family.
