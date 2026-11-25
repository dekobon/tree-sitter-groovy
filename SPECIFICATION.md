# tree-sitter-groovy — Grammar Specification

Status: Draft v2 — 2026-05-17
Author: Elijah Zupancic.

This specification was informed by independent review of two prior
community Groovy grammars — `amaanq/tree-sitter-groovy@master` and
`murtaza64/tree-sitter-groovy@main` (both at default branch on
2026-05-17) — as inspiration only; no code from either project is
incorporated here. The authoritative references are the official
Apache Groovy language documentation (`https://groovy-lang.org/operators.html`,
`syntax.html`, `semantics.html`, `objectorientation.html`, `dsls.html`)
and the Apache Groovy reference grammar
(`https://github.com/apache/groovy/blob/master/src/antlr/GroovyParser.g4`,
`GroovyLexer.g4`) — both files retrieved 2026-05-17.

Revision history:

- v1 (initial draft) — section structure, all operators from issue
  #247, all murtaza64 open issues, external-scanner sketch.
- v2 (this revision) — corrected precedence levels to match Apache
  reference grammar exactly (range = shift, ternary = Elvis);
  added safe-chain-dot `??.` (Groovy 4); added contextual keywords
  `async` / `await` / `defer` / `module` / `goto` / `val` /
  `threadsafe`; documented `!instanceof` and `!in` trailing-context
  lexer rule from Apache; clarified subscript-chained-with-call
  composition; added triple-quoted edge cases from murtaza64
  string corpus.

This document is a **specification only** — no grammar code is generated
from it directly. It is the contract for the implementation work that
follows.

---

## 1. Goals and non-goals

### 1.1 Goals

1. Produce a tree-sitter grammar named `groovy` that parses idiomatic
   Groovy 2.x–6.x source files without `ERROR` nodes for the constructs
   listed in §3 ("Operator coverage matrix") and §4 ("Statement and
   declaration coverage").
2. Emit **distinct named nodes** for every Groovy-specific operator
   (Elvis, safe navigation, spread, identity, spaceship, regex find /
   match, exclusive ranges, `!instanceof`, `!in`, `as`, method pointer,
   direct field, safe index, Elvis-assign, logical implication, method
   reference `::`). Downstream tooling must be able to identify each
   operator by node kind alone — no `MISSING`-child inspection, no
   tree-shape heuristics. This is the contract that closes
   `dekobon/big-code-analysis#246` and `#247`.
3. Be a **standalone grammar** (no `grammar(Java, …)` inheritance). The
   primary failure mode of amaanq's grammar is that Java's grammar
   shapes the parse tree even where Groovy's syntax has diverged
   (e.g. `x as String` → two `local_variable_declaration` nodes,
   `trait T {}` → `juxt_function_call` + closure). Inheritance is too
   blunt a tool for a language with this many surface-syntax
   differences.
4. Preserve every strength documented in the murtaza64 grammar:
   purpose-built operator table, multi-flavor strings with
   interpolation, slashy and dollar-slashy strings, closures with `->`
   parameters, switch / try / for / while / for-in, Jenkins `pipeline`
   keyword, Groovydoc comments with `@param` / `@throws` tags, quoted
   identifiers, command-chain (`juxt_function_call`) support.
5. Fix the bugs catalogued in murtaza64's open issues (#5, #16, #22,
   #36, #37, #38, #39) — see §10 ("Issue resolution map") for the
   per-issue plan.
6. Ship a Cargo crate suitable for use by
   `dekobon/big-code-analysis` (`tree-sitter-groovy` ≥ 0.2.0,
   semver-compatible replacement for the existing `=0.1.2` pin), with
   the same `tree-sitter` minor-version requirement as the rest of the
   crate's grammar dependencies.

### 1.2 Non-goals

1. **No GroovyDoc semantics.** Block comments matching `/** … */` are
   recognised as `groovydoc_comment` so editors can theme them, but
   `@param` / `@return` parsing is not in scope for v1 (the murtaza64
   approach over-tokenises and is the root cause of #16 — empty
   `/***/` / `/**/` killing highlighting).
2. **No semantic checks.** "`!instanceof` requires a type on the
   right" is a runtime/compiler rule. The grammar accepts any
   expression and lets downstream tools check.
3. **No AST transformations / macros.** Groovy ASTs at runtime are
   compiler-internal.
4. **No Jenkinsfile-only mode.** `pipeline { … }` is supported as a
   regular declaration (resolving #37), but Jenkins-specific step
   methods (`sh`, `agent`, `stage`) are ordinary method calls — no
   special node kinds.
5. **Target the latest released Groovy.** We target the Groovy 6.x
   grammar (6.0.0 at the time of writing). Earlier versions parse
   where their syntax is a subset of it. Where versions read the same
   text differently, the latest release's reading wins: nested array
   initializers after two or more `[]` follow Groovy 5 (GROOVY-8551,
   §3.2), and `(x) as Foo` is the coercion of `(x)`, as in Groovy
   6.0.0 (GROOVY-10355, §5.15). Exceptions are noted where they are
   made, such as a `{` among the elements after a single `[]`, which
   stays a closure as in Groovy 3 and 4 (§3.2), and `(a) + b`, which
   stays the cast of `+b` as in Groovy 3 to 5 (§5.15). No tracking of
   Groovy 7+ syntax until it is released. Coverage of the syntax that
   Groovy 5 and 6 added is incomplete: `async`, `await`, `defer` and
   `val` have no construct and parse as names, and
   `docs/divergences-from-spec.md` lists the other gaps.

---

## 2. Architecture

### 2.1 High-level shape

```text
tree-sitter-groovy/
├── grammar.js              # the entire grammar, hand-authored
├── src/
│   ├── parser.c            # generated
│   ├── scanner.c           # external scanner (see §6)
│   ├── tree_sitter/
│   │   └── parser.h
│   ├── node-types.json     # generated
│   └── grammar.json        # generated
├── queries/
│   ├── highlights.scm
│   ├── locals.scm
│   ├── injections.scm     # regex bodies, GString embedded expressions
│   ├── indents.scm
│   └── tags.scm
├── bindings/
│   ├── node/
│   ├── python/
│   ├── rust/
│   ├── go/
│   └── swift/
├── test/corpus/            # tree-sitter test files, see §8
├── Cargo.toml
├── package.json
├── pyproject.toml
├── binding.gyp
├── Package.swift
├── go.mod
├── tree-sitter.json
├── Makefile
├── README.md
└── SPECIFICATION.md        # this file
```

Layout matches the sibling grammars in this checkout
(`tree-sitter-tcl/`, `tree-sitter-irules/`), which is the integration
target for `dekobon/big-code-analysis` and several other consumers.

### 2.2 Why a standalone grammar (not `grammar(Java, …)`)

amaanq's choice to extend `tree-sitter-java` was pragmatic — Groovy
*claims* Java-syntax compatibility — but the cost is that every Groovy
feature absent from the Java parent has to fight the Java production
that *also* matches the same tokens. The dekobon issue #247 table is
that cost made concrete:

- `x as String` matches Java's variable declaration before Groovy's
  cast — emits two `local_variable_declaration`s.
- `trait Greeter { … }` matches Java's command-style call before the
  Groovy keyword — emits `juxt_function_call` + `closure`.
- `x !instanceof T` matches `! (x instanceof T)` because `!instanceof`
  is not a Java token.

The fixes for *each* of these would mean overriding the parent
production, at which point the parent rule is dead code in the child
grammar. The murtaza64 grammar's purpose-built operator table proves
the standalone approach scales: a single `binary_op` rule covers 30+
operators without any inherited noise.

We **do** keep a Java-shaped *layout* for class bodies, method
declarations, formal parameters, and annotation syntax — Groovy's
class-body syntax really is a Java superset there. We just refuse to
inherit from `tree-sitter-java` to get it.

### 2.3 Inputs the grammar borrows from each parent

| Feature | Source | Rationale |
|---|---|---|
| Operator precedence table (15-level) | **Apache reference grammar `expression` rule** | Authoritative source; we follow Apache's level ordering exactly (range = shift same level, ternary = Elvis same level). amaanq and murtaza64 each deviate in different ways. |
| Distinct `range_expression` node | amaanq | Keeps ranges out of `binary_expression`; eases metric impls |
| `juxt_function_call` rule | both | murtaza64's is more complete; we adopt its argument list, fix #5 with the precedence strategy in §5.5 |
| Multiline string fragments | amaanq | Cleaner; murtaza64's regex-heavy approach is the cause of #16 |
| `closure` with `->` parameters | murtaza64 | amaanq's closure has no parameter form |
| Slashy / dollar-slashy strings | murtaza64, scanner-promoted | murtaza64 has them as regular rules; we promote to external scanner to fix the division-vs-slashy ambiguity (see §6.2) |
| Groovydoc as a single token | murtaza64 (idea) + amaanq (impl) | murtaza64's per-tag parsing is the proximate cause of #16; we tokenise the whole block |
| `pipeline` statement | murtaza64 | we keep the rule but remove the `optional($.pipeline)`-at-end-of-file constraint (fix for #37) |
| Switch with arrow form | new | neither grammar has it; fix for #36 |
| Multi-catch | new | neither grammar has it; fix for #39 |
| Multi-assignment | new | neither grammar has it; fix for #22 |
| Trait, enum, record, sealed | new | fix for amaanq #247 trait line and murtaza64 #36 enum line |
| Named arguments | murtaza64 + spec | `map_item` in argument position; we tighten the rule |
| Method reference `::` | new | Groovy 3+ |
| Annotation `@interface` | murtaza64 | keep |
| Spread `*x`, `*.method()`, `*: m` | new | murtaza64 has spread as a prefix `access_op`; we split it into three productions for clarity and metric correctness |

---

## 3. Operator coverage matrix

This is the single source of truth for which operators MUST be
recognised and what node kind each emits. The precedence column is
the level in the table in §3.1; higher number = higher precedence
in the grammar's PREC table (tree-sitter convention is opposite of
the Groovy spec's "1 = highest", but we keep the spec's relative
order).

### 3.1 Precedence table — matches Apache reference grammar

This is the precedence ordering encoded in
`apache/groovy:src/antlr/GroovyParser.g4` (rule `expression`, lines
899–1013 on master and 779–878 on 4.0.x, in the copies of November 2026).
The numbering follows Apache's "level" comments
(1 = highest, 15 = lowest); the tree-sitter PREC constants run the
other way, higher = tighter, and only their *relative* order matters.
Where one of Apache's levels holds several alternatives, the order of
the alternatives decides: in a left-recursive ANTLR rule an earlier
alternative binds tighter, so a level splits in two (7a and 7b below).

| Apache level | Name | Operators | Associativity | Notes |
|---:|---|---|---|---|
| 1 | UNARY_NOT | `~` `!` | right | |
| 2 | POWER | `**` | left | `2 ** 3 ** 2` → `(2 ** 3) ** 2`, 64 in Groovy 3: `left=expression op=POWER nls right=expression` has no `<assoc=right>` in 3.0.x, 4.0.x, 5.0.x, master or 6.0.x. Before 0.7.0 the grammar read it right-associative. Prefix `+`, `-`, `++` and `--` (level 3) take the powers after them, also in a power's right operand: `-2 ** 2` is `-(2 ** 2)`, `2 ** -2 ** 3` is `2 ** (-(2 ** 3))`, and `++a ** 2` is `++(a ** 2)`, as Groovy 3 reads it. A cast does not: Apache's `castExprAlt` is `castParExpression castOperandExpression`, whose operand is a postfix expression or a prefix operator before one (3.0.x, 4.0.x, 5.0.x, master and 6.0.x), so `(int) a ** b` is `((int) a) ** b` (Groovy 3: `(int) 2.9 ** 2` is 4) and `a ** (T) b ** c` is `(a ** ((T) b)) ** c`; `PREC.CAST` is above POWER. Before 0.7.0 the grammar read `(int) (a ** b)`. A prefix `-` or `+` in the cast's operand still takes the powers after it: `(int) -a ** b` is `(int) (-(a ** b))`, Groovy's `((int) (-a)) ** b` (`docs/divergences-from-spec.md` §32). |
| 3 | UNARY_ADD | prefix `++` `--`, unary `+` `-` | right | |
| 4 | MUL | `*` `/` `%` | left | |
| 5 | ADD | `+` `-` (binary) | left | |
| 6 | SHIFT_OR_RANGE | `<<` `>>` `>>>` `..` `..<` `<..` `<..<` | left | **Apache puts ranges and shifts at the same level**, with left-associative resolution. `0..n<<1` parses as `(0..n)<<1`. |
| 7a | TYPE_TEST | `instanceof` `!instanceof` `as` | left | Apache's level 7 starts with these, ahead of the relational alternative, so they bind tighter: `1 < 2 instanceof Boolean` is `1 < (2 instanceof Boolean)` and `1 in [1] as String` is `1 in ([1] as String)` (observed with Groovy 3); `a + b as T` stays `(a + b) as T`. One alternative, `op=(AS \| INSTANCEOF \| NOT_INSTANCEOF) nls type`, in 3.0.x and 4.0.x; two in 5.0.x (`INSTANCEOF`, then `AS \| NOT_INSTANCEOF`) and three in master and 6.0.x (`INSTANCEOF`, `NOT_INSTANCEOF`, `AS`). Each takes a *type* on the right, not an expression, so their order among themselves changes no tree, and one level holds them. |
| 7b | REL | `<` `<=` `>` `>=` `in` `!in` | left | Apache's second alternative of level 7, `op=(LE \| GE \| GT \| LT \| IN \| NOT_IN) nls right=expression`; the right operand is an expression. |
| 8 | EQ | `==` `!=` `<=>` `===` `!==` | left | |
| 8.5 | REGEX | `=~` `==~` | left | Apache's `regexExprAlt`, its own alternative after `equalityExprAlt` and before `andExprAlt` in 3.0.x, 4.0.x, 5.0.x, master and 6.0.x, so looser than `==` and tighter than `&`: `"x" =~ "x" == 1` is `"x" =~ ("x" == 1)` and `1 == 1 =~ "x"` is `(1 == 1) =~ "x"` (observed with Groovy 3). Before 0.7.0 the grammar put both at one level, as the 4.0.x comment's "level 8.5" was read for one. |
| 9 | BAND | `&` | left | |
| 10 | BXOR | `^` | left | |
| 11 | BOR | `\|` | left | |
| 12 | LAND | `&&` | left | |
| 13 | LOR | `\|\|` | left | |
| 13.5 | IMPL | `==>` | right | Apache uses `<assoc=right>`. |
| 14 | TERNARY_OR_ELVIS | `? :` and `?:` | right | **Apache puts ternary and Elvis at the same level**, in the same rule alternative (`conditionalExprAlt`). `a ?: b ?: c` parses as `a ?: (b ?: c)` by right-associativity. `a ? b ?: c : d` parses as `a ? (b ?: c) : d` because the recursive descent on `tb=expression` between `?` and `:` re-enters `expression` at this same level. |
| 15 | ASSIGN | `=` `**=` `*=` `/=` `%=` `+=` `-=` `<<=` `>>=` `>>>=` `&=` `^=` `\|=` `?=` | right | |

And primary / postfix / member-access levels, which Apache models
inside `postfixExpression` rather than the `expression` alternatives:

| Tier | Constructs | Associativity |
|---|---|---|
| POSTFIX | postfix `++` `--`, subscript `[]`, safe-index `?[]`, call `()` | left |
| ACCESS | `.` `?.` `??.` (safe chain dot) `*.` `.&` `.@` `::` | left |
| PRIMARY | literal, identifier, `()`, `new`, closure, list, map | n/a |

Notes on the corrected ordering versus v1 of this spec:

- **Range and shift are the same level.** v1 split them (claiming
  Apache treats ranges as a separate production). That was wrong:
  the Apache `shiftExprAlt` rule explicitly alternates `(LT LT |
  GT GT GT | GT GT)` with `rangeOp=(RANGE_INCLUSIVE | …)`. amaanq's
  grammar puts ranges above shifts, which is a *deviation* from
  Apache, not the Apache rule. We follow Apache.
- **Ternary and Elvis are the same level.** v1 placed Elvis one
  level below ternary so `a ? b ?: c : d` would parse as
  `a ? (b ?: c) : d`. That parse is correct — but Apache achieves
  it differently. In Apache, both `? :` and `?:` are in
  `conditionalExprAlt` as the *same* alternative; the parse
  `a ? (b ?: c) : d` falls out because the middle slot recursively
  re-enters `expression`, and within that recursion the same
  conditional rule matches again on `b ?: c`. We replicate this
  with a single `conditional_expression` rule that internally
  chooses between the ternary and Elvis forms, then aliases the
  result to the appropriate distinct node kind (`ternary_expression`
  or `elvis_expression`) — so downstream tools still see the
  distinct kinds promised in §3.2.
- **Implication `==>` is right-associative** per Apache
  `<assoc=right>`. v1 was correct on associativity but worth
  emphasising: `a ==> b ==> c` parses as `a ==> (b ==> c)`.
- **`as` and `!instanceof` take a type, not an expression.** Apache
  has `op=(AS | INSTANCEOF | NOT_INSTANCEOF) nls type` in 3.0.x and
  4.0.x, splits it in 5.0.x (`INSTANCEOF nls matchingType`, then
  `(AS | NOT_INSTANCEOF) nls type`) and in master and 6.0.x into three
  (`matchingType`, `notInstanceofType`, `coercionType`), each a type,
  versus `op=(LE | GE | …) nls right=expression`, and the first alternative
  binds tighter (level 7a above): before 0.7.0 the grammar put both
  at one level, so `a < b instanceof T` was `(a < b) instanceof T`. The grammar must
  thread this through — the RHS of `as` is `_type`, RHS of `<` is
  `_expression`. See §3.2 cast_expression and
  instanceof_expression fields. A dotted name there is one type,
  whatever the case of its parts (`x as a.b.c`), as Apache's
  `qualifiedClassName` takes every name and dot before the last
  name. It takes array dimensions and type arguments as `_type`
  does (`o instanceof JTextComponent.KeyBinding[]`,
  `x as java.util.List<String>`), the type arguments only where they
  close (§3.2.2): `x as a.B<c` compares the cast with `c`. Every dot
  of the name continues the type, so the tokens never read as field
  accesses on `x as a`, and a quoted name or `new` after a dot
  (`x as a."b"`) is an `ERROR`; a weight on the dotted type
  (`DOTTED_CAST_TYPE_WEIGHT`) ranks the cast against other readings
  of the same lines (`docs/divergences-from-spec.md` §5). Before
  these operators, a `)` closes no cast (§6.11): `(x) instanceof Foo`
  tests `(x)`.

### 3.2 Operator → node kind map

Every cell maps to a distinct named node so that
`get_op_type()` in big-code-analysis can do exact node-kind matching
instead of fishing through tree shape. This is the explicit ask in
issue #246's "Suggested fix" path 1.

| Symbol | Node kind | Field shape | Notes |
|---|---|---|---|
| `+` `-` `*` `/` `%` `<<` `>>` `>>>` `&` `^` `\|` `&&` `\|\|` `<` `<=` `>` `>=` `==` `!=` | `binary_expression` | `left`, `operator`, `right` | Java-shaped; precedence per §3.1. |
| `**` | `power_expression` | `left`, `right` | Left-associative (§3.1); separate node so metric counts power-ops distinctly. |
| `?:` | **`elvis_expression`** | `value`, `default` | Resolves #246. No `MISSING` child anywhere. |
| `? :` | `ternary_expression` | `condition`, `consequence`, `alternative` | Standard. |
| `==>` | `logical_implication_expression` | `left`, `right` | Rare; emit distinct kind. |
| `===` | `identity_expression` | `left`, `operator: "==="`, `right` | Distinct kind avoids being lumped with `==`. |
| `!==` | `identity_expression` | `left`, `operator: "!=="`, `right` | Same kind as `===`; distinguished by operator field. |
| `<=>` | `spaceship_expression` | `left`, `right` | Distinct kind. |
| `=~` | `regex_find_expression` | `subject`, `pattern` | `pattern` is an expression (commonly slashy string). |
| `==~` | `regex_match_expression` | `subject`, `pattern` | As above. |
| `as` | `cast_expression` | `value`, `type` | `type` is a `_type`. NOT `binary_expression` — `x as String` is a coercion, not an arithmetic op. (Distinguishes from C-style cast `(String) x` which is `parenthesized_type_cast`.) |
| `in` | `membership_expression` | `element`, `operator: "in"`, `collection` | Distinct from generic binary so the `in` of `for (x in xs)` doesn't conflict. |
| `!in` | `membership_expression` | `element`, `operator: "!in"`, `collection` | Same kind, different operator. |
| `instanceof` | `instanceof_expression` | `value`, `operator: "instanceof"`, `type` | `type` slot is a `_type`. |
| `!instanceof` | `instanceof_expression` | `value`, `operator: "!instanceof"`, `type` | Same kind, different operator. |
| `..` | `range_expression` | `start`, `operator: ".."`, `end` | |
| `..<` | `range_expression` | `start`, `operator: "..<"`, `end` | |
| `<..` | `range_expression` | `start`, `operator: "<.."`, `end` | |
| `<..<` | `range_expression` | `start`, `operator: "<..<"`, `end` | |
| `?.` | `safe_navigation_expression` | `object`, `property` | `property` is `identifier` or `quoted_identifier`. |
| `??.` | `safe_chain_dot_expression` | `object`, `property` | **Groovy 4+.** Per `apache/groovy:GroovyLexer.g4:849` (`SAFE_CHAIN_DOT`), distinct from `?.`. Where `?.` only short-circuits when the *immediate* receiver is null, `??.` propagates null through the whole chain (Apache's own docstring: "safer than `?.`"). v1 of this spec omitted it; restored in v2. |
| `.` | `field_access` | `object`, `field` | The plain dot. |
| `.&` | `method_pointer_expression` | `object`, `method` | Distinct kind. |
| `.@` | `direct_field_access_expression` | `object`, `field` | Distinct kind. |
| `*.` | `spread_dot_expression` | `object`, `property` | Property/method access. |
| `*` (prefix in arg) | `spread_arguments` | `value` | Only valid inside argument lists, list literals and subscripts. |
| `*:` (prefix in map) | `spread_map_entry` | `value` | Only valid inside map literals. |
| `?[]` | `safe_subscript_expression` | `object`, `index` | Distinct from regular `subscript`. `index` repeats, as for `[]`. |
| `[]` | `subscript_expression` | `object`, `index` | After Apache `indexPropertyArgs` (`expressionList[true]`): one `index` per element, any of them a `spread_arguments` (`y[1, *a]`, `x[*a]`), so `index` is `multiple` in `node-types.json`. No empty or trailing-comma index (`y[]` is an array type where one may stand). |
| `T[]` as a value | `array_type` | none | `String[].class`, `int[]::new`, `@A(String[])`: the `array_type` of a declaration, with `type_identifier` or `qualified_type` inside, under the `field_access`, `method_reference_expression` or `method_pointer_expression` that `String.class`, `Foo::new` and `String.&new` have. Groovy reads `a[]` with any `a` as an index with no arguments and makes it a class when `a` names one; here only a primitive or capitalized name is taken (§6.15). |
| `::` | `method_reference_expression` | `target`, `name` | Groovy 3+. |
| `++` / `--` (prefix) | `unary_update_expression` | `operator`, `operand` | |
| `++` / `--` (postfix) | `update_expression` | `operand`, `operator` | |
| unary `+` `-` `!` `~` | `unary_expression` | `operator`, `operand` | |
| `new` | `object_creation_expression` | `type`, `arguments`, `body` | Constructor-only diamond (`new List<>()`) uses `generic_type` with empty `type_arguments`; ordinary types and nested type arguments require at least one argument. The optional `body` is the `class_body` of an anonymous inner class (`new Runnable() { … }`); a `{` after the arguments is always that body, never a trailing closure (#68). That is wider than Groovy, which calls a closure on the new object when the `{` starts a later line or cannot be a class body (`docs/divergences-from-spec.md` §11). Qualified creation (`outer.new Inner(...)`) has no production. |
| `new T[n]`, `new T[] { … }` | `array_creation_expression` | `type`, `dimensions`, `value` | After Apache `creator`: sized dimensions (a `dimensions_expr` per `[n]`) and then empty ones (one `dimensions` node for all the `[]`), or empty dimensions and an `array_initializer` in `value`, with a trailing comma allowed after the last element. `type` has no dimensions, here and in `object_creation_expression`, so `new int[]()` is an `ERROR`. A `[` on the line of the previous dimension is another dimension, so `new int[2][3]` is two-dimensional; a `[` on the next line is a list (§6.9). After one `[]` the elements are expressions, as in Groovy 3 and 4, so a `{` among them is a closure (`new Closure[] { { it * 2 } }`). After two or more `[]` an element may also be a nested initializer (`new int[][] {{1, 2}, {3}}`), which only Groovy 5 accepts (GROOVY-8551); there a `{` is an initializer unless it can only be a closure (`{ -> 1 }`, `{ a, b -> a }`), as Groovy 5 tries `arrayInitializer` first, and `{ x = 1 }` is the exception and reads as a closure. Also wider than Groovy 4: `new int[3] { it }` and `new int[2][3] {1}` are a call with a closure, as is a `{` on the next line (divergence §11). Groovy 3 reads the same-line braces as an initializer after sized dimensions and rejects it ("dimension should be empty"); the 4.x `creator` allows `nls` before an initializer, so it reads the next-line ones the same way. Narrower than Groovy 5: `new int[2][][0]` is an `ERROR`, as Groovy 3 and 4 reject it ("dimension cannot be empty"), where Groovy 5's grammar reads a subscript. Annotations on the type or a dimension (`new @A int[3]`, `new int @A [3]`) have no production. |
| `=` and all augmented assignments | `assignment_expression` | `left`, `operator`, `right` | One node kind, operator field distinguishes. Includes `?=` (Elvis assign). |
| `?=` | `assignment_expression` (operator `"?="`) | as above | Distinct via operator field; semantically "assign if currently null/falsy." |
| `(a, b) =` | `multi_assignment_expression` | `name` (one per name), `value` | Two or more names and `=` only; see §4. The declaration form `def (a, b) =` is `multi_assignment_declaration`. |
| `->` (lambda) | `lambda_expression` | `parameters`, `body` | Groovy 3+. `parameters` is a `formal_parameters` (`(a, b) -> …`, `() -> …`, `(int a, String b = 'x') -> …`) or one `identifier` (`x -> …`); `body` is a `block` or an expression or a parenthesis-free call. The shape of tree-sitter-java's, whose untyped parameters are an `inferred_parameters` instead. See §5.16. |

The full list of *symbols* required by issue #247 and resolved here:
`?:` ✓ `?.` ✓ `as` ✓ `..<` ✓ `*args` (spread args) ✓ `*.` ✓ `*:` ✓
`=~` ✓ `==~` ✓ slashy `/…/` ✓ `===` ✓ `!==` ✓ `<=>` ✓ `!instanceof` ✓
`!in` ✓ `?=` ✓ `?[]` ✓ `.&` ✓ `.@` ✓ `$/…/$` ✓ `==>` ✓.

Additional Groovy 4 operators not in issue #247 but recognised by
this spec: `??.` (safe chain dot), `::` (method reference, also
Groovy 3+).

Issue #247 also lists **`trait`** as a parse failure; covered in §4.

#### 3.2.1 Composability — subscript chained with call

`stepImplementations[Platform.ALL](ctx)` appears in murtaza64's
`realcode.test`. The construct is "subscript that yields a callable,
then invoked." Grammar implication: `subscript_expression` and
`safe_subscript_expression` are postfix forms that compose with the
following `argument_list` to produce a method invocation. The parse
tree shape:

```text
(method_invocation
  function: (subscript_expression
              object: (identifier "stepImplementations")
              index:  (field_access object: … field: …))
  arguments: (argument_list (identifier "ctx")))
```

This works automatically if `method_invocation`'s `function` field
accepts any `postfix_expression` (which includes subscripts), rather
than only an `identifier`. The amaanq grammar's `method_invocation`
uses `primary_expression` for the callee slot, which suffices. The
spec inherits that breadth and explicitly tests this case.

Field name is `function:` (not `name:` or `object:`) because the
callee may be any `_expression` — identifier, field-access,
subscript, or spread-dot — and `name:` / `object:` each describe
only a subset of those shapes. All §8 corpus snippets and the
`queries/groovy/highlights.scm` patterns use this field name.

#### 3.2.2 `!instanceof`, `!in` and a cast type's `<` — lexer rules

Apache `GroovyLexer.g4` defines these tokens with a positive
trailing-context predicate:

```antlr
NOT_INSTANCEOF : '!instanceof'
   { isFollowedBy(_input, ' ', '\t', '\r', '\n') }?;
NOT_IN         : '!in'
   { isFollowedBy(_input, ' ', '\t', '\r', '\n', '[', '(', '{') }?;
```

…i.e., the lexer only emits `!instanceof` if the next character is
whitespace, and `!in` if followed by whitespace or an opening
bracket. Without this rule, `!instanceofX` would be a single
identifier-looking token (or the parse would be ambiguous between
`!instanceof X` and `! instanceofX`).

`!in` is a token of the external scanner (§6.12), which emits it only
when the character after it is one of the permitted ones and leaves
that character in place, so `a !in[1, 2]` and `a !in(b)` parse.
`!instanceof` is an in-grammar token that includes the whitespace
after it; it needs no bracket form. The grammar rule for the unary `!`
operator otherwise consumes the `!` and parses the rest as an
expression.

After the name of the type of `as`, `instanceof` or `!instanceof`, a
`<` with no space before it opens type arguments only where they
close: the external scanner token CAST_TYPE_ARGUMENTS_OPEN is that
`<`, and anywhere else the `<` is a comparison's. Groovy takes type
arguments there where they parse and compares otherwise
(`x as a.B<c` and `v as java.lang.Integer<max && min>v` are
`(x as a.B) < c` and `((v as java.lang.Integer) < max) && (min > v)`);
without the token the `<` always opened them, and such input was an
`ERROR`.

```text
Emit CAST_TYPE_ARGUMENTS_OPEN for a `<` that is the first character
the scanner sees (no space before it) when what follows reads as
type arguments up to the `>` that closes them: names, `.`, `,`, `?`,
`[`, `]` and a `<` right after a name, with spaces, tabs, line
escapes, line breaks and comments between them, at most 256 items.
On anything else, or past the bound, return false; the in-grammar
lexer then reads the `<` of a comparison.
```

As in every `_type`, a space before the `<` makes it a comparison:
`x as List <String>` is an `ERROR`, where Groovy, whose lexer drops
the space, reads the type arguments. The scanner reads up to the `>`,
so the token is lexed again after any edit there. It reads as far in
the state after an expression, where the token is not valid and a
reparse that reuses the cast lexes the in-grammar `<`, so there too an
edit that closes the type arguments makes the runtime lex the `<` again
where the token is valid (`bindings/rust/tests/incremental_reparse.rs`
pins it). It is not emitted during error recovery.

The `<` of any other generic type's arguments (`type_arguments`) is
either the in-grammar `<` right after the name, which a diamond
(`new Foo<>()`) and type parameters share, or the external
TYPE_ARGUMENTS_OPEN, which the same read decides. The in-grammar one
has the lower lexical precedence of the two in-grammar `<` tokens.
Where a comparison may follow the name as well as type arguments,
after a name in parentheses, which may be a cast's type, and where a
statement starts, which may be a declaration, both match. The lexer
took the type arguments' there, so `(i<n)`, `(i<n && j>m)`, `i<n` and
`{ it<2 }` read type arguments that never closed, an `ERROR`, where
Groovy compares (Groovy 3; Apache's `castParExpression` takes a type
and then an operand after the `)`, and `localVariableDeclaration` a
type and then a name). Now the comparison's `<` wins there, and type
arguments open with TYPE_ARGUMENTS_OPEN:

```text
Emit TYPE_ARGUMENTS_OPEN for a `<` that is the first character the
scanner sees, where it is valid and CAST_TYPE_ARGUMENTS_OPEN is not
emitted, when the `>` does not follow at once (a diamond) and the read
above finds type arguments that close or reaches its bound.
```

So `(List<String>) x`, `List<String> xs = []` and type arguments past
the bound (more than about 85 arguments with a space after each comma)
read as before. Where only a type may follow, the in-grammar `<` is the
only one, and the token changes no tree there. A chain of comparisons
past the bound, `(a<a<a<…<1)` with more than about 128 of them, reads
as type arguments and is an `ERROR`, as it was. Type arguments that
close and that Groovy reads otherwise stay an `ERROR`: `(a<b>c)`, which
Groovy 3 reads as the call `a.call(c)`, and `(i<n>>1)`. A `<` after a
name, of every generic type and of every comparison without a space
before it, starts the bounded read: `dotted_cast_type_cost.rs` checks
nests of generic types and chains of comparisons.

---

## 4. Statement and declaration coverage

Productions that must be supported. Every row that says "new" is a
fix relative to *both* parent grammars.

| Construct | Status | Notes |
|---|---|---|
| `package` declaration | port from amaanq | both grammars have it; trim trailing-`;` requirement to optional |
| `import` (including static and aliased) | port from amaanq | `import static a.b.C.*`, `import a.b.C as D`. Annotations may come first (`@Grab('g:a:1') import a.B`), on `package` too. |
| `class` / `interface` / `@interface` declaration | port from murtaza64 | Body members: fields, multi-assignment declarations (`def (a, b) = …`, see Multi-assignment below), methods, constructors, `static { … }`, nested `class` / `interface` / `trait` / `enum` / `record` / `@interface` declarations, and instance initializers — a bare `block` child of `class_body` (#60, #68). `record`, `trait` and `sealed` stay usable names next to the nested declarations because their keywords come from the scanner (§6.7, §6.13; `docs/divergences-from-spec.md` §12). An `@interface` element may take a default, as in Apache's `methodDeclaration` (`DEFAULT elementValue`): `String value() default 'x'` is a `method_declaration` whose `default` field holds an expression, an annotation, or a `list_literal` of annotations (#101). Only in an `@interface` body may `default` follow the `()`, on the same line or the next: in an interface, Groovy reads `void a()` followed by `default void b() { }` on the next line as two methods, telling them apart by the line break, which is whitespace here. A method takes the `default` modifier (Apache's `classOrInterfaceModifier`), in any type body and at the start of a statement: `default void b() { }`, with annotations before it, other modifiers before or after it, type parameters, and `def`, a return type or neither; with neither it is a method, not a constructor (`default m() { }`). The modifier is an anonymous `default` child of `method_declaration` before the `name`, and an element's default is the `default` field after the parameters; no method has both. A class, interface, trait, enum or record declaration takes it among its modifiers too, at the top level or nested (`default class C { }`). Fields, constructors, compact constructors and local variables do not take it, and each is an `ERROR` with it. `default` is a keyword wherever a member or a statement can start, as Groovy reserves it, so a member or statement on the line before ends there; as a name in those positions (`default = 1`, `return default`, `String default`) it is an `ERROR`, and `default:` there is still a label (`docs/divergences-from-spec.md` §20). |
| `trait` declaration | **new** | identical body to `class`; distinct node kind `trait_declaration`. Closes the `trait` row in #247. |
| `enum` declaration | **new** | optional `implements` clause (`super_interfaces`, as on classes); constants list (each constant may carry annotations and arguments), then class members after `;`, `,;`, or nothing (the newline Groovy reads is whitespace here, #55). A member may also follow a trailing `,` directly, which Groovy accepts up to 4.x and Groovy 5 rejects (#88). This grammar takes a narrower set than Groovy there: a member that starts with a modifier, `def` or a declaration keyword, with annotations before it allowed; a constructor needs an access modifier. A member that starts with a bare name is not accepted: `A,` then `String x` or `E() { }` is an `ERROR`, the second because it reads as a constant with an anonymous class body (`docs/divergences-from-spec.md` §13). Arrow switch is the typical use site (#36). |
| `record` declaration | **new** | `[modifiers] record Name[<T>](Type a, Type b) [implements I] { … }`, after Apache's `classDeclaration`. Groovy 4+. Top-level and as a class member (#60). The body is optional here where Groovy requires it; records cannot `extends`. The body takes the members of a class body plus a compact canonical constructor, Apache's `compactConstructorDeclaration`: `public Name { … }`, with annotations, modifiers, `def` after the modifiers, or none of them, and the `{` on the same line or the next, is a `compact_constructor_declaration` with `name` and `body` (#72). The body node is still `class_body`. Only a record body takes it; Apache's parser takes it in any class body and rejects it afterwards outside a record. Its name is not compared with the record's, so any `Name { … }` there is one. A modifier-only field followed by a block (`static count`, then `{ … }` on the next line) reads as a compact constructor too, and a `;` after the field keeps the field and the block. `static count` followed by `Name { … }` is that field and a compact constructor, by a dynamic precedence, so `static Foo bar { … }` is a field `Foo` and a constructor `bar`. A type nested in a record carries a dynamic precedence too, so after a typed field with modifiers (`private String y`, then `class C extends B { }` on the next line) it stays a type declaration rather than splitting into fields named by its keywords and a compact constructor (`docs/divergences-from-spec.md` §6). A string-literal name is not supported (`docs/divergences-from-spec.md` §17). |
| `sealed` / `non-sealed` / `permits` modifiers | **new** | parsed as modifier tokens that take a `permits` clause on class/interface. Groovy 4+. |
| Method declaration | port from murtaza64 | `def` or type, optional throws clause. Modifiers alone also do (`private foo(x) { … }`), and at a script's top level and in a type body, annotations alone and modifiers with type parameters (`@Memoized fib(n) { … }`, `private <T> first(xs) { … }`), as in Apache's AstBuilder ("Modifiers or return type is required"). A script method of those two forms needs a body, and a block or a closure, which in Groovy declare no method, takes neither (`docs/divergences-from-spec.md` §34). In a class body `@A setX(a) { … }` is a constructor (#91). With no `def` and no return type the name is Apache's `methodName`, an identifier that is no keyword (§5.2) or a string with no interpolation (`@A 'a b'(n) { … }`; `@A "a${x}"(n) { … }` is an `ERROR`, §33 there) |
| Method definition (with body) | port from murtaza64 | |
| Multi-assignment | **new** | `def (a, b) = expr` is a `multi_assignment_declaration` (murtaza64 #22). `(a, b) = expr` is a `multi_assignment_expression` (#94), after Apache `#multipleAssignmentExprAlt`: two or more plain names, then `=` and no other operator, with line breaks allowed around the `=`. It is an expression, so it nests: `x = (a, b) = list`, `foo((a, b) = list)`, a `for` init, after `return`. Its names are `identifier`s rather than `variable_declarator`s, as nothing is declared. With one name, `(a) = x` is an `assignment_expression` of a parenthesized expression, as in Apache. A `(` that opens a line after an expression opens the next statement, as in Groovy (§6.18), so two multi-assignments on consecutive lines, `(a, b) = [1, 2]` then `(c, d) = [3, 4]`, are two statements, and so are the swap `def b = 2` then `(a, b) = [b, a]`. One line can be read as two statements: `def f(a, b) = [1, 2]`, which Groovy rejects, is a declaration of `f` followed by a multi-assignment. That reading of a method's head ends at its first `,` where the parentheses go on as a method's parameters, which the scanner reads there (§6.20), so `def f(a, b) { }` takes as many parse steps as before multi-assignments. The value of either form may be a parenthesis-free call, as Apache's `statementExpression` and `variableInitializer` allow: `(a, b) = f 3` and `def (a, b) = f 3` (§5.5, #95). The declaration form follows Apache's `variableDeclaration` with `typeNamePairs`: annotations, then `def`, `final def`, `final var`, `final` alone or `var` alone (`final (a, b) = …`, `@Field def (a, b) = …`, `var (a, b) = …`). `var(a, b)` and `var (a, b).foo()` call a method named `var`, as in Groovy (observed with Groovy 3): the scanner takes `var` alone as the keyword only when names, `)` and `=` follow (§6.16). A name may have a type (`typeNamePair`): `def (int a, String b) = …`, `final (List<String> xs, b) = …`. A typed name is a `variable_declarator` with a `type` field before its `name`; an untyped name keeps the `variable_declarator` with a `name` alone. Two Groovy 6 forms are accepted, after Groovy 6's `GroovyParser.g4` and `AstBuilder`; the Groovy 3 this grammar is checked against rejects both. A rest binding, `def (a, *b) = list` or `def (h, List<Integer> *t) = list`, is a node of its own, a `rest_declarator` with the optional `type` and the `name` of a typed name around its anonymous `*`, in any place of the list (`def (*init, last) = list`, `def (a, *b, c) = list`); the other names stay `variable_declarator`s. A keyed pair, `def (name: n, age: int a) = map`, is a `variable_declarator` with a `key` field before the optional `type` and the `name`. Groovy's list holds names or keyed pairs, not both; here one list takes both, so `def (name: n, b) = m` parses, which Groovy rejects (with a list of pairs alone, recovery from an error after the `=` could not go back into the list at a plain name, and a run of such errors took time quadratic in its length, `docs/divergences-from-spec.md` §29). A declaration whose `=` does not come, as while it is typed (`def (a, b)` before the next statement), is an error production: its `value` is a missing `identifier`, and the statements after it stay statements. Where a token that starts no statement stands in place of the `=` (`def (a, b) : l`), recovery wraps the names in an `ERROR` and reads on after `def`, and the statements after that line stay statements too. A `,` may follow a declaration without a value, with its `=` or without (`def (a, b) =,` or `def (a, b),` before `def c = 1`), so that the next line stays a statement; on one line, before a name and a `)`, the list goes on (`def (a, b) =, c) = 1` declares three names with an `ERROR`). A `,` after a value is an `ERROR`. The declaration is also a `for` init (`for (def (i, n) = [0, 5]; i < n; i++)`) and a member of a type body, Apache's `fieldDeclaration`, with the annotations and modifiers of a field and `def` or `var`, or modifiers alone (`static final def (a, b) = …`, `private var (a, b) = …`, `private (int a, b) = …`). Groovy parses the member and then drops it: it declares no field (Groovy 3 to 6). Not accepted, each an `ERROR`: annotations alone (`@A (a, b) = …`), where the `(` would open the annotation's arguments or the names, and other modifiers where a statement starts (`static def (a, b) = …`), which Groovy 3 takes and ignores where it rejects `static def x = 1`. In a type body `var x = 1` is a field with `var` a modifier, as in Groovy (§6.16). Wider than Groovy: `def (a = 1, b) = …` parses. A keyword or `val` before a name is its type: `def (def a, b)` has the type `def`, and so do `var`, `val`, `final` and `default` in that place, where Groovy 6 reads `def`, `var` and `val` as no type and Groovy rejects `final` and `default`. A reserved word is a key (`def (class: v) = m`, `def (if: v) = m`; `new` and `case` are not, §5.2), as it is a name in `def (a, if) = l`: a keyword with no action lexes as an identifier. More than one rest binding parses (`def (*a, *b) = …`), which Groovy rejects after parsing. As a line break is whitespace, `foo final (a, b) = c` and `foo def (a, b) = c` are a statement `foo` followed by a declaration, where Groovy rejects them. A line that ends in a parenthesized name, dotted name or primitive type (`def y = (x)`, `x = (b)`, `return (x)`, `(Foo.Bar)`, `(int)`) followed by a multi-assignment declaration on a later line is two statements, as in Groovy, when the declaration starts with `final` or with `var` alone, has a rest binding or keyed pairs, or is a member of a type body. Before those forms were accepted the second line was read with no `ERROR` as the operand of a cast, a call of a method named by its first word: `(x) def(h, *t) = l` (`docs/divergences-from-spec.md` §15). After a parenthesized generic type the cast reading remains: `def y = (List<String>)` then `def (a, *b) = l`, or `def (a, b) = l`, is still `(List<String>) def(a, *b) = l`, where Groovy reads two statements. |
| Local variable declaration | port from both | including `def` and explicit types; annotations may come first (`@Field def x = 1`), and `@Library('shared') _`, an annotation on the placeholder name `_`, is a declaration of `_`. Apache takes any name after annotations alone; only `_` is accepted here, because a newline is whitespace and `@A x` followed by `y = 1` would also be the typed local `x y = 1`. A typed declaration needs no initializer where a statement starts, after annotations and as a `for` init (`String s`, `int i;`, `List<String> xs`, `foo.Bar q`, `@Field int a, b`, `for (int i; i < n; i++)`), and after the first declarator any declarator may go without one (`String a, b = 1`, `String a = 1, b`, `final String a, b`); which statements are such declarations the scanner decides by Apache's rule (§6.21). The node is `local_variable_declaration` with one `type` field, a `name` field per declared name and a `value` field after the name it belongs to, as for a typed declaration with initializers. |
| Closure expression | port from murtaza64 | `{ a, b -> … }` and `{ … }`; parameters take annotations, `final`, `def` or a type, varargs and a default (§5.8). |
| `if` / `else` | port | |
| `while` / `do-while` | port | |
| `synchronized` statement | **new** | `synchronized (lock) { … }`; the body must be a block. Apache `#synchronizedStmtAlt`. |
| `for(;;)` | port | `init` accepts a single expression (`i = 0`), a `local_variable_declaration` (`int i = 0`, `def i = 0`, incl. multiple declarators) or a `multi_assignment_declaration` (`def (i, n) = [0, 5]`). Apache `forInit: localVariableDeclaration \| expressionList` also allows a comma expression list (`i = 0, j = 1`) — not yet supported. |
| `for (x in xs)` | port | The loop variable takes `final` and `def`, `var` or a type (`for (var x : xs)`). |
| `switch` with `case … :` | port | A case takes several values (`case 1, 2:`), as below. |
| `switch` with `case … -> …` | **new** | arrow form. Closes murtaza64 #36 (second half). A case takes several values, a `value` field each (`case 1, 2 ->`, `case String, Integer ->`), as Apache's `switchExpressionLabel` is `CASE expressionList[true]` before `->` or `:` (4.0.x to master; master also takes a pattern). Its `switchLabel`, a switch statement's, takes one expression, but a statement may be a switch expression; Groovy 3 has no switch expressions and rejects both. A spread value (`case *xs ->`, which `expressionList[true]` allows) is not taken. |
| `try` / `catch` / `finally` | port | |
| Multi-catch | **new** | `catch (Foo \| Bar e)`. Closes murtaza64 #39. A catch parameter takes `final` and `def`, `var` or a type. |
| Try-with-resources | **new** | `try (Foo f = …) { … }`; a resource takes `final` and `def`, `var` or a type. |
| `return`, `break`, `continue`, `yield` | port + fix | break/continue accept an optional label. The value of `return` and the label must start on the keyword's line, as in Groovy: a line break after the keyword ends the statement (§6.23). `yield` is a keyword before an expression wherever a statement starts, not only in a switch expression, and a name elsewhere (§6.16). |
| `throw` | port | |
| `assert e`, `assert e : msg` and `assert e, msg` | port from murtaza64 + fix | power-assertion shape; the message's separator is `:` or `,`, on either line, as Apache's `assertStatement` takes `NL* (COLON \| COMMA) NL*`. |
| Labeled statement | port from murtaza64 | `name: statement`. Disambiguation vs. map literal in §5.3. |
| Block statement | **new** | A `{ … }` that opens a statement, in a file, a block, a closure or a colon case, or after a label, is a `block`, as Apache's `statement` lists `block` first: `{ int x = 1; foo(x) }` in a method, `lbl: { … }`, `case 1: { … }`. Where an expression continues the braces, on their line or the next, they are a closure, as in Groovy: `{ foo() }()`, `{ foo() }.call()`, `{ foo() } as Runnable`, `{ } + 1`, `{ }` followed by `.call()` on the next line. Braces with parameters (`{ a -> }`, `{ -> }`) are a closure, and a `{` after an expression, on its line or the next, is that expression's trailing closure (`foo()` followed by `{ x }`). The scanner decides at the `}` (§6.22). Groovy 2's parser, which Groovy 3 still runs with `-Dgroovy.antlr4=false`, rejects an unlabeled block there ("Ambiguous expression could be either a parameterless closure expression or an isolated open code block"); the parser of Groovy 3 accepts it as a block through class generation (observed), and the grammars of 4.0.x to 6.0.x take it the same way. |
| Command chain (`juxt_function_call`) | port from murtaza64 + §5.5 | A statement's call of a name with a closure that closes on its line is the object of a member access that follows it, on the line or a later one (`foo { }.bar()`), and the receiver of a parenthesis-free call where a first argument follows the accesses (`foo { }.bar 1`), as in Groovy (§6.24). |
| `pipeline { … }` followed by anything | port + fix | Promote `pipeline` to a regular statement, drop `optional($.pipeline)`-at-EOF. Closes #37. |
| Annotation usage `@Foo(args)` | port | including `@Foo` and `@Foo.Bar(x = 1)`. |
| Shebang line | port from amaanq | `#! …\n` at start of file. |
| Line comment `// …` | port | Ends before `\n` or `\r`, so a lone CR ends the line as it does for Groovy's lexer, and in a CRLF file the `\r` is not part of the comment. |
| Block comment `/* … */` | **fixed** | Single `block_comment` token. Must accept `/**/` and `/***/`. Closes murtaza64 #16. |
| Groovydoc `/** … */` | port from murtaza64 (but tokenised) | Single `groovydoc_comment` token; tag parsing is out of scope per §1.2(1). |

---

## 5. Lexical and syntactic edge cases

This section is the "sequential thinking" output: each tricky bit of
Groovy gets a discrete decision rather than being hand-waved.

### 5.1 Numeric literals

```text
integer_literal  = bin | oct | hex | dec [type_suffix]
float_literal    = digits "." digits [exponent] [type_suffix]
                 | "." digits [exponent] [type_suffix]
                 | digits exponent [type_suffix]
type_suffix      = one-of  G g L l I i D d F f
```

- `_` separators allowed between digits per Java rules.
- Leading sign is **not** part of the literal — `-3` is `unary_expression`
  over `3`. (murtaza64's grammar bakes the sign into the literal regex,
  which breaks `a-3` parsing in column position; we don't repeat that.)
- The `G` suffix promotes to `BigInteger`/`BigDecimal` per the Groovy
  spec — this affects the type system but the grammar treats it as a
  pure suffix.

### 5.2 Identifiers and reserved-but-contextual words

An identifier starts with a character of the Unicode general
categories L, Nl, Sc or Pc, and continues with those or Nd, Mn or Mc.
That is the rule of Groovy's lexer (`GroovyLexer.g4` `JavaLetter` and
`JavaLetterOrDigit`): Java's `isJavaIdentifierStart` and
`isJavaIdentifierPart`, without the identifier-ignorable characters.
`$` is in Sc and `_` in Pc, so non-ASCII names such as `名前`, `café`
and `€uro` are identifiers, and `a·b` (a middle dot, Po) is not. The
Unicode `XID_*` classes differ from this rule in both directions and
are not used. `grammar.js` writes the ASCII characters out before the
classes (`identifierPattern`); that is for lexer speed and does not
change the set.

Groovy's keyword set, derived from `apache/groovy:GroovyLexer.g4`
lines 400–501, splits into three groups:

**Hard reserved keywords** (always keywords, can never be
identifiers): `abstract`, `assert`, `boolean`, `break`, `byte`,
`case`, `catch`, `char`, `class`, `const`, `continue`, `default`,
`do`, `double`, `else`, `enum`, `extends`, `final`, `finally`,
`float`, `for`, `if`, `goto`, `implements`, `import`, `instanceof`,
`interface`, `int`, `long`, `native`, `new`, `package`, `private`,
`protected`, `public`, `return`, `short`, `static`, `strictfp`,
`super`, `switch`, `synchronized`, `this`, `throw`, `throws`,
`transient`, `try`, `void`, `volatile`, `while`. Also the
Groovy-specific hard keywords `def`, `module`, `threadsafe`.

The implementation reserves two of them, `new` and `case` (`reserved`
in `grammar.js`). Every other keyword lexes as an identifier wherever
it has no parse action, so it is a name there, wider than Groovy. A
reserved word is no name: `case = 1`, `foo(case)`, `x = case`,
`def m(int case) { }`, `{ case -> }`, `[(case): 1]`, the label
`case:`, `case new Foo()` outside a switch, `import a.case.B` and
`enum E { case }` are an `ERROR`, as Groovy rejects them. It stays a
name where Groovy takes a keyword: after a member access
(`x.case`, `x?.case()`, `x.&case`, `x::case`, which reserve no word),
as a map key or a named argument's key (`[case: 1]`, `foo(case: 1)`,
`foo case: 1`, `foo a: 1, case: 2`, Apache's `mapEntryLabel` and
`namedArgLabel`), and as an annotation element's name (`@A(case = 1)`,
`elementValuePairName`). Where a method's name may follow, no word is
reserved either, so `def case = 1`, `def case() { }` and
`class C { int case }` parse, which Groovy rejects, as they do with
`new`. The name of a method with no `def` and no return type, after
modifiers, annotations or type parameters (`_untyped_method_name`),
reserves every keyword above that is a token of this grammar, as
Apache's `methodName` takes an `identifier`: `private case(x) { }`,
`@A true(x) { }` and `private <T> static(x) { }` are an `ERROR`. A
parse state takes the reserved set of one of its positions, and there
it also covers the type that may stand in the name's place
(`private int x`), so `this`, `void` and the primitive types, which are
identifiers here, and `goto`, `const` and `threadsafe`, which are no
tokens of this grammar, stay names in that position
(`docs/divergences-from-spec.md` §33). Only `case` is a key
everywhere: `[new: 1]` is an `ERROR`, and `new` is a key after a comma
only on the comma's line (§6.8). A
keyed multi-assignment pair takes neither as its key
(`def (case: n) = m` is an `ERROR`; Groovy 6's `keyedPair` takes an
`identifier`). `case` is reserved because, as a name, error recovery
on `case 1:` lines outside a switch read each line as a label nested in
the one before, in time quadratic in the lines (§13). Reserved, it can
be shifted only in a switch body, and recovery skipped it: `A:` of
`case A:` became that label instead, which made `case A:` lines and
`x = case` lines quadratic. So a `case` label where a statement starts
is an error production (`_stray_case`), `case`, an expression and `:`
before `UNREACHABLE`, a token the scanner never emits (§6). Recovery
inserts that token as a missing `switch`, and each line is an
`expression_statement` of its own, with the value and
`(MISSING "switch")`: `case A:` and `case 1:` outside a switch, under a
header that is no `switch` (`swit (x) {`) and in a block inside a case.
In a switch body a `case` after a case's statements still ends the case
(`switch_case` is left-associative), so no valid tree changes. A label
with several values (`case 1, 2:`) takes the last one; those before it
are an `ERROR` in the same statement. In expression position the
production does not apply, and runs of `foo(case,` and a space and of
`x ? y :` followed by a line with `case` still take time above linear
in their length (`docs/divergences-from-spec.md` §29).

**Contextual keywords** (keywords only in specific syntactic
positions, identifiers elsewhere): `as`, `in`, `var`, `record`,
`trait`, `sealed`, `non-sealed`, `permits`, `yield`, `async`, `await`,
`defer`, `val` (the last four added in v2 of this spec from the
Apache lexer; `val` is `'val' {isValEnabled()}?` — conditionally
enabled). Each appears in only one position-specific rule:

- `as` — RHS operator of `cast_expression`; also `import X as Y`.
- `in` — RHS operator of `membership_expression`; also `for (x in xs)`.
- `var` — replaces `def` as a declaration type token.
- `record`, `trait`, `sealed`, `non-sealed`, `permits` —
  class-declaration keywords, prefixes and clauses.
- `yield` — switch-expression branch.
- `async` — closure/lambda prefix (`async closureOrLambdaExpression`).
- `await` — unary prefix on an expression or expression list.
- `defer` — block prefix.

The grammar encodes contextual keywords as ordinary tokens with
explicit fallbacks where they may also be identifiers:

```js
identifier_or_contextual: $ => choice($.identifier, 'var', 'record',
                                     'sealed', 'non-sealed',
                                     'permits', 'yield', 'async',
                                     'await', 'defer', 'val')
```

…and uses `identifier_or_contextual` everywhere the lexer might
otherwise hide a usable identifier behind a keyword (variable names,
property accesses, named arguments).

The implementation takes a different route for `record`, `trait` and
`sealed`: each keyword is an external-scanner token emitted only
before the declaration it opens (§6.7, §6.13), so the word lexes as a
plain `identifier` in every other position and no fallback choice is
needed. `var`, `yield`, `in` and `as` take the same route (§6.16):
`var` is a keyword only before a declared name or a multi-assignment's
names, `yield` and `in` only before an expression, and `as` only before
a type. `permits` stays a grammar literal: it has an action only after
a type header, where no name can stand. `non-sealed` is not a name in
Apache's grammar. Groovy 6 adds `async`, `await`, `defer`, `module` and
`val` to Apache's `identifier` rule; this grammar has no construct for
them, so they are identifiers everywhere already.

Quoted identifiers (`map."key with spaces"`, `def "test name"() { … }`)
are supported by allowing a string-literal in identifier position
inside method declarations and after `.`. Per the Groovy syntax
docs, this is how methods with keyword names work too:
`def "abstract"() { true }`.

### 5.3 Label vs. map literal vs. ternary

Three syntactic positions where `:` is ambiguous:

1. `LABEL: stmt` — statement label.
2. `[ k : v ]` — map entry.
3. `c ? t : e` — ternary.

Disambiguation strategy:

- **Labels** are only matched at statement-start position; the
  statement rule sequences `optional($.label)` *before* the
  statement choice. The label rule itself is
  `seq(field('name', $.identifier), ':', /[ \t]*/, NEWLINE_OR_NONE)`
  — but tree-sitter cannot peek arbitrarily, so we use a positive
  lookahead via the external scanner (token `_label_colon`) that
  succeeds only when the colon is at statement-start and the next
  non-whitespace token is not a keyword that starts a sub-expression.
- **Map entries** sit inside `[ … ]` or argument lists. The map rule
  pre-empts the list rule via tree-sitter precedence.
- **Ternary colons** are an internal `:` already nested inside the
  ternary production; no ambiguity once `?` has been seen.

murtaza64's grammar gets the label/map/ternary distinction working
*mostly* by precedence (`assignment` at `-1`, `parenthesized_expression`
at `PREC.PRIORITY`); we make the rule explicit with the external scanner
token because the precedence-only approach failed on the test cases in
murtaza64 #5 (`a b, c ? d : e` and `f y:b ? … : …`).

### 5.4 Slashy strings vs. division

`/foo/` is a slashy string regex literal. `a / b` is division. The
disambiguator is a context-sensitive lexer token:

- A `/` *can* start a slashy string only if the preceding non-trivia
  token is one of:
  - statement boundary (start-of-file, `;`, `\n` at statement-end,
    `{`, `(`, `[`, `,`, `:`, `=`, `=>`, `->`),
  - any operator that takes an expression on its right (`+`, `-`,
    `*`, `/` itself if a previous `/` was already eaten as division,
    `==`, `!=`, etc.),
  - the keywords `return`, `yield`, `throw`, `in`, `as`,
    `instanceof`, `case`, `when`.
- Otherwise the `/` is division (or `/=`).

This decision lives in the external scanner (§6.2). The corresponding
test (`def s = /pat/`, `a = b / c`, `[1, /pat/, 2]`) is in §8.

Dollar-slashy `$/…/$` does not have this ambiguity — `$` is not a
binary operator — so it is a simple grammar rule.

### 5.5 Command chains (`juxt_function_call`)

The murtaza64 grammar's `juxt_function_call` has open issue #5 —
optional parentheses produce ambiguities with ternaries and map items.
The resolution we adopt:

1. **Command chains are statements and values, not general
   expressions.** In `GroovyParser.g4` (4.0.x) `a b c d` is a
   `commandExpression`, the only form of `statementExpression`. A
   `statement` takes one (`#expressionStmtAlt`), and so do
   `variableInitializer` (through `enhancedStatementExpression`), the
   right side of `#assignmentExprAlt` and of
   `#multipleAssignmentExprAlt`, `expressionInPar` and `lambdaBody`.
   An operand and the value of `return`, `throw`, `assert` or `yield`
   take an `expression`, so `return foo 1` is rejected by Groovy and is
   `return foo` followed by `1` here. The
   grammar accepts a command chain as a statement, and as a value: of
   a declarator (a local variable, a field, `def (a, b) = …`) or of an
   assignment with any operator (`x = foo 1`, `x.y += foo 1`,
   `(a, b) = f 3`), and in parentheses (`(sh script: 'x').trim()`),
   also those of the condition of `if`, `while` and `do`, of a
   `switch` value and of a `synchronized` lock
   (`if (fileExists 'pom.xml')`), and in a `${…}` interpolation
   (`"${sh script: 'x'}"`), which Groovy reads as a closure whose one
   statement is the call. That covers Jenkins'
   `def out = sh script: 'make', returnStdout: true`. A value in
   parentheses is a primary, so it nests wherever an expression does:
   `return (foo 1)`, `c ? (foo 'a') : 2`, `[(foo 1)]`. An assignment is
   an expression, so the value form also nests where an assignment
   does: `println(x = foo 1)`. The value form is the same
   `command_chain` node with the same fields; in parentheses it is the
   child of a `parenthesized_expression`, and in a condition or an
   interpolation it is the `condition` (`value`, `lock`) itself.
   Groovy also reads `println(foo 1)` as a call of `println`, but not
   as `println` called with the chain: it is a command whose first
   argument is the parenthesized expression `(foo 1)` (Apache's
   `argumentList` after the receiver of a `commandExpression`). So that
   expression is all of the first argument and may go on:
   `println(foo 1).trim()` is `println((foo 1).trim())`,
   `bar(foo 1) + 2` is `bar((foo 1) + 2)`, `bar(foo 1, 2)` is
   `bar(foo(1, 2))` and `println(foo 1), 2` passes `2` to `println`
   (observed with Groovy 3). The grammar reads each the same way: a
   `command_chain` whose first argument holds the
   `parenthesized_expression` (item 3). That call is a command chain
   itself, so it is accepted where one is (`x.add(foo 'a')`,
   `def y = bar(foo 1)`, `"${bar(foo 1)}"`), and Groovy rejects it
   where one is not: `assertEquals(3, foo 1)`, `bar(a, foo 1)`,
   `a + bar(foo 1)`, `[bar(foo 1)]` and `return bar(foo 1)`. The last is
   `return bar` followed by `(foo 1)` here (item 2). Not accepted here,
   and taken by Groovy (observed with Groovy 3): a call whose argument
   in parentheses has such an argument itself (`println(bar(foo 1))`),
   as the scanner looks into one parenthesis only (§6.14); and more
   than one statement, a declaration, another statement that is no
   expression or a closure's parameters in an interpolation
   (`"${a; b}"`, `"${def x = 1}"`, `"${return 1}"`, `"${y -> y}"`). Each
   is an `ERROR`. The `->` of a closure without parameters is taken, the
   lazy form (`"${-> x}"`, `"${-> foo 1}"`), as a `closure_parameters`
   leaf before the value; parameters before it would cost 27 parse
   states, as the interpolation's start would hold every item of a
   closure's parameter list. A keyword Groovy reserves is no receiver of a value
   (§6.14): `"${return 1}"` is no call of `return`. Nor is a first
   argument on the line after the receiver inside parentheses, `(foo`
   followed by `1)`,
   which Groovy reads as a call because its lexer drops line breaks
   inside parentheses: the scanner tracks no brackets (§6.9), so this
   is an `ERROR`, as before. A lambda's body is accepted too
   (`x -> println x`, `(a) -> foo 1, 2`, §5.16). Apache's array initializer
   (`variableInitializers`) takes the rule too, and `AstBuilder` then
   rejects a command there. A newline is whitespace (§5.10), so a value
   form needs its first argument on the receiver's line: `def x = foo`
   followed by `1` on the next line stays two statements, as in Groovy.
   The scanner makes that decision before the receiver (§6.14).
   Keeping the chain out of general expressions removes most of
   murtaza64's nested-juxt-call conflicts.
2. **The receiver is a name, or a name followed by member accesses**,
   not any expression: `(x + y) z` is two expression statements, not a
   juxt-call on a parenthesised expression. The accesses are `.`, `?.`,
   `??.`, `*.`, `.&` and `.@`, each followed by a name, written without
   spaces (`obj.method 1`, `a?.b.c 'x'`, `System.out.println 'x'`), and
   each is the node it is in any other expression: the receiver of
   `obj.method 1` is a `field_access`. Groovy calls the last name in
   every one of these (observed with Groovy 3). A dotted receiver holds
   a scanner token: in a value it is the one before the receiver (item
   1), in a statement an empty token between the first name and the
   first access (§6.14). Either is emitted only when the first argument
   starts on the receiver's line, so `result.size` followed by `2` on
   the next line, or by `where:`, stays two statements. A statement
   whose receiver is a single name is matched in the grammar instead,
   and keeps its next-line reading (#90), unless its first argument is
   one the grammar cannot tell from a following statement (item 3):
   such a statement opens with a third scanner token, emitted before
   the name (§6.14). Not accepted, though Groovy
   takes each as a call (observed): spaces or a line break inside the
   receiver (`obj . m 1`, `obj` followed by `.m 1` on the next line;
   §6.14 lists two exceptions, `def a = foo .bar 1` among them), a
   receiver not made of names (`foo().bar 1`, `foo[0].bar 1`, `'s'.m 1`,
   `(obj).m 1`), a quoted name (`obj.'name' 1`), `::` (`obj::m 1`), and
   a value whose first name is `new`, `switch`, `true`, `false` or
   `null` (`def x = true.x 1`). Each parses as before, as the
   expression followed by the argument. A statement's receiver may start
   with a call of its first name with a closure that closes on the
   name's line, its accesses on that line or later ones
   (`foo { }.bar 1`, `foo { }` then `.bar x` on the next line, Groovy's
   `foo({ }).bar(1)`): the scanner reads the braces, the accesses and the
   first argument before the name (§6.24). Without an argument the call
   is the object of the accesses (`foo { }.bar()`), where the statement
   `foo { }` alone is a `command_chain`.
   A dotted receiver that starts a statement is a call also after
   another statement on the same line: `x.y a.b 'c'`, `foo a.b 'x'` and
   `foo 1 a.b 'c'` are the first statement followed by the call
   `a.b 'c'`. Groovy continues the first statement there instead
   (`x.y(a.b).c`, item 4). After `return` on its line the name is the
   returned value, so `return obj.m 'x'` is `return obj.m` followed by
   `'x'`, as before #52, where Groovy rejects it. The same holds for a
   name before `new`, an array type value or a command chain in
   parentheses after `return`, and for a label after `break` or
   `continue`: `return foo new Foo()` is `return foo` followed by the
   creation. On the next line the call is a statement of its own after a
   bare `return`, as Groovy reads `return` followed by `obj.m 'x'` or
   `foo new Foo()` on the next line: `return null` and the call (observed
   with Groovy 3; §6.23).
   A capitalized last name before a quoted name and `(` is the return
   type of a method, as in Groovy at script and class level:
   `java.util.List 'foo'() { }` declares `foo`, while `obj.m 'a'() { }` passes `'a'({ })` to `m`. Groovy
   decides the first by trying the parameter list, so
   `java.util.List 'foo'(1)`, a call there, is the expression followed
   by `'foo'(1)` here.
3. **Argument forms allowed**: the first argument is a literal, a
   closure, or a named arg (`k: v`). It may also be an object or array
   creation (`println new Date()`, `foo new int[3]`,
   `foo new Foo() { }`), an array type that is a value
   (`foo String[].class`, `foo int[]::new`, §6.15) or a command chain in
   parentheses (`println(foo 1)`, item 1), each starting on the
   receiver's line and each the start of a whole expression:
   `foo new Foo().bar()` passes the result of `bar()`, as Groovy does
   (observed with Groovy 3). `foo` followed by `new Foo()` on the next
   line, also with a comment opening that line, stays two statements.
   A literal first argument (a number, a quoted string, `true`, `false`
   or `null`) is the start of a whole expression too: `println "a" + b`
   passes the sum, `timeOutInMs 30 * 60 * 1000` the product,
   `foo "a".trim()` the call's result and `foo 1..3` the range, and
   `foo "a" { }` is `foo("a"({ }))`, a call of the method named `a` with
   the closure (observed with Groovy 3). On the line after the literal the
   argument goes on only where Groovy lets an expression go on across a
   line break: at an operator other than `+`, `-` and `~`, a closure's
   `{`, `as`, `in` or `instanceof` (`foo "a"` then `.trim()`, `?: b` or
   `{ }`). `foo 1` followed by `-x`, `++x`, `(x)`, `[0]`, a name or a
   literal on the next line stays two statements; Groovy's grammar lets no
   such token open a line within an expression (`additiveExprAlt` has no
   `nls` before its operator). The scanner decides it before the receiver
   (§6.14). A literal is no such argument after a comment between it and
   the receiver (`foo /* c */ "a" + b`), after the receivers `var`,
   `record`, `trait` and `sealed`, whose own keywords the scanner decides
   there, and after `super`, `permits` and `_`, keywords of the grammar:
   each is the call of the literal alone and `+b`, where Groovy passes the
   sum. Nor is a literal on the line after the receiver: `foo` followed by
   `"a" + b` is the call `foo "a"` and `+b`, where Groovy reads `foo` and
   the sum, as before.
   The first argument may also start with a name, `!` or `~`
   (`println x`, `foo !x`, `obj.m arg`, `def r = foo bar`), which is
   Apache's `argumentList` after a receiver that the name cannot
   continue. It is the whole expression as well: `println x + 1` passes
   the sum, `foo bar()` the call's result and `foo bar { }` is
   `foo(bar({ }))`, as in Groovy (observed with Groovy 3).
   Groovy tells such a call from a declaration with
   `SemanticPredicates.isInvalidLocalVariableDeclaration`, the same from
   Groovy 3 to 6: a statement is tried as a declaration first when its
   first name, or for a dotted name its last, starts with an upper-case
   letter, is a primitive type name or a modifier, or is followed by `<`
   or `[`, or when `=` follows the name after it, comments hidden. The
   scanner applies that rule before the receiver (§6.14), so `String s`,
   `int x`, `foo.Bar x`, `Été x`, `foo bar = 1` and
   `myType x /* c */ = compute()` are no calls, while `foo Bar`,
   `foo.bar baz` and `foo bar == 1` are; the first four are declarations
   without an initializer (§6.21). It tells upper case by
   `A` to `Z` and the capitals of Latin-1, Greek and Cyrillic; a capital
   of another script reads as lower case. A receiver that may name a type
   takes no name in a value either: `def x = Foo bar` is `def x = Foo`
   followed by `bar`, where Groovy calls `Foo(bar)`, so that
   `Point(int x)` and `foo(String a)` hold no call. Where a statement
   starts, `var`, `yield`, `in` and `as` take no name argument as
   receivers (`in x` is two statements, Groovy's `in(x)`, §6.16); in a
   value they do (`def x = in y`, `(in y)`). A name is no first argument
   when it is a keyword Groovy reserves (`if (c) foo else bar`), `in` or
   `as`, which go on with the receiver as operators; when `->` follows
   it, also on a later line or after comments, or a list that ends in
   `->` (`{ foo a, @A b = 1 -> }`), which makes closure parameters, the
   first of type `foo`, unless an element of the list starts with `(` or
   a key, as no closure parameter does: that `->` is a lambda's
   (`assertThrows IllegalArgumentException, () -> foo()`,
   `foo x, k: () -> b`); and when it names a method whose parentheses
   hold parameters (`foo bar(int a) { }`, `foo bar(a, String... b) { }`
   and `foo bar(int` then `a) { }` stay methods, which Groovy rejects).
   With untyped parameters or none it is a call (`foo bar(a) { }`,
   `task copy(type: Copy) { }`), as in Groovy, which takes no lowercase
   return type. An operator after the receiver goes on with it, as
   below (item 5): `foo -1`, `foo [1]`, `foo *x`, `foo (1)`,
   `foo in xs`, `foo as T`, `foo instanceof T`, `foo ? a : b`, `foo++`,
   `foo != x`, `foo !in xs`. A bare array type is a first argument as
   well (`foo String[]`, Groovy's `foo(String[])`).
   After a comma an argument is a
   named arg, any expression or a spread argument
   (`assertEquals 'v', a + b`, `foo 'a', x ? 1 : 2`, `foo 'a', *args`,
   `foo a: 1, *b`, also on the line after the comma). That is Apache's
   `argumentListElement`, whose `expressionListElement[true]` takes `*`.
   The spread takes the whole expression after it, as in parentheses:
   `foo 1, *a = foo 2` is Groovy's `foo(1, *(a = foo(2)))`. A first
   argument is never a spread: `foo *a` is a product, as in Groovy, and
   `foo *a, 1`, which Groovy calls with the spread first, is an `ERROR`.
   A spread map is taken after a comma (`foo a: 1, *:m`, §5.11) and is
   no first argument either (`foo *:m`, an `ERROR`).
   A named-arg key is an identifier; Apache also accepts string keys
   (`foo 'a': 1`), which are not implemented yet (§6.8). Disallowed:
   nested command chains, other than as the value of an assignment
   (`bar 1, x = foo 2`, item 1). Not yet accepted, though Groovy calls each
   (observed with Groovy 3): `new` after a keyword of the grammar that
   Groovy takes as a name (`super new Foo()`, `permits new Foo()`), or
   after `record`, `trait`, `sealed` or `var`, whose own keyword the
   scanner decides at the same place (§6.7, §6.13, §6.16): each is the
   name followed by the creation, and `var new Foo()` is an `ERROR`;
   `yield new Foo()` is a `yield_statement` (§6.16). Nor a
   qualified inner class creation (`foo.new Inner()`, which is
   `foo.new` followed by `Inner()`), an annotated created type
   (`foo new @A Foo()`, an `ERROR`), and an array type with a package
   before its class or with type arguments
   (`foo java.lang.String[].class`, an `ERROR`, §6.15).
   In a value (item 1), and after a dotted receiver (item 2), the first
   positional argument is any expression that starts with a literal, an
   object or array creation, an array type value, a command chain in
   parentheses or a name, as in Apache's `argumentList`:
   `def x = foo 'a' + b` and
   `log.info 'x: ' + y` pass the sum, as `println 'a' + b` does with a
   one-name receiver (above). Neither takes a closure as the
   first argument: `def c = foo { }` and `list.each { }` stay a
   `method_invocation`, which is how Apache reads a closure after a
   name. `def x = foo bar` and `a.b c` are calls (`a.b.C x = 1`
   declares `x`). After a
   value's arguments a `,` continues the call, as Apache's greedy
   argument list does: `def x = foo 1, y = 2` passes `y = 2` to `foo`
   rather than declaring `y`, and in `[x = foo a: 1, b: 2]` the `b: 2`
   is the call's named argument, not a map entry.
4. **Chain continuation**: after the first call, subsequent `name`
   tokens become property/method dispatches. The Apache grammar uses
   `commandArgument: pathElement+ argumentList?` for the chain
   continuation. It is not implemented: `foo 1 b 2` parses as the two
   chains `foo 1` and `b 2`, in a value as well, and `foo 'a' obj.m 'b'`
   as `foo 'a'` and `obj.m 'b'`. After a name argument the same holds:
   `foo a bar` is the call `foo a` and the statement `bar`, and
   `a b c d` the calls `a b` and `c d`, where Groovy reads `foo(a).bar`
   and `a(b).c(d)`. Where a first argument as the grammar takes it after
   a name (a literal, a key, a creation or an array type value), or a
   call in parentheses, follows the name on its line, the name is no
   argument and the statement reads as it did before names were
   arguments: `foo bar 1`, `foo bar baz: 1`, `foo bar new Baz()`,
   `foo bar(baz 1)` and `foo x.y 'a'` are `foo` (`foo`) followed by the
   call `bar 1` (`bar baz: 1`, …), where Groovy reads `foo(bar).1`,
   `foo.bar(baz: 1)`, `foo.bar(new Baz())`, `foo.bar(baz(1))` and
   `foo(x.y).a` (observed with Groovy 3). So is a name and `=` after the
   name: `foo bar x = 1` is `foo` and the declaration `bar x = 1`, where
   Groovy reads `foo.bar(x = 1)`; a name before parentheses that hold
   parameters or a parenthesis-free call (`foo bar baz(int a) { }` is
   `foo` and a method); a `bar sh(println` then `List)` on the next line
   (`bar` and the method `sh`, Groovy's `bar.sh(println(List))`); and any
   name after `var`, `yield`, `record`, `trait` or `sealed`, which start
   declarations and statements (`foo var x, y` is `foo` and `var x, y`,
   Groovy's `foo.var(x, y)`). In a value any name after the name argument
   is such a continuation: `def x = foo bar baz` is `def x = foo` and the
   call `bar baz`, and a field's `def r = a b c` stays a field `r` and a
   field `c` of type `b` in a class body. Taking the name would leave the
   rest a statement that is often no statement (`foo bar 1, 2`,
   `foo bar x = 1, y = 2`), or no member of a class body.
5. **Conflicts** with `binary_expression` (where `a - b` could be
   "call `a` with `-b`") are resolved by giving `binary_expression`
   higher precedence: a command chain *only* matches when the token
   after the receiver can start a first argument (item 3), with or
   without a space before it (`foo'a'`). `foo -1` is the subtraction
   `foo - 1` and `foo [1]` a subscript, as in Groovy. A name, `!` or
   `~` after the receiver is a first argument only through the scanner's
   token before the receiver (§6.14), so `foo bar` needs no conflict.

This is meaningfully more conservative than murtaza64's rule, which
attempts to allow command chains in argument lists. We trade DSL
expressiveness for parse-tree reliability — the metric tooling and
syntax-highlighter targets are better served by reliable parses on
the 95% case than ambitious parses on the 5%.

### 5.6 Optional parentheses

Groovy lets you write `println foo` for `println(foo)`. That **is** a
juxt-function call (§5.5) and is the dominant form. Method calls
*nested in expressions* require parentheses (`return f(x)` not
`return f x`), except as the whole value of a declaration or an
assignment (`def x = f 1`) and as the whole content of parentheses
(`return (f 1)`, §5.5 item 1). The grammar does not need a separate
rule.

### 5.7 GString interpolation

The double-quoted-string body alternates between literal fragments,
escape sequences, and interpolation segments. Two interpolation
forms:

- `$identifier` — followed by an immediate identifier (with optional
  dotted property access, no method call). The identifier follows
  §5.2, without `$`, so `"値は$名前です"` interpolates `名前です`, as
  in Groovy.
- `${ expression }` — full expression, recursively parsed, or a
  parenthesis-free call (`"${sh script: 'x'}"`, §5.5 item 1).

Grammar:

```js
interpolation: $ => choice(
  seq('$', alias($.identifier, $.interpolation_identifier),
        repeat(seq(token.immediate('.'), $.identifier))),
  seq('${', $.expression, '}'),
)
```

Triple-quoted strings allow interpolation only in `"""…"""` form.
Single-quoted `'…'` and `'''…'''` are *plain* — no interpolation.

Slashy strings allow GString interpolation. Dollar-slashy strings
allow interpolation with `$identifier` and `${expr}` — and treat
`$/` and `$$` as escapes.

In every string but a dollar-slashy one a `\` takes the character after
it, and a line break whole: Apache's `EscapeSequence` includes
`LineEscape`, a `\` before `\r\n`, `\n` or `\r`, so `'a\` then `b'` on
the next line is `'ab'`. In a dollar-slashy string a `\` is a character
of its own, as `$/` and `$$` are its escapes: `$/a\/$` is the string
`a\` (Groovy 3: two characters).
The single-line strings (`'…'`, `"…"`, `/…/`) take `\r\n` after the
`\` as one escape, in the grammar's text tokens and in the scanner's
look-aheads through strings, so a file with CRLF line endings gives
the tree of the same file with LF; before, the `\n` after the escaped
`\r` ended the string, an `ERROR`.

The lexer state machine for interpolation lives in the external
scanner (§6.3). This avoids tree-sitter's "ambiguity over the entire
file" trap when a stray `"` mid-source could otherwise re-anchor the
string lexer.

**Triple-quoted edge cases** (from murtaza64's `string.test`):

1. **Empty triple-quoted string**: `""""""` is valid — opener `"""`,
   empty body, closer `"""`. The scanner must not greedily consume
   five quotes as `"""""`+`"` looking for a longer match.
2. **Embedded one-or-two quotes before interpolation**:
   `print("""hello world ""${0}""")` — the body contains literal
   `""` immediately before `${0}`, then literal `""` before the
   closing `"""`. The scanner emits body fragments that may
   *contain* one or two consecutive `"` characters, only stopping
   on the third consecutive `"` (or on `$` introducing interpolation).
3. **Embedded `"` inside body**: `"""this has "quotes" inside"""`.
4. **Backslash before interpolation in triple-quoted**:
   `"""text \$ ${expr}"""` — the `\$` escapes the dollar, the
   subsequent `${expr}` is normal interpolation.

The scanner contract for `"""` bodies:

```text
Consumes characters until:
  - three consecutive unescaped `"` → emit BODY (without the closing
    `"""`), return; the grammar then matches the literal `"""`.
  - `$` followed by `{` or identifier-start → emit BODY, return for
    interpolation handling.
  - `\` consumes the next character as escape.
  - one or two consecutive `"` followed by anything other than `"`
    are included in the body fragment, not treated as a close.
```

The implementation is in-grammar (`_triple_gstring_text`). One or two
quotes join the text fragment when an ordinary character or an escape
follows them. Directly before a `$` each quote is a `string_fragment`
of its own, so that the `$` is left for the interpolation:
`"""say "$name"""` holds two fragments, the text before the quote and
the quote itself, and then the interpolation.

**Dollar-slashy edge cases** (from murtaza64's `string.test`):

```groovy
x = $/
hello
world
$$ dollar          // $$ is literal $
/                  // bare slash is literal
\                  // bare backslash is literal
$interp            // GString interpolation
$dotted.interp     // dotted property interpolation
$/ escaped slash   // $/ is literal /
/$
```

Escape rules:

- `$$` → literal `$`.
- `$/` → literal `/`.
- `\` (anywhere except before the closer) → literal `\`.
- `$identifier` → interpolation.
- `${expr}` → interpolation.
- `/$` (only when followed by no further content within the literal)
  → terminator.
- `/` directly before the terminator is text: `$/a//$` holds `a/`. The
  text token cannot look ahead, so it ends before a run of slashes
  that reaches `$`. Each slash of that run except the last is then a
  `string_fragment` of its own, and the last one starts the
  terminator.
- `$` directly before the terminator, as in `$/a$/$`, is not
  supported. When a later `/$` exists in the file, Groovy runs the
  string on to it, and the text holds `a/$` and what follows. When
  none exists, Groovy ends the string there and the text is `a$`. This
  grammar always reads `$/` as the escaped slash, so it matches the
  first case and the second is an `ERROR`. Write `$$` for the dollar:
  `$/a$$/$`.

`$` starts a name, and the external scanner's look-aheads (§6) take a
lone `$` before `/` for the start of a dollar-slashy string, as the
in-grammar lexer does wherever a string may start: one that read the
name `$` went on at the `/` and took the body of `$/*.txt/$` for a
block comment, read to the next `*/` or to the end of the input
(§6.6). A longer
name that ends in `$` is a name (`x = a$/b` divides), as in Groovy's
lexer. What a string decides is decided at the `$`: a dollar-slashy
string is a first argument after any receiver, as a quoted one is
(§6.14), and no lambda, typed declaration, `record` or `trait` header,
cast type arguments or multi-assignment starts there. Where a
look-ahead reads Groovy's tokens one by one (closure parameters,
§6.14; a lambda's defaults and annotations, §6.17), the string is one
item, read across lines to its `/$` with `$$` and `$/` escaped and an
interpolation's braces counted, for at most
DOLLAR_SLASHY_LOOKAHEAD_CHARACTERS (1,024) characters; a longer string,
or one that does not close, ends that look-ahead. Groovy's lexer
starts the string only where a `/$` closes it and reads the name `$`
otherwise, so `x = $/* c */ -> 1`, its lambda with the parameter `$`,
is a dollar-slashy string with a `MISSING` `/$` here, as at v0.6.0.

### 5.8 Closure parameters and the `it` shorthand

`{ a, b -> … }` declares parameters; `{ … }` with no `->` implicitly
binds `it`. A parameter has the shape of a method parameter, as in
Apache's grammar (`formalParameterList`): annotations, `final`, then
`def`, `var` or a type (varargs included), the name, and a default
(`{ String sep = ',', def xs -> … }`). `grammar.js` builds
`formal_parameter` and `closure_parameter` from one helper,
`parameter()`. `def` and `var` are keywords there, so such a parameter
has no `type`. Apache's `variableModifiersOpt` allows more than this
order; `docs/divergences-from-spec.md` §14 lists what is not accepted.
Until the `->`, the parameter list and a body that opens with a
declaration or an assignment (`{ a = 1 }`) parse side by side. The
grammar rule:

```js
closure: $ => seq('{',
  optional($.closure_parameters),
  repeat(choice($._statement, ';')),
  '}'),

closure_parameters: $ => seq(
  optional(seq($.closure_parameter, repeat(seq(',', $.closure_parameter)))),
  '->'),
```

We do **not** emit a synthetic `it` parameter. Downstream tools that
care can match `closure` nodes with no parameter list.

### 5.9 Annotations

`@Foo`, `@Foo(arg)`, `@Foo(k1=v1, k2=v2)`, `@a.b.Foo(…)`, `@Foo @Bar`
(stacked). Annotation `@interface` declaration (Java annotation type
syntax) is a separate class-declaration form (§4), whose elements may
take a `default` value.

An argument, the value of a `k=v` pair, and an element's `default` are
Apache's `elementValue`: an expression, an annotation, or a
`list_literal` of annotations, so `@Foo(@Bar)`, `@Foo(k = @Bar(1))`
and `@Foo([@Bar, @Baz])` nest to any depth. The argument list is an
`argument_list`; a pair is an `assignment_expression` whose `left` is
an `identifier`, the shape `k=v` has in a call. An annotation is a
value only there: `foo(@Bar)` and `[@Bar]` elsewhere are errors.

### 5.10 Statement terminators

Groovy treats both newline and `;` as statement terminators *most of
the time*. The grammar handles the two cases differently, both
in-grammar (no external `_terminator` token — see §6 on why dormant
scanner branches are avoided):

- **Newline** is whitespace (`/\s/` is in `extras`), so statements are
  delimited purely structurally: a statement list is `repeat(...)` and
  two adjacent complete statements (`x()` then `y()`) need no explicit
  separator. Inside `(`/`[`/`{` a newline is likewise just whitespace,
  matching Groovy's rule.
- **Line escape**: a `\` directly before a line break (`\n` or `\r\n`)
  is whitespace, as in Apache's lexer (`WS : ([ \t]+ | LineEscape+)`).
  It is a second pattern in `extras`, and the scanner's same-line
  tokens (§6.7–§6.9, §6.18) skip it as they skip spaces and tabs, so
  the next line continues the current one: `x \` followed by `[0]` is
  a subscript, and `foo \` followed by `a: 1` is a named-argument call.
  A `\` anywhere else outside a string is an `ERROR`. Two cases differ
  from Apache. A `\` before a lone `\r` is an `ERROR` when more input
  follows the `\r`. At the very end of the file a `\`, alone or with a
  `\r` after it, is skipped with no `ERROR`, because the lexer has
  already taken it as the start of the pattern.
- **A `(` that opens a line** after an expression opens the next
  statement where one may start, as Apache's `arguments` takes no line
  break before it: `foo` followed by `(1)` is two statements, and so is
  `foo()` followed by `(a, b) = [1, 2]`. The scanner emits such a `(` as
  a token of its own, which a call's arguments do not take (§6.18).
  Inside parentheses and brackets, where no statement may start, the
  `(` still calls, as Groovy's lexer drops line breaks there. Every
  other token on the next line binds as it would on the line: a `{` is
  a trailing closure, as in Groovy, and an operator continues the
  expression, also `-`, `+`, `++`, `--`, `**` and `?[`, where Groovy
  ends the statement (`docs/divergences-from-spec.md` §3). After the `}`
  of a lambda's brace body and of a block that opens a statement the
  scanner decides as Groovy does instead (§6.19, §6.22): there `(`, `+`,
  `-`, `{`, `[` and a slashy string on the next line start a statement.
- **A line break after `return`, `break` or `continue`** ends the
  statement, as Groovy takes their value or label only on the keyword's
  line: `return` followed by `foo()` on the next line is a bare `return`
  and the call. The scanner emits an empty token there (§6.23).
- **Semicolon** `;` is an optional, anonymous element of every
  statement list (`repeat(choice($._statement, statementBlock($), ';'))`,
  `statementList` in `grammar.js`) and member list
  (`repeat(choice($._class_member, $.block, ';'))` in `class_body`
  and `enum_body`, where the `block` is an instance initializer). It
  never appears as a named
  node in the AST, so wherever it attaches the named-node tree is
  identical. This covers terminators (`x();`), separators (`x(); y()`),
  empty statements (`;`, `;;`), and empty / terminated members
  (`class C { ; }`, `void f();`, `int x = 1;`). The list contexts are
  `source_file`, `block`, `closure`, the classic `switch`
  `case`/`default` bodies, and the two member bodies, `class_body`
  and `enum_body`.
- Three single-statement (non-list) bodies take an explicit
  `optional(';')` so their terminator is not claimed by the *enclosing*
  list or rejected before a required follow-on token: the braceless
  `if` consequence (without it the `;` in `if (x) a(); else b()` would
  be eaten by the outer list and the `else` severed into a stray
  declaration), the braceless `do`-`while` body (`do a(); while (c)`,
  where `while` must follow), and the `switch` arrow-case body
  (`case 1 -> foo();`, matching the Java/Groovy `case L -> expr;`
  form). For `if`, the `;` sits *after* the consequence and *before*
  the optional `else`, so the no-`else` form (`if (x) a(); b()`) does
  not error.

An earlier draft proposed a `_terminator` external scanner token
matching `;`-or-meaningful-newline; it was never wired, because the
in-grammar split above is simpler and avoids a dormant scanner branch
(§6). amaanq's grammar uses `DELIMITER = choice(';', /\n/, '\0')`
literally in every statement rule — this works for tree-sitter but
produces ambiguity at line continuations (`a +\n b` would split into
two statements), which the structural-newline approach sidesteps.

### 5.11 Spread in three positions

| Position | Symbol | Production |
|---|---|---|
| Argument list | `*expr` | `spread_arguments` |
| List literal element | `*expr` | `spread_arguments` |
| Subscript index | `*expr` | `spread_arguments` |
| Member access | `recv*.prop` / `recv*.method(args)` | `spread_dot_expression` |
| Map literal entry | `*: expr` | `spread_map_entry` |
| Argument list | `*: expr` | `spread_map_entry` |

A list literal takes a spread element anywhere, as Apache's
`list: LBRACK expressionList[true]? COMMA? RBRACK` does (`[*a, 1, *b]`),
a subscript takes several indices, any of them a spread, as
`indexPropertyArgs: (SAFE_INDEX | LBRACK) expressionList[true]? RBRACK`
does (`y[1, *a]`, `x[*a]`, `y?[1, 2]`; in 4.0.x to master, and Groovy 3,
whose rule is `QUESTION? LBRACK expressionList[true]? RBRACK`, reads
them alike), and an argument list takes a spread map anywhere among its arguments,
as Apache's `namedArg` and `namedPropertyArg` have the alternative
`MUL COLON nls expression` (`foo(*:m, x: 1)`, `foo(a: 1, *:m)`, and after
a comma in a parenthesis-free call, `foo a: 1, *:m`). Neither is the
first argument of a parenthesis-free call (`foo *a, 1`, `foo *:m`, calls
in Groovy): `foo *a` is a multiplication, so taking `*a` as the call's
first argument would need the scanner to find the `,` past it.

murtaza64 collapses these under `access_op` which mis-shapes the
spread-args case (`*x` in a call) and the spread-map case (`*:`).
We split them.

Apache `GroovyParser.g4` rule `argumentList` confirms the split:
`expressionListElement: MUL? expression` — the optional asterisk
at expression-element start is the spread, not a multiplication
operator. The tree-sitter rule mirrors this with an explicit
alternative for `seq('*', $.expression)` inside `argument_list`.

### 5.12 `def` in different positions

`def` can prefix:

- a local variable declaration (`def x = …`), also before a type
  (`def String s = …`, §6.21),
- a field (`def x` in a class or enum body), also before a type
  (`private def Foo foo`, §6.21),
- a method definition (`def f() { … }`),
- a method or closure parameter (`def f(def x)`, `{ def x -> … }`),
- a `for`-in variable, a `catch` parameter and a `try` resource
  (`for (def x in xs)`, `catch (def e)`, `try (def r = open())`),
- a multi-assignment (`def (a, b) = …`).

The grammar uses `def` as a token in each rule rather than as a
shared `_optional_def` non-terminal, because the conflicts each rule
allows are different (a method definition allows a return-type-like
slot before `def`; a variable declaration does not).

### 5.13 Trailing commas

Lists, maps, argument lists and enum constants accept an optional
trailing comma. Parameter lists do not, as in Apache's
`formalParameterList`: `def m(a, b,) { }` is an error. Each list rule
spells the comma handling out; there is no shared helper.

### 5.14 Method reference `::`

Groovy 3+: `String::length`, `obj::method`, `Foo::new`, and on an
array type `String[]::new`, whose `target` is the `array_type` of
§6.15. Grammar:

```js
method_reference_expression: $ => prec.left(PREC.ACCESS, seq(
  field('target', choice($.identifier, $._type, $.expression)),
  '::',
  field('name', choice($.identifier, 'new'))))
```

Conflicts with C++ `::`-scoped names are not relevant in Groovy.

### 5.15 Parenthesized type cast

`(String) x` — C-style cast. Distinct from `x as String`. Grammar:

```js
parenthesized_type_cast: $ => choice(
  weighted(CAST_WEIGHT, prec.right(PREC.CAST, seq(
    '(', $._type, alias($._cast_close, ')'), $._expression))),
  weighted(-CAST_WEIGHT, prec.right(PREC.CAST, seq(
    '(', $._type, ')', $._expression))))
```

Ambiguity with parenthesized expression: `(a) b` could be a cast
(if `a` is a type) or a juxt-call. Resolved by precedence and
GLR: if `a` matches `_type`, prefer the cast; otherwise it's a
parenthesized expression followed by a juxt-call (which is a
syntax error in expression position, by §5.5).

Three rules settle it, the first and the last from Apache's
`castParExpression castOperandExpression`, the second from Groovy's
lexer and from Groovy 6.0.0's reading of a cast of `as` or `in`:

- On one line the cast wins wherever it parses, as the cast
  alternative comes first in Apache's `expression`: `(a) + b`,
  `(a) [0]` and `(a) (b)` are casts, `(a) * b` and `(a).size()` are
  not. The first alternative carries this: it closes with the scanner
  token `CAST_CLOSE` (§6.11), a `)` with something after it on its
  line, and has a positive dynamic precedence. This is the reading of
  Apache's grammar and of Groovy 3 to 5. Groovy 6.0.0 keeps the
  grammar and rewrites the tree afterwards (GROOVY-10355,
  `AstBuilder.visitCastExprAlt`): where the parentheses hold a bare
  name whose last segment starts with no upper-case letter, `(a) + b`
  and `(a) - b` become the sum and the difference, and `(A) - b` stays
  the cast. That rewrite is not followed here: the scanner decides at
  the `)`, where it no longer sees the name
  (`docs/divergences-from-spec.md` §15).
- A relational operator that Groovy applies to the parentheses ends
  the cast: `instanceof`, `!instanceof` and `!in` are operators
  wherever Groovy's lexer meets them, and `as` and `in` with what they
  take on the line are the operators from Groovy 6.0.0 on
  (GROOVY-10355), so `(x) instanceof Foo`, `(x) !in y`, `(x) as Foo`
  and `(x) in y` are no cast of a name. The `)` there is the scanner
  token RELATIONAL_CLOSE (§6.11), which only a parenthesized
  expression takes.
- The rule has no `nls` between its two parts, so where a line break
  can end a statement the operand is on the line of the `)`: `x = (a)`
  followed by `foo()` on the next line is two statements. The second
  alternative carries this: a cast whose operand is on a later line
  has a negative dynamic precedence and loses to any other complete
  reading. Inside `(…)` and `[…]` Groovy ignores line breaks and reads
  a cast. Here the cast stands there when no other reading parses
  (`foo((String)` followed by `x)`), and loses when the later line can
  also continue the expression (`foo((a)` followed by `- b)` is a
  subtraction); see `docs/divergences-from-spec.md` §15. Where the
  later line opens with `(` and no bracket holds both lines, the
  scanner ends that reading at the `)` unless the type is an array or
  generic type, which no expression is (§6.11).

Without the two weights the readings were unranked and the result
depended on the surrounding code.

### 5.16 Lambda expressions

Groovy 3 added Java's lambdas next to closures. Apache's grammar
(`GroovyParser.g4`, the same rules from 4.0.x to 6.0.x) has two forms:

- `lambdaExpression`, with parameters in parentheses
  (`formalParameters`), is a `primary`: an expression wherever one is.
  `(a, b) -> a + b`, `() -> 42`, `(int a, String b = 'x') -> { … }`
  are operands (`1 + (a) -> a * 2`), list elements, the value of
  `return`, `throw` or `assert`, ternary branches, a cast's operand
  (`(Runnable) () -> { }`), statements and lambda bodies
  (`(a) -> (b) -> a + b`).
- `standardLambdaExpression` also takes one name with no parentheses
  (`x -> x + 1`), and Apache lists it only in
  `enhancedStatementExpression` (the value of a declarator, of an
  assignment other than `(a, b) = …`, and what is in parentheses,
  conditions included), `enhancedArgumentListElement` (an argument in
  parentheses) and `enhancedExpression` (the value of a map entry or a
  named argument, Groovy 5 and later). The comment there gives the
  reason: `{ a -> a * 2 }` must stay a closure. So `return x -> x`,
  `[x -> x]`, `c ? x -> x : y`, `foo 1, x -> x` and `a -> b -> c` are
  errors in Groovy (observed with Groovy 3) and here.

`lambdaBody` is a `block` or a `statementExpression`: an expression or
a parenthesis-free call (`x -> println x`), taking all it can, so
`(a) -> a + 1 as String` casts the sum. A `{` opens a block wherever
one parses, as Apache tries the block first, and a closure only where
no block does (`x -> { y -> y }`). `x -> { a = b -> c }` is a block
that assigns a lambda, not a closure with the parameter `a`. A line
break may come before and after the `->` (`NL* ARROW NL*`).

The braces are the whole body only where nothing after the `}`
continues an expression; otherwise they are a closure that starts the
body, which takes what follows: `x -> { y -> y }()`, `x -> { y -> y }[0]`,
`x -> { y -> y } { z }`, `x -> { } + 1` and `(x) -> { y -> y }.call()`
call, subscript, pass a closure to and add to the closure (observed
with Groovy 3). A lambda in parentheses is a `primary`, so after a block
the lambda itself goes on: `(x) -> { foo() }.call()` calls the lambda.
A lambda without parentheses cannot, so there a block that something
continues is a closure: `x -> { foo() }.call()` calls the closure. After
a line break where a statement may start, `.`, `?.`, `::`, `*`, `?:` and
the other operators Apache takes after `nls` continue the body, and `(`,
`[`, `{`, `+`, `-`, type parameters and a slashy string start the next
statement: `x -> { foo() }` followed by `+ 1` on the next line is a
lambda and the statement `+1`. In a class body `{` and type parameters
on the next line start an initializer and a generic method. Inside
parentheses and brackets Groovy's lexer drops the line break, so
`foo(x -> { y -> y }` followed by `+ 1)` adds. An interpolation holds
one expression here, so there an operator on the next line continues the
body too: `"${(a) -> { y -> y }` followed by `(1)}"` calls the closure,
where Groovy, whose interpolation in braces is a closure, reads the
lambda and the statement `(1)`. The scanner decides it after the `}`
(`LAMBDA_BODY_END`, §6.19).

The node is `lambda_expression` with the fields `parameters` and
`body`, as in tree-sitter-java. The parameters in parentheses reuse
`formal_parameters`, whose `formal_parameter` already takes Groovy's
untyped names, defaults, annotations, `final`, `def`, `var` and
varargs; tree-sitter-java's separate `inferred_parameters` would only
duplicate it. The one name without parentheses is the `identifier`
itself. `locals.scm` makes the lambda a scope and its parameters
definitions, and `highlights.scm` highlights the name as a parameter,
as for a closure.

Both forms open with an empty scanner token, `_lambda_start` before
the `(` and `_bare_lambda_start` before the name, emitted only when
`->` follows (§6.17). The parser cannot wait for the `->`: `(a)` is
also a parenthesized expression, a cast and a multi-assignment's
start, and `x` in an argument or a value is also an expression, so a
decision there would fork the parse at every `(identifier)` or cost
every such name a reduction. The tokens are valid only where the
grammar takes each form, so the scanner reads names only in those
positions.

Where an expression may be followed by `->`, the end of a closure
parameter's default and of a case value, the same tokens can also be
the default or the value. Groovy's choice depends on what the `{` may
open:

- Where only a closure can stand, a value or an argument
  (`def c = { a = (b) -> c }`, `foo { a = b -> c }`), Groovy tries the
  closure's parameters first: the closure has the parameter `a`.
  `case (A) -> 1` is likewise a case with the value `(A)` (observed
  with Groovy 3 for the closures; Groovy 3 has no arrow cases).
- Where a block can stand too, the body of `if`, `else`, a loop, an
  arrow case and a lambda, Groovy tries the block first (Apache
  `statement` and `lambdaBody`): `if (x) { a = b -> c }`,
  `while (x) { String a = (b) -> c }` and `f(x -> { a = b -> c })` are
  blocks that hold the lambda (observed with Groovy 3).

The grammar keeps all readings and ranks them with dynamic precedences.
A parenthesized expression after `_lambda_start`
(`_lambda_parenthesized_expression`) and a closure parameter's default
that is a name after `_bare_lambda_start` (`_bare_lambda_name`) keep the
closure's reading; the lambda's negative weight (`LAMBDA_WEIGHT`) ranks
a lambda below it. A lambda whose whole body is a closure ranks below
a hundred lambdas (`DEMOTED_LAMBDA_WEIGHT`), one whose whole body is a
parenthesized expression one step below a lambda
(`PARENTHESIZED_BODY_WEIGHT`), and a block in the body of `if`, `else`,
a loop or an arrow case outweighs a lambda whose body is a closure
(`BODY_BLOCK_WEIGHT`). The consequence of `if`, that block
(`_body_block`) or a statement, is a rule of its own
(`_if_consequence`), so that its two readings, a block and a closure,
reduce to one symbol at its `}`, where they merge and the weight
decides; the other bodies end their statement, which is reduced there.
Inline in `if_statement` they were ranked only where the statement was
reduced, at the end of an `else if` chain: every link kept both
readings alive, a chain that ended the input took time far above
linear in its length, and in a method a chain whose blocks each start
with the assignment of a lambda lost a link's block to the closure from
twelve links on, as the runtime keeps a few stack versions only
(`bindings/rust/tests/if_chain_cost.rs`). The rule costs every `if` a
reduction, about 1% more instructions on code made of `if` statements.
A block where Groovy tries a block first and that
ends what it opens, a lambda's brace body and a `{` that opens a
statement (§6.22), outweighs the lambda in it by one step
(`STATEMENT_BLOCK_WEIGHT`), and a lambda's block adds that step to the
lambda's weight. The closure reading makes the first assignment in the
braces a parameter, so it holds a lambda less and loses that step, and
the blocks win for nests of any depth, each in the block of the one
before: `if (x) { a = b -> { p -> p } }`,
`if (x) { a = b -> { c = d -> e } }`, `f(x -> { a = b -> { c = d -> e } })`,
`x -> { a = b -> { p -> p } }` and `{ a = b -> { c = d -> e } }` that
opens a statement are blocks, as in Groovy. A `{` that opens a statement
and holds a lambda whose body is a closure, `{ a = b -> { p -> p } }`,
also after a label and in a colon case, is the closure with the parameter
`a`, where Groovy reads a block that holds the lambda: the block's one
step does not make up the demoted lambda's weight, and a block weight that
did would add up over the blocks of a file. The step is on every block
rather than on the outermost one: tree-sitter keeps a few stack versions
only, those of the highest weight so far, and a weight that only the last
`}` of a nest added let a reading of closures that had gained the weight
of a statement block early push the blocks' reading out at ten lambdas.

Because the step is on every block, a reading gains one for each line of
braces it takes for blocks where another takes them for closures that a
call takes: `def c = { a = b -> { p -> p }` followed by lines of `{ z }`
is the parameter `a` and a closure called with each `{ z }` in Groovy, and
also a lambda whose body is a closure and a block per line. The demoted
lambda and the body block weigh enough that such lines change no reading
below 75 of them (`LAMBDA_NESTING_DEPTH` is 100; it was 10, and eight lines
were enough). Of 5,600 inputs, 40 forms of such a lambda in seven contexts
followed by 1 to 20 lines of braces, of calls with a closure, of closures
with parameters or of braces after `;`, Groovy 3 reads 2,673 as the
grammar now does; 116 that the grammar before blocks read as Groovy does
read otherwise with the depth at 10, and 12 at 100, all the next case.
Where the readings differ by a step only, two lines do:
`def c = { a = (b) -> (d) -> e` followed by `{ z }` twice is the parameter
with the default `(b) -> (d)`, the call `e { z }` and a block, where Groovy
reads the parameter `a = (b)` and the lambda `(d) -> e` called with both
closures, and `{ a = b -> c` followed by `{ z }` twice and `}` is the
closure with the parameter `a = b`, the call `c { z }` and a block, where
Groovy reads a block. Both come from a call of a name with a closure,
`e { z }` and `c { z }`, which takes no closure after its first: Groovy
reads `foo { a }` followed by `{ b }` as `foo({ a }, { b })`, and the
grammar as the call and a block (`docs/divergences-from-spec.md` §28).

`{ a = (b) -> (d) -> e }` keeps the parameter `a = (b)` and the body
`(d) -> e`, as Groovy, which tries the parenthesized expression first at
each `(`, reads it. Where the parentheses before the first `->` hold
more than an expression, Groovy's default is a lambda that ends at the
parenthesized expression, and so it is here:
`{ T a = (b, c) -> (d) -> e }` and `{ T a = (U) (b, c) -> (d) -> e }`
have the parameter `a` with the default `(b, c) -> (d)`, cast in the
second, and the body `e`. In the second the weights tie, as the cast's
step makes up the parenthesized body's, and tree-sitter's last resort
keeps that reading. A `{` that opens a statement, a labeled statement
or a colon case's statement is a block (§6.22), so there
`{ a = b -> c }` is a block that assigns a lambda, as in Groovy, and in
`def c = { a = (b) -> { } }` the closure has the parameter `a` and a
block, which outweighs a closure that assigns `a` a lambda with a block
by the lambda's weight. An assignment in a closure parameter's default
or a case value (`{ p = a = b -> c }`, `case x += y -> z`) and `var`
called in parentheses there (`{ a = (var b) -> c }`) are read as lambdas
instead; see `docs/divergences-from-spec.md` §26.

Read differently from Groovy (observed with Groovy 3): a lambda without
parentheses as an assignment's value, which Groovy's assignment is
continued past (`(v = a -> { foo() }) == 1`): here the closure takes the
operator, `v = a -> ({ foo() } == 1)`, as in a declaration. A lambda in
parentheses whose block starts with what is also a closure's parameter
(`(a) -> { b = (c) -> { } }.call()`,
`(x) -> { def g = y -> y }.call()`), which Groovy reads as a block and
continues the lambda: the operator makes it the closure with the
parameter `b` and takes the closure, as a closure reading holds a lambda
less than the block's, and a weight on the lambda's block that outranked
it would also make `def c = { a = (b) -> { }.call() }` assign a lambda;
the weight a lambda's block has is the one of a block that ends it, which
a block that something continues lacks. After a line break, `{`, `as`
and `in` continue a closure body in Groovy and start a statement after a
block there; here `{` ends the body and `as` and `in` continue it. After
a line break `+` and `-` start a statement in Groovy, also after a lambda
in parentheses, which here they apply to, as to any expression before a
line break. The weight of a block in
the body of `if`, `else`, a loop or an arrow case does not depend on
what follows its `}`, so `if (x) { a = b -> { p -> p } } - 1`, which
Groovy reads as the closure with the parameter `a` minus one, as no
block takes the `-`, is a block and the statement `-1`. With three
arrows, `{ T a = (U) (b, c) -> (d) -> (e) -> f }`, the default
`(b, c) -> (d)` before the body `(e) -> f`, Groovy's reading, weighs as
much as the default `(b, c) -> (d) -> (e)` before the body `f`, and
tree-sitter's last resort keeps the second. `{ a = (String b) -> (d) -> e }` has the
default `(String b) -> (d)` and the body `e`: Groovy's default is
`(String b)`, a call of `String` in parentheses, which the grammar does
not read (§6.14).

Not accepted, though Groovy takes each (observed with Groovy 3): a
lambda as the parenthesis-free first argument of a call or appended to
one, `foo (x) -> x`, `list.each (x) -> println x`, also `foo()`
followed by `(a) -> a` on the next line, which are the call `foo(x)`
followed by an `ERROR`; a lambda without parentheses as a named
argument of a parenthesis-free call, `foo a: x -> x` (Groovy 5 and
later); and a parameter whose type is a lowercase name,
`(foo a) -> a`, where the scanner's §6.14 rules read `foo a` as a call.
Each is an `ERROR`, as before lambdas.

---

## 6. External scanner design

The external scanner is the C file `src/scanner.c`. It exists to
handle the context-sensitive lexing situations that pure tree-sitter
grammar rules cannot disambiguate. Each token below has a single
emit/no-emit decision per call. The implementation exports, in the order of grammar.js's `externals`
array and of `TokenType` in §6.1: the slashy opener (§6.2); the three
comment tokens (§6.6); the contextual `record` (§6.7), `trait` and
`sealed` (§6.13) keywords; the contextual `var` before a declared name,
`var` alone before a multi-assignment's names, `yield`, `in` and `as`
(§6.16); the named-argument key of a parenthesis-free call (§6.8) and
the key that a contextual word makes (§6.16); the start of such a call
that is a value, of a statement's dotted receiver, and of a statement
whose first argument is a `new`, an array type, a name, such a call in
parentheses or a literal that the argument goes on past (§6.14); the
same-line subscript opener (§6.9); the start of an array type that is a
value (§6.15); the `)` of a cast with something after it on its line
(§6.11); the `!in` operator (§6.12); the start of a lambda, before its
parameters in parentheses and before its one name (§6.17); the empty
marker of an unterminated block comment (§6.6); `STRING_BODY` (§6.10)
and `TYPE_START` (§6.15), never emitted, which mark a string body and
where a type may start; a `(` that opens a line after an expression
where it may open a statement (§6.18); the end of a lambda's brace body
(§6.19); `LAMBDA_BODY_REREAD` (§6.19, §6.22), which no rule references
and which makes the parser lex the `}` of a lambda's body or a
statement's block again; the first comma of a multi-assignment (§6.20);
the start of a typed local declaration (§6.21); the end of a block that
opens a statement (§6.22); the `)` before a relational operator that
Groovy applies to the parentheses (§6.11); the `<` of a cast type's
type arguments where they close (§3.2.2); `CLOSURE_PARAMETERS_START`
(§6.21), `TYPE_ONLY_CLOSE` (§6.11) and `BODY_BLOCK_END` (§6.17), never
emitted, which mark where closure parameters may start, the `)` of a
cast of an array or generic type whose operand is on a later line, and
the end of the block that is the body of `if`, `else`, a loop or an
arrow case; the end of a `return`, `break` or `continue` at a line
break (§6.23); the start of a statement whose first name a closure on
its line and a member access follow, and of such a call as the start of
a parenthesis-free call's receiver (§6.24); `UNREACHABLE` (§5.2), never
emitted, which ends the error production for a `case` label outside a
switch, so that error recovery inserts it as a missing `switch`; the
`<` of a generic type's arguments where they close or run past the
read (§3.2.2); the `)` of a cast before a name on its line (§6.11); and
`ERROR_SENTINEL`, never emitted, which no rule references and which
only signals error recovery. The tokens listed after the enum in §6.1
are met in-grammar, see `docs/divergences-from-spec.md`.

### 6.1 Tokens exported

```c
enum TokenType {
  SLASHY_STRING_START,    // §5.4 — opening `/` of slashy regex
  LINE_COMMENT,           // §6.6 — `//` to end of line, single token
  BLOCK_COMMENT,          // single-token block comment incl. /**/ and /***/
  GROOVYDOC_COMMENT,      // single-token /** ... */ comment
  RECORD_KEYWORD,         // §6.7 — `record` only at the start of a record header
  TRAIT_KEYWORD,          // §6.13 — `trait` only at the start of a trait header
  SEALED_KEYWORD,         // §6.13 — `sealed` only before `class`, `interface` or `trait`
  VAR_KEYWORD,            // §6.16 — `var` before a declared name, or before `(` after `final`
  VAR_NAMES_KEYWORD,      // §6.16 — `var` alone before the `(…) =` of a multi-assignment
  YIELD_KEYWORD,          // §6.16 — `yield` before an expression
  IN_KEYWORD,             // §6.16 — `in` before an expression
  AS_KEYWORD,             // §6.16 — `as` before a type
  COMMAND_NAMED_KEY,      // §5.5 — empty; before the `name` of `name:` in a parenthesis-free call
  CONTEXTUAL_KEY,         // §6.16 — `var`, `yield`, `in` or `as` as such a key, the word itself
  COMMAND_VALUE_START,    // §6.14 — empty; before the receiver of such a call that is a value
  COMMAND_PATH_START,     // §6.14 — empty; after the first name of such a call's dotted receiver in a statement
  COMMAND_STATEMENT_START, // §6.14 — empty; before a statement's one-name receiver whose first argument is a `new`, an array type, a name, such a call in parentheses or a literal the argument goes on past
  SUBSCRIPT_OPEN,         // §6.9 — `[` on the same line as the token before it
  ARRAY_VALUE_START,      // §6.15 — empty; before an array type that is a value
  CAST_CLOSE,             // §6.11 — `)` with something after it on its line
  NOT_IN,                 // §6.12 — `!in` before whitespace or an opening bracket
  LAMBDA_START,           // §6.17 — empty; before the `(` of a lambda's parameters
  BARE_LAMBDA_START,      // §6.17 — empty; before a lambda's one name without parentheses
  UNTERMINATED_COMMENT,   // §6.6 — empty; at the first `/*` with no `*/` after it
  STRING_BODY,            // §6.10 — never emitted; valid only inside a string body
  TYPE_START,             // §6.15 — never emitted; valid only where a type may start
  STATEMENT_PAREN,        // §6.18 — `(` that opens a line after an expression and opens a statement
  LAMBDA_BODY_END,        // §6.19 — empty; after the `}` of a lambda's brace body that nothing continues
  LAMBDA_BODY_REREAD,     // §6.19 — empty, in no rule; after a block shifted whole that something continues
  MULTI_ASSIGNMENT_COMMA, // §6.20 — the first `,` of `(a, b) = …`, unless the parentheses go on as a method's parameters
  DECLARATION_START,      // §6.21 — empty; before the type of a typed local declaration (`String s`, `int a, b`, `String s = 'a'`)
  STATEMENT_BLOCK_END,    // §6.22 — empty; after the `}` of a block that opens a statement and that nothing continues
  RELATIONAL_CLOSE,       // §6.11 — `)` before a relational operator Groovy applies to the parentheses; closes no cast
  CAST_TYPE_ARGUMENTS_OPEN, // §3.2.2 — `<` after a cast type's name, where the type arguments close
  CLOSURE_PARAMETERS_START, // §6.21 — never emitted; valid only where closure parameters may start
  TYPE_ONLY_CLOSE,        // §6.11 — never emitted; valid only before the `)` of a cast of an array or generic type whose operand is on a later line
  BODY_BLOCK_END,         // §6.17 — never emitted; valid only after the `}` of the block that is the body of `if`, `else`, a loop or an arrow case
  JUMP_STATEMENT_END,     // §6.23 — empty; after `return`, `break` or `continue` where a line break follows on the keyword's line
  CLOSURE_CALL_START,     // §6.24 — empty; before a statement's name whose closure closes on its line and a member access follows
  CLOSURE_COMMAND_START,  // §6.24 — empty; the same where a first argument follows the accesses (`foo { }.bar 1`)
  UNREACHABLE,            // §5.2 — never emitted; ends the error production for a `case` label outside a switch
  TYPE_ARGUMENTS_OPEN,    // §3.2.2 — `<` after a generic type's name, where the type arguments close or run past the read
  CAST_WORD_CLOSE,        // §6.11 — `)` of a cast before a name on its line
  ERROR_SENTINEL,         // never in a rule; valid only during error recovery
};
```

Specified in the first design and met in-grammar instead
(`docs/divergences-from-spec.md`):

```c
  AUTOMATIC_SEMICOLON,    // §5.10 — newline-as-terminator
  SLASHY_STRING_BODY,     // §5.4 — body (until unescaped `/`)
  SLASHY_STRING_END,      // §5.4 — closing `/`
  DOLLAR_SLASHY_STRING,   // §5.7 — single-token `$/.../$` when no interpolation
  GSTRING_BODY,           // §5.7 — literal fragment of a double-quoted string
  GSTRING_INTERPOLATION_START, // `${` or `$` followed by identifier
  LABEL_COLON,            // §5.3 — colon that closes a statement label
```

### 6.2 SLASHY_STRING — implementation contract

```text
Inputs visible to scanner:
  - previous_non_trivia_token_kind  (tracked in scanner state)
  - current lookahead char

State (scanner state struct):
  - last_token: one of OPERATOR, OPEN_BRACKET, COMMA, COLON,
    KEYWORD_RHS, IDENTIFIER, LITERAL, CLOSE_BRACKET, NONE
  - paren depth, bracket depth, brace depth (for GString containment)

SLASHY_STRING_START emits iff:
  - lookahead is `/`
  - AND NOT immediately followed by `=` (rule out `/=` augmented assign)
  - AND NOT immediately followed by another `/` (rule out `//` comment)
  - AND NOT immediately followed by `*` (rule out `/*` comment)
  - AND last_token ∈ { OPERATOR (any binary except postfix),
                       OPEN_BRACKET, COMMA, COLON, KEYWORD_RHS,
                       NONE (start-of-file or right after newline-as-terminator) }

SLASHY_STRING_BODY consumes chars until:
  - unescaped `/` → emit BODY (possibly empty), do NOT consume `/`, return
  - `\\/` → consumed as escape, continue
  - `\\` followed by any other char → consumed literally, continue
  - `$` followed by `{` or identifier → emit BODY, return (caller
    matches GSTRING_INTERPOLATION_START)
  - `\n` is allowed (slashy strings are multi-line per spec)

SLASHY_STRING_END emits iff lookahead is `/` AND state says we are
mid-slashy.
```

As implemented, the grammar's valid tokens stand in for `last_token`,
and the scanner emits only SLASHY_STRING_START, after the `/`; the
grammar reads the body and the closing `/`. The opener needs a body
that is not empty, does not start with whitespace, and reaches a
closing `/` on its line, an escaped character (`\/` among them) read as
one (`slashy_body_follows`). That look-ahead runs from every `/` where
a slashy string may start, which during error recovery is every `/`,
and an escaped slash ends none of them, so a line of `\/` was read from
each of its slashes to its end: 2,505 bytes read per input byte for
10,000 bytes and 25,005 for 100,000 (17 seconds), in `x = /` followed
by `\/` repeated and in `@` followed by `/\` repeated. The look-ahead
now stops at SLASHY_LOOKAHEAD_ESCAPED_SLASHES (64) escaped slashes and
takes the string to close there, so a character is read from at most
that many slashes before it: 1.0 and 6.0 bytes per input byte for the
two lines at 100,000 bytes. Real regexes hold a few escaped slashes (at
most five on a line of the Java set of 24,548 files, none in the
Groovy set), and a slashy string that does close reads as before
however many it holds. Past the bound a `/` with no closing `/` on its
line is a slashy string whose `/` is missing, an `ERROR`, where it was
division or another `ERROR`; on the line after a statement it starts
the next statement rather than continuing the one before (§6.21).
No tree of the corpus, the real and Java sets and 40,000 fuzzed lines
of regexes with escaped slashes, long lines and division chains
changed, but for the inputs with a line past the bound, all of which
had an `ERROR` before and after (1,583 of 17,762).
`bindings/rust/tests/slashy_string_cost.rs` checks the two lines.

The valid tokens stand in for `last_token` only where an operand may
start, and the grammar lets one start right after an operand: the next
statement may follow on the same line, and a parenthesis-free call's
first argument after its receiver. So a `/` after an operand opened a
slashy string wherever a second `/` closed one on the line:
`x = w/2 + h/2` was `x = w`, the string `/2 + h/` and `2`,
`println a/2 + b/2` the call `println a` and the same string, and
`x = (a)/2 + (b)/2` a cast of a string, each without an `ERROR`.
Groovy's lexer reads `/` after a name, a literal, `this`, `)`, `]`,
`}`, `++`, `--` or the end of a string as division (`isRegexAllowed`,
`REGEX_CHECK_ARRAY` in `GroovyLexer.g4`; Groovy 6 adds value-ending
keywords and a member name, `obj.class / 2`). NOT_IN is valid exactly
where an expression ends, so the scanner opens no slashy string where
it is valid and no line break comes before the `/`; past a line break
it may, as Groovy's lexer emits a newline token there (`x = a` and
`/re/` on the next line are two statements). A `/` after a `)` that
opens no comment closes no cast either (RELATIONAL_CLOSE, §6.11).
The scanner consumes leading whitespace with a comment token, so a
block or Groovydoc comment on the next line can hide that newline from
the following scan (`x = a` then `/* c */ /re/`). A fourth state bit
remembers it only where a statement may start and the next slash
passes the §6.2 body look-ahead. Consecutive block comments carry the bit; an
intervening name, a comment on the operand's line or a division inside
a call does not. Otherwise a division inside `foo(a` then
`/* c */ /2)` could make a later `b/3 + c/4` read `/3 + c/` as a string.

Inputs Groovy rejects are an `ERROR` now: `println /abc/`,
`foo /a/, 1`, `obj.m /x/` and `(String) /a/`, where Groovy reads a
division with no right operand. After the `}` of a call's trailing
closure in a statement (`foo { }/2`) the statement still ends, as a
call with a closure is a command chain there, and a slashy string
opens the next; Groovy reads `foo({ }) / 2`.

One reparse differs from a parse from scratch, by the runtime's limit
that §6.18 records: a node is reused, and broken down for a changed
token after it, only while one stack version exists. After a statement
that keeps two alive, such as `(x) { }`, a `return`, `throw` or
`assert` statement is reused whole at the fork, and the token after it
is lexed in the state after a statement, where a slashy string may
open. So `return x // c` edited into `return x /x/ c` reparses to
`return x`, the string `/x/` and `c`, where a parse from scratch reads
`return (x / x) / c`; before the change above both read the string. A
statement that starts with a name is not reused there, and none is
without the fork. `bindings/rust/tests/incremental_reparse.rs` pins it.

### 6.3 GSTRING — implementation contract

Double-quoted strings, triple-double-quoted strings, slashy, and
dollar-slashy all share the GString machinery (interpolation rules,
escape rules). Differences encoded by per-flavor state.

```text
After a `"` or `"""` is consumed in the grammar:
  GSTRING_BODY consumes chars until:
    - matching `"` (or `"""`) → emit BODY (possibly empty), return
    - `\\` followed by any char → consumed as escape
    - `$` followed by `{` → emit BODY, return (caller matches `${`)
    - `$` followed by an identifier start (§5.2, without `$`) → emit
      BODY, return
    - `\n` is allowed in triple-quoted variants; not in single
      double-quoted (emit ERROR, let grammar handle).
```

Brace depth tracking inside `${ … }` is essential: a `}` only ends
the interpolation when the interpolation's own brace counter is at
zero. The scanner increments the counter on `{` and decrements on `}`
within an active interpolation context. When the counter hits zero,
the `}` is emitted as the interpolation terminator and the scanner
returns to GSTRING_BODY mode.

### 6.4 AUTOMATIC_SEMICOLON — implementation contract

```text
Emits iff:
  - lookahead is `\n` (or sequence of `\n`s)
  - previous non-trivia token can end a statement
    (identifier, literal, closing bracket, `++`, `--`, etc.)
  - we are NOT inside `(` or `[` (brace depth doesn't suppress)
  - the next non-trivia token would NOT continue the expression
    (i.e., it is not a binary operator, not a dot, not `?`, not `:`
    inside a known ternary)

Else: consume newline as whitespace.
```

This resolves the line-continuation problem and is the underlying
reason murtaza64's grammar can't parse `pipeline {}\ndef foo = 5`
(issue #37). The pipeline rule there ends at `}`; the newline becomes
a statement terminator; the next statement begins. The murtaza64
grammar instead requires `pipeline` to be the *final* statement
because it does not have AUTOMATIC_SEMICOLON.

### 6.5 LABEL_COLON — implementation contract

```text
Emits iff:
  - the immediately preceding tokens form an `identifier` at
    statement-start position (we have not consumed any non-newline
    non-comment tokens since the last statement terminator)
  - lookahead is `:`
  - the token after `:` is NOT a sub-expression-starting token (a
    digit, a quote, a `(`, a `[`, an operator, `?`, `:`)

Else: do not emit; the `:` is consumed by whatever rule is in
flight (ternary, map item, switch case).
```

### 6.6 Block comment scanner

A single token `BLOCK_COMMENT` (or `GROOVYDOC_COMMENT` when the open
sequence is `/**`) matches the entire comment. This fixes
murtaza64 #16: the regex `seq('/**', token.immediate(/[*\n\s]+/), …)`
in murtaza64's grammar requires at least one whitespace character
after `/**`, so `/**/` (empty groovydoc) and `/***/` (immediate `*/`)
both fail. A scanner-driven token simply consumes from the opener to
the matching `*/` regardless of intermediate content.

A `/*` with no `*/` after it is not a comment: the scanner emits no
comment token, and the in-grammar lexer reads `/` and `*` as
operators, as ANTLR's lexer does. Learning that there is no `*/`
means reading to the end of input, so a file with N such openers was
read N times (#75). The scanner now keeps one bit of state, "there is
no `*/` from here on"; its other three bits are described below. The
bit is a property of the input and stays true at every later position, so
later `/*` openers, and the block comments the keyword look-aheads
(§6.7, §6.13) skip, fail at once while it is set.

tree-sitter stores scanner state only in a token the scanner emits,
and restores it from the last such token before every call, so a
failed scan cannot leave the bit behind. The scan that reaches the end
of input therefore emits `UNTERMINATED_COMMENT`: an empty, hidden
token, listed in `extras` because a comment may start anywhere. The
runtime accepts an empty extra token only when the state changed with
it, which holds for the marker that sets the bit, once per parse path.

The same marker carries a second bit, set after the `}` of a lambda's
brace body when the scan read past the start of the next token, and
cleared by the next call, which reads that token (§6.19), and a third,
set after `return`, `break` or `continue` when the scan read past a
block comment and the spaces after it to something other than a line
break, and cleared by the next call, which reads the comment (§6.23).
Each marker that sets one changes the state, so the runtime accepts it
too. The state is empty while every bit is unset and one byte holding
them otherwise.

The fourth bit belongs to a block or Groovydoc comment after a line
break where a statement may start (§6.2). The comment's scan reads
past same-line spaces to the next block comment or a slash that passes
the §6.2 body look-ahead, then serializes the bit with the comment. Consecutive
block comments carry it; a line comment clears it, and a division
inside parentheses does not set it. The slashy opener clears it after
reading the string.

An edit does not leave the bit stale. The marker read to the end of
input, so after any edit behind it the runtime lexes it again, and a
node lexed with the bit set is reused only where the new parse has
the bit set too. A change of included ranges is not an edit: a range
added later that contains `*/` leaves the marker in place, because
the runtime checks only a node's own look-ahead, as it does for every
other token.

One reparse used to differ from a parse from scratch because the
marker is empty. In `x = (a)/* n` followed by `c` on the next line, a
cast and a parenthesized expression are both possible when the parser
reaches `/*`. The parse path of the cast shifts the marker, fails on
`/` and is dropped before the other path reduces, so `(a)` is reduced
with one path alive and is not marked as the outcome of a conflict.
Inserting `*/` before `c` made the text valid, and the reparse reused
that `parenthesized_expression`: `x = (a)`, a comment, and `c` as a
second statement, where a parse from scratch gives one statement with
a `parenthesized_type_cast` over `c`. The look-ahead for a lambda
(§6.17) now reads from the `(` through the `)` and the comment, and
the runtime counts what the scanner read against the `(`, so the
insertion makes it lex the `(` again and the reparse is the parse from
scratch. The same holds for `foo((String)/* why` followed by `arg)`
and for the types without a name the look-ahead reads on through
(`x = (int)/* n`, `(a.b.C)`, `(String[])`, `(List<String>)`).
`bindings/rust/tests/incremental_reparse.rs` checks reparses against
parses from scratch, both texts among them.

Input that parsed without an error has no unterminated comment, never
sees the marker and is unaffected. On malformed input the marker is
one more node on the parse stack, so error recovery near the first
unterminated `/*` can choose a different repair than it did without
the marker, and it can be worse as well as better. Over 13_245 single
`/*` insertions into the corpus inputs, 233 trees (1.8%) changed: 86
with more bytes inside `ERROR`, 59 with fewer, 88 with the same. In
`ResponseEntity get(/*@PathVariable("id") Long id, @RequestBody Map
body) { }` the method declaration with a 2-byte `ERROR` became two
expression statements, a 54-byte `ERROR` and a closure.

The look-ahead after `record`, `trait` or `sealed` cannot set the bit,
because its token would start at the keyword, before the position the
fact holds from; the comment branch reaches the same `/*` next and
sets it, so that costs one extra read of the input, once. That holds
outside a string body only: in one no scan reaches a `/*` (§6.10), so a
look-ahead that took the body of `$/*.txt/$` for a comment read to the
next `*/` or to the end of the input once per such string, 2.2 s for
8,000 lines of `println $/*.txt/$`. The look-aheads read `$/` as a
dollar-slashy string instead (§5.7), and those lines take about 10 ms,
as at v0.6.0 (`bindings/rust/tests/dollar_slashy_string_cost.rs`).

### 6.7 RECORD_KEYWORD — implementation contract

Apache's `identifier` rule lists `RECORD`, so `record` names fields,
methods, variables and enum constants (`private Record record`,
`record.key()`). With `word: $.identifier`, tree-sitter substitutes
a keyword literal for the identifier wherever that keyword has an
action in the current parse state, and it cannot fall back once the
rule behind the keyword fails to match. Because `record_declaration`
is a statement and a class member, a literal `'record'` would be the
keyword at every statement and member start, which is exactly where
those identifier uses begin. The scanner makes the choice instead.

```text
Emit RECORD_KEYWORD when, from the current position:
  - the next six characters are `record`, and the character after
    them is not an identifier character (as below); then
  - at least one of: spaces, tabs, line escapes (§5.10), block
    comments — but no line break and no line comment, because
    `classDeclaration` in GroovyParser.g4 has no `nls` between RECORD
    and the name; then
  - a name (§5.2, but with every non-ASCII code point counted as an
    identifier character; the in-grammar lexer reads the real name
    afterwards, see §6.8); then either
  - `<` immediately (type parameters; the grammar's `type_parameters`
    opener is `token.immediate` as well), or
  - any mix of spaces, tabs, line escapes, line breaks, block and
    line comments (the rule has `nls` before `formalParameters`) and
    then `(`.
The token is `record` alone: `mark_end` is called after it and the
look-ahead is not kept. On any other input return false; the
in-grammar lexer then reads the word as `identifier`.
```

The grammar binds the token as `alias($._record_keyword, 'record')`,
so the anonymous `record` node, `node-types.json`, and the query
capture `"record" @keyword` are the same as with a literal.

Known residuals:

- A command-style call whose argument is itself a call,
  `record Point(1, 2)`, matches the pattern and parses as a record
  header with an `ERROR` in its components. It was an error at
  v0.4.0 too, where `record` was a keyword at every statement start
  regardless of what followed.
- A method with no modifier whose return type is spelled `record`
  (`record foo() { … }`) reads as a record declaration, as it did at
  v0.4.0 at statement start; in a class body it was a method there.
  The reverse holds for `trait`: `trait T(int x) { }` is a method,
  where Groovy reports an error (§6.13).
- A line break or a line comment anywhere between `record` and the
  name (after a block comment too), with the name and `(` on the
  next line, is not a header: the word is an identifier and the rest
  reads as a method declaration with return type `record`. Groovy
  rejects that input; v0.4.0 read it as a record because comments
  and newlines are `extras` around a literal keyword.
- With modifiers accepted before `record`, a method whose modifiers
  are followed by a return type spelled `record`
  (`private record bar() { … }`) reads as a record declaration too;
  it was a method at v0.4.0.

### 6.8 COMMAND_NAMED_KEY — implementation contract

Emitted only where a command chain argument may start, and never during
error recovery (`ERROR_SENTINEL` valid). The token is empty: it says
that a key starts here, and the in-grammar lexer then reads the key as
an ordinary `identifier`. The scanner skips spaces, tabs and line
escapes (§5.10) but not newlines, marks the token end, then looks ahead
over a name, the same whitespace and block comments, which may span
lines as Groovy's lexer hides them (`foo a /* c */ : 1` is
`foo(a: 1)`), for a single `:` (a following second `:` means a method
reference, so no emit). `default` is
rejected so a switch label after a statement on the same line stays a
label. `case` is not: a `case` label needs a value before its `:`, so
`case:` is always a key (`foo case: 1`, and
`switch (x) { case 1: foo case: 2 }` is `foo(case: 2)`, as in Groovy).
A key spelled `var`, `yield`, `in` or `as` is CONTEXTUAL_KEY
instead, a token that spans the word (§6.16), and takes no comment
before its `:`: `foo in /* c */ : 1` is the statement `foo` and the
label `in: 1`, where Groovy reads `foo(in: 1)`.

The scanner has no Unicode tables, so its look-ahead counts every
non-ASCII code point as part of the name. That is wider than §5.2, and
it is safe because the scanner does not emit the name. On valid input
the two agree, since outside strings and comments a non-ASCII character
can only be part of a name. On other input (`foo a` + U+00A0 + `: 1`)
the lexer reads a shorter name and the parse fails there. The scanner
used to emit the key itself, with an ASCII-only class. Once #82 made
non-ASCII names identifiers, `foo 名前: 1` parsed with no `ERROR` as a
statement and a label.

Keeping the decision scanner-side gives the parser a distinct lookahead
token, so `Type name = …` stays a declaration without a precedence
change, and a bare statement followed by a label on the next line
(Spock's `where:`) stays two statements. A keyword is a valid key:
after the empty token it has no parse action, so it lexes as an
identifier, and the key position reserves no word, which covers `new`
and `case` (§5.2).
An empty external token is ignored by the tree-sitter runtime directly
after a parse error, so a key there is not recognized until the parser
has shifted another token.

After a comma the statement cannot end, so a key on a continuation line
(`sh script: 'make',` then `returnStdout: true`) is matched in the grammar
as a plain identifier followed by `:` and needs no scanner token.

Known limitations:

- String keys (`foo 'a': 1`, `foo "$k": 1`) are an `ERROR`. Apache's
  `namedArgPrimary` accepts them.
- `default` cannot be a key as the first argument. Apache's
  `namedArgLabel` accepts every keyword, so `foo default: 1` is a
  named-argument call there and an expression statement plus a label here.
  The divergence is deliberate: the one-line
  `switch (x) { case 1: foo default: bar }` depends on it. After a comma
  it is accepted as a key (`foo 'a', default: 1`), on the same line or
  the next: the scanner still rejects it, and the grammar's continuation
  rule takes the keyword as a plain identifier. `case` is a key in both
  places; after a comma the continuation rule takes the reserved word
  itself (`foo a: 1, case: 2`).
- The keys `new`, `true`, `false`, `null` and `switch` are accepted on the
  same line as the preceding argument (`foo a: 1, new: 2`) but are an
  `ERROR` on a continuation line (`foo a: 1,` then `new: 2`). `non-sealed`
  is an `ERROR` in both positions, because `-` cannot be part of an
  identifier. Every other keyword is accepted in both positions (`if`,
  `def`, `in`, `as`, `final`, `class`), as are `this` and `super`.
- A block or groovydoc comment before a label defeats the newline guard.
  The comment branch consumes the newline when it emits the comment, so
  `result` followed by a line reading `/* note */ where:` or
  `/** note */ where:` parses as a command chain with key `where`.
- A trailing argument continues onto the next line. After a comma an
  argument is a full expression, and expressions do not stop at a newline,
  so a next line that starts with something that can continue an expression
  joins the last argument, even a literal one. `foo 1, 2` followed by a
  line reading `-x` parses as the subtraction `2 - x`. The same happens
  when the next line starts with `{` (a trailing closure), `+` (a binary
  operator), `++` or `--` (a postfix update: `foo 1, 2` followed by `++x`
  parses as `2++` and then `x`), `in`, `as` or `instanceof`. A next line
  that starts with `[` or `(` is not affected: a `[` after a newline never
  subscripts (§6.9) and such a `(` opens the next statement (§6.18), so
  `foo 1, 2` followed by `[a].each { }` or by `(a)` is two statements. A
  chain whose one argument is positional (`foo 'a'` then `-a`) is not
  affected either, because that argument is not a full expression; in a
  value it is one, and joins (§6.14). The value of a named argument is
  one, so it joins even as the only argument: `foo a: 1` followed by `-b`
  has the value `1 - b`. Expressions outside command chains already
  behave this way: `def x = 'b'` followed by `-a` is a subtraction. A
  spread after a comma takes the whole expression after its `*` (§5.5
  item 3), so it joins the same way: `foo 1, *a` followed by `+ 1` is
  `foo(1, *(a + 1))`, where Groovy calls `foo(1, *a)` and reads `+1` as
  a statement (observed with Groovy 3), as it does for `foo 1, a`
  followed by `+ 1`. Two forms Groovy rejects parse without an `ERROR`
  for the same reason: a `*` that ends the line (`foo 1, *` followed by
  `a`), as a line break is whitespace after a unary operator too
  (`foo 1, -` followed by `a`), and a `,` that opens the next line
  (`foo 1, *a` or `foo 1, a` followed by `, 2`), which goes on with the
  call.
- A dangling comma takes the first token of the next statement. `foo a: 1,`
  followed by a blank line and `bar b: 2` parses without an `ERROR`, as the
  chain `foo a: 1, bar` followed by the labeled statement `b: 2`. The
  dangling comma was an `ERROR` before.

### 6.9 SUBSCRIPT_OPEN — implementation contract

Emitted wherever a `[` may follow, and never during error recovery
(`ERROR_SENTINEL` valid). Skips spaces, tabs and line escapes (§5.10)
but not newlines; if the next character is `[`, consumes it.
Subscripts, array types and the dimensions of an array creation accept
only this token. List and map literals accept it and the in-grammar
`[`, so a `[` on the same line resolves as it did with a single token.
At a newline it emits nothing, so the in-grammar `[` lexes instead and
starts a list or map literal: Groovy ends a statement at a newline,
which is what lets a Grails controller action return `[model: value]`
on its own line.

Two known differences from Groovy are accepted. The scanner tracks
no brackets, so a newline before `[` ends the expression everywhere,
including inside `(…)`, `[…]`, `${…}` and conditions such as `if (a`
followed by `[0])` on the next line. Groovy's lexer ignores newlines
inside `(…)`, `[…]` and `?[…]`, so `foo(a` followed by `[0])` on the
next line, `[a` followed by `[0]]`, and the array-type forms `(String`
followed by `[]) x` and `def m(String` followed by `[] a) { }` are
valid Groovy and an `ERROR` here (pinned in
`test/corpus/operators-subscript.txt` and
`test/corpus/declarations-array-types.txt`). Matching Groovy needs a
stack of open parentheses, brackets and braces in the scanner. A block
comment at the start of the `[` line also hides the newline before it:
`a` followed by `/* c */ [0]` on the next line parses as a subscript,
where Groovy ends the statement at the newline.

### 6.10 STRING_BODY — implementation contract

Never emitted. `"…"`, `"""…"""`, `/…/` and `$/…/$` are lexed in pieces
so that interpolations become child nodes, and the parser calls the
scanner at the start of every piece. Comments are `extras`, so they are
valid there as everywhere else. At v0.5.0 the scanner therefore emitted
a comment whenever a piece started with `//` or `/*`, with or without
whitespace before it: `"//div"` lost its closing quote to a line
comment, and `"${dir}/**/*.xml"` held a `groovydoc_comment`.

Every body lists `_string_body` among its segments, so the token is
valid exactly where a segment may start. When it is valid and
`ERROR_SENTINEL` is not, the scanner returns without emitting anything
and the in-grammar lexer reads the text. Inside `${…}` the token is not
valid, so a comment there is still a comment. During error recovery
every external is valid and the check is skipped, as for the other
context-sensitive tokens.

### 6.11 CAST_CLOSE — implementation contract

`castParExpression castOperandExpression` in GroovyParser.g4 has no
`nls` between the two, so where a line break can end a statement,
`x = (a)` followed by `foo()` on the next line is two statements.
Newlines are `extras` here, so the grammar alone read that input as a
cast of `foo()`, with no `ERROR`. The token tells the parser whether
the operand of a possible cast is on the line of its `)`.

```text
Emit CAST_CLOSE for a `)` when, after it:
  - any spaces, tabs, line escapes (§5.10) and block comments that
    close on the same line are skipped, and
  - the next character is not a line break, not the end of the input,
    and not the `//` of a line comment.
A block comment that runs past the end of the line counts as something
on the line: Groovy's lexer drops the comment with its line breaks, and
reading on would scan an unterminated comment to the end of the input.
On any other input return false; the in-grammar lexer then reads `)`.
```

`parenthesized_type_cast` and `parenthesized_expression` both close
with this token or with a plain `)`, so `(a) * b`, `((a))` and
`foo((a), b)` stay open to both readings until the next token decides.
The token only sets the rank of the cast (§5.15): above a
parenthesized expression when it closes with the token, below one
when it closes with a plain `)`. It cannot forbid the second form,
because inside `(…)` and `[…]` a line break does not end anything and
the scanner does not know which brackets enclose it. The token is
aliased to `)` and the tree is the same either way. It is not emitted
during error recovery.

The same scan emits CAST_WORD_CLOSE instead of CAST_CLOSE where a name
follows:

```text
Emit CAST_WORD_CLOSE for a `)` when it is valid and, after the trivia
above, the line goes on with a word that is
  - no keyword of grammar.js other than `_`, `permits` and
    `pipeline`, which Groovy takes as names, nor a primitive type name
    or `void`,
  - not `instanceof`, nor `as` or `in` before what it takes (below),
  - not the `$` that opens a dollar-slashy string (§5.7).
Emit CAST_CLOSE there when CAST_WORD_CLOSE is not valid.
```

Both rules take the token, aliased to `)`. `parenthesized_type_cast`
takes it as it takes CAST_CLOSE; `parenthesized_expression` takes it
with a dynamic precedence of `-TYPED_FIELD_WEIGHT`. In Groovy the name
is the cast's operand: Apache tries `castExprAlt` first, and a type
body needs a separator between members. Here members need none, and the
reading that ends the value at the `)` takes the name as the type of a
member that starts there, with that member's weight: `def k = (Foo) x`
then `def m() { }` on the next line was the field `k = (Foo)`, a field
of type `x` named `def` (a keyword lexes as a name where it has no
action) and a constructor `m() { }`, which weighed as much as the cast
and the method and won the tie, and `public C() { }` after the same line
was a field `x public` and a constructor, which outweighed the cast.
Where a statement may start, the same split made the name a statement
of its own on the line: `x = (Foo) y` then `{ -> 1 }` and `{ foo() }`
was the assignment of `(Foo)`, the call `y { -> 1 }` and a block, where
Groovy calls `y` with both closures; v0.6.0 read the cast there as
well. The penalty is the weight of the
typed field that the split adds, so the split weighs less than the cast
wherever it adds one typed member and the members after it weigh the
same. Where the name is followed on its line by `as`, `in` or
`instanceof` and another name, the split adds two, as each word is a
name to a member's start: `def k = (Foo) x as Bar` then `def m() { }`
is the field `k = (Foo)`, the fields `x as` and `Bar def` and a
constructor `m`, and `def k = (Foo) x in y` then `private int z` the
fields `x in`, `y private` and `int z`, in every build since v0.6.0,
where Groovy reads one field and a method or a field. A larger penalty
would need a token of its own for that case (`docs/divergences-from-spec.md`
§15). Where no cast parses, the
parenthesized expression is the only reading and keeps it:
`(c + c) x` is still `(c + c)` and the statement `x` (Groovy calls the
parentheses with `x`). A keyword after the `)` that starts no operand
closes no cast (RELATIONAL_CLOSE, below), and one that starts an
operand but no type (`new`, `super`, `switch`, `true`, `false`, `null`,
`static` before a `.`) is left to CAST_CLOSE. The read stops after the word, or after what follows
`as` or `in`. Not emitted during error recovery.

Measured against the grammar before: STATE_COUNT 4,777 to 4,789,
LARGE_STATE_COUNT 1,831 to 1,843, one external token. The penalty is
on the rule's own alternative; carried by a hidden rule of one token it
would be lost, as tree-sitter removes such unit reductions, and limited
to the parentheses that hold an `_expression` it cost 300 more states.
No tree of the corpus or of 266 real files changes; of 24,548 Java files
one with an `ERROR` changes (`x = (T) y;` then `else`, now the cast and
the `else`), and of 4,000 shape inputs two, each `x = (a.b.C) y` then
closures on the next lines, now Groovy 3's reading.

The same scan emits RELATIONAL_CLOSE instead of CAST_CLOSE when what
follows the `)` is a relational operator that Groovy applies to the
parentheses:

```text
Emit RELATIONAL_CLOSE for a `)` when, after the trivia above, the line
goes on with
  - `instanceof` or `!instanceof` as a whole word, `!in` and
    whitespace, `[`, `(` or `{` (GroovyLexer.g4 NOT_IN, §3.2.2), or
  - `as` or `in` as a whole word, then only spaces, tabs, line
    escapes and block comments that close on the line, then a name
    that is not `as`, `in` or `instanceof`, nor `new` after `as`, or
    after `in` a digit or a quote, or a `[` whose brackets hold a `,`
    with no other bracket open within 256 characters
    (`MEMBERSHIP_LIST_LOOKAHEAD_CHARACTERS`), or
  - a `/` that opens no comment, a division after a `)` to Groovy's
    lexer, which no cast's operand starts with (§6.2), or
  - a keyword of grammar.js that starts no operand: any but `new`,
    `super`, `switch`, `true`, `false`, `null`, `static` before a `.`,
    and `_`, `permits` and `pipeline`, which Groovy takes as names.
Emit it also in a state where it is not valid, where only a cast's type
takes the `)`: no rule takes it there, so that reading ends.
```

A keyword that starts no operand, such as `else`, ends the cast's reading
too: the in-grammar lexer reads a keyword as a name where the state has
no action for it, so `if (a) (b) else c` read the cast `(b)` of a name
`else` and the statement `c`, with no `ERROR`, and `if (a) (b) else (c)`
the cast of a call `else(c)`, where Groovy runs the `if` with an `else`
(observed with Groovy 3). `(b) return` and `(b) while (x) y`, which
Groovy rejects, are the parenthesized name and the next statement.

`instanceof` is reserved in Groovy, but here a keyword lexes as a name
where it has no action, so `(x) instanceof Foo` read the cast `(x)` of a
name `instanceof` and the statement `Foo`, and `(x) !in y` the cast of
`!` applied to the name `in`. `!instanceof` without whitespace after it
is no operator to Groovy's lexer but `!` and the keyword, which starts
no operand either. The cast closed with CAST_CLOSE and
outranked the test. A weight could not settle it: the cast reading also
takes the weight of the statement after the name, and
`def ok = (x) instanceof Foo` followed by `bar() { }` read the method
`Foo bar() { }`. Only `parenthesized_expression` takes the token. Where
only a cast's type takes the `)`, as in the version of `(a.b)` that
forked at the dot to read `a.b` as a type and lexes the `)` on its own,
no rule takes it, like `_lambda_body_reread` (§6.19), and that version
ends. Parentheses that hold only a type (`(List<String>) instanceof T`)
are then an `ERROR`, where Groovy tests a class literal.

`as` and `in` are names (§6.16): `(Foo) in` casts the name `in`, and a
`.`, `(`, `[`, `{`, an operator or the end of the line after the word
goes on with the name (`(Foo) in.read()`, `(byte) in[i]`, and
`(x) in -1`, which Groovy reads as `((x) in) - 1`). Groovy 3 reads
`(x) as Foo` and `(x) in y` at the start of a statement, as the value
of a declaration and as a condition as a call of the cast of a variable
`as` or `in` (`((x) as)(Foo)`), and as the operator where an expression
must stand (`return (x) as Foo`, `foo((x) in y)`). Groovy 6.0.0
(GROOVY-10355, in Apache's AstBuilder of the 6.0.0 tag and later, not
in 4.0.x, 5.0.x or 5.1.x) rewrites that call into the operator when it
has one argument and no further command arguments and the parentheses
hold a name, possibly dotted, that is no primitive type, and for `as`
when the leftmost operand at relational precedence of the argument
spells a type; it also reads `(Foo) in [..]`, a subscript of the name
`in`, as `Foo in [..]` in any context. That is the reading here, with
these differences: `(int) as Foo` coerces `(int)`, where Groovy casts
the name `as`; `(x) in y, z` is the test and an `ERROR`, where Groovy
calls the cast with two arguments; `(Foo) in [0]` casts the subscript,
as the scanner cannot tell `(Foo)` from `(byte)` in `(byte) in[i]`,
while `(Foo) in [1, 2]` is the test: a subscript takes several indices
too, so the scanner reads the brackets for the `,` of a list of several
elements, on Groovy's tokens, and `(byte) in[i, j]` is the test as
well; `(x) in !y` casts the name `in`, as `!` counts as no
operand; and `(x) as foo()` calls the coercion `(x) as foo`, where
Groovy calls the cast of `as` with `foo()`
(`docs/divergences-from-spec.md` §5).

The read stops at the first character after the word, or after the
trivia and the word or character that follow `as` or `in`, so a line
of `(x) in y && …` reads each part once
(`bindings/rust/tests/relational_close_cost.rs`). A block comment that
runs past the end of the line after `as` or `in` counts as no operand,
so `return (x) in /*` then `*/ y` on the next line casts the name `in`,
where Groovy, which drops the comment, tests `(x)`. The token is
aliased to `)`. It is not emitted during error recovery.

The scan emits RELATIONAL_CLOSE as well for a `)` that ends its line
before a line that opens with `(`, where no bracket holds both lines:

```text
Emit RELATIONAL_CLOSE for a `)` when TYPE_ONLY_CLOSE is not valid and,
after the trivia above, the line ends (a line break, the end of the
input or a line comment), and after the rest of the line, spaces, tabs
and line breaks, the next line opens with `(`, unless, read from that
`(` on Groovy's tokens for at most
ENCLOSING_BRACKET_LOOKAHEAD_CHARACTERS (256) characters,
  - a `)` or `]` with no bracket open comes before a `}` with none
    open: brackets around both lines, inside which Groovy drops the
    line break; or
  - `->` follows the first parentheses (`(Foo)` then `(x) -> x`).
The tokens are read as the other look-aheads read them: strings whole
(a triple-quoted one across lines, a `${…}` with the strings in it, a
dollar-slashy one), a slashy string where an operand may start, and
comments as trivia. The end of the input, the bound, and a string or a
comment that does not close within it end the read with no bracket
found.
```

Where a line break ends a statement that `(` opens the next statement
or a call's arguments (§6.18), and Groovy reads no cast across the
line break. The cast whose operand is on a later line ranks below
every other reading (§5.15) and lost there, but kept alive it was
reduced again at every such line: since §6.18 opened a statement at
each such `(`, a run of lines like `(x)`, `(a.b)` or `(a) = (b)` took
time quadratic in its length, 2.1 s for 4,000 lines of `(x)`, where
v0.6.0 took 12 ms. The token ends that reading at the `)`, as at a
relational operator. An array or generic type is no expression, so the
cast of one is the only reading of its parentheses: that later-line
cast is an alternative of its own in `parenthesized_type_cast`, with
TYPE_ONLY_CLOSE, which is never emitted, valid before its `)`, and the
scan keeps it (`(List<String>)` then `(x)` is still a cast, where
Groovy reads two statements; `docs/divergences-from-spec.md` §15).

The scanner has no stack of brackets, so inside `(…)` and `[…]` it
reads on for the bracket that closes them, and the cast stays
(`foo((Foo)` then `(Bar) y)`, `foo((Foo)` then `(Bar) ')')`). A bracket
beyond the bound goes unseen, and the cast inside it is turned down,
which leaves an `ERROR` or another reading where Groovy casts: with the
call's `)` more than 256 characters on, `foo((Foo)` then
`(Bar) y, '…')` is an `ERROR`, and `foo((Foo)` then `(bar) + 1, '…')`
is the sum of the call `(Foo)(bar)` and `1`, with no `ERROR`, and
`(bar)[0]` there a subscript of that call. Kept alive, the cast still
ranks below every other reading, so `foo((a)` then `(b))` is the call
`(a)(b)`. Where its operand is a cast on its own line, `[(Foo)` then
`(y)(z)]`, the two weights cancel and the readings tie: a list's first
element is the cast of the cast, as in Groovy and at v0.6.0, and a
subscript's index, a later element, a map value or an argument is the
call `(Foo)(y)(z)`. Two such trees of one hidden rule between the same
stack nodes are merged before the rule's parent is reduced, and the
runtime keeps the first; the list's element is inlined into the list
(grammar.js `inline`) so that its trees are compared instead, which
costs 3 parse states, where the subscript's indices would cost about
40 (`docs/divergences-from-spec.md` §15). Counted on characters
instead of tokens, a bracket in a string or a comment had kept a cast
alive or turned a valid one down. The read runs from every `)` that
ends a line before such a `(`, so in a run of short lines each
character is read from the `)` of each line up to 256 characters
before it: 65 to 67 bytes asked for per input byte at 1,000, 4,000 and
16,000 lines of `(x)`, flat, where the parse before asked for 7;
those runs take 3, 12 and 45 ms. Code with few lines that open with `(` reads as
before: the 24,548 Java files take as long. The cost test is
`bindings/rust/tests/next_line_cast_cost.rs`, which counts the
reductions per line.

Measured against the grammar before: STATE_COUNT 4,648 to 4,656,
LARGE_STATE_COUNT unchanged (1,788), one external token
(`_type_only_close`), one hidden rule (`_cast_name_type`) and one
conflict added. No tree changes on the corpus, on 266 real Groovy,
Gradle and Jenkins files, on 24,548 Java files and on 4,000 inputs
generated from statement templates; of 6,048 inputs made of a
parenthesized line, a line break and parentheses, 75 change, each a
ternary (`a ? (x)` then `(y) + 1 : b`) that Groovy 3 rejects, and of
40,000 random token soups 369, each with an `ERROR` before and after.
Runs of lines whose cast's operand would be on
a next line that does not open with `(` stay quadratic, as at v0.6.0:
`x.y = z * (w)` (2.0 s for 4,000 lines) and
`this.total = base + (int) (offset)` (4.1 s).

### 6.12 NOT_IN — implementation contract

GroovyLexer.g4 has
`NOT_IN: '!in' { isFollowedBy(_input, ' ', '\t', '\r', '\n', '[', '(', '{') }?`,
so `a !in xs` uses the operator and `!inside` is `!` applied to a
name.

```text
Emit NOT_IN for the three characters `!in` when the character after
them is a space, a tab, a line break, `[`, `(` or `{`. That character
is read and not consumed.
On any other input return false; the in-grammar lexer then reads `!`.
```

The in-grammar form of this token was a regex that had to include the
character after `!in`. With a `[` or `(` inside the operator,
`a !in[1, 2]` and `a !in(b)` were an `ERROR`. The token is aliased to
`!in`. It is not emitted during error recovery.

### 6.13 TRAIT_KEYWORD and SEALED_KEYWORD — implementation contract

Apache's `identifier` rule lists `TRAIT` and `SEALED`, so both words
name fields, methods, variables and parameters (`private String trait`,
`return trait`). They are keywords for the reason `record` is (§6.7):
`trait_declaration` and the sealed class, interface and trait start
wherever a statement or a member can, and a literal there is the
keyword at every such start. After `private String`, which is already a
complete field named `String`, `trait` read as the keyword and opened
a trait declaration that took the following members (#73).

```text
Emit TRAIT_KEYWORD when, from the current position:
  - the next five characters are `trait`, not followed by an
    identifier character; then
  - a name on the same line, as for RECORD_KEYWORD (§6.7):
    `classDeclaration` has no `nls` between TRAIT and the name; then
  - any mix of spaces, tabs, line escapes, line breaks, block and line
    comments, and then `{`, `<`, or the word `extends`, `implements`
    or `permits`.
Emit SEALED_KEYWORD when, from the current position:
  - the next six characters are `sealed`, not followed by an
    identifier character; then
  - any mix of the trivia above, line breaks included (Apache's
    modifiers end in `nls`); then
  - the word `class`, `interface` or `trait`, the words the grammar
    accepts after the modifier; or, when no line break was crossed,
    `@`, `non-sealed`, `record`, `enum`, `default` or a word of
    `_modifier`.
Each token is the keyword alone; the look-ahead is not kept. On any
other input return false; the in-grammar lexer then reads the word as
`identifier`.
```

The grammar binds the tokens as `alias($._trait_keyword, 'trait')` and
`alias($._sealed_keyword, 'sealed')`, so the anonymous nodes,
`node-types.json` and the `@keyword` captures are those of the
literals. `non-sealed` is not an identifier in Apache's grammar and
stays an in-grammar token.

Groovy's parser takes `sealed` among the other class modifiers in any
order, and before an annotation. The grammar accepts no modifier after
`sealed`, nor `sealed record` or `sealed enum` (which Groovy rejects
later), so `sealed abstract class S { }`, `sealed @A class S { }` and
`class O { sealed static class C { } }` are an `ERROR` at `sealed`
until modifiers are accepted in any order. The keyword is still
emitted there, so that the `ERROR` stays at the word and the
declaration keeps its node: read as a name, the word gave trees with
no `ERROR`, such as a field of type `sealed` named `static`, or lost
the class to the `ERROR`. That is done only on the same line. Across a
line break the same words also start the member after a field named
`sealed` (`private boolean sealed`, then `static class D { }` or
`public int size`), which the keyword would turn into an `ERROR`; a
modifier after `sealed` can only be an `ERROR` while the order is
fixed, so the field keeps its reading.

A line break is whitespace to the grammar, so the scanner's look-ahead
is what tells a declaration from a name. Groovy also sees the line
breaks before the word, and the scanner does not, so these readings
differ:

- `trait` at the end of a line is never the keyword. In a member list
  `trait` followed by `T { }` on the next line is then a field `T` of
  type `trait` and an instance initializer; Groovy rejects that input
  (observed with Groovy 3). At statement level it is the name `trait`
  followed by the call `T { }`, as in Groovy (observed). At v0.6.0
  both were trait declarations.
- `sealed` at the end of a line before a nested `class`, `interface`
  or `trait` is the keyword. `private boolean sealed` followed by
  `class Inner { }` on the next line is a field named `boolean` and a
  sealed class; Groovy reads a field `sealed` and a class (read off
  `GroovyParser.g4`, not run).
- In a class body Groovy's parser reads `trait T` followed by `{` on
  the next line as a field of type `trait` and an instance
  initializer (observed with Groovy 3; its member alternatives try a
  field first), which fails to resolve later. Here it is a trait
  declaration.
- `sealed` alone at the end of a line before another modifier is a
  name. `sealed` followed by `public class C { }` on the next line is
  the name `sealed` and a class at statement level, and a field of
  type `sealed` named `public` and a class in a member list; Groovy 4
  reads a sealed class (read off `GroovyParser.g4`). At v0.6.0 both
  were an `ERROR`.
- `sealed` at the end of a line before any other member is a name.
  `private sealed` followed by `def run() { }` or `int x = 1` on the
  next line is a field `sealed` and then that member, as in Groovy 3,
  where `sealed` is no keyword (observed). Groovy 4 to 6 read
  `private sealed` as the modifiers of that member: their `modifiers`
  takes line breaks between modifiers and `sealed` is one (read off
  `GroovyParser.g4` 4.0.x, 6 and master). The grammar takes `sealed`
  only before `class`, `interface` or `trait`, so the keyword would make
  such a member an `ERROR`. At v0.6.0 `sealed` was an `ERROR` inside
  the member.

One residual does not involve a line break. A name followed by `(` is
not a trait header, so `trait T(int x) { }` is a method `T` with
return type `trait` and no `ERROR`, at statement level and in a member
list; Groovy 3 rejects it (observed), and v0.6.0 gave an `ERROR`. It
is the counterpart of the `record foo() { … }` residual of §6.7.

### 6.14 COMMAND_VALUE_START, COMMAND_PATH_START and COMMAND_STATEMENT_START — implementation contract

A parenthesis-free call may be a value: of a declarator or an
assignment, or the content of parentheses or of an interpolation (§5.5
item 1), and its receiver may have member accesses (§5.5 item 2). A
newline is whitespace, so with only the grammar `def x = foo` followed
by `1` on the next line would be one call, and so would `result.size`
followed by `2`; Groovy ends the statement at the line break (observed
with Groovy 3). Both forms therefore hold an empty token, which the
scanner emits only when the first argument starts on the receiver's
line. So does a statement whose receiver is one name and whose first
argument starts with `new`, is an array type that is a value, is a
call in parentheses or starts with a name (§5.5 item 3): `foo` followed
by `new Bar()` or `bar` on the next line is two statements in Groovy,
and the grammar, which matches a literal after such a receiver itself,
cannot tell. A literal that the argument goes on past opens with the
same token (`println "a" + b`): the grammar's literal form takes the
literal alone, and a form that took an expression there would read
`foo 1` followed by `-x` on the next line as one call, where Groovy ends
the statement at the line break. Nor can the grammar tell
`println(foo 1)` from `println(x)` at the `(`, where it must already
have made `println` the receiver or an expression, nor `foo bar` from
`String s` at `bar`.

```text
Emit COMMAND_VALUE_START, empty, where a value may start, when:
  - after whitespace, line breaks and line escapes (a value may start
    on the line after its `=`), a name starts, read as in §6.8; and
  - the name is not a value keyword, as below; and
  - any member accesses follow it, as below; and
  - the first argument follows, as below, a name argument included
    when the receiver takes one; a quoted string only when it closes on
    its line: its closing quote, a second quote right after the opening one
    (an empty string or a triple quote) or, in a double-quoted string,
    a `${` comes before the line break, a `\` taking the character
    after it, a line break included.
The token ends before the name.

Emit COMMAND_PATH_START, empty, directly after the first name of a
statement, when:
  - a member access starts there, with nothing before it; and
  - one or more member accesses follow, as below; and
  - the first argument follows, as below, a name argument included,
    with the rules of a statement, when the receiver takes one; and
  - not: the last name of the receiver starts with `A` to `Z`, the
    argument is a string that closes on the line with no `$` in it,
    or a dollar-slashy string that closes with no interpolation in it,
    and `(` follows it on the line.
The token ends after the name.

Emit COMMAND_STATEMENT_START, empty, where a statement may start,
when:
  - after whitespace, line breaks and line escapes, a name starts that
    is not a keyword of grammar.js, or is one that is no value keyword
    and that a call in parentheses follows (`super(foo 1)`), and no
    member access follows it; and
  - not: the name is `record`, `trait`, `sealed`, `var`, `yield`, `in`
    or `as` and its own token (§6.7, §6.13, §6.16) is valid there; the
    scanner decides that token instead; and
  - a creation, an array type value or a call in parentheses follows,
    as below; or a literal follows that the argument goes on past, as
    below, a quoted string after a receiver that takes no name argument
    only when no `(` follows it on its line (`String 'foo'() { }`); or
    the receiver takes a name argument, which is not `var`, `yield`,
    `in` or `as`, and a name argument follows with the rules of a
    statement.
The token ends before the name.

A value keyword: a keyword of grammar.js that Groovy reserves, which
includes `new`, `switch`, `true`, `false` and `null`, the words that
start a value as keywords. `super`, `permits`, `pipeline` and `_` are
names to Groovy and are none, and so is `static` before a `.`.
Member accesses: each `.`, `?.`, `??.`, `*.`, `.&` or `.@` followed by
a name, with no space anywhere.
The first argument: after spaces, tabs and line escapes, but no line
break and no comment, a digit, `'`, `"`, the `$/` of a dollar-slashy
string (§5.7), the word `true`, `false` or
`null`, a creation, an array type value, a key: a name followed on
the same line by `:` and not `::`, other than `default` (§6.8), or a
call in parentheses.
A call in parentheses: `(`, then Groovy's trivia, then a name that is
no value keyword, its member accesses as above and a first argument
other than a call in parentheses, as COMMAND_VALUE_START reads them
after the `(`.
A receiver that takes a name argument: one name that is no primitive
type name (boolean, byte, char, double, float, int, long, short), not
`void` and does not start with an upper-case letter (`A` to `Z`, or a
capital of Latin-1, Greek or Cyrillic); or a name with member accesses
whose last name does not start with one and is not `new`
(`x.new Inner()` creates). Where a statement starts, also not `var`,
`yield`, `in` or `as`. This is Apache's
isInvalidLocalVariableDeclaration on the spelling (a modifier is a
keyword here, and `<` or `[` after the receiver starts no argument).
A name argument: after spaces, tabs and line escapes, on the line:
  - `!` that is not `!=`, nor `!in` before whitespace or a bracket,
    nor `!instanceof` before whitespace; or `~`; or
  - a name that is no value keyword, not `in` and not `as`, then, after
    any member accesses or a `[` directly after it, and after spaces,
    block comments (which may span lines) and a line comment, none of:
      - a literal, or a name that starts a chain continuation (§5.5
        item 4): `true`, `false`, `null`, a key, a creation, an array
        type value, a name and `=` that is not `==` or `=~`, a name
        before `(` that holds parameters or a call in parentheses as
        below; any name after `var`, `yield`, `record`, `trait` or
        `sealed`; and in a value any name other than `in`, `as` and
        `instanceof`;
      - `(` that holds, after Groovy's trivia, a call in parentheses'
        receiver and first argument, or a name followed after a line
        break by another, or parameters: an element that starts with
        `@`, `final` or `def`, or a receiver that may not take a name
        argument followed, after trivia and only what a type holds
        (type arguments, which hold names, `.`, `,`, `?` and
        dimensions; dimensions, each a `[` and its `]`; the three dots
        of varargs), by a name that is not `instanceof`, nor `as` or
        `in` unless `,`, `)` or `=` follows it, so that
        `println max(VALUES[i])`, `println foo(MAX as long)` and
        `println valid(MIN < x && MAX > x)` are calls; the elements are
        read up to the first that is neither a parameter nor a lone
        name, such as a `/` that opens no comment, a slashy string
        (`foo bar(x, /re/)`);
      - `->`;
      - and, in a statement (COMMAND_PATH_START and
        COMMAND_STATEMENT_START), `=` that is not `==` or `=~` (a
        declaration), or `->` after a line break and trivia, or `,` and
        then Groovy's tokens up to `->` (closure parameters): with
        brackets counted; comments, strings, `++` and `--` read whole (a
        string's `${…}` with the strings in it, §6.17), a slashy string
        where an operand may start, no `=` follows its `/` and it closes
        on its line read whole, and a dollar-slashy string read whole,
        across lines, within 1,024 characters (§5.7; one that does not
        close within them ends the list with no `->`); an element, the
        first item after a `,` outside brackets, that is `(` or a key
        ending the list with no `->` (`foo x, () -> y` passes a
        lambda); and a line break ending the
        list unless a comma, a modifier, an annotation (its name and
        arguments), an operator or an open bracket comes before it (a
        postfix `++` or `--` and a slashy string end an operand), or
        after it `->` or an operator that Groovy lets a
        line open with, other than `(` and `,` (`{ foo a, b = i++` then
        `* 2 -> a }`), or, after `var` that starts an element, a name
        that `->`, `,` or `=` follows on its line (`{ String a, var`
        then `b -> a }`; Apache's `variableModifiers` takes `nls` after
        each modifier and annotation).
  Each list is read for at most 256 items (a word, a number, a string, a
  comment, a line break or a punctuation character); at the end of the
  budget the answer is no parameters, and a list read to the budget
  after a lowercase-typed first closure parameter is a call's arguments.
  Inside the parentheses that a call in parentheses or parameters are
  read from, a `(` after the name is not read into.
A literal that the argument goes on past: after spaces, tabs and line
escapes, but no line break and no comment, a number, a quoted string,
or the word `true`, `false` or `null` that is no key, read whole: a
number's digits, letters and `_`, a `.` before a digit and the sign of
a decimal exponent (`1e-5`); a string to its closing quote, on its line
unless triple-quoted, a `\` taking the character after it, with each
`${…}` read as code to its `}`, braces counted and strings in it read
whole, at most 8 strings deep, within
LITERAL_ARGUMENT_LOOKAHEAD_CHARACTERS (256) characters from its
opening quote: a longer string is not one. Then the argument goes on at once at a
number's `.` that no digit follows (`1.m()`, `1..3`), and otherwise
after Groovy's trivia at the first token: on the literal's line at any
but `,`, `;`, `}` and the end of input, and on a later line at a `/`
that opens no comment, at `.`, `?`, `*`, `%`, `=`, `!`, `<`, `>`, `&`,
`|`, `^`, `:` and `{`, and at the word `as`, `in` or `instanceof`.
A creation: the word `new`, then, after Groovy's trivia (whitespace,
line breaks, line escapes and comments), a name. The key `new:` is
not one.
An array type value: a capitalized name and any `.Name` after it with
a capitalized name, or a primitive type name, then dimensions and a
member access or `::`, as ARRAY_VALUE_START reads them where a type
may start (§6.15).
The in-grammar lexer reads the receiver and the argument. On any other
input return false. None of the three tokens is emitted during error
recovery.
```

The three tokens are decided where nothing hides a line break from the
scanner. Comments are `extras`, so after a comment the parser is where
it was and the scanner is called again, past the line break it can no
longer see. A token placed after the receiver would read `def x = foo`
followed by `/* c */ 1` on the next line as one call, and `foo`
followed by `/* c */ new Bar()` as well.
COMMAND_VALUE_START and COMMAND_STATEMENT_START are decided before the
receiver, and COMMAND_PATH_START between its first name and the
operator after it, where no whitespace, comment or line break may
come. Every form with a line break after the receiver parses as it did
before.

A call in parentheses as the first argument (`println(foo 1)`,
`x.add(foo 'a')`, `def y = bar(foo 1)`) is decided by the token before
the outer receiver, or after its first name, from what follows the `(`;
the inner call then opens with COMMAND_VALUE_START, which the scanner
decides again once the parser is past the `(`. The two read the same
input with the same rules, so they agree. Inside the parenthesis the
look-ahead skips line breaks and comments, as Groovy's lexer does there,
with the state of §6.6, and it stops at the first argument of the inner
call: a call in parentheses there is not read into, so that each part
of the input is read a bounded number of times. `println(bar(foo 1))`
is therefore an `ERROR`, while `(bar(foo 1))` and `def x = bar(foo 1)`
are calls, as the outer parenthesis there needs no look-ahead. A `(`
on the line after the receiver is never a first argument, so
`println` followed by `(foo 1)` on the next line is two statements, as
in Groovy: that `(` opens the next one (§6.18).

The look-ahead reads the first name inside the parentheses of each call
that starts a statement or a value or follows a dotted receiver, where
the scanner stopped at the `(` before. On real Groovy and Gradle files
the scanner advances over 3.4% more characters, with no more calls, and
the parse has the same number of steps and no new step with more than
one stack version, on real files, on Java files and on code made only of
calls (`foo(a, b)`, `obj.m(a, 'x')`). Code made only of such calls takes
1.6% more instructions, real files and Java files 0.3% more. One kind
of tree with no `ERROR` changes: a method header whose parameter is
named `true`, `false` or `null` after a name, with no body, or with one
when the name before is on the line above. `foo bar(baz true)` was the
method `bar` with return type `foo`, and is now the statement `foo` and
the call `bar(baz true)`; Groovy reads the call `foo.bar(baz(true))`
(§5.5 item 4). `x` followed by `foo(it null, a, b)` on the next line is
now `x` and the call `foo(it(null, a, b))`, as in Groovy, and `x`
followed by `C(Foo.Bar true) { }`, which Groovy rejects, is `x` and a
call of `C`. The same continuation without a literal,
`foo bar(baz 1)`, was an `ERROR` and is now `foo` and the call
`bar(baz 1)`, where Groovy reads `foo.bar(baz(1))`.

A keyword that Groovy reserves is no receiver of a value: with
`def x = return 1` Groovy calls nothing, and it reads `"${return 1}"`
and `"${throw new E()}"` as statements, which an interpolation here
does not hold (§5.5 item 1). The receiver of a value was any name but
`new`, `switch`, `true`, `false` and `null` before; `def x = return 1`
was a call of `return` and is now the declaration of `x` with the value
`return` (a name to the in-grammar lexer, as no keyword is valid there)
and the statement `1`, and `x = throw 1, 2` is an `ERROR`. Groovy
rejects each such input (observed with Groovy 3 on 600 of them that a
fuzzer changed). The names Groovy takes stay receivers:
`def x = super.foo 1`, `def x = static.foo 1`, `(permits 'x')`,
`def x = pipeline 'x'`, `(_ 1)`. At the start of a statement those
names are receivers only before a call in parentheses, where the
statement form has no other reading: `super(foo 1)` in a constructor
is a call, as `this(foo 1)` is, and `super new Foo()` stays `super`
and the creation (§5.5 item 3).

COMMAND_PATH_START makes the scanner read on after a statement's first
name only when a member access follows it directly. Those statements
are read once more. A
value's receiver is read before it, as for a single name, and its
accesses are read once more as well. Measured against the grammar
before #52, code made only of such lines (`a.b.c()`, `x.y = 1`,
`def x = a.b`) parses about 3% to 4% slower, other short lines up to
about 2% slower (the parse table grew), and real Groovy and Gradle
files about 1% slower. The two places differ because the parser can
tell a statement's first name from the start of an expression only by
the token after it, while in a value the name is an expression, and a
rule that took the name itself would cost every value a reduction. A
statement whose receiver is a single name is matched in the grammar
(§5.5 item 3), unless COMMAND_STATEMENT_START opens it. `record`,
`trait` and `sealed` followed by a member
access are names (§6.7, §6.13), so `record.add 'x'` is a call.

COMMAND_STATEMENT_START is valid wherever a statement may start, so the
scanner reads the first name of every statement that starts with one,
and the in-grammar lexer reads it again. A name that cannot be followed
by a call's first argument is turned down at the character after it
(`foo(`, `x =`, `obj.m`). A name that may name a type is turned down
after the next name once that one can no longer be `new`, a primitive
type name or a capitalized class name (`String name`); after any other
the next name is read whole, as it may be the first argument
(`println x`). A literal after the name is read whole, with the trivia
and the first token after it (`println "a"`, below). The same name may
be `record`, `trait`, `sealed`, `var`, `yield`, `in`, `as` or the start
of an array type value, whose tokens are valid at a statement start too;
the scanner reads it once and decides each of them there, as its other
branches decide them where no statement may start. A keyword of
grammar.js is turned down as soon as it is read, because after the
token the parser takes only a name, and the in-grammar lexer would read
`return` in `return new Foo()` as one; a test compares the scanner's
words with `src/grammar.json`. The keywords that are names to Groovy,
`super`, `permits`, `pipeline` and `_`, are read on when a call in
parentheses follows them: `super(foo 1)` in a constructor is a call,
while `super new Foo()` stays `super` and the creation, as before. A
statement may also follow a bare `return`, `break` or `continue`, and
`final Type`, which declares
`Type`, so the token is valid after each, and each takes it before its
operand or name: `return foo new Foo()` returns `foo`, as
`return foo 1` does, and `final String s new Foo()` declares `s`. The same
reading of the name decides DECLARATION_START (§6.21), which is valid
where a statement may start and after its annotations, so a statement
that starts with two names is a call or a declaration from one decision.

Measured against the grammar before: STATE_COUNT 4249 to 4298,
LARGE_STATE_COUNT 1565 to 1577, one external token and no conflict or
precedence added. The scanner reads the first name of each statement
once more: it advances over about 23% more characters on real files,
with no more calls, and the parse has the same number of steps and no
new step with more than one stack version, on real files, on Java files
and on code made only of statements that start with a name. With the
parse table's layout held fixed, real Groovy and Gradle files take
about 1.4% more time and 1.6% more instructions, Java files about 1.0%
and 1.1%, and code made only of statements that start with a name about
0.9% and 2.1%. The table change alone costs about 0.1% of the
instructions. A token after the receiver, decided as COMMAND_NAMED_KEY
is (§6.8), costs about 0.1% but reads `foo` followed by
`/* c */ new Bar()` on the next line as one call.

Trees that change for input that parsed without an `ERROR`: each form
of §5.5 item 3 with a creation or an array type value as the first
argument, as a statement, after a dotted receiver or as a value, was
the receiver followed by the argument as a statement of its own (or a
declaration whose value is the receiver, followed by that statement).
It is now one `command_chain`. That holds as well after another
statement on the same line, where Groovy continues that statement
instead (§5.5 item 4): `foo 1 bar new Baz()` and `foo bar new Baz()`
are `foo 1` or `foo`, then the call `bar new Baz()`; and
`foo new Foo() bar 1` is the call `foo new Foo()` and the chain
`bar 1`. A call in the branch of an `if` now lets an `else` after it
continue the `if`: `if (x) foo new Foo() else bar new Bar()` was the
`if`, the creation, and `else` and `bar` read as names. Every such
change was checked mechanically on the cases a fuzzer found: with a
line break put between each such call's receiver and its first
argument, the new grammar gives the old tree, and, but for that `else`,
the call sits where the old tree had its receiver, so it takes no
operand of the statement before it.

A name as the first argument (§5.5 item 3) needs no change to the
grammar: after each of the three tokens the grammar takes any
expression as the first argument. The scanner emits them before more
receivers, by Apache's rule on the spelling, read off the receiver, the
name after it and what follows that name. After the name the look-ahead
reads at most one more word, one parenthesis deep, as above, and two
lists: the elements of a parenthesis after the name (typed parameters)
and, after a comma in a statement, a closure's parameters up to `->`.
Malformed input can restart a statement at each element of a long list
(`{ bar a,` then `bar b,` and so on with no `->`), and each restart reads
ahead from where it starts; without a bound that made the parse
quadratic (a list of 4,000 such elements took 71 times the
instructions it took before, four times more with each doubling, and
229 KB took 4.3 s for 18 ms before). Each list is therefore
read for at most 256 items, so each part of the input is read by at
most that many restarts and the parse stays linear (the same list now
takes a constant 7 times the instructions it took before, 0.1 s for the
229 KB; `bindings/rust/tests/name_argument_cost.rs` compares the bytes
the parser asks for at two sizes). Whitespace and comments before `->`
are read once per call, and no statement can start inside them. A
closure parameter list after a lowercase-typed first parameter that is
longer than the budget is read as a call's arguments, an `ERROR`.
`var`, `yield`, `in` and `as` take no name argument as receivers where a
statement starts: whether their own tokens are valid depends on the
token before them (`in` and `as` follow an expression), which an
incremental reparse need not restore, so the reading would depend on it
too. For the same reason `as` is a receiver only before a name that is
an argument (`as new Foo()`, `as String[].class`). After an expression
the scanner reads `as` for the operator, marks its end, and finds no
type after it before a literal, a `(` or a `{`; the word is then a
name, and no empty token can be put before it any more. A reparse that
reuses the statement before reads the word in the state after that
statement, where the operator is not valid, and would emit the token:
`x = 1` then `as { }.bar 1` was an `ERROR` from scratch and a call
after an edit further on. So `as` takes no token there in either state:
`as "a" + b` is `as "a"` and `+ b` (Groovy's `as("a" + b)`),
`as (foo 1)` an `ERROR`, and `as { }.bar()` one too (§6.24). `in` needs
no such rule: an expression is its operand, and every first argument
starts one, so after an expression `in "a" + b` is the membership test.
`def x = Foo bar`, `x.new Inner()` and `foo bar(int a) { }` are
turned down for the reasons in §5.5 item 3.

Measured against the grammar before: STATE_COUNT, LARGE_STATE_COUNT and
`parser.c` are unchanged (4330, 1575 and 8,207,368 bytes), and no
external token, conflict or precedence is added. On the real Groovy,
Gradle and Jenkins files whose trees do not change, the scanner makes
the same number of calls and advances 0.1% further, and the parse takes
0.1% more instructions; Java files take 0.06% more, and code made only
of names or only of calls with parentheses 0.3% more. Of 266 real
files, 98 change their trees: 28 that had an `ERROR` have none (294
`ERROR` and `MISSING` nodes become 75), 60 without one hold 352 lines
that were two statements and are now one call, and no file gains an
`ERROR`. Of 24,548 Java files only 14 change, each with an `ERROR`
before and after.

Trees that change for input that parsed without an `ERROR`, each a call
where Groovy reads one (observed with Groovy 3):

- A statement whose receiver is a name or a dotted name and whose first
  argument starts with a name, `!` or `~` (`println x`, `foo bar()`,
  `obj.m arg`, `foo !x`, `task copy(type: Copy) { }`) was the receiver
  and the argument as statements of their own, and is one
  `command_chain`. With more arguments (`assertEquals a, b`) it was an
  `ERROR`. That holds in every place a statement
  may stand: a label's (`then: thrown X`), a closure's, an `if`
  branch's, and after another statement on the same line
  (`foo 1 bar baz` is the call `foo 1` and the call `bar baz`). In an
  `if` branch an `else` after the call now continues the `if`:
  `if (x) foo bar else baz qux` was the `if`, then four statements.
- A declarator's or an assignment's value whose first argument is a
  name (`def r = foo bar`, `x = foo bar`, `def y = a.b c`) was the
  receiver as the value, then the argument as a statement.
- The argument absorbs what continues it, as any first argument does:
  `foo bar { }`, also with the `{` on the next line, is
  `foo(bar({ }))`; before, `foo` and the chain `bar { }`. `println x`
  followed by `(1..3).each { }` on the next line passed
  `x(1..3).each { }` as well, until a `(` that opens a line where a
  statement may start opened that statement (§6.18), as in Groovy.
- A script method with a lowercase return type and untyped parameters
  or none, with a body (`foo bar() { }`, `task build(a) { }`), was a
  `method_declaration` and is the call `foo(bar() { })`, as in Groovy.
- A dollar-slashy string after a receiver (`obj.m $/x/$`,
  `def x = foo $/a/$`) is a first argument, as a quoted string is, also
  after a receiver that may name a type (`def x = Foo $/a/$`,
  `obj.Bar $/a/$`): before, the receiver and the string.
- A name argument and an operator that started a declaration's value
  before: `foo bar =~ x` was the declaration of `bar` with the value
  `~x`, and is `foo(bar =~ x)`.
- A name argument and a line that starts with `=` or a name and `=`:
  `foo bar` then `= 1`, and `a.b.c x` then `y = 1`, were declarations
  across the line break (`foo bar = 1`, `x y = 1`), and are
  `foo(bar = 1)`, and `a.b.c(x)` then `y = 1`.
- `pipeline` as a name argument: `foo pipeline { }` was `foo` and a
  `pipeline_statement`, and is `foo(pipeline({ }))`.

Every such change was checked mechanically on the cases a fuzzer found:
of 15,904 trees without an `ERROR` that changed among 235,000 inputs,
putting a line break between each new call's receiver and its first
argument gives the old tree for 15,048; 816 have a line escape or a
comment there, which keep the call, and the other 40 have nothing
between the two to break (`foo!x`, `foo~x`) or a dollar-slashy string.
101 inputs went from no `ERROR` to one; Groovy 3 rejects all but 14:
six in which a `(` on the line after a name argument continues it, as
above (`println var` then `(int a) { }`), four in which a method header
with no return type follows a name argument on the next line
(`foo bar` then `x(Map<K, V> a, int b) { }`), two with a generic type
as the name argument (`foo List<String> x(y` then `Z)`), and two made of
keywords and names Groovy reads as chains (`task var` then
`default \` and `var`).
A generator of closure parameter lists after a lowercase-typed first
parameter (with defaults, annotations, comments and line breaks; 5,673
inputs) finds none that this grammar reads with fewer parameters than
the grammar before.

A string after a receiver whose last name is capitalized, followed by
`(`, is the name of a method: Groovy declares `foo` in
`java.util.List 'foo'() { }` and `java.util.List '''foo''' () { }`, and
calls in `obj.m 'a'() { }`, `java.util.list 'a'() { }` and
`java.util.List "$x"() { }` (observed). The scanner reads the string,
triple-quoted or not, to its close on the line. A dollar-slashy string
there is read to its `/$` (§5.7), and one without an interpolation names
the method as well: `java.util.List $/foo/$() { }` declares `foo` and
`obj.Bar $/$a/$() { }` calls, as in Groovy.

A quoted string is the first argument of a value only where it closes
on its line, or opens a triple quote or an interpolation, which may
span lines (`def v = foo "a ${` followed by `b` and `}"` is a call).
One that does not is still being typed, and as an argument it
committed the parser to a call whose string never closes: in
`String a = "${x"`, an interpolation that still lacks its `}`, the
closing quote of the string around it opened the argument of a call
`x "…`, and the `ERROR` took the class or the file around it, where the
name alone leaves one `MISSING` `}`.

A slashy string is not a first argument, as in Groovy, which reads `/`
after a name as division. Known differences from Groovy:

- A comment between the receiver and its first argument on one line,
  `def x = foo /* c */ 1`, `x.y /* c */ 1` or `foo /* c */ new Bar()`,
  is not a call: the
  look-ahead reads no comment there, so it never scans an unterminated
  one to the end of the input. The tree is the one from before, the
  receiver, the comment and the argument. Groovy reads a call (observed
  with Groovy 3), also when the comment spans lines. After `new` the
  look-ahead does read comments and line breaks, as Groovy's
  `new nls creator` allows (`foo new /* c */ Bar()` is a call), with
  the state of §6.6.
- A name after a receiver that may name a type (`def x = Foo bar`,
  `(Foo bar)`), and after `var`, `yield`, `in` or `as` as receivers
  where a statement starts (`in x`), is not taken; Groovy calls
  `Foo(bar)` and `in(x)`. Nor are the chain forms of §5.5 item 4
  (`foo a bar`, `foo bar 1`). A comment between the receiver and the
  name argument stops the look-ahead as above (`foo /* c */ bar` is two
  statements and the comment); after the name argument comments are
  trivia (`myType x /* c */ = 1` declares `x`).
- `def x = foo .5`, Groovy's `foo(0.5)`, is an `ERROR`: `.5` is not a
  number in this grammar at all.
- The first positional argument is an expression, and like every
  expression it continues onto a next line that can continue it. A
  next line that starts with `-`, `+`, `++` or `--`
  makes a binary or postfix expression: `x.y 1` followed by `-1`
  passes `1 - 1`, and `x.y 2` followed by `++i` passes `2++` and leaves
  `i`. Groovy ends the statement at the line break in each case
  (observed). That is the general reading of such a line
  (`docs/divergences-from-spec.md` §3), and an argument after a comma
  does the same in statement position (§6.8). Before, the receiver was
  followed by the same expression as a statement of its own, `1 - 1`:
  only where that expression is attached changed. A next line that
  starts with `(` opens the next statement (§6.18): `def x = foo 1`
  followed by `(c)`, and `foo bar` followed by `(1)`, are two
  statements, as in Groovy.
- The receiver is read without spaces, comments or line breaks inside
  it, so `obj . m 1` and `obj` followed by `.m 1` on the next line are
  not calls; Groovy reads both as `obj.m(1)` (observed). Both read as
  they did before #52. Two exceptions come from where the look-ahead
  starts, and are not otherwise intended. In a value where a lambda may
  start (a declarator's or an assignment's value, parentheses, not an
  interpolation or the value of `(a, b) =`), the look-ahead for a
  lambda's arrow (§6.17) has skipped the spaces or tabs after the first
  name, and a member access after them is read into the receiver:
  `def a = foo .bar 1` and `x = (foo ?.bar 2)` are calls, as in Groovy.
  Whether a name is a first argument there is decided by the first name,
  not the last: `def b = Foo .bar baz` is `Foo .bar` and `baz`, and
  `def c = foo .Bar y` a call, where Groovy calls both;
  `def d = foo .new Bar()` calls a method `new` with `Bar()`, where
  Groovy creates `new Bar(foo)`. The array types of §6.15 are read the
  same way (`def e = Map .Entry[].class`, as in Groovy). And a
  statement's receiver may have a comment right before its first
  access (`foo /* c */.bar 4` is a call, as in Groovy), as the parser
  skips it before the token of §6.14, but not a space after the comment.
  Turning each of these down, as the rule says, moves trees that agree
  with Groovy away from it, and reading the first access after spaces
  everywhere touches every look-ahead that reads a receiver; neither was
  done (`docs/divergences-from-spec.md` §19).

A literal as a statement's first argument (§5.5 item 3) needs no change
to the grammar either: after COMMAND_STATEMENT_START the first argument
is any expression. The scanner reads the literal after the receiver
whole, as the in-grammar lexer does next, then the trivia and the first
token after it, as above. It emits the token only where the argument
goes on past the literal, and leaves a literal that is the whole
argument to the grammar's literal form, `command_chain`, which gives the
same tree. That split saves time, not trees: with the token before every
literal, code made only of `foo "a"` lines took about 23% longer to
parse, as the token and the reductions of an expression cost the parser
more than the literal form does, while the look-ahead itself costs about
1%. On a later line the tokens that go on are those that Groovy's
grammar lets open a line within an expression, with `NL*` (`nls` in
4.0.x) before the operator in Apache's `expression` alternatives or
before a `pathElement`: the binary operators but `+`, `-` and `**`, the
ternary and Elvis operators, `as`, `in` and `instanceof`, `.`, `?.`,
`*.`, `.&`, `::` and a closure (GroovyParser.g4 of 4.0.x, 6 and master).
There `+` and `-` open a unary expression, and `(` and `[` the next
statement (§6.9, §6.18), so the scanner leaves the literal alone before
them, as before. Before a `(` that also keeps `foo "a"` followed by
`(x) -> x` the call and a lambda statement, as before, where Groovy
passes the lambda to a call of the method `a`; an argument that took the
`(` would leave the `->` an `ERROR`. On the literal's line the scanner
leaves the literal alone for the same reason before a `(` that the
look-ahead of §6.17 reads as a lambda's parameters (`foo "a" (x) -> x`,
`foo 1 (x) -> x`: the call and a lambda statement); any other `(` there
opens the literal's call (`foo "a" (x)`, `foo("a"(x))` as in Groovy).
After such a call, or a literal's call with a closure, that ends its
line (`foo "a" (x)`, `sh "a" { }`), a lambda in parentheses on the next
line is an `ERROR`, as after a name argument (`sh foo { }`): before `->`
the `(` stays the call's arguments (§6.18), where Groovy appends the
lambda. `**` starts with a character that goes on, and opens no line in
Groovy (Apache's `powerExprAlt` has no `NL*` before the operator), so
`foo 1` then `** 2` is a call where Groovy rejects it. An assignment does
open a line in Groovy (`x` then `= 2` is `x = 2`: `assignmentExprAlt` has
`NL*` before the operator), and the argument goes on there as it does
in Groovy (`foo "a".b` then `= c`); Groovy rejects `foo "a"` then `= b`
because a literal is not assignable, which the grammar does not check.
At a number's `.` that no digit follows (`foo 1..3`, `foo 1.abs()`) the
argument goes on, and the scanner reads no further: the end of the
argument is not found without reading the expression, so a later line
is not looked at, and the argument takes what opens it as after a name
argument. `++`, `--`, `, b` and `, k: 1` on the next line then go on
with the call, and `~b` or `!b` after `foo 1..3` on its line is a
statement of its own, where Groovy rejects all of them, as it does
`foo a..b` then `++`, which the grammar reads the same way. The words
that Groovy reserves and does not use, `goto`, `const` and
`threadsafe`, are names in this grammar wherever a name may be
(`goto x`, `def y = goto`), so as a receiver each takes a literal that
the argument goes on past (`goto "a" + b`), where Groovy rejects every
use of the word.

Measured against the grammar before: STATE_COUNT, LARGE_STATE_COUNT,
SYMBOL_COUNT and EXTERNAL_TOKEN_COUNT are unchanged (4658, 1789, 408 and
38), as is `parser.c`. Read one byte per call, the lexer reads 8% more
of the real files concatenated (1.95 bytes per byte, 1.80 before) and
the same of the Java files; neither set takes measurably more time (the
266 real files 51.6 ms against 51.7 ms, the fastest of 40 runs; the
24,548 Java files 14.8 s in two orders of five runs, against 14.7 s and
15.1 s). At 1,000, 4,000 and 16,000 lines the bytes read per byte stay the
same on runs of literal arguments and of input meant to defeat the
look-ahead: strings, triple-quoted strings, interpolations and comments
that do not close after the receiver, literals of 2,000 characters, and
50 blank or comment lines after a literal. A run of `foo "${ {` lines,
a nest of strings that does not close, reads 11.4 bytes per byte (2.4
before) at each length; read whole, each of its strings would make it
quadratic (`bindings/rust/tests/literal_argument_cost.rs`).

Of 266 real files, 12 change their trees: 11 that had an `ERROR` have
none, and the 35 calls that changed in them read as Groovy 3 reads them;
one without an `ERROR` passes a sum (`src "${MODEL_URL}/" + f`). No Java
file changes, one corpus test does, and 17 of 4,000 generated statement
sequences (`sh 'ls'` followed by `{ ->` or `{ foo() }` on the next line,
Groovy's `sh('ls'({ … }))`). Trees that change for input that parsed
without an `ERROR`, each a call where Groovy reads one (observed with
Groovy 3):
`+ b`, `- b`, `[0]`, `(b)`, `as T`, `in b`, `!in b`, `instanceof T` and
`!instanceof T` after the literal on its line were the call of the
literal and statements of their own; `{ }` on its line or the next was
the call and a block, and is a call of the literal with the closure,
Groovy's call of the method a string names; and `as T`, `in b`, `!in b`,
`instanceof T` and `!instanceof T` on the next line go on with the
argument. Over 13,792 edits of texts with literal arguments, a reparse
gives the tree of a parse from scratch.

One reparse differs from a parse from scratch, through the runtime.
During error recovery it ignores an empty external token that
leaves the scanner's state as it was, and a later reparse reuses the
leaf after it without calling the scanner there. Typing `def` on the
line before `obj.Bar $/$a/$() { }` reads `def obj` and, by error
recovery, a method whose return type is `.Bar` with a missing name
before the dot. Deleting `def` again reuses that `.`, so
COMMAND_PATH_START is not emitted after `obj`, and the reparse reads a
`method_declaration` that returns `obj.Bar` and is named by the string,
where a parse from scratch reads Groovy 3's call
`obj.Bar($/$a/$() { })`. Neither tree has an `ERROR`; v0.6.0 reads the
method from scratch too, so it has no difference to show. The same
edits differ the same way with `"$a"`, `"${a}"` or `"""$a"""` in place
of the dollar-slashy string, but not with a string without an
interpolation, a lowercase name (`obj.bar`), two accesses (`a.b.Bar`)
or without the `()` or the closure. A probe of 11.77 million single
edits of the corpus inputs, each reparsed and then undone, found 17
that differ this way, all in one input of that shape. A fix would need
another bit of scanner state and a token that ends the path, so the
limit is documented instead (`docs/divergences-from-spec.md` §19), and
`bindings/rust/tests/incremental_reparse.rs` pins both trees.

Known differences from Groovy, besides those above:

- An argument that goes on past the literal and ends its line takes a
  next line that starts with `+` or `-`: `foo "a" + b` followed by `-c`
  passes `"a" + b - c`, as `println x` followed by `-y` passes `x - y`.
  Only a literal that ends its line is decided at the line break.
- `foo` followed by `"a" + b` on the next line is the call `foo "a"` and
  `+b`: the literal form takes a literal on the line after the receiver,
  as before (Groovy reads `foo` and the sum).
- The receivers `var`, `record`, `trait` and `sealed`, whose own
  keywords the scanner decides at the same name, and `super`, `permits`
  and `_`, keywords of the grammar, keep the literal alone:
  `super "a" + b` is `super "a"` and `+b`. So does a comment between the
  receiver and the literal (`foo /* c */ "a" + b`), as for every first
  argument.
- A quoted string longer than LITERAL_ARGUMENT_LOOKAHEAD_CHARACTERS
  (256) characters keeps the literal alone: `foo "<257 characters>" + b`
  is the call `foo "…"` and `+b`, and `.trim()` or `?: c` after it an
  `ERROR`, as at v0.6.0. Read to its end, a string that is the whole
  argument was read twice, by the look-ahead and by the in-grammar
  lexer: `foo` and a string of 10,000 characters asked for 2.00 bytes
  per input byte, 1.00 at v0.6.0. Such strings are most often the whole
  argument, a build step's script (`sh """…"""`): in the 266 real
  files no string that the argument goes on past is longer than 62
  characters, and of 1,242 reads of a string there 53 were of one
  longer than 256. With the bound the bytes the parser asks for per
  input byte of those files went from 1.966 to 1.914, with no tree
  changed (1.904 at 128 characters). The shorter strings are still
  read twice: with no string read at all, at the cost of every string
  that the argument goes on past, the figure would be 1.827. It was
  1.801 before literal arguments were read and 1.601 at v0.6.0.

### 6.15 ARRAY_VALUE_START and TYPE_START — implementation contract

An array type can be a value: `String[].class`, `int[][].class`,
`String[]::new`, `String[].&new`, `@A(String[])` and
`Class c() default int[]` (observed with Groovy 3, which reads each as
a class). Apache's grammar has no rule for it. `String[]` there is a
`primary` or a dotted name followed by an `indexPropertyArgs` with no
index, so `a[]` is an index with an empty list for any `a`, and the
compiler turns it into a class when `a` names one.

The grammar reads such a value as the `array_type` of a declaration,
`(array_type (type_identifier))`. The parser cannot learn that a name
is a type at the `]`: by then it has reduced the name to an
expression, and a rule that kept the name apart until the `]` would
cost every name in an expression a reduction (measured: about 3%
slower on real files). The scanner decides at the name instead.

```text
Emit ARRAY_VALUE_START, empty, where an expression may start, when:
  - after whitespace, line breaks and line escapes, a name starts that
    is a primitive type name (boolean, byte, char, double, float, int,
    long, short), or a capitalized name (A to Z first) followed by any
    `.Name` with a capitalized name and no space; and
  - after spaces, tabs and line escapes, `[` follows on the line; and
  - after Groovy's trivia inside brackets (whitespace, line breaks,
    line escapes and comments), `]` follows; and
  - where a type may also start (TYPE_START valid): every further `[`
    on the line of the `]` before it opens one more `[` `]`, and after
    the last `]` and spaces on the line comes `.` followed by a name or
    `&`, or `::`.
The token ends before the name. On any other input return false. It
is not emitted during error recovery.

TYPE_START is never emitted. `_type` references it, so it is valid
exactly where a type may start.
```

Where a type may start, `String[]` is a type unless what follows the
dimensions cannot follow one, so `String[] xs`, `(String[]) x`,
`for (String[] a : xs)` and `String[]` followed by `xs = foo()` on the
next line read as before. That covers every statement start, `(`, a
closure's start, and the value after `return`, which a statement may
follow. There `String[].class` and `int[]::new` are values; a bare
`String[]` is not (`return String[]`, `(String[])`), and stays the
error it was.

The rule on the spelling keeps the look-ahead cheap. Names in
expressions are mostly variables and methods, written in lowercase; a
lowercase name is read only while it can still become a primitive type
name, so most are turned down at their first or second character, and
`a[0]` costs nothing more. Where a statement may start the scanner reads
the whole name for COMMAND_STATEMENT_START (§6.14) and decides this
token from it as well. `java.lang.String[].class` (a package
before the class), `foo[]` with a lowercase class `foo`, and a generic
element type are not taken and read as before: the first two are a
subscript with a missing index, and `List<String>[].class` is a chain
of comparisons with no error, which is kept. In `.Name` the names are
read without spaces, as in §6.14, but where a lambda may start the
spaces or tabs after the first name have been skipped for its arrow,
so `def e = Map .Entry[].class` and `println(String [].class)` are
array types, as in Groovy.

The grammar's value rule takes a further `[` after `String[]` as a
dimension, by precedence, so `int[][]` is two dimensions and
`String[][0]` is an error, as before. Typed declarations, parameters,
casts, `as`, `instanceof`, type arguments and array creation do not
reach the value rule and are unchanged.

Measured against the grammar before: real Groovy and Gradle files
parse about 1% to 1.5% slower, class bodies and other code made of
short declarations about 2% slower (TYPE_START changes the parse
table's layout, not the parser's work), statements that start with a
typed array declaration (`String[] xs = foo()`) about 2.5% slower, and
subscripts (`xs[i] = ys[j]`, `foo(a[1], b[2])`) up to about 2%.

### 6.16 VAR, YIELD, IN and AS keywords — implementation contract

Apache's `identifier` rule lists `AS`, `IN`, `VAR` and `YIELD` (the
4.0.x, 5.0.x and 6.0.x grammars), so the four words name variables,
fields, parameters, methods and labels: `int var = 1`,
`InputStream in = open()`, `in.close()`, `return yield`, `var: x()`.
As grammar literals they were keywords wherever they had an action,
for the reason `record` was (§6.7). `var` and `yield` start
statements, and a statement may follow any statement that can end, so
`int var`, `return var` and `println var` met the keyword. `in` and
`as` are operators after any expression, so `InputStream in = …`,
`int as = 1` and `foo()` followed by `in.read()` on the next line did.
The scanner now makes each choice from what follows the word.

```text
On the word `var`, `yield`, `in` or `as`, not followed by an
identifier character, where its token is valid, look past any mix of
spaces, tabs, line escapes, line breaks, block and line comments:
  VAR_KEYWORD        a name follows, other than `instanceof` followed by
                     a type and a keyword that is no modifier and starts
                     no operand (`else`, `if`; §6.11), and in a type
                     body (VAR_NAMES_KEYWORD valid, DECLARATION_START
                     not) a name that `(` follows, past trivia and line
                     breaks; or `(` follows where VAR_NAMES_KEYWORD is
                     not valid and no expression may start (after
                     `final`, in a parameter). Never
                     where JUMP_STATEMENT_END is valid, right after
                     `return`, `break` or `continue` on their line
                     (§6.23).
  VAR_NAMES_KEYWORD  `(` follows, then only names, `,`, `*`, `:`, `.`,
                     `<`, `>`, `?`, `[`, `]` and trivia up to `)`, then
                     `=` that is not `==` or `=~`.
  YIELD_KEYWORD,     an expression starts, for YIELD_KEYWORD on the
  IN_KEYWORD         word's line (past spaces, tabs, line escapes and
                     block comments only): a name other than
                     `instanceof` followed by a type, a digit, a quote,
                     `[`, `{` or `~`; `(` unless `)` follows it; `!`
                     unless `=` or `instanceof`, whitespace and a type
                     follow it; `+` or `-` unless `=` or `>` follows it,
                     and `++` or `--` only when an expression starts
                     after it on its line (spaces, tabs, line escapes
                     and block comments only); `/` (a slashy string)
                     unless a space or `=` follows it.
  AS_KEYWORD         a name, the start of a type, follows.
Each token is the word alone; the look-ahead is not kept. On any other
input return false; the in-grammar lexer then reads the word as
`identifier`. Where a statement may start, `in` or `as` right before
`.` or `<`, which start no operand, is not decided here: the word is
read on as the statement's first name (§6.14, §6.21).
```

That last rule keeps a type whose name starts with `in` or `as` a type
after an expression: `foo()` then `in.gov.Foo x` declares `x`, as in
Groovy, where the operator's branch returned at the word and the line
was the access `in.gov.Foo` and the statement `x`. A reparse that
reuses the statement before reads the word where the operator is not
valid, and read the declaration, so the two parses differed.

The scanner tells the words from other names by their first letters,
so that `if`, `it` or `apply` after a statement costs a character or
two. Where COMMAND_VALUE_START (§6.14) or BARE_LAMBDA_START (§6.17) is
valid too, any name may start there, and it reads the whole name
first, as those tokens need, and decides the word only when the name
is exactly one of them. That place is an enum's first constant's
arguments, which are also a constructor's parameters (`A(var x) { }`),
and the parentheses after LAMBDA_START: `enum E { A(v -> v) }` and
`A(value -> value)` are lambdas, where a name that started like `var`
was turned down at its first letters and stayed a name, an `ERROR`
before `->`. `enum E { A(var -> var) }` is still one: the keyword's
token ends at the word, which the scanner marks before it reads on to
what follows, and the lambda's token is empty before the word, so one
scan cannot offer both; `var` is decided as the keyword there.

Apache's `yieldStatement` is `YIELD expression` with no `nls` (4.0.x,
5.0.x and master), and Groovy's lexer emits `NL` at a line break there,
so the value of `yield` starts on its line, as that of `return` does
(§6.23): `yield` then `x` on the next line is the name `yield` and the
name `x`, as in Groovy 3 too, which has no `yield` statement, and so is
`yield // c` or `yield /* c */` before the line break. It was a
`yield_statement` of `x`, at v0.6.0 as well. The binary `in` takes
`nls` after it, so its operand may start on a later line.

The grammar binds the tokens as `alias($._var_keyword, 'var')` and so
on, so the anonymous nodes, `node-types.json` and the query captures
are those of the literals, which are gone: no grammar literal spells
any of the four words, so none is ever a keyword of the in-grammar
lexer. `as` after an import's name is AS_KEYWORD as well, and the `in`
of `for (x in xs)` is IN_KEYWORD, which keeps the `for` header's one
fork between the loop variable and an expression (§7) on one token.

`var` alone has a token of its own because only where it may open a
multi-assignment can `var (` also be a call: `var(a, b)` and
`var (a, b).foo()` call a method named `var`, and `var (a, b) = l`
declares, in Groovy (observed with Groovy 3) and here. The look-ahead
reads only what a list of names and types holds, so it reads no `(`
and never the same text twice. After `final` the `var` before `(` is
always the keyword, as it was before `var` could be a name; that keeps
`final var (a = 1, b) = l`, wider than Groovy, a declaration. Where a
parameter and an expression may both start, as in the arguments of an
enum's first constant, `var` before `(` is a name that is called
(`enum E { A(var(1)) }`); it was the keyword there, and an `ERROR`.

Three choices keep every reading that did not depend on the word being
a name:

- A line break is not a boundary. `var` at the end of a line before
  `x = 1` declares `x`, as in Groovy (observed). `return var` followed
  by `x = 1` on the next line is still `return` and the declaration
  `var x = 1`, where Groovy returns `var` and assigns `x`; the scanner
  does not see what precedes the word. `String yield` followed by
  `foo()` is still `String` and `yield foo()`.
- `yield` before an expression is the keyword wherever a statement
  starts, not only in the arm of a switch expression, where Groovy 4
  and later take it (`{ inSwitchExpressionLevel > 0 }?`, read off
  `GroovyParser.g4`). So `yield x`, `yield (x)`, `yield -1`,
  `yield [1]` and `yield { }` outside a switch expression are
  `yield_statement`s, as before; Groovy calls, subtracts from or
  indexes a variable named `yield` there (observed with Groovy 3, in
  which `yield` is no keyword).
- `var` before any name is the keyword, `in` and `as` included.
  `var in xs` is the declaration of `in` followed by `xs`; Groovy
  declares `xs` with the type `in` (observed).

`instanceof` is never a name in Groovy, so `instanceof` or
`!instanceof` followed by a type makes the word before it a name that
is tested: `var instanceof T`, `yield instanceof T`,
`yield !instanceof T`, and `in instanceof T` on the line after an
expression, as in Groovy (observed with Groovy 3). The type may be on
the next line, as Groovy allows. Where no type follows, the word keeps
its reading: `var instanceof = 1`, which Groovy rejects, is still a
declaration of `instanceof`; taking the word as a name there would
turn that tree into an `ERROR`.

After `return`, `break` or `continue` on their line `var` is the value
or the label, as Groovy takes no declaration there: `return var` then
`x` on the next line returns `var` (observed with Groovy 3), where the
keyword declared `x`, and `if (c) return var` then `else x` declared a
local named `else`. A keyword after `var` that names nothing, being no
modifier and starting no operand, makes it a name too:
`if (c) var` then `else x` and `var` then `if (a) b` are a name and
the `else` or the `if`, as in Groovy. Before a keyword that starts an
operand the word stays the keyword, as it was before `var` could be a
name: `var new Foo()`, Groovy's `var(new Foo())`, stays an `ERROR`
rather than two statements, and `var true`, `var super` and `var` then
`null` on the next line declare locals named `true`, `super` and
`null`, with no `ERROR`, where Groovy calls `var(true)` and reads the
name `var` and then `null` (`docs/divergences-from-spec.md` §22).

`in++` and `yield++` update a name. After `++` or `--` the look-ahead
stays on the line, so the operator is the name's postfix update when
the line ends after it: `yield++` followed by `foo()` on the next line,
and `x = 1` then `in++` then `foo()`, are separate statements, as in
Groovy (observed). With an operand after it on its line the word is
the keyword: `x in ++y` and `yield ++y` are a membership test and a
`yield_statement`, the second where Groovy calls `(yield++)` with `y`
(observed). A line break between the word and `++` is crossed, so
`a in` followed by `++b` on the next line is `a in ++b`, as in Groovy
(observed).

In a type body `var` before a name is a modifier, as in Groovy, so
`var x = 1` there is a field `x` with `var` an anonymous child, as
`def x = 1` is (§6.21). VAR_NAMES_KEYWORD is valid there, after a
member's modifiers or alone: `private var (a, b) = l`. Before a name
that `(` follows, past trivia and line breaks, `var` stays a name, the
method's return type, as at v0.6.0: `var m() { }`, `@A static var
m(int a) { a }`. Groovy 3 rejects such a method ("var cannot be used
for method declarations"); Groovy 6's `methodDeclaration` takes `var`
among its modifiers, and its AST builder rejects the method ("val/var
cannot be used for method return types"). As the keyword the word
opened a field, and the parameters were an `ERROR` that could take the
members after them out of the type. The scanner tells a type body by
its valid tokens: VAR_NAMES_KEYWORD is valid where a member's
modifiers may end, and DECLARATION_START wherever a statement or a
`for` header starts, which no type body holds; after `final` in a
block neither is valid. No member starts with `(`, so the look-ahead
may cross a line break, as the grammar does between a method's name
and its parameters (`var m` then `()` on the next line).

The key of a named argument (§6.8) may be one of the words:
`foo in: 1`. After a statement's first name the scanner reads the word
to tell `it in: 1` from `it in xs`, and its keyword must end at the
word, which an empty COMMAND_NAMED_KEY cannot do once the spaces after
the word are read (`foo in : 1`). Such a key is `CONTEXTUAL_KEY`, which
spans the word; the grammar aliases it to `identifier`, so the tree is
the one of any key. An empty key aliased to `identifier` was tried
first: error recovery then used it as a key of no length.

`int` is the one element type name (§6.15) that shares a first letter
with the four words, and a statement may start with `int[].class`
where `in` may follow the statement before it, so ARRAY_VALUE_START is
decided in the same branch. Each word is turned down at its first
character that differs (`if`, `it`, `implementation`, `apply`,
`value`), which is what keeps the cost below. Where a statement may
start, COMMAND_STATEMENT_START (§6.14) reads the whole name first and
the four words are decided from it, with the same result. The tokens
are emitted during error recovery, as RECORD_KEYWORD is: each spans the
word, and the look-ahead decides as it does on valid input.

Known residuals, each an `ERROR` as before:

- `for (var in xs)`, a loop variable named `var`: the keyword takes
  `var` because a name follows.
- A method named `in` with a return type where a statement starts,
  `int in(int x) { x }`: before `(` the word is the operator, as in
  `a in (b)`. Groovy declares the method (observed). In a type body
  the method parses. `as` has no such case: `int as(int x) { x }`
  parses, as `as` before `(` is a name.
- `final var(x) { }`, a script method named `var` with the modifier
  `final` alone, which Groovy declares (observed).
- A typed loop variable named `in` or `as` with the `in` form,
  `for (String in in xs)` and `for (int as in xs)`, which Groovy takes
  (observed). After `String`, which a classic `for` may also read as
  an expression, the first `in` is the operator because a name follows
  it. The `:` form parses.
- `yield(1, 2)`, which Groovy calls (observed): before `(` the word is
  the keyword unless `)` follows, so it is a `yield_statement` with an
  `ERROR` in its parentheses.

`println var` and `println yield` are calls, as `println x` is
(§5.5 item 3); `println in` and `println as` are two statements, as the
words go on with the receiver as operators. As a receiver where a
statement starts, none of the four takes a name argument (§6.14):
`in x` is two statements, where Groovy calls `in(x)`. In a value they
do: `def x = in y` and `(in y)` are calls, as in Groovy.

Four kinds of tree change for input that parsed without an `ERROR`,
found by fuzzing the grammar before against this one and checked
mechanically on every case found: in each, the new tree is the one
the old grammar gave the same input with an ordinary name, or with
`def`, in place of the word.

- Where two readings fork, one could die at the word because it was a
  keyword there, and the other then won. Now both live and the dynamic
  precedences rank them, as for any name. `x = (b)` followed by `in` on
  the next line was a cast of `in`, and is now two statements, as in
  Groovy (observed); `x.y` followed by `var(a, b)` was a method
  `x.y var(a, b)` without a body and is now two statements;
  `in f(var.List<String> items) { items }` is now a method, as
  `foo f(bar.List<String> items) { items }` is.
- `var (a, b) = l` on the line after one that ends in a parenthesized
  name, `x = (b)` then `var (a, b) = l`, was the cast `(b)` of the call
  `var(a, b)`, assigned `l`. It is now two statements, as with
  `final` (§4), and as in Groovy (observed).
- `var instanceof T`, `yield instanceof T`, `yield !instanceof T` and,
  after an expression, `in instanceof T` and `in !instanceof T` were a
  declaration of `instanceof`, a `yield_statement` or a membership
  test, followed by `T`. Each is now the test of the word.
- `yield++` or `yield--`, and `in++` or `in--` after an expression,
  at the end of a line before an expression on the next (`yield++`
  then `foo()`) were a `yield_statement` or a membership test of a
  prefix update. Each is now the postfix update of the name, and the
  next line a statement of its own.

The last two kinds can turn a tree without an `ERROR` into one with
an `ERROR`, the `ERROR` an ordinary name gets in the same place. After
`yield++` the next line is read as a statement, so a line this grammar
does not take there is now an `ERROR`: `yield++` followed by
`private int f` or `static count`, a local with a modifier, which
Groovy declares, was `yield ++private` and `int f`. And `instanceof`
followed by a word reads as a test even where Groovy rejects it as
anything: `@A` followed by `var instanceof T`, `var instanceof new
Foo()`.

Whether the word is a keyword depends on what follows it, and the
runtime counts the scanner's look-ahead past the word against the
word's own token only, so an edit after the word could leave a
finished node before it in place on reparse. No such reparse is
known. `x = a` followed by `in = 1` on the next line is an assignment
and a second statement; editing the second line to `in y = 12` makes
the text the assignment `x = ((a in y) = 12)`, on reparse as from
scratch, and `import a.B` followed by `as = C`, edited to `as l= Cx`,
gets the alias `l` both ways (`bindings/rust/tests/incremental_reparse.rs`
holds both). No reparse differed over 32,033 single edits of 24 texts
that hold the four words, nor over 175,077 pairs of edits of those two.

Measured against the grammar before: STATE_COUNT 4179 to 4249,
LARGE_STATE_COUNT 1555 to 1565, six external tokens and no conflict
or precedence added, and the same number of parse steps and of steps
with more than one stack version on real files. Real Groovy and
Gradle files and Java files take about 0.1% to 0.2% more instructions
(about 0.8% in wall-clock time, near the noise of a second build of
the same grammar). Code dense with `as`, `in`, `var` or `yield` parses
within about 2% of before, faster or slower depending on the code.

### 6.17 LAMBDA_START and BARE_LAMBDA_START — implementation contract

A lambda (§5.16) starts like an expression the parser has to treat
otherwise until the `->`: `(a)` is also a parenthesized expression, a
cast's type and the start of `(a, b) = …`, and `x` in an argument or a
value is also a name. Deciding at the `->` cost either a fork at every
`(identifier)` or, for `x`, a reduction of every name in an argument
or a value of its own: tree-sitter skips the step from a name to an
expression only in a state with no other action, and the shift of the
`->` after the name is one. On calls whose arguments are names that
was a fifth more reductions and 8% more instructions. The scanner
decides before the `(` or the name instead, as it does for the
receiver of a parenthesis-free call (§6.14).

```text
Emit LAMBDA_START, empty, at a `(` where an expression may start, when
NOT_IN is not valid (no expression ends here; after one the `(` opens a
call's arguments), and:
  - after the `(` and Groovy's trivia (whitespace, line breaks, line
    escapes and comments), `)` follows, or parameters separated by `,`,
    each read as: annotations, each `@`, a dotted name and arguments in
    parentheses read token by token with brackets counted; words and the
    punctuation of a type (`.`, `...`, `[`, `]`, and `<` … `>` with `,`,
    `?`, `&` and a wildcard's bound, `extends` or `super`, inside); a
    last word that may name the parameter (no
    keyword of grammar.js other than `final` and `def`, `_`, `permits`
    and `pipeline`, no primitive type name and no `void`, not right after
    one `.` and not inside `<…>`); and after `=`, not `==` or `=~`, a
    default value read token by token, brackets counted, strings
    skipped whole (a triple-quoted one across lines, a `\` before a
    line break continuing a single-quoted or double-quoted one, a
    `${…}` read as code with the strings in it, a dollar-slashy one
    across lines within 1,024 characters, §5.7), a slashy string where
    an operand may start and no `=` follows its `/` skipped whole when
    it closes on its line (one that does not turns the token down), and
    the `,` of type arguments that a `<` right after a capitalized name
    opens taken for theirs, to the `,` or `)` that ends it. A
    parameter with no such name (`(int)`, `(a.b.C)`, `(String[])`) is
    read on as well, and the token is turned down after the `)`;
  - after the `)` and Groovy's trivia, `->`;
  - all within LAMBDA_PARAMETERS_LOOKAHEAD_ITEMS (256) items: a word, a
    string, a comment, an annotation or a punctuation character.
Emit BARE_LAMBDA_START, empty, before a name where the grammar takes a
lambda without parentheses (an argument, the value of a map entry, a
declarator, an assignment other than `(a, b) = …`, a closure
parameter's default, what is in parentheses, a condition), when after
the name and spaces, tabs and line escapes comes `->`, or a `-`, `/`
or line break after which Groovy's trivia lead to `->`, and the name
is none of the words above that cannot name a parameter, nor `this`,
nor the `$` of a dollar-slashy string (`def glob = $/*.txt/$`, §5.7).
Neither is emitted during error recovery.
```

The look-ahead reads the shape, not the grammar: what it takes and
`formal_parameter` does not, the order of modifiers for one, is an
`ERROR` either way, as it was before lambdas. Its early stops make it
cheap where no lambda is: `(a + b)` ends at the `+`, `(x == y)` at the
`==`, `(foo())` at the second `(`, `('s')` at the quote, `((a))` at
the inner `(`, and `(String) x` at the `x` after the `)`. A name with
no `-`, `/` or line break after it costs one read of the name. Where
COMMAND_VALUE_START or ARRAY_VALUE_START is valid too, the name is
read once for all of them (`scan_value_start`,
`scan_bare_lambda_start`), and a `-`, `/` or line break after it
starts no first argument of a call, so a value that is no lambda there
is no call either.

In a default, a `<` right after a capitalized name with no bracket open
opens type arguments, whose commas end no parameter, as in §6.18:
`(m = new HashMap<String, Integer>()) -> m` and
`(a = x as Map<String, Object>, b) -> a` are lambdas. At anything type
arguments do not hold the `<` was an operator, and the commas read
since are not read again: the parameters they started are read as the
default's tokens, so `(a = X<b, c = d) -> a` is a lambda with the
parameters `a` and `c`, as in Groovy, and `(a = X<b, int) -> a`, which
Groovy rejects, one with a parameter named `int`. So is
`(Foo<super> g) -> g`, which Groovy rejects too, as `super` and
`extends` bound a wildcard in type arguments
(`(List<? super T> l) -> l`).

The look-ahead does not run after an expression, where NOT_IN is
valid. There a `(` is a call's arguments unless the next statement
starts, and reading every call's arguments for the rare lambda
statement after an expression would cost more than it is worth:
`foo (x) -> x`, which Groovy reads as `foo((x) -> x)`, stays the call
`foo(x)` and an `ERROR`, and so does `foo()` followed by `(a) -> a` on
the next line.

One place where NOT_IN is valid is a statement's end all the same:
after the `}` of the block that is the body of `if`, `else`, a loop or
an arrow case, where the braces may also be a closure, the statement's
expression, and the weight of the block ranks it first. A reparse that
reuses the statement lexes the `(` after it at a statement's start,
where the look-ahead runs, so `if (c) { a() }` followed by `(x) -> x`
was an `ERROR` from scratch and a lambda statement after an edit
elsewhere. BODY_BLOCK_END, which is never emitted, is valid after such
a `}` (`_body_block` in `grammar.js`), and there the scanner reads the
`(` as at a statement's start, the lambda look-ahead included, so both
parses read the lambda statement, as Groovy does on the next line. On
the line of the `}` too: `if (c) { a() } (x) -> x` is the `if` and a
lambda statement, where Groovy reads one `if` statement. The token
costs two states (STATE_COUNT 4,656 to 4,658, LARGE_STATE_COUNT
1,788 to 1,789); no tree changes for
input that parsed without an `ERROR` (`docs/divergences-from-spec.md`
§26), and `bindings/rust/tests/incremental_reparse.rs` pins the
reparse.

The budget keeps the parse linear. A default value may hold a `(`
where an expression starts, which the scanner reads again, so without
it `x = (a = (a = (a = …` was read in time quadratic in the nesting
(the bytes the parser asks for per input byte went from 256 to 2,007
for 500 and 4,000 levels; with it, 70 and 74).
`bindings/rust/tests/lambda_parameters_cost.rs` checks that, nested
parentheses, a long list of names with no `->` and many lambdas on one
line. A parameter list longer than the budget reads as no lambda's.
A string is one item, so a triple-quoted one that does not close is
read to the end of input by each look-ahead that reaches it within the
budget: at most a fixed number of them, as many as for a string that
does not close on a long line, so the parse stays linear (`(a =`
repeated, then an unterminated `'''` and 9,000 lines: 93 bytes read
per input byte whether `(a =` comes 85, 400 or 1,000 times; the same
test checks it, and `name_argument_cost.rs` the closure parameters of
§6.14 before such a string). The same test checks defaults with type
arguments, also where the `<` turns out to be an operator. The `${…}`
of a double-quoted string is read as Groovy's lexer reads it, as code
with its braces counted and the strings in it read whole
(`(a = "${x.join(")")}") -> a`), and so in every look-ahead that reads a
string whole (§6.14, §6.18, §6.21). A string nested more than
STRING_NESTING_DEPTH (8) strings deep is taken not to close, which
ends the look-ahead: without the bound a run of `"${` that never closes
would be read to the end of the input from each of its quotes, in time
quadratic in its length; real strings nest once or twice. A
dollar-slashy string is one item as well, read for its `/$` within
DOLLAR_SLASHY_LOOKAHEAD_CHARACTERS (1,024) characters (§5.7): its opener
and its closer differ, so in a nest of interpolations that each open
another (`$/ ${ (a = $/ ${ (a = …`) every look-ahead would read to the
end of the input, and past the bound a string ends the look-ahead as
one that does not close does.

The scanner reads only the shape and the `->`, and an expression may
be followed by `->` as a closure parameter's default or a case value
(§5.16). The grammar keeps those readings next to the lambda's: after
LAMBDA_START a parenthesized expression, after BARE_LAMBDA_START at a
closure parameter's default a name. Both tokens are empty and sit
before what they open, so a comment between them and the `(` or the
name cannot hide a line break (§6.14).

Because the look-ahead at a `(` reads through the `)` and the trivia
after it, for a cast's type as well as for parameters, an unterminated
`/*` there is read to the end of input, once per `(` that reaches it
within the budget. That fixed the one reparse that differed from a
parse from scratch (§6.6), for a name, a primitive, a dotted, an array
and a generic type alike.

One reparse differs that this look-ahead decides. Where a script method's
head forks before its `(` (`final g`, `final def g`, an annotation and
`final`), the method's version reads its own `(` in a state where no
lambda may start, and the look-ahead runs only for the other version,
which ends at the first typed parameter's name. The method's nodes are
then not fragile, and a reparse reuses a method that has no body after an
edit below its `)`: `foo()`, `final g`, `(int a, int b)`, then `-> a`
typed on the next line, is a lambda from scratch and an `ERROR` on
reparse (`bindings/rust/tests/incremental_reparse.rs` pins it; §6.20).

Another reparse differs where a closure parameter's default and a
lambda's body share a parenthesized expression (§5.16).
`def d =  T a = (b, c) -> (d) -> e }` with the space at byte 8 replaced
by `{` is `def d = {T a = (b, c) -> (d) -> e }`, which a parse from
scratch reads as the closure parameter `T a` with the default
`(b, c) -> (d)` and the body `e`, Groovy 3's reading; the reparse
reuses the old `lambda_expression` `(d) -> e` and reads a closure whose
body declares `T a = (b, c) -> (d) -> e`. Neither tree has an `ERROR`
(`bindings/rust/tests/incremental_reparse.rs` pins both trees).

Nested lambdas whose bodies are expressions parse in time quadratic in
the nesting, a known cost limit. The parenthesized reading after
LAMBDA_START (`_lambda_parenthesized_expression`, kept for a closure
parameter's default and a case value) is reachable wherever an
expression starts, and the stack version that holds it reaches the
`->` after an expression, where nothing takes it; before it dies there
it reduces every lambda still open before it, as the parentheses may be
the whole body of each. `x = (a) -> (a) -> … a`, `(a) ->` repeated,
takes 32,378, 127,253 and 504,503 reductions for 250, 500 and 1,000
levels, about half the square of the depth, and 0.07, 0.29 and 1.1
seconds for 1,000, 2,000 and 4,000 levels, 28.6 seconds for 20,000
(140 KB); the same nest with block bodies takes 1,754, 3,504 and 7,004
reductions. A cast's operand (`(T) (a) -> …`, 4.8 seconds at 4,000
levels), a sum (`(a) -> 1 + …`), a ternary's branch
(`a ? b : (c) -> …`) and nests in a closure parameter's default, a
closure's first statement and a case value (2.1 to 2.4 seconds at
4,000) are quadratic too. Nests with block bodies, without parentheses
(`a -> b -> c`, an `ERROR` as in Groovy), in arguments
(`f((a) -> f((a) -> …))`) and of closures are linear. A grammar fix was
prototyped with derived "arrow tail" twins of rules (172 more states
with two contrived divergences, or 236 with none). It left the cast,
closure default, first-statement and case-value nests quadratic, so it
was not taken; the patches are kept outside the repository. Real code
nests a few lambdas.

Measured against the grammar before: STATE_COUNT 4330 to 4516 (+4.3%),
LARGE_STATE_COUNT 1575 to 1693, `parser.c` 8.2 to 8.9 MB, two
external tokens, one precedence (`PREC.LAMBDA`, below assignment, so
the body takes all it can), three dynamic precedences (`LAMBDA_WEIGHT`,
`BODY_BLOCK_WEIGHT`, `DEMOTED_LAMBDA_WEIGHT`) and one conflict
(`[_bare_lambda_name, _bare_lambda_expression]`) added. The rankings
of §5.16 account for 26 of the states. About 0.6% more instructions on
real Groovy and Gradle files and on Java files without a lambda, 1.5%
on code made of calls with names for arguments and 2% on code made of
parenthesized expressions, casts, or `if` and loop statements, whose
block bodies take one more reduction each, and 5% on code of closures
of which a tenth have a name as their first parameter's default
(`{ a = b -> c }`), where the parser keeps the parameter and the
lambda readings to the end;
Java files with lambdas, which were errors, take 11% fewer. A build
with the tables and none of the look-ahead ran as many instructions as
the grammar before, so the rest is the scanner's reading. Two other designs were measured and
dropped: deciding `x ->` in the grammar (+3.6% states, a fifth more
reductions on such calls), and deciding it by context to keep the
closure readings (+8% states, as every expression state in a closure
parameter's default and a case value got a copy of its own).

### 6.18 STATEMENT_PAREN — implementation contract

Apache's `pathElement` takes a call's `arguments` with no `nls` before
them, unlike a closure (`nls closureOrLambdaExpression`) and the member
accesses, and Groovy's lexer makes a line break a token outside `(…)`,
`[…]` and `?[…]`. So a `(` that opens a line after an expression opens
the next statement: `foo` followed by `(1)` is two statements, so is
`foo()` followed by `(a, b) = [1, 2]`, and `println x` followed by
`(1..3).each { }` is a call and a second statement (observed with
Groovy 3, in a script, a method, a closure, a block, a `case` and a
braceless `if`). Inside parentheses and brackets the `(` calls:
`bar(foo` followed by `(1))`, `[foo` followed by `(1)]`. A line escape
joins the lines (§5.10), and so does a block comment that spans the
line break (`foo /* a` followed by `b */ (1)`), which Groovy's lexer
drops with its line breaks. A newline is whitespace here, so the
grammar alone took that `(` as the arguments of a call on either line.

A call's arguments take only the in-grammar `(`. The rules that may
open a statement with one, a parenthesized expression, a cast and a
multi-assignment without `def` (`statementParen` in `grammar.js`), take
STATEMENT_PAREN as well, which the scanner emits only where such a `(`
opens the next statement.

```text
Emit STATEMENT_PAREN for a `(` when NOT_IN is valid (an expression ends
here) and STATEMENT_PAREN is, and:
  - a line break comes before the `(`: spaces, tabs and line escapes
    are skipped, and a `(` on the line is turned down at once; and
  - the parentheses can open a statement. They cannot, and stay the
    arguments of a call, when they are empty (`()`); when an element is
    a spread (`*a`) or a named argument, a key of any kind and `:` that
    is not `::` with no `?` before it in the element (`a: 1`, `'a': 1`,
    `"$a": 1`, `1: 2`, `(a): 1`), other than an argument of the call
    below; when `,`, `->` or `else` follows them; or when they hold a
    `,` with no bracket and no type arguments open, unless they hold
    only names and `=` that is not `==` or `=~` follows them
    (`(a, b) = l`), or the first element is a parenthesis-free call: a
    name that is no value keyword (§6.14), its member accesses, and on
    its line a name other than `in`, `as` and `instanceof`, a number or
    a string (`(foo 1, 2)`, `(sh script: 'x', returnStdout: true)`).
  - Type arguments open at a `<` right after a capitalized name
    (`(data as Map<String, Object>)`, `(Map<K, V>) m`) and close at the
    matching `>`. At anything else than a name, `.`, `,`, `?`, `&`, a
    bracket or another `<` or `>` inside them, the `<` was an operator,
    and the commas read inside them count.
  - The elements are read as Groovy's tokens: brackets counted,
    strings read whole (a triple-quoted one across lines, a `\` before
    a line break continuing a single-quoted or double-quoted one, a
    `${…}` read as code with the strings in it, §6.17), inside an
    element a slashy string where an operand may start and no `=`
    follows its `/` read whole to its close on its line, so that its
    `,` and `:` are none of the element's (`(text =~ /a, b/)`), and
    comments skipped, within LAMBDA_PARAMETERS_LOOKAHEAD_ITEMS (256)
    items (§6.17). Where the reading cannot go on, at the end of the
    budget or the input, at a string or comment that does not close, at
    such a slashy string that does not close on its line, at a bracket
    that does not match, at a `/` that opens an element, and at the
    `$/` of a dollar-slashy string anywhere in one, which this reading
    does not enter (`(foo $/a/$, 2)`, §5.7), the parentheses are judged
    as if
    they closed there: they open a statement unless a `,` came, and
    then only when the first element is a parenthesis-free call.
The token is the `(` alone. On any other input return false; the
in-grammar lexer then reads the `(` of a call's arguments. It is not
emitted during error recovery.
```

STATEMENT_PAREN is valid after an expression exactly where a statement
may start, as only rules that open a statement take it. Inside
parentheses and brackets no statement may start, so the `(` on a later
line is a call's arguments there, as in Groovy, and the scanner needs no
stack of brackets for it. That also holds where Groovy keeps the line
break but no statement may start here: the `(` continues the expression
in a class body (`def x = foo` followed by `(1)`) and in a ternary's
middle (`a ? foo` followed by `(1) : 2`), both of which Groovy rejects,
and in an interpolation (`"${foo` followed by `(1)}"`), which Groovy
reads as a closure of two statements, which an interpolation here does
not hold (§5.5 item 1). The arguments of an enum constant, a creation
and an annotation, a method's parameters and the `(` after `if`,
`while`, `catch` and the like keep the in-grammar `(`, and may follow a
line break as before; Groovy allows that for a creation and an
annotation only.

Groovy rejects a line whose parentheses cannot open a statement:
`foo` followed by `(a, b);`, `(a: 1)`, `('a': 1)`, `(*a)` or `(x), y`.
Java, which much Groovy code is written like, reads a call, and so did
the grammar before, so the parentheses stay its arguments and the tree
is the one from before, whatever the kind of key or element. Read as
statements, such lines turned 72 of 24,548 Java files without an
`ERROR` into files with one (`return GetInstance.getInstance` followed
by `(type, getSpiClass(type), algorithm).toArray();` on the next line,
as the JDK wraps long calls); with the look-ahead none does. An `else`
after them makes the line above the consequence of an `if`:
`if (x) foo` followed by `(1) else bar` keeps its `else`, which as two
statements would be left a name. The verdict once the reading stops
early is the one the rest of the parentheses most often confirms, and it
reads an unfinished line the way it is read once closed: `(a, b` while
it is typed is a call's arguments, as `(a, b)` is, and `(a + b` a
statement, as `(a + b)` is. A long list whose first element is no name
is decided at its first `,` and read no further. Before `->` Groovy
appends the lambda to the expression (`pathElement`), which this
grammar does not take (§5.16), so the parentheses stay the arguments
there as well: `def c = {` then `x = y` then `(a) -> a` keeps the
closure parameter `x = y(a)`, where Groovy reads `x = y((a) -> a)`.

A comment is an extra, so after one the scanner is called past the line
break it can no longer see: `foo` followed by `/* c */ (1)` on the next
line is a call, where Groovy reads two statements, as for `[` (§6.9).

Measured against the grammar before: STATE_COUNT and
LARGE_STATE_COUNT unchanged (4516 and 1693), `parser.c` 8,853,428 to
8,904,570 bytes, one external token, no conflict or precedence added.
The token was listed last but for `_error_sentinel` when it was
added (later tokens follow it now): after
`_subscript_open` it renumbered the symbols after it, and error
recovery then wrapped `enum E<T> { A }` in an `ERROR` with a closure,
where it keeps the `enum_declaration` with an `ERROR` at `<T>`.
The parse has the same number of steps, steps with more than one stack
version, shifts and reductions on the 264 real files and on 391 of 400
Java files whose trees do not change, and on code made only of calls.
Real Groovy and Gradle files take 0.4% more instructions, Java files
0.5% to 0.7%, code made of calls or parentheses 0.2% to 0.3%, code made
of closures, conditions or class bodies 0.4% to 0.5%, of which reading
the parentheses where no expression ends (below) is about 0.2%; code
made of lines that start with `(` after an expression (`foo`, then
`(a, b) = l`) takes 10% more, for trees that differ and the
look-ahead. A token for the `(` of every call instead, with the line
rule at the call, gave the same trees for valid input and was dropped: STATE_COUNT 4516 to 4552,
LARGE_STATE_COUNT 1693 to 1721, 0.7% more instructions on real files
and 3.8% more on code made only of calls, as an external token is a
leaf of its own that the parser keeps on the heap, where the in-grammar
`(` is stored inline.

A closure in the parentheses holds statements, so a nest of `x` and
`({ x` lines puts such a `(` on every line, and each reads the rest of
the nest. The budget keeps that linear: 54 and 57 bytes asked for per
input byte at 500 and 4,000 levels, 204 and 1,565 without it
(`bindings/rust/tests/statement_paren_cost.rs`).

The 10% above was measured on lines that open with `(` after a name
(`foo`, then `(a, b) = l`). On lines that each hold parentheses alone
(`(x)`, `(a.b)`, `(a) = (b)`) the `)` of every line may also close a
cast of the next line, the reading ranked last (§5.15); with this `(`
opening a statement, that reading stayed alive and was reduced again
at every line, in time quadratic in the length of the run: 2.1 s for
4,000 lines of `(x)`. The `)` before such a `(` now closes no cast
where no bracket holds both lines (§6.11), and those runs are linear
again (12 ms).

Whether the `(` opens a statement decides whether the statement before
it ends, and it depends on what the parentheses hold. That statement
ends before the line break and its own look-ahead does not reach the
parentheses; the runtime counts what the scanner read against the `(`,
so an edit inside the parentheses makes it lex the `(` again and parse
the statement before anew. A reparse that reused that statement lexes
the `(` at a statement's start instead, where STATEMENT_PAREN is valid
but no expression ends, so the rule is: wherever STATEMENT_PAREN is
valid and a `(` follows a line break, the scanner reads the same text
whether or not NOT_IN is valid. Where it is not, the lambda look-ahead
(§6.17) reads first, and the scanner then reads on to the `)` and past
the trivia and the word, `=` or `-` after it, and emits nothing.
Without that, `foo` then `(c, d) = [3, 4]`, edited by a space after the
`=` and then by a second `=`, kept the multi-assignment with an `ERROR`
on reparse, where a parse from scratch reads `foo(c, d) == [3, 4]`.

One limitation remains. The runtime reuses a node, and checks whether a
reused node must be broken down, only while a single stack version
exists, and a typed declaration after `final` keeps two alive to its
end, so the statement after it is reused at the fork although it no
longer ends there. In `final String s = "a"` then `println x` then
`(a)`, inserting `,` before the `)` makes the parentheses `x`'s
arguments, and the reparse reads the first line as `final String` and
`s = "a"`, where a parse from scratch reads a
`local_variable_declaration`; `bindings/rust/tests/incremental_reparse.rs`
pins it. The class is wider than that one example: the reused statement
may be any call with one name argument (`foo bar` then `(a, b)` does
the same), the edit may make the parentheses `(a, b)`, and the
declaration may have annotations or an array type
(`@A final String s = "a"`, `final String[] s = []`). Of the shapes
tried, a primitive, generic or dotted type (`final int i = 0`,
`final List<String> s = []`, `final a.B s = 1`), a reused statement
with two name arguments or a dotted receiver (`foo bar baz`,
`x.y z`) and one that is a name alone do not reproduce it. Without `final`
the scanner opens the declaration with DECLARATION_START (§6.21), which
then does not fork, and `String s = "a"` in the same place reparses as
a parse from scratch reads it. The counts below are from before that.
Over 240,000 random sequences of four edits of inputs with such lines, 22 end in a
reparse that differs from the parse from scratch under the grammar
before, 1,697 without the rule above and 135 with it, each of which a
single edit reproduces. Of 1,200 sequences that type such a line one
character at a time, 27 differ, all typing `(a, b)` two lines after
`String s = "a"`: for the four keystrokes from the `,` to the `)`, after
which the trees agree again. The grammar before had 18, 12 of them to
the end.

The same limit shows in a type body, where `final String z` before
`static class C { }` on the next line is also `final String` and a
field `z` named `static` before `class C { }`, which
MODIFIER_ONLY_FIELD_WEIGHT ranks below. Typing `w` after a last
`final String`, which the `}` after it made a field named `String`,
reuses that field where one stack version is left; the reduction after
it splits the two readings of the lines before, the version that holds
the reused field meets an `ERROR` at `w` and ends, and the other reading
stays, with no `ERROR`. It is the same at v0.6.0; the fuzz seed
4420640461419634763 found it on a record's corpus test, and
`bindings/rust/tests/incremental_reparse.rs` pins it.

A statement that forks in two readings until a later token decides
shows it too: `(a) { }` is the cast of the closure or the call of `(a)`
with it, and the two stay alive past the statement after it. In
`(a) { }` then `foo { yield / 2 }.bar()` then `(b) { }`, deleting `b`
leaves `()`, which opens no statement and goes on with the call before
it; the reparse reuses that call whole at the fork and keeps the call
reading of `(a) { }`, where a parse from scratch keeps the cast, both
with no `ERROR` (`x++ / 2` in the closure does the same; at v0.6.0
neither text parses without an `ERROR`). The class is a fork that a later line decides, such as
`(x) { }`, a `return`, `throw` or `assert` before it (§6.2), or a typed
declaration after `final`, followed by a statement that is reused whole
while both readings are alive; `bindings/rust/tests/incremental_reparse.rs`
pins this one with the others.

Trees that change for input that parsed without an `ERROR`: a `(` that
opens a line after an expression where a statement may start, and that
can open one, was the arguments of a call of that expression, and opens
the next statement. That holds for the multi-assignments of
`docs/divergences-from-spec.md` §3, for a value's or a named argument's
last argument (`def x = foo 1` then `(c)`, `foo a: 1` then `(b)`), and
for a cast on the next line (`x = (a)` then `(b) y`). Groovy reads two
statements in each (observed with Groovy 3). Of the 1,261 corpus tests
7 change, all of them divergences that this removes; of 266 real Groovy,
Gradle and Jenkins files 2, both a cast on the line after a call that
had taken it as an argument; of 24,548 Java files 105, each a call whose
one argument opens the next line, and 233 that had an `ERROR` change it
(2 with fewer bytes in it, none with more). No file gains an `ERROR`.
Of 176,400 inputs made from a template of a line, a line break and
parentheses, 76,032 change, and each has as many top-level statements
as Groovy 3 reads; none gains an `ERROR`. Of 237,660 inputs made from
other templates and from mutations of those files, 25,897 without an
`ERROR` changed: inserting `;` before the line break of such a `(` gives
the new tree under the grammar before for 25,858, and the rest are
inputs where that check cannot be applied mechanically (a `//` inside a
dollar-slashy string, a line escape before the line break). 5 gained an
`ERROR`; Groovy 3 rejects 3, and reads the other two as a lambda
appended across the line break. Of the 16,125 with an `ERROR` before
and after, 658 have more bytes in it and 1,537 fewer. An `ERROR` also
appears where the new statement is one this grammar does not take:
`foo bar` followed by `(List<String> a) { }` was the chain
`foo(bar(…) { })`, and is `foo bar` and a statement with an `ERROR`,
where Groovy calls `List<String>(a, { })` (§5.5 item 3).

### 6.19 LAMBDA_BODY_END — implementation contract

A lambda's body is a brace body only where nothing after the `}`
continues an expression (§5.16): `x -> { y -> y }` is a closure body and
`x -> { y -> y }()` a call of that closure. The grammar cannot see that
at the `}`: a closure as the whole body ranks below a block that holds
lambdas (`DEMOTED_LAMBDA_WEIGHT`), and a static precedence on it, which
kept a closure that an expression went on with from carrying no weight,
also kept the expression from going on. `x -> { y -> y }()` was an
`ERROR`, `x -> { y -> y } { z }` two statements and
`x -> { y -> y }[0]` a lambda and a list. The scanner decides it after
the `}` instead, with an empty token that the brace bodies take: a
closure and the block of a lambda without parentheses need it, the
block of a lambda in parentheses may take it, and without it the braces
are a closure that starts the body. A block that opens a statement ends
by the same scan, with a token of its own (§6.22).

```text
Emit LAMBDA_BODY_END, empty, at the `}` that may end a lambda's brace
body, before the trivia after it, unless an expression continues there.
Read after the `}`: spaces, tabs, line escapes, line breaks and up to
LAMBDA_BODY_END_COMMENTS (8) comments, then one token.
  - Any operator, `(`, `{`, `::`, `instanceof`, `!=`, `!in` and
    `!instanceof` as the scanner and the grammar read them, `as` or `in`
    that open a type or an expression (§6.16), and on the line of the
    `}` a `[` where SUBSCRIPT_OPEN is valid (§6.9) continue it.
  - After a line break where a statement may start (STATEMENT_PAREN
    valid, §6.18), `(`, and `+` and `-` other than `+=` and `-=`, which
    Groovy's grammar does not take after `nls`, end it, as they start a
    statement.
  - After a line break where a statement or a class member may start
    (STATEMENT_PAREN or RECORD_KEYWORD valid, §6.7), `{` and a `<` that
    opens type parameters (`@`, or a name and then `>`, `,` or `extends`,
    on its line) end it, as they start a block or an initializer and a
    generic method or constructor.
  - After a line break where a statement may start, a `/` that opens a
    slashy string ends it, as Groovy reads a statement; any other `/`
    continues it, and so does every `/` on the line of the `}`, after
    which Groovy's lexer starts no slashy string
    (`def f = x -> { x } /2/ 3` divides).
Anything else ends the body: `)`, `]`, `}`, `,`, `;`, `:` other than
`::`, a word, a literal, a `[` on a later line, the end of the input, a
`/*` that does not close (or one the §6.6 bit says cannot), and a ninth
comment. Inside parentheses, brackets and interpolations neither
STATEMENT_PAREN nor RECORD_KEYWORD is valid, so there a line break
changes only what `[` does.
Where it continues, the scan returns false and the in-grammar lexer
reads the operator, unless the scan read past the start of the next
token: a comment came first, or the next token is `as` or `in`, which
the scanner emits. Then it emits UNTERMINATED_COMMENT (§6.6), empty,
with a second state bit set, and the next call, at the same position and
in the same state, reads that token without deciding again and clears
the bit; a comment lexed so reads on as far as the decision did. `!in`
and the `[` of a subscript, which the scan has just read, it emits
itself.
Where LAMBDA_BODY_END is valid and NOT_IN is not, only LAMBDA_BODY_END
is: the parser shifted the block of a lambda without parentheses whole,
reused from an old tree or read beside the closure the braces also are.
There decide as above with STATEMENT_PAREN and RECORD_KEYWORD invalid
and every other token valid, so that the body ends only where it ends
in every state after the `}`, and where it continues emit
LAMBDA_BODY_REREAD, empty, which no rule takes.
At a `}` where STATEMENT_PAREN is valid, which closes a list of
statements, and that a comment or a line escape follows on its line,
read as far as above with every token valid and return false.
None of these is emitted or read during error recovery.
```

The token ends at the `}`, so the lambda keeps its range, and where the
body ends a comment after the `}` stays outside the lambda, which the
second bit makes possible: a token that ends at the `}` cannot be one
that ends after the comment, and deciding after the comment would put a
comment before the token, and so in the lambda. Where the body goes on,
the comment is inside the expression that continues it, as after any
operand. The bound on comments keeps the scan's cost fixed: after the
`}` and after each comment of a run before an operator the scan reads
the rest of the run again, k² + k comment reads for k comments, 72 at
the bound. Runs of comments between a body and an operator are a line
or two in practice; a longer run ends the body, and the operator then
applies to the lambda (`docs/divergences-from-spec.md` §26).

The rules after a line break follow Groovy's `nls`, where Groovy and the
grammar both continue the operators it lists after it (`x` then `.y` on
the next line), and where Groovy starts a statement or a member, the
grammar's other rules for those characters read one too: a `(` that
opens a line (§6.18), a `[` on a later line (§6.9), a block or an
instance initializer, and a generic method. A `{`, `as` and `in` on the
next line continue a closure in Groovy and start a statement after a
block, which the scanner cannot tell apart; `{` ends the body and `as`
and `in` continue it, the readings of the grammar before. An
interpolation, which holds one expression here and statements in Groovy,
takes no statement after the lambda, so an operator on the next line
continues the body there (§5.16). A lambda in parentheses whose block an
operator follows keeps its block when the braces are a closure as well
(`(x) -> { foo() }.call()`): the block's static precedence settles that
reduction. Where the braces differ as a block and a closure
(`(a) -> { b = (c) -> { } }.call()`), the weights do, and the closure,
which holds a lambda less, wins (§5.16).

The decision ends the lambda, so a node that ends at the `}` depends on
what follows it, and a reparse may reuse such a node in a state that
decides differently from the one a parse from scratch decides in. Two
devices make it decide in the same state. Where the scan returns false,
the runtime counts what it read against the token after the `}`, not the
`}`, so a reparse may reuse the closure as the expression it was and
take the next token in a state where LAMBDA_BODY_END is not valid. That
corrects itself where the token does not fit, as the parser breaks the
closure down and lexes there again, except after a comment, which it
takes as an extra first: the decision would then fall after the comment
and put it in the lambda. So at a `}` of statements that a comment
follows on its line the scanner reads as far as the decision would and
returns false, and the `}` itself carries the read. The `}` of an
interpolation and of a class body is not read past, as no lambda's body
ends with one: after an interpolation's `}` a `/*` in the string opens
no comment that could set the §6.6 bit, and each such `}` read to the
end of the input when it was. A reused block of a lambda without
parentheses is the other case: shifted whole, it puts the parser in a
state where only LAMBDA_BODY_END is valid, and a comment shifted there
would stay before the token once the parser broke the block down.
LAMBDA_BODY_REREAD makes the parser break the block down at once and lex
the `}` again in the state after it; in a parse from scratch it ends the
version that read the braces as a block, where the closure's version
goes on. One case is left: a comment on the line after the `}`, before
an operator that an edit removes, ends up in the lambda on reparse.
Reading past every `}` that ends a line would close it, at 0.55% more
instructions on real files and 1% on Java files.
`bindings/rust/tests/incremental_reparse.rs` pins it.

Measured against the grammar before this section and the weights of
§5.16: STATE_COUNT 4516 to 4576, LARGE_STATE_COUNT 1693 to 1742,
`parser.c` 8,904,570 to 9,155,821 bytes, two external tokens and no
conflict added. A token of its own for the marker added 180 KB to
`parser.c`, as an extra is valid in every state. Real Groovy and Gradle
files take 0.3% more instructions, Java files 0.2%, code made of
closures, conditions, calls or parentheses 0.2% to 0.6%, class bodies
0.1%, code made of lambdas 1%, and code made of lambdas with brace
bodies and operators after them 3%. No tree of the corpus, the 266 real
files or the 24,548 Java files without an `ERROR` changes; the corpus
test `enum E<T> { A }`, an `ERROR` either way, recovers into a closure
statement instead of keeping its `enum_declaration`, and 4 Java files
recover differently with as many bytes in an `ERROR`. Of 61,246 template
inputs of lambdas with brace bodies in contexts, 3,895 without an
`ERROR` change, of which Groovy 3 accepts 2,793 and reads 2,322 the new
way and 88 the old way; of 60,319 mutations of the corpus and probes,
2,361 change, of which Groovy accepts 2,182 and reads 1,653 the new way
and 43 the old way. The old way is the divergences of §5.16, nearly all
of them a lambda as an assignment's value that an operator follows and a
block of a lambda in parentheses that also reads as a closure with a
parameter. One mutation gains an `ERROR`, `def v = a -> { foo() }[]`, an
empty subscript, which the grammar takes nowhere, and 151 mutations and
45 template inputs lose one. Over 240,000 sequences of four edits of the
§6.18 texts, 96,000 of lambda-rich texts edited around the `}` and 5,488
that type or delete an operator after a lambda's body one character at a
time, the reparses that differ from a parse from scratch went from 135,
49 and 0 to 135, 43 and 0, all of the lambda ones in trees with an
`ERROR`; over 440,000 edits of lambda texts with comments, blocks and
members around the body, the one in a tree without an `ERROR` is the
case above. The scan reads the trivia, at most 8 comments and one word,
a few characters or the start of type parameters, and a comment's token
reads only what follows it: 1.1 and 1.0 bytes asked for per input byte
for 500 and 4,000 comments between a lambda's body and an operator, 2.8
for lambdas each with 7 comments there, and 1.7 and 1.5 for 500 and
4,000 Gradle tasks whose GString holds `${buildDir}/*`, which read to
the end of the input from each interpolation's `}`, 252 and 2,009 bytes
per byte, before the read was kept to the `}` of statements
(`bindings/rust/tests/lambda_body_end_cost.rs`).

### 6.20 MULTI_ASSIGNMENT_COMMA — implementation contract

After `def f` or `final f` where a statement may start, the `(` opens a
method's parameters or, with `def f` a declaration of its own, the next
statement (the `[variable_declarator, _property_name]` conflict in
`grammar.js`). `def f` followed by `(a, b) = l` on the next line is two
statements, and a line break is whitespace, so both readings run from the
`(`. The second ended at the first `,` until multi-assignments without
`def` (§4, #94) took `(a, b)` as their names: it then lived to the token
after the `)`, and a script method's head parsed each untyped parameter
twice. A file of one head took 16 parse steps for `def f(a, b) { }` where
13 had done, 24 for four parameters where 17 had, and 40 for eight where
25 had. The first `,` of a multi-assignment is now the scanner's token,
aliased to `,`, which it turns down where the parentheses go on as a
method's parameters.

```text
Emit MULTI_ASSIGNMENT_COMMA, the `,` alone, at the first `,` after the
first name of a multi-assignment without `def`, where the token is valid,
after any spaces, tabs, line breaks, form feeds and vertical tabs before
it, unless what follows it goes on as a method's parameters. Read after
the `,`, with Groovy's trivia (whitespace, form feeds and vertical tabs
included, line escapes and comments) between the items, at most
MULTI_ASSIGNMENT_LOOKAHEAD_NAMES (16) names, each after a `,`:
  - after a `,`, `@` (an annotation) turns the token down; anything else
    that is no name emits it;
  - after a name, `,` reads on, `)` is read as below, `==` and `=~` emit
    it, and anything else turns it down: another name (`(a, int b)`,
    `(a, b c)`), `=` (a default value), `.`, `<` or `[` (a type's), which
    go on as a method's parameters, and any other character
    (`(a, b + c)`, `(Ab, Cd>)`), which no list of names holds;
  - after a name and `)`, the `{` or the word `throws` of a method's body
    or throws clause turns it down; anything else (`=`, `;`, a line's next
    statement, the end of the input) emits it.
A `/` that opens no comment, a `/*` that does not close (or one the §6.6
bit says cannot), a `\` that is no line escape, and the end of the bound
emit it. On a turned-down token return false; the in-grammar lexer then
reads the `,`, which the multi-assignment does not take. Not emitted
during error recovery.
```

The token is turned down only at text that a multi-assignment cannot
take, so the reading that ends there would have ended at a later token of
the same parentheses, or at the `{` or `throws` after them; where it
emits, the grammar ends the reading at the token it did without the
look-ahead. Names are read wider than the grammar's (keywords, non-ASCII
characters), which can only emit. So no tree changes for input that
parsed without an `ERROR`. Text that no list of names holds turns the
token down so that an error stays in its statement: emitted there, the
token committed the parser to a multi-assignment inside a call's broken
arguments, and the `ERROR` took the rest of the file (`foo((Ab, Cd>)y)`
followed by `bar()`). The token is valid only in the states after the
first name in parentheses where a multi-assignment may start, where the
grammar takes no other `,`.

What a turned-down token depended on is inside the parentheses, or is the
`{` or `throws` after them, and so inside the method that the parse keeps:
a reparse that reuses a node of that method has read the same text. That
is not true of text after a method that has no body, and the scanner does
not decide there. When a head forks before its `(` (after `final g`,
`final def g` or an annotation), the method's version reads its own `(`
and `,` in a state where neither look-ahead runs, and once the other
version ended, the method's nodes are not fragile. With the token turned
down at `foo()`, `final g`, `(a, b)`, the bodiless method was reused after
`= l` was typed below it, and the reparse kept it with an `ERROR` where a
parse from scratch reads `final g` and a multi-assignment. The same holds
for the lambda look-ahead (§6.17), from before this section: `foo()`,
`final g`, `(int a, int b)`, then `-> a` typed below it is an `ERROR` on
reparse (`bindings/rust/tests/incremental_reparse.rs` pins both).

A token of the `,` itself costs a multi-assignment no parse step of its
own, which an empty token before the `,` did (13% more instructions on
`(a, b) = [1, 2]`). The bound keeps the read linear where error recovery
re-enters the state after the first name at each later name of a list
that no method takes (`(a0, …, a254) { }`, `(a0, …, a254 x)`), and each
comma starts a look-ahead: with a bound of 256 names, 20 lists of 255
names that a `{` follows took 9.8 bytes of input per byte, and 2.0 with
the bound of 16.

The other look-aheads' trivia skipping (`skip_header_trivia` and the scan
function's own) does not skip form feeds and vertical tabs, which the
grammar's extras do. A form feed there decides as if a token stood in its
place, as it did before: `var (a,\fb) = l` calls a method named `var`,
`foo` followed by `(a,\fb) = l` on the next line passes the parentheses
to `foo`, and `record\fR(int a) { }` is a method. Skipping them there
would change those trees, so only this look-ahead does.

Measured against the grammar before this section, with 2,000 heads of
each shape in a file: `def f(a, b) { }` takes 14.2% fewer instructions,
with four parameters 25.8% and with eight 32.8%; `final f(a, b) { }`,
`def f(a, b) throws E { }` and a `{` on the next line 12.9% to 14.2%;
`def f(a, int b) { }` 8.7% and `def f(a, b = 1) { }` 7.5%; with a
three-statement body 8.1%. Each takes as many parse steps as the grammar
of 0.6.0. They are still 9% to 11% above that grammar, as are typed
parameters, `def f(int a, int b) { }`, whose steps never changed: about
5.7% is the growth since then that `def f() { }` shows as well, 2.8% the
look-ahead for a lambda that reads the parameters at the `(` (§6.17), and
1.4% this one. A head with more untyped parameters than the bound is not
helped and takes 2.4% more (`def f(a0, …, a19) { }`). Plain
multi-assignments take from 7.8% fewer instructions (four names) to 2.9%
more (after a call, `foo()` then `(a, b) = l`): `(a, b) = [1, 2]` 2.3%
more, `x = (a, b) = l` and `def (a, b) = l` 0.1%. Heads with one untyped
parameter, typed parameters or modifiers alone other than `final` are
within 0.1% of before, and class bodies 0.5% fewer to 0.2% more.
STATE_COUNT 4576 to 4600, LARGE_STATE_COUNT 1742 to 1754, `parser.c`
9,155,821 to 9,208,773 bytes, one external token and no conflict or
precedence added. An empty token before the `,` took 4 states and 13%
more instructions on `(a, b) = [1, 2]`, and the names after the first as
a hidden rule 6 states and up to 9% more on multi-assignments
(`x = (a, b) = l`). The parse has the same number of steps
on the 266 real files and on 400 Java files, which hold no such head; they
take 0.07% and 0.10% more instructions. 16 scripts with 50 such heads
take 0.4% fewer. No tree of the corpus, the 266 real files, the 24,548 Java
files or 1,205,700 fuzzed inputs without an `ERROR` changes, of 2,670,000
inputs: 1,290,000 made from templates of method heads, multi-assignments,
casts and calls with closures in many contexts and from mutations of the
corpus, the real files and the Java files, and 1,380,000 from other
templates, mutations and trivia between tokens. Trees with an `ERROR`
change, most of them for the token itself, which a scanner that always
emits it changes as well: two tests of the corpus, `(a, b)` and `enum E<T> { A }`,
the second of which now keeps its `enum_declaration`; 8 Java files, one
with fewer bytes in an `ERROR`; and 97,539 of the first fuzzed inputs,
46,488 with fewer bytes in an `ERROR` and 10,603 with more. Where a list
is complete and a `{` follows, the reading now ends earlier: `(a, b) {`
typed in a closure recovers with 35 bytes in an `ERROR` where 8 were;
`(a, b,` and `(a, b) +=` there recover as before. Over 1,000,000 and
200,000 random edits of texts with multi-assignments and method heads,
and 20,448 edits that type `= l` and more after such heads, the reparses
that differ from a parse from scratch went from 141, 34 and 0 to 188, 51
and 0. The two without an `ERROR` are the same before and after; every
other one has an `ERROR` both ways, and a scanner that always emits the
token has as many as before (141 and 34): a method whose head no longer
forks is not fragile, and error recovery reuses it. The
look-ahead reads 2.6 and 2.1 bytes per input byte for a list of 500 and
4,000 names, 3.7 and 2.4 for a method head with as many parameters, and
the same 2.6 and 2.1 for lists that a `{` or a second name ends, which took
259 and 2,065 with no bound
(`bindings/rust/tests/multi_assignment_comma_cost.rs`).

### 6.21 DECLARATION_START — implementation contract

Groovy takes a typed local declaration without an initializer:
`String s`, `int i;`, `List<String> xs`, `foo.Bar q`, `String a, b = 1`.
Its tokens are also the expression `String` followed by the statement `s`
(and those of `String s = 'a'` the expression and the assignment),
and Groovy ends the statement at a line break after the type (`String`
then `s` on the next line is two statements) and, where a statement may
follow, after the name. A newline is whitespace here (§5.10), so the
grammar cannot tell them apart. Apache tries the declaration first unless
`isInvalidLocalVariableDeclaration` turns it down (§5.5 item 3), and reads
a command expression where it does not parse. The scanner applies the same
rule, before the type, where COMMAND_STATEMENT_START (§6.14) decides a
statement's receiver, and from the same reading of its first name.

```text
Emit DECLARATION_START, empty, before the type of a statement, where the
token is valid (where a statement starts, after its annotations, and as
a `for` init), when, after whitespace, line breaks and line escapes, a
type follows by Apache's rule on the spelling, and then a declared name:
  - the type is a name that starts with an upper-case letter (as §6.14
    tells upper case) or is a primitive type name, then on its line a
    name; or any name or dotted name (names joined by `.` with no space)
    that type arguments follow directly (`<`, then names, `,`, `?`, `.`,
    `[`, `]` and spaces on the line, to the matching `>`, where a `<`
    directly follows a name: `Foo<<Baz>> x` is shifts), then any
    dimensions; or any name or dotted name that dimensions follow on its
    line (`[`, Groovy's trivia, `]`, and each further `[` on the line of
    the `]` before it); or a dotted name whose last name starts with an
    upper-case letter; a dotted name whose first name is `this` is none
    (`this.Foo x` is Groovy's call `this.Foo(x)`);
  - between the type and the name spaces, tabs, line escapes, form feeds,
    vertical tabs and block comments (which may span lines); the name is
    none that names no variable in Groovy: a keyword Groovy reserves, a
    primitive type name, `void`, `this` or `super`;
  - after the name, past spaces, tabs, line escapes, form feeds, vertical
    tabs and block comments (which may span lines, as Groovy's lexer hides
    a line break in one): the end of input, `;`, `}`, the word `else`, a
    line comment and then a line break as below, or a line break, past
    Groovy's trivia, before a line that does not go on with the statement
    (below); or `,`, Groovy's trivia (line breaks included) and a further
    name as above, for the whole list; or, after any name of the list, on
    its line or the next, the `=` of an initializer (not `==`), unless
    closure parameters may start here (CLOSURE_PARAMETERS_START valid,
    also after a parameter's annotations) and what follows it to `->`
    reads as closure parameters (`parameters_follow`, §6.14:
    `{ String a = 1 -> }`, `{ @A String a = 1 -> }`).
  - In a `for` init, where Groovy's lexer drops line breaks, the trivia
    after a name include line breaks, and only `;` and `=` end the scan
    with a declaration: `,` goes on with the list, and anything else is
    no declaration (`for (String s : xs)` and `for (String s in xs)` are
    loops).
A line goes on with the statement, as Groovy reads it, when it starts
with what no statement starts with: `.`, `?`, `*`, `%`, `^`, `|`, `&`,
`<` unless it opens the type parameters of a generic method (a name or
an annotation, then `>`, `,` or `extends`: `String s` then
`<T> T m(T t) { }`), `>`, `:`, `,`, `)`, `]`, `==`, `!=`, `!in` or
`!instanceof` before
what makes them operators (§6.12), `+=`, `-=`, `->`, `/=`, the word
`instanceof`, a `/` that opens no comment and no slashy string (the
string needs a closing `/` on the line, `slashy_body_follows`:
`Foo foo` then `/ 2` is Groovy's `Foo(foo / 2)`), and `(` when the
parentheses hold parameters (`call_arguments_follow`, §6.14: `Foo f`
then `(int a) { }`), where a call in parentheses, unlike after a name
argument, does not count, as it opens a statement of its own
(`String s` then `(foo 1)`, and `Foo f` then `(foo bar) { }`, Groovy's
declaration and call `foo(bar, { })`), and a word that names no
variable is no parameter's name (`String s` then `(Foo new Bar())` is
the declaration and the call `Foo(new Bar())`). A line that starts with
`in` or
`as` starts the next statement, as in Groovy (`InputStream in` then
`in.close()`).
The type arguments, the names of the list and the value after an `=` are
read for at most DECLARATION_LOOKAHEAD_ITEMS (256) items: past them there
is no declaration, and a value read to the end of the budget is no
closure parameter's default. The in-grammar lexer reads the type, the
names and the values. On any other input return false. Not emitted during
error recovery.
```

COMMAND_STATEMENT_START takes a name argument only after a receiver that
cannot name a type (§6.14), and this token only such a receiver or a name
that type arguments or dimensions follow, so a statement that starts with
two names is a call, a declaration or neither, and never both. Both
tokens come from one reading of the first name, and each is emitted only
where it is valid: where the reading answers the other one, the scan
returns false. `foo bar` is a call and `String s` a declaration in every
state; `Foo new Bar()` and `Foo String[].class` stay calls (§5.5 item 3).

The grammar takes the token in one form of `local_variable_declaration`:
annotations, the token, the type, then declarators that each may have an
initializer (`String a, b = 1`, `String s = 'a'`), with the node and
fields of the typed form with initializers. That form, which still takes
a declaration after modifiers (`final String s = 'a'`) and where the
token is not emitted, and the blank final (`final String a, b`), now
take declarators without an initializer after their first
(`String a = 1, b`). Nothing else competes for the token, so the form has
no weight. As the token opens the typed form with an initializer as well,
the declaration and the two statements no longer fork at its type; the
fork is left after `final` and other modifiers. `return`, `break` and
`continue` take the token before their operand or label on their line, as
they take COMMAND_STATEMENT_START, so `return String s` is the return of
`String` followed by `s`, as before; `return` then `String s` on the next
line is a bare `return` and the declaration, as Groovy reads it, since a
line break ends the `return` (§6.23). A statement that ends
at a token only this look-ahead decides has an extent that depends on text
past its own tokens, which a reparse that reuses the statement does not
read again: with the token ending the `return`, `(a) { }`, `return`,
`String s` was a `return` and a declaration, and typing `x` before
`String` reparsed as that `return` followed by the call `xString s`, where
a parse from scratch reads `return xString` and `s`
(`bindings/rust/tests/incremental_reparse.rs` holds the text).

In a `for` header Groovy's lexer drops line breaks, so the scan reads
across them there: `;` ends the declaration (`for (int i` then
`; i < n; i++)`), and `:` and `in` make it no declaration, which leaves
the loop to the grammar (`for (String s` then `in xs)`). A header still
being typed (`for (int i` then the next statement) emits nothing, so
error recovery keeps the statements around it, as before the token
(`test/corpus/statements-for.txt` pins it). Positions where a type may
start (TYPE_START) include every one where the token is valid but the one
after `def` below, which the scan counts as one, so the
array type value (§6.15) is decided in the same branch: `String[] arr` is
a declaration and `String[].class` a value.

The token is also valid after `def`, alone or after `final`, in that form
of `local_variable_declaration`, and in the typed form of
`field_declaration` after its modifiers: Apache's `variableDeclaration` is
`modifiers nls type? variableDeclarators`, and `def` is a modifier, so
`def String s = 'a'`, `final def List<String> xs = []` and, in a type
body, `private def Foo foo = mock(Foo)` declare their names with the
type, as in Groovy (checked with Groovy 3), with `def` an anonymous child
of the node. Where the scan does not emit it, `def` takes the name as
before (`def x = 1`, `def Foo = 1`); a line break after the type ends the
declaration as after a statement's type (`def String` then `s = 1` on the
next line is two statements, as in Groovy), and one between `def` and the
type does not, as in Groovy's `nls` (`def` then `long where = f()`). No
call may start there, so the scan reads the type by the rule that holds
after annotations: a lowercase type needs `<` or `[` (`def foo bar` is
`def foo` and `bar`, where Groovy declares `bar`). Not taken: a modifier
after `def` (`def final String x`, `def static String x`, Groovy's
declarations, which stay `def final` and a declaration of `x`), a return
type after `def` (`def String m() { }`, Groovy's method, which stays
`def String` and a call, or a field and a constructor in a type body),
and a parameter's (§5.12, `docs/divergences-from-spec.md` §14). The two
forms cost 34 parse states. `var`, also a modifier in Apache's `modifier`
rule, and `final var` take the local's form as `def` does: where the
scanner emits `var` (§6.16) and then the token,
`var String s = 'a'`, `var String t`, `final var List<String> xs = []`
and `var` then `String u = 1` on the next line declare their names with
the type, with `var` an anonymous child of the node, as in Groovy 3,
where they were `var String` and the statement `s = 'a'` (`t`) with no
`ERROR`. `var String` then `v = 1` on the next line stays two
statements, as in Groovy, and `var var = 1`, `var x = 1`, `var(1)` and
`var.x` read as before. That cost no parse state: STATE_COUNT went from
4,789 to 4,781, as the two keywords now share the states after them.
In a type body `var` is a modifier as `def` is, and takes the field's
typed form after it the same way (`var String s = 'a'`,
`private var List<String> xs`), for 3 parse states: before, it was a
type name there, so `var x = 1` was a field of type `var` (now a field
with `var` an anonymous child, as `def x = 1` is) and `var String s` a
field `String` of type `var` and a field of type `s` with a `MISSING`
name. Before a method's name `var` stays its return type (§6.16).

Trees that change for input that parsed without an `ERROR` are the
declarations themselves, each a `local_variable_declaration` where the
type and the name were statements of their own (`String s`, also in a
closure, a block, a switch case, after a label, as the body of `if`,
`else`, `while` or `for`), a field access and a name (`foo.Bar q`), a
comparison (`java.util.List<String> l`, `Foo.Bar<T> a`) or, across the
line break, other readings of the name and the next line: `String s`
then `{ }` was `String` and the call `s { }`, `String s` then `/x/` the
call `s /x/`, `String s` then `(a) { }` the method `s`, `String s` then
`+ 1` the sum `s + 1`, `String s` then `in xs` or `as T` the test
`s in xs` or the cast `s as T` (now the declaration and then `in` and
`xs`, or `as` and `T`, as statements, where Groovy calls `in(xs)` and
`as(T)`), `InputStream in` then `foo()` the test `InputStream in foo()`,
and `final x` then `String s` or `String s = 'a'` the blank final
`final x String` followed by `s` or `s = 'a'`. Each declaration is one
of Groovy 3, which was checked by the declarations of the changed lines:
on every new declaration in the 1,971 Java files of 24,548 that changed
to a tree with no `ERROR` (4,216 names, all agree; Groovy rejects 250 of
the files), on 2,547 of the 2,817 changed fuzzed inputs with no `ERROR`
before or after that Groovy accepts (the grammar before agreed on 229),
and on the corpus. The rest are two kinds of input where Groovy reads
something this grammar has no reading for: a second statement on the
line of a first, which may now be a declaration where Groovy goes on with
the first (`foo() String s` is `foo()` and the declaration, Groovy's
`foo().String(s)`; `move left By steps` is the call `move left` and the
declaration `By steps`, where it was three statements, Groovy's
`move(left).By(steps)`; `Foo T name` is `Foo` and the declaration; also
after a line escape), and a declaration that Groovy reads otherwise only
because what follows it does not parse (`do Foo in` then `x while (c)`;
`if (c) Foo f` then `as T` then `else { }`, Groovy's `Foo(f as T)` in the
`if`). `return String s` stays `return String` and `s`, which Groovy
rejects, and `return java.util.List<String> s` stays comparisons, as
Groovy reads it. A statement after `Type name` on the same line, a type
with a space before `<` (`a < b > c`, `List <String> xs`, Groovy's
declarations), shifts (`Foo<<Baz>> result`), a path from `this`
(`this.Foo x`, Groovy's `this.Foo(x)`) and `=~` after a name
(`Foo f =~ /x/`, the declaration of `f` with the value `~/x/`, Groovy's
`Foo(f =~ /x/)`) are read as before.

A block comment between the type and the name is trivia, as after the
name: Groovy's lexer hides it, and the line breaks in it
(`String /* c */ s`, `String /* a` then `b */ s`, `int/*unsigned*/ mask;`
are Groovy 3's declarations). Only the declaration reads past it: after a
capitalized or a primitive type name the comment makes the statement no
call (`Foo /* c */ new Bar()` stays `Foo` and the creation, where Groovy
calls `Foo`; §6.14), and a lowercase name before one stays a statement of
its own (`foo /* c */ bar`). A line comment there ends the line, and a
form feed or a vertical tab is read as a space, as the grammar's extras
read it (Groovy's lexer rejects both). The look-ahead reads the comment
once per declaration: lines of `T /* c */ a` cost 2.67 bytes asked per
input byte at 500 and 4,000 lines, a nest of `T /* c */ a, b = {` 36.1
and 37.3, and a comment of 500 or 4,000 lines between `T` and `a` 2.00
(`bindings/rust/tests/declaration_start_cost.rs`). No tree of the corpus,
of 266 real files or of 4,000 shape inputs changes; of 24,548 Java files
two do, each at `int/*unsigned*/ mask;` and the like, which are now
declarations.

Not taken, each read as before: annotations
before a lowercase type with no `<` or `[` (`@A foo bar`, Groovy's
declaration), type arguments that span lines, a name after the type on
the next line (`String` then `s`, two statements in Groovy too), a
declaration whose next line goes on with it (`String s` then `.trim()` is
`String` and `s.trim()`, Groovy's `String(s.trim())`; `String s` then
`* 2`; `Foo foo` then `/ 2`; `String s` then `/= 2 // c`, where Groovy
reads the slashy string `/= 2 /`, which this scanner does not start with
`=`), and a resource without an initializer (`try (String s) { }`, an
`ERROR`, which Groovy declares). Modifiers other than `final` before the
type (`static String s`) stay an `ERROR` (§4). In an interpolation no
statement is taken, so `"${String s}"` stays an `ERROR`.

Of 394,000 fuzzed inputs made from templates of types, names, separators
and what may follow them in 70 contexts and from mutations of the corpus,
the real files and the Java files, 137 went from no `ERROR` to one, and
Groovy 3 rejects all but 10: three where a closure on the next line is
followed by a `,` (`Integer _` then `{ }, b`, Groovy's call
`Integer(_({ }), b)`), four where Groovy reads `in` after a declared name
as the operator because the declaration does not parse in a `do` body
(`do long in` then `[1] while (c)`), and three where the declaration is
right and the line after it is one this grammar has no statement for
(`foo<T>` then `it`, `foo()` then `-> x`, `++i` then `List< x`). Error
recovery changed in one test of the corpus: `enum E<T> { A }` lost its
`enum_declaration` again (§6.20), as the in-grammar lexer, whose states
the grammar's change regrouped, reads its `<` as the one a statement may
start with. `String s =` at the end of the input is the declaration of
`s` and an `ERROR` of the `=`, where it was a declaration with a
`MISSING` value, as the token now opens it. A `for`
header still being typed keeps the statements around it (above).

The look-ahead reads, per input byte, 41 and 43 bytes for a nest of 500
and 4,000 closures that open with `T a, b =`, 45 and 46 for as many
unclosed, 50 and 51 for as many broken lines of such lists and 60 and 64
for a nest of `Closure c = {`, where the grammar before read 1.5 to 3.2;
with no bound it took 254 and 2,004, 251 and 2,001, 501 and 4,001, and
254 and 2,004 (`bindings/rust/tests/declaration_start_cost.rs`). The
value after an `=` is read to its end, or to the budget, as a closure
parameter's default may hold a closure (`{ Closure a = { 1 } -> a() }`
is Groovy's closure with one parameter), so no read is cut short. On the
real files the parser reads 1.77 bytes per byte, 1.75 before. Over
123,840 edits that type and delete declarations one character at a time
between 18 kinds of text before and 8 after, no reparse differs from a
parse from scratch, as before; over 720,000 random edits of texts with
declarations the reparses that differ went from 48 to 53, 5 and 5 of them
with no `ERROR`. The two of those added are the reparse of a `return`
after `(a) { }` that the grammar before this section has too (`(a) { }`,
`return`, then `x` typed on the next line), and two of the grammar before
are gone; every other one has an `ERROR` both ways.

Closure parameters may start only after the `{` of a closure, and only
there may a `->` after an initializer's `=` end them. Elsewhere it is a
lambda's in the value: `final a` then `String b = x -> y`,
`Runnable r = () -> { foo() }` or `Comparator c = (x, y) -> 0` on the
next line is the blank final and a declaration, as Groovy reads it. The
scanner learns where closure parameters may start from
CLOSURE_PARAMETERS_START, an external token that is never emitted and
is valid only there: `closure_parameters` takes it in an alternative
that never matches, as `_type` takes TYPE_START (§6.15). Before it the
look-ahead turned the token down after `final a` as well, and the blank
final `final a String` followed by the assignment `b = x -> y` (a weight
of 2 steps) outweighed the blank final `final a` and the typed local
(1 step), as both readings hold the lambda's weight; every other value
was decided by the token. The marker added one state, one external
token and one external scanner state, and changed no tree of the
corpus, the real, Java and fuzzed sets other than such declarations.

A parameter's annotations come before the type that the scanner
decides, and after them the marker of `closure_parameters` is no longer
valid: `{ @A String a = b -> c }` read a declaration of `a` with a
lambda, and `{ @A String a = 1 -> a }` an `ERROR`, where v0.6.0 read the
parameter. `closure_parameter` takes the marker too, after its
annotations, in a second alternative that never matches (STATE_COUNT
4,789 to 4,859, LARGE_STATE_COUNT 1,846 to 1,860; in
`closure_parameters` the alternative costs 36 states and lists
`annotation` among that node's children in `node-types.json`). The
braces of a body stay a block: `if (x) { @A String a = b -> c }`
declares `a`.

The blank final with several names (`final String a, b = …`) weighs one
step, as the typed form with an initializer does, and with one name two
steps as before: a later name's value may hold a lambda, and at two
steps `{ final String a, b = (T) (x, y) -> z }` tied the closure with
two parameters, which Groovy reads, and the declaration won the tie.

Measured against the grammar before: STATE_COUNT 4600 to 4637,
LARGE_STATE_COUNT 1754 to 1773, `parser.c` 9,208,781 to 9,328,518 bytes,
one external token and one conflict
(`[local_variable_declaration, closure_parameter]`, the fork of
`{ final String a, b -> }` against `{ final String a, b }`) added. The
266 real Groovy, Gradle and Jenkins files take 0.31% fewer instructions,
the 265 whose trees do not change 0.25% fewer; 400 Java files 3.6% fewer,
the 331 of them whose trees do not change 2.2% fewer. A file of typed
declarations without an initializer in method bodies takes 81% fewer (it
had an `ERROR` in every method), one of typed declarations with an
initializer 31% fewer, as they no longer fork at the type, statements
with a dotted name 1.2% more, and calls and names 0.6% and 0.8% more.

### 6.22 STATEMENT_BLOCK_END — implementation contract

Apache's `statement` lists `block` first, so a `{` that opens a
statement is a block where it parses as one, and a closure where an
expression continues the braces: `{ foo() }()`, `{ foo() }.call()`,
`{ foo() } as Runnable`, `{ } + 1` and `{ }` followed by `.call()` on the
next line call, cast and add the closure, while `{ }` followed by `(1)`,
`{ b }`, `[0]` or `+ 1` on the next line is a block and a statement
(observed with Groovy 3). The grammar cannot see that at the `}`: reduced
there by its static precedence over a closure, a block takes every token
a statement may start with. Tried so, `{ foo() }()` was a block and an
`ERROR`, `{ } + 1` a block and the statement `+1`, and a nested array
initializer needed a precedence above both. A block that opens a
statement takes an empty token after its `}` instead, which the scanner
emits where the braces end.

```text
Emit STATEMENT_BLOCK_END, empty, at the `}` of a block that opens a
statement (a statement in a file, a block, a closure or a colon case, or
after a label), before the trivia after it, unless an expression
continues there. The scan is §6.19's, with the state's valid tokens:
STATEMENT_PAREN and RECORD_KEYWORD are valid after such a `}`, so after a
line break `(`, `+`, `-`, `{`, `[`, a slashy string and type parameters
start the next statement, and the other operators continue the braces.
On the line of the `}` every binary and postfix operator, `(`, `[`,
`{`, `as` and `in` continue them; a word, `new` included, a literal, `!`
other than `!=`, `!in` and `!instanceof`, `~`, `;`, `}` and the end of the
input end them.
Where only STATEMENT_BLOCK_END is valid, after a block the parser shifted
whole or read beside another reading of the braces, decide as §6.19 does
there (BODY_END_HAND_BACK) and emit LAMBDA_BODY_REREAD where it continues.
None is emitted during error recovery.
```

The braces are a closure wherever the token is not emitted, as they were
before this section in every statement position, so a statement keeps
the reading it had where an expression goes on with the braces, and
braces with parameters (`{ a -> }`, `{ -> 1 }`) are closures as before.
The token is only valid after a block that opens a statement: a body of
`if`, `else`, a loop or an arrow case takes its block without it
(`BODY_BLOCK_WEIGHT`, §5.16), and a `{` after an expression is that
expression's trailing closure, shifted before any statement could start
(`foo()` or `foo` followed by `{ x }` on the next line is `foo({ x })`, as
in Groovy). With the block in `_statement` instead, the bodies reached a
second block whose `}` the scanner decided after every body: 13% more
instructions on code made of conditions.

The token is not `LAMBDA_BODY_END`, though the scan is the same, so that
the states after the `}` of a statement's block and of a lambda's brace
body stay apart: tree-sitter merges no two states one of which takes an
external token the other does not. With one token,
`def c = { a = a -> { foo() }` followed by `+ 1` on the next line, where
Groovy reads the parameter `a`, a block and the statement `+1`, was a
lambda with a block: the two readings shared the state at the `}`, the
stack version split off at the block's reduction lexed again in the
state after the block, where only the token is valid, and the scan there
handed the `}` back, as `+` on the next line continues a lambda's body
inside parentheses, so that reading ended. In the state after a block
shifted whole the scan hands back every `}` that §6.19 hands back, as
after a lambda's block: with a scan that took a statement to follow
there, 76 of 60,000 random edit sequences around such blocks reparsed to
a tree other than a parse from scratch, against 45: the 31 added have an
`ERROR` both ways, and the one without an `ERROR`, a tie of weights, is
among both counts (`bindings/rust/tests/incremental_reparse.rs` pins an
added one).

Measured against Groovy 3 on 8,448 inputs, 24 contexts (a script, a
method, a closure, a block, a label on the line or the one before,
`case 1:` on the line or the one before, `default:`, and the line after
`if (c) x`, a name, a call, `x = (a)`, `x = a[0]`, `println 1`,
`foo();`, a declaration, a typed local, a block, a closure argument,
`return`, `break` and a lambda's body) by 11 brace shapes and 32 things
after the `}`, classified by what the node at the `{` is in Groovy's AST
and in the tree: 1,360 trees changed. 1,080 now read as Groovy does where
the grammar before did not: a block where it read a closure statement,
two blocks where it called a closure with a closure, and a block and a
statement where it added `+ 1` on the next line. No input of this set
that the grammar before read as Groovy does reads otherwise; outside it
some do: two of the lambda templates (`docs/divergences-from-spec.md`
§28) and a lambda before two or more lines of braces where the readings
differ by one step (§5.16). 280 changed and read
otherwise still: a word or a literal after the `}` on its line
(`{ } foo`, Groovy's `{ }.call(foo)`) and a brace on the line after
`println 1` or `foo { a }`; each was a closure statement and is a block.
(Groovy passes the brace after `foo { a }` to that call as one more
argument, and calls the literal of `println 1` with it; the second is
the reading here since a literal argument goes on past its line,
§6.14.) Of the 1,155
inputs that did not change and read otherwise, 544 have the brace on the
line after `println 1` or `foo { a }` with something after it, 286 a
brace on the line after a name that something continues, an `ERROR` here
and an argument of the name's call in Groovy, 120 `as T` on the next line
(Groovy's block and `as(T)`, the cast here), 115 `return` on the line
before, and 90 are braces with parameters followed by a word on their
line or `+ 1` on the next.

Read differently from Groovy (observed with Groovy 3): `as` and `in` on
the line after the `}` continue the braces, as after a lambda's body
(§6.19); a word or a literal on the line of the `}` ends the block, where
Groovy calls the closure with it; a `{` on the line after `foo { a }`
opens a block, where Groovy adds it to the call's arguments (after
`println 1` it is the closure that the literal is called with, as in
Groovy, §6.14); a block that holds a lambda whose body is a
closure is the closure with the lambda's assignment for a parameter, and
`{ a = b -> c` followed by two lines of braces is too (§5.16); and more
than eight comments between the `}`
and an operator end the block (§6.19), which leaves the operator an
`ERROR` where the grammar before read the closure as Groovy does. Taking
the braces for a closure past the bound instead has the scan run again
after each comment of the run, and a run of 255 comments before an
operator is served 31 bytes per byte of input, against 1.2
(`bindings/rust/tests/lambda_body_end_cost.rs`, which allows 8). A
labeled block followed by a comment on the next line and an operator
after it, `lbl: { }` then
`// c` then `.x()`, reparses with the comment inside the labeled
statement when an edit removes the operator, the limitation of §6.19:
42 of 18,000 sequences that type and delete what follows such a `}`, in
a label and a case, all of them that kind
(`bindings/rust/tests/incremental_reparse.rs`).

Measured against the grammar before this section: STATE_COUNT 4637 to
4655, LARGE_STATE_COUNT 1773 to 1784, SYMBOL_COUNT 396 to 397, `parser.c`
9,328,518 to 9,395,199 bytes, one external token and no conflict added.
The 266 real Groovy and Gradle files and code made of conditions,
closures, calls, class bodies, lambdas, parentheses, names, GStrings and
parenthesis-free calls take 0.02% to 0.09% more instructions, the 24,548
Java files 0.14% fewer. Code that holds blocks that open statements
takes 2.8% to 10.4% more (lines of `{ a() }` and `foo()`, `{ a() }` in
methods, class bodies with initializers and such blocks, lines of
`{ a() };`), about 1% of it the scan and the rest the token and the
block's reductions, and code made only of consecutive lines of such
blocks 28.7% more, which the grammar before read as one closure called
with the next. No tree of the 266 real files
changes. Of the corpus inputs 11 trees without an `ERROR` change, and of
the Java files 152, each only by braces that open statements, now
blocks, also where two followed each other on consecutive lines and read
as a closure called with a closure; 587 Java files with an `ERROR`
change, 346 only by such blocks, and their bytes in an `ERROR` go from
1,366,605 to 1,375,607, nearly all in two JDK test files whose class
holds fields with Java array initializers, one of them empty and the
others with a trailing comma (`OpenType[] t = { SimpleType.LONG,
SimpleType.STRING, };` after `String[] n = { };`): the error recovery,
which kept the class and an `ERROR` at each comma, now takes the whole
file for one `ERROR`. No variant of the grammar or its weights tried
avoids it. Of 231,760 mutations of
the corpus, the truth-table inputs, lambda templates, real files and
Java files, 41 lost a tree without an `ERROR`, each a brace that opens a
statement followed on the next line by what no statement starts with
(`()`, `(a, b)`, `++`), which Groovy 3 rejects in all 41; with
`LAMBDA_NESTING_DEPTH` at 100, 40 of them. Over 18,000
sequences that type and delete what follows such a `}` and 60,000 random
edits around such blocks, the reparses that differ from a parse from
scratch went from 0 and 53 to 42 and 45, one of the 45 without an
`ERROR`: a closure whose two readings weigh the same, decided by
tree-sitter's last resort. The lambda and declaration sequences of §6.19
and §6.21 differ in 0, 11, 0 and 10 reparses, 0, 13, 0 and 10 before,
with the same one without an `ERROR`, and 702,863 edits of typed
declarations in 144, 150 before, each with an `ERROR`.

### 6.23 JUMP_STATEMENT_END — implementation contract

```text
Emits iff (an empty token, ending at the keyword):
  - it is valid, which it is only right after `return`, `break` or
    `continue`, and the parser is not recovering from an error
  - past spaces, tabs and line escapes, a line break, the end of input,
    a line comment, or a block comment that a line break or the end of
    input follows past spaces and tabs only, follows
Else: emits nothing; the spaces stay skipped for the branches below.
Where a block comment, spaces or tabs and then something else follow,
emits the empty UNTERMINATED_COMMENT at the keyword with
`jump_continues` set; the next call reads the comment without deciding
again.
```

Apache's `RETURN expression?`, `BREAK identifier?` and
`CONTINUE identifier?` take no `NL` before the value or the label, and
Groovy's lexer emits `NL` at a line break outside parentheses and
brackets, which is everywhere these statements may stand. So the value
or the label must start on the keyword's line, and a line break after
the keyword ends the statement. Observed with Groovy 3: `if (c) continue`
then `process(f)` on the next line is a `continue` and the call;
`break` then `label`, `break // c` then `label` and `break /* c */` then
`label` are a `break` and the statement `label`; `continue label`,
`break /* c */ label`, `break /* c` then `*/ label` and `break \` then
`label` take the label; `return` then `foo()`, `x`, `{ foo() }`,
`obj.m 'x'`, `foo new Foo()` or `String s` on the next line is
`return null` and that statement. A line comment is an `NL` to Groovy's
lexer, and so is a block comment that only whitespace follows on its
line; one that something follows on its line is hidden, also when it
spans lines.

The grammar reads a line break as whitespace (§5.10), so the statements
took the next line: `if (c) continue` then `process(f)` was
`continue process` and the parenthesized `(f)`, with no `ERROR`,
`if (c) break` then `foo = 1` an `ERROR`, and `return` then `foo()` the
return of the call. Now `return_statement`, `break_statement` and
`continue_statement` may end with the token. The scanner decides it before the same-line tokens (§6.9,
§6.18, §6.8), as the first branch that skips spaces: at a line break or
the end of input it emits the token; at anything else it falls through
with the spaces skipped. At a comment the comment branch emits the token
in place of a line comment and of a block comment that only spaces and
tabs follow before the line break (Apache's `ML_COMMENT` is an `NL` where
`isFollowedByWhiteSpaces` holds, which only whitespace before the line's
end makes true), and the comment is emitted by the next call, after the
statement (`break // c`, `return /* c */` then `x`, `break /**/` and
spaces, then `x`). v0.6.0 put it there only where nothing followed the
comment (`break // c`); with a statement on the next line it took that
statement as the value or the label, with the comment inside
(`return /* c */` then `x` returned `x`). A block comment that something follows on its
line is emitted inside the statement and skipped before the label or the
value, as Groovy hides it (`break /* c */ outer`). Telling the two apart
past spaces reads past the end of the comment's token, which a token
cannot move back to: where something follows them, the scan emits the
empty marker of §6.6 at the keyword instead, with a third bit,
`jump_continues`, set, and the next call, at the same position, reads
the comment and clears the bit. Without the bit, the statement ended at
the line break after such a comment and held it, `/**/` included, and a
reparse that reused a statement lexed with `;` after the keyword
(`return ;/* c */` and spaces, then the `;` deleted) put the comment outside it:
of 144,704 reparses of the reviewer's targeted chains, 84 differed from a
parse from scratch in where the comment sat, none now. `throw` and `assert`
still take a value on the next line, which Groovy rejects (`throw` then
`x`).

The token ends at the keyword, before the spaces: the branch marks its end
there before it skips them, and every token that a later branch emits
past them marks its own end (`SUBSCRIPT_OPEN` too, which needed no mark
before). Where it followed the spaces, the statement and the statement
around it (an `if`, a label, a case) ended after trailing spaces, and after
a comment, where at v0.6.0 they end at the keyword; and a reparse that
reused a statement lexed without the token (`continue ;` then `/* d */`,
with the `;` deleted) ended it at the keyword where a parse from scratch
ended it past the space or the comment: of 35,238 reparses of the single
edits of four texts with such statements and an undo of each, 93 differed
from a parse from scratch in where the statement ended or the comment sat,
none now.

The token is empty and the statement's own last token, so the runtime
counts the text the scanner read for it against the statement, and an
edit that removes the line break parses the statement again. Past the
token the scan reads on over line breaks and comments to the first
character of the next token, which a parse from scratch does not need:
without it, a reparse that reused the ended statement in one of two
stack versions, after a statement that forks such as `(a) { }`, a cast
or a call, let that version reach an `ERROR` on the next line before
the other version did and kept the other reading, where a parse from
scratch keeps the first. That happened in 36 of the 91,199 single edits
of the texts in `bindings/rust/tests/incremental_reparse.rs`, each at
the start of the line after `return` and each with an `ERROR`; with the
read, none. Over 47,472 reparses of every single edit (and its undo) of
nine texts with such forks before `return`, `break` and `continue`,
none differs from a parse from scratch, where the grammar before
differed in 128, 126 of them without an `ERROR`, each the undo of an
edit near a `return` that took its value from a later line.

Measured against the grammar before: STATE_COUNT 4726 to 4732,
LARGE_STATE_COUNT 1815 to 1818, SYMBOL_COUNT 408 to 409, one external
token and no conflict or precedence added. The lexer is served 20 more
bytes over the 266 real files (1,433,325 before) and 121 more over
2,046 of the Java files (27,451,125 before). No tree of the real files
changes; three corpus trees change, each a pinned divergence that now
reads as Groovy does, and 66 Java files, 42 of them without an `ERROR`,
each at a `return` that ends its line before its value, which Java
continues and Groovy does not.

---

### 6.24 CLOSURE_CALL_START and CLOSURE_COMMAND_START — implementation contract

```text
At the first name of a statement (COMMAND_STATEMENT_START or
DECLARATION_START valid, JUMP_STATEMENT_END not, no error recovery) that
is no keyword of the grammar and no contextual word whose own token the
scanner decides there, and that `{` follows on its line:
  read on Groovy's tokens from the `{`, within
  CLOSURE_CALL_LOOKAHEAD_CHARACTERS (128) characters:
  - the braces must close on the name's line;
  - past spaces, line breaks and comments a member access must follow
    the `}`: `.` other than `..`, `?.`, `??.`, `*.`, `.&`, `.@` or `::`;
  - then each access's name (trivia and line breaks between them), the
    brackets after it (`(…)`, `[…]`, `?[…]`, `{…}`, read past to the
    one that closes them, line breaks included), and after the last
    name or brackets on their line what follows.
Emits CLOSURE_COMMAND_START, empty and before the name, iff a first
argument that `obj.m 1` takes (§6.14, any name included) follows the
last name on its line, or parentheses after it hold a
parenthesis-free call (`.bar(baz 1)`), no brackets come before, and
every access is one of `.`, `?.`, `??.`, `*.`, `.&`, `.@` with a plain
name.
Emits nothing where such an argument, or a name after brackets, follows
brackets or a later access's name, or a name follows `new` as the last
name.
Emits CLOSURE_CALL_START, empty and before the name, iff no such argument
follows.
Else, and wherever the read reaches the bound before brackets after an
access: emits nothing.
```

Groovy reads `foo { }.bar()` as `foo({ }).bar()`. Apache's
`pathExpression` is `primary (pathElement)*`, and a `pathElement` is a
`closureOrLambdaExpression` or an access, each with `nls` before it, and
after the operator too. In 4.0.x and 5.0.x an access to a name is

```antlr
nls ((DOT | SPREAD_DOT | SAFE_DOT | SAFE_CHAIN_DOT) nls
        (AT | nonWildcardTypeArguments)?
    | METHOD_POINTER nls
    | METHOD_REFERENCE nls) namePart
```

and in 6.0.x and master the same with `NL*` for `nls`, the type
arguments of `::` aside. `commandExpression` is, in all three,

```antlr
expression
    ({ !SemanticPredicates.isFollowingArgumentsOrClosure($expression.ctx) }?
        argumentList
    | /* if pathExpression is a method call, no need to have any more arguments */)
    commandArgument*
```

so `foo { }` takes no argument list after it, being a call with a
closure, while `foo { }.bar 1`, whose path ends in a name,
is `foo({ }).bar(1)` (observed with Groovy 3, as are the other readings
here). In this grammar the statement `foo { }` is a `command_chain`
whose first argument is the closure (§5.5 item 3), and nothing went on
with it: `foo { }.bar()` was that chain, an `ERROR` and the call
`bar()`, here and at v0.6.0, and `foo { }.&bar` the chain and a method
pointer with a missing object. A value takes the call already
(`x = foo { }.bar()`, `return foo { }.bar()`), as `method_invocation`.
An alternative of `_expression` that started with the name and the
closure would let no state after a name in a value be a lone unit
reduction; the scanner decides before the name instead, as for §6.14.

The braces are counted on Groovy's tokens, as §6.11 counts brackets
before a cast: strings whole, a `${…}` with the strings in it, a slashy
string where Groovy's lexer opens one, a dollar-slashy string and block
comments, through a `BoundedLexer` that ends at the line break until the
`}`. Groovy's lexer reads `/` as division only after the last token of an
operand (GroovyLexer.g4 `isRegexAllowed`: a name, a number, a string,
`this`, `true`, `false`, `null`, `)`, `]`, `}`, `++`, `--`), so after a
keyword a `/` opens a string (`foo { return /}/ }.bar()`, `assert`,
`case`, `throw`, `yield`, `else`, `do`); `in` and `as` are the operators
after an operand and names elsewhere, and a keyword after a member
access, `::` included, is a name, as Groovy 6 reads them
(`isSlashyForbidden`): `foo { x::return / 2 }.bar()` divides. After
`yield` a `/` that whitespace follows opens no string, as the grammar
then reads `yield` as a name (§6.16), so `foo { yield / 2 }.bar()`
divides too; both were an `ERROR` before. `!in` after an operand is the
operator as well (`foo { x !in /}/ }.bar()`). The look-aheads of §6.11
(the brackets around a cast and its operand on the next line, the list
after `(x) in`), §6.17 (a parameter's default value) and §6.18 (the
elements of parentheses that open a line) keep the same state from
token to token (`SlashState` in `src/scanner.c`). Read a character at
a time, they opened a string after `x++` and divided after `in`:
`def f = (a = x++ / 2) -> a` and `(a = x in /x,y/) -> a` were an
`ERROR`, and `foo` then `(x in /a,b/)` on the next line a call. A
slashy string that opens the parentheses after an access is read whole
too (`foo { }.bar(/x/)`), so that an argument after them is found:
`foo { }.bar(/x/).baz 1` emits nothing and is an `ERROR`, as
`foo { }.bar(1).baz 1` is, where stopping at the `/` made the accesses
a call and `1` a statement of its own. The
words that the grammar reserves and Groovy does not (`_`, `permits`,
`pipeline`), and `super`, end an operand. Read as a division,
`foo { return /}.x 1/ }` was an `ERROR` and `foo { assert /}.x/ }` a
`method_invocation` without an access. A string, a comment or an interpolation that runs past the line, a
line comment and a line escape end the read as a brace that does not
close does, and so does the bound. After the `}` the view reads on past
line breaks and comments, which Groovy's lexer hides before an access.

`CLOSURE_CALL_START` opens an alternative of `method_invocation` that
holds the name and the closure, which the access then takes as its
object like any other expression: `foo { }.bar()`, `foo { } .bar(1)`,
`foo { }` then `.bar()` or `// c` and `.bar()` on later lines,
`foo { }?.bar()`, `*.`, `??.`, `.&bar`, `.@bar`, `::bar`,
`l: foo { }.bar()`, `if (c) foo { }.bar() else baz { }`,
`a { }.b { }.c()`, `foo { }.bar { }`, `foo { }.bar = 1`,
`foo { }.bar + 1`, `foo { }.bar[1]`, and `foo { }.bar` then `1` on the
next line, which stays two statements. Without an argument after them
the accesses are an expression, so the statement ends where an
expression does.

Where a first argument follows the accesses on the last name's line the
accesses are a parenthesis-free call's receiver, and the call alone
would leave the argument a statement of its own (`foo { }.bar` and `1`,
with no `ERROR`). `CLOSURE_COMMAND_START` opens the receiver instead:
`_statement_command_path`, whose first object is now one rule,
`_statement_command_object`, a statement's first name before
`_command_path_start` or the name and its closure after the token,
aliased to `method_invocation`. So `foo { }.bar 1`, `foo { }.bar "a" + b`,
`foo { }.bar x: 1`, `foo { }.bar x`, `foo { }?.bar.baz 1` and `foo { }`
then `.bar 1` on the next line are a `command_chain` whose receiver is
the access of the call, as Groovy reads them. The argument is the one the
scanner reads after a statement's dotted receiver (§6.14), but for a name
after a capitalized last name: a receiver that starts with a call names
no type, so `foo { }.Bar x` calls, as in Groovy, where `foo.Bar x`
declares `x`. Where the accesses hold `::` or a quoted name, which the
receiver rule does not take, no token is emitted when an argument
follows, and `foo { }::bar 1` and `foo { }.'bar' 1` stay the `ERROR` they
were. Parentheses after the last name that hold a parenthesis-free call
are its first argument, as §6.14 reads `obj.m (foo 1)`: `foo { }.bar(baz 1)`
is a `command_chain` whose argument is `(baz 1)`. Other brackets after an
access are read past, to the bracket that closes them, line breaks
included, and the read goes on after them; so are parentheses where
`in`, `as` or `instanceof` follows a first name (`.bar(a in b)`), an
operator on it, not a first argument. Before, the read ended there and
`foo { }.bar(a in b).qux 1` became the accesses and a statement `1`
without an `ERROR`. Where a first argument follows
them or a later access's name (`foo { }.bar().baz 1`, `a { }.b { }.c 1`,
`a { }.b(1).c 2`, `foo { }.bar[0] 1`, `foo { }.bar(1,` then `2).baz 1`,
`foo { }.bar(a in b).qux 1`),
which Groovy passes to `baz`, to `c` or to the element, no receiver of
the grammar takes the call, as none takes `foo().baz 1` (§5.5 item 2), and
the call alone would leave the argument a statement of its own: no token
is emitted, and each stays the `ERROR` it was. So does `foo { }.new X`,
Groovy's `foo({ }).new(X)`, which the grammar reads as `obj.new X` is,
the access and the statement `X`. Brackets that do not close within the
bound are the call's arguments, and what follows them is read as after
`foo().bar(…)`: the bound cuts nothing there, so that `foo { }.each(…)`
with a long argument stays a call.

After `return`, `break` and `continue` on their line neither token is
emitted, so that `return foo { }.bar 1` stays the return of
`foo { }.bar` followed by `1` and `break foo { }.bar()` the `break` of
the label `foo`, as `return obj.m 'x'` and `return foo new Foo()` are
(§5.5 item 2); Groovy rejects all three. A value of `return` takes the
call without the token.

Not read: braces that span lines (`foo {` then `}.bar()`), braces and
accesses past the bound, and the names whose own
tokens the scanner decides at a statement's start (`record`, `trait`,
`sealed`, `var`, `yield`, and `in` where valid), `as` in any state
(§6.14: after an expression it is read for the operator, and the
states must agree) and the grammar's
keywords (`super { }.bar()`). Each stays as before, an `ERROR` but for
`yield { }.bar()`, a `yield_statement`. `in { }.bar()` is the call
where no expression comes before it, and the membership test of the
closure's member after one. An operator other than a member
access after the `}` is read as before, as a statement of its own or an
`ERROR` (`foo { }[0]`, `foo { } + 1`, `foo { } as X`, `foo { }()`,
`foo { } ? a : b`), where Groovy applies it to the call;
`docs/divergences-from-spec.md` §30 lists the trees.

The bound counts from the `{`. It keeps the read linear on a line of
nested calls with closures, which is read from each `{`: a line of 1,000
to 16,000 `a {` is read 35 times per character at 128 (about 19 at 64,
66 at 256 and 130 at 512), against 3 before, and closed (`a { … } }`) 28 times; a
file of short lines (`a { b { c { } } }`) 4.6 times per character, 2.9
before, and one of `foo { a }.bar()` lines 3.1, where it was an `ERROR`
read 4.9 times. 128 characters cover a statement's closure and access on
a line of 120 columns; the longest closure that opens and closes on a
statement's line in the 266 real files is 66 characters. A line of 16,000
unclosed `a {` parses in 24 ms, 10 ms before (37 ms at 256). A line of
unclosed calls with accesses (`a { }.b { a { }.b { …`), whose brackets
the read goes into, is read 15 times per character, 3 before it did.
`bindings/rust/tests/closure_call_cost.rs` pins the bytes served per
byte at two lengths, which grow eightfold without the bound.

Each token is the statement's first, so the runtime counts what the
scanner read for it against the statement, and an edit in the braces,
after the `}` or in the argument lexes the name again;
`bindings/rust/tests/incremental_reparse.rs` checks every single edit of
thirteen such texts and four sequences against a parse from scratch.

Measured against the grammar before: STATE_COUNT 4754 to 4759,
LARGE_STATE_COUNT 1828 to 1832, SYMBOL_COUNT 412 to 416, two external
tokens and no conflict or precedence added. The `method_invocation`
alternative alone cost 12 states; with the receiver form, and the first
object of a statement's receiver folded into one rule, the total is 5.
No corpus, Java or shape tree changes; of the 266 real files one
changes, from an `ERROR` to no `ERROR`
(`pom {}.writeTo("${projectDir}/pom.xml")`). Parse time, the minimum of
30 runs (5 for the Java files): unchanged on the real files, 0.6% more
on the Java files and on Gradle- and Jenkins-style files, 0.7% more on a
file of nested empty blocks.

---

## 7. Tree-sitter conflict declarations

This list was written before the grammar and is kept as a record of
the design. None of its entries was needed. The conflicts the grammar
declares are the `conflicts` array in `grammar.js`, where each entry
has a comment; `docs/divergences-from-spec.md` §1 says why the two
differ.

The expected `conflicts` list, in order of root cause:

```js
conflicts: $ => [
  // Standard Groovy/Java ambiguity: label vs. expression-statement
  [$.label, $.identifier],
  // Map entries vs. labeled statements (resolved by external scanner)
  [$.map_entry, $.label],
  // Generic method call vs. less-than expression (`f<X>()` vs. `f < X > ()`)
  [$._type_arguments, $.binary_expression],
  // Type cast vs. parenthesized expression: (Foo) x  vs  (Foo)
  [$.parenthesized_type_cast, $.parenthesized_expression],
  // Closure as map literal: `{ k: v }` is a closure with a label,
  // `[k: v]` is a map. Inside a closure-only position (lambda body)
  // we need to prefer closure.
  [$.closure, $.map_literal],
  // Command chain vs. expression statement: `a b c` vs `a; b; c`.
  [$.command_chain, $.expression_statement],
  // Method reference vs. ternary fallback path
  [$.method_reference_expression, $.ternary_expression],
  // Trait body shares closure rule until we hit a method declaration
  [$.trait_declaration, $.class_declaration],
];
```

The amaanq grammar's 14 inherited conflicts collapse to roughly half
this set once we stop inheriting Java's `_unannotated_type`
ambiguities.

---

## 8. Test corpus plan

Tests live in `test/corpus/`. Each file is a tree-sitter test corpus
(name on top, source between `=` lines, S-expression after `---`).

Per issue #247, "regression tests anchored per AGENTS.md and lesson
1" — each operator gets a test pinned to its known-good S-expression.

### 8.1 Per-operator corpus

`test/corpus/operators-elvis.txt`:

```text
================================
Elvis at top level
================================

def x = a ?: b

---

(source_file
  (local_variable_declaration
    (variable_declarator
      name: (identifier)
      value: (elvis_expression
        value: (identifier)
        default: (identifier)))))

================================
Elvis chain (issue #246 reproducer)
================================

def pick(a, b, c) {
    return a ?: b ?: c
}

---

(source_file
  (method_declaration
    name: (identifier)
    parameters: (formal_parameters
      (formal_parameter name: (identifier))
      (formal_parameter name: (identifier))
      (formal_parameter name: (identifier)))
    body: (block
      (return_statement
        (elvis_expression
          value: (identifier)
          default: (elvis_expression
            value: (identifier)
            default: (identifier)))))))
```

`test/corpus/operators-safe-navigation.txt`:

```text
================================
Safe navigation single
================================

obj?.field

---

(source_file
  (expression_statement
    (safe_navigation_expression
      object: (identifier)
      property: (identifier))))

================================
Safe navigation chained
================================

obj?.field?.other

---

(source_file
  (expression_statement
    (safe_navigation_expression
      object: (safe_navigation_expression
        object: (identifier)
        property: (identifier))
      property: (identifier))))
```

`test/corpus/operators-spread.txt` (covers spread args, spread-dot,
spread-map):

```text
================================
Spread arguments
================================

f(*args)

---

(source_file
  (expression_statement
    (method_invocation
      function: (identifier)
      arguments: (argument_list
        (spread_arguments value: (identifier))))))

================================
Spread dot
================================

items*.size()

---

(source_file
  (expression_statement
    (method_invocation
      function: (spread_dot_expression
        object: (identifier)
        property: (identifier))
      arguments: (argument_list))))

================================
Spread map
================================

def m = [*: other, x: 1]

---

(source_file
  (local_variable_declaration
    (variable_declarator
      name: (identifier)
      value: (map_literal
        (spread_map_entry value: (identifier))
        (map_entry key: (identifier) value: (number_literal))))))
```

Other per-operator corpus files mandated:

- `operators-regex.txt` (=~, ==~ with slashy and double-quoted RHS)
- `operators-identity.txt` (===, !==)
- `operators-spaceship.txt` (<=>)
- `operators-range.txt` (.., ..<, <.., <..<)
- `operators-membership.txt` (in, !in)
- `operators-instanceof.txt` (instanceof, !instanceof)
- `operators-coercion.txt` (`x as String`, `def y = list as Set` —
  paired with `def y = (Set) list` to keep cast vs. coercion distinct)
- `operators-method-pointer.txt` (.&)
- `operators-direct-field.txt` (.@)
- `operators-subscript.txt` ([], ?[])
- `operators-method-reference.txt` (::)
- `operators-assignment.txt` (=, the augmented forms, ?=)
- `operators-implication.txt` (==>)
- `operators-power.txt` (** including `1 ** 2 ** 3` left-assoc test)
- `operators-safe-chain-dot.txt` (??.) — Groovy 4+; v2 addition
- `expressions-lambda.txt` (`->` lambdas, §5.16) — every position of
  each form, the bodies, the arrow after a line break or a comment,
  the forms Groovy rejects, and the closure defaults and case values
  that keep their reading next to a lambda
- `operators-precedence-tiebreaks.txt` — explicit tests that pin
  the Apache precedence ordering on the tie-break cases:
  - `0..n<<1` → `(0..n)<<1` (range/shift same level, left-assoc)
  - `1 < 2 instanceof Boolean` → `1 < (2 instanceof Boolean)` and
    `1 in [1] as String` → `1 in ([1] as String)` (type tests and `as`
    tighter than the relational operators)
  - `"x" =~ "x" == 1` → `"x" =~ ("x" == 1)` and `a =~ b & c` →
    `(a =~ b) & c` (regex operators between equality and `&`)
  - `a ? b ?: c : d` → `a ? (b ?: c) : d`
    (Elvis nests inside ternary's middle slot)
  - `a ?: b ?: c` → `a ?: (b ?: c)` (right-assoc)
  - `a ==> b ==> c` → `a ==> (b ==> c)` (right-assoc)
  - `1 ** 2 ** 3` → `(1 ** 2) ** 3` (left-assoc)

### 8.2 Per-issue regression corpus

`test/corpus/regressions.txt` — one example per closed issue, with a
header comment naming the issue:

```text
=========================================================
murtaza64 #5 — juxt_function_call ternary disambiguation
=========================================================

def x = foo "bar"
print foo y, z

---

(source_file
  (local_variable_declaration
    (variable_declarator
      name: (identifier)
      value: (command_chain
        receiver: (identifier)
        argument: (string_literal))))
  (command_chain
    receiver: (identifier)
    argument: (command_chain
      receiver: (identifier)
      argument: (identifier)
      argument: (identifier))))

=========================================================
murtaza64 #16 — empty groovydoc / block comment
=========================================================

/**/
def a = 1
/***/
def b = 2
/* */
def c = 3

---

(source_file
  (groovydoc_comment)
  (local_variable_declaration
    (variable_declarator name: (identifier) value: (number_literal)))
  (groovydoc_comment)
  (local_variable_declaration
    (variable_declarator name: (identifier) value: (number_literal)))
  (block_comment)
  (local_variable_declaration
    (variable_declarator name: (identifier) value: (number_literal))))

=========================================================
murtaza64 #22 — multiple assignment
=========================================================

def (a, b) = [1, 2]

---

(source_file
  (multi_assignment_declaration
    (variable_declarator name: (identifier))
    (variable_declarator name: (identifier))
    value: (list_literal
      (number_literal)
      (number_literal))))

=========================================================
murtaza64 #36 — enum + switch arrow
=========================================================

enum Color { R, G, B }

def toS(c) {
    switch (c) {
        case Color.R -> 'r'
        case Color.G -> 'g'
        case Color.B -> 'b'
    }
}

---

(source_file
  (enum_declaration
    name: (identifier)
    body: (enum_body
      (enum_constant name: (identifier))
      (enum_constant name: (identifier))
      (enum_constant name: (identifier))))
  (method_declaration
    name: (identifier)
    parameters: (formal_parameters (formal_parameter name: (identifier)))
    body: (block
      (switch_expression
        value: (identifier)
        body: (switch_block
          (switch_arrow_case
            value: (field_access
              object: (identifier)
              field: (identifier))
            body: (string_literal))
          (switch_arrow_case
            value: (field_access
              object: (identifier)
              field: (identifier))
            body: (string_literal))
          (switch_arrow_case
            value: (field_access
              object: (identifier)
              field: (identifier))
            body: (string_literal)))))))

=========================================================
murtaza64 #37 — pipeline followed by definition
=========================================================

pipeline {}
def foo = 5

---

(source_file
  (pipeline_statement body: (closure))
  (local_variable_declaration
    (variable_declarator name: (identifier) value: (number_literal))))

=========================================================
murtaza64 #39 — multi-catch
=========================================================

try {
    risky()
} catch (IOException | RuntimeException e) {
    handle(e)
}

---

(source_file
  (try_statement
    body: (block (expression_statement (method_invocation function: (identifier) arguments: (argument_list))))
    (catch_clause
      parameter: (catch_formal_parameter
        type: (multi_type
          (type_identifier)
          (type_identifier))
        name: (identifier))
      body: (block (expression_statement (method_invocation function: (identifier) arguments: (argument_list (identifier))))))))

=========================================================
dekobon #246 — Elvis as elvis_expression, not ternary
=========================================================

def pick(a, b, c) {
    return a ?: b ?: c
}

---

(source_file
  (method_declaration
    name: (identifier)
    parameters: (formal_parameters
      (formal_parameter name: (identifier))
      (formal_parameter name: (identifier))
      (formal_parameter name: (identifier)))
    body: (block
      (return_statement
        (elvis_expression
          value: (identifier)
          default: (elvis_expression
            value: (identifier)
            default: (identifier)))))))

=========================================================
dekobon #247 — trait declaration
=========================================================

trait Greeter {
    def greet() {
        "hello"
    }
}

---

(source_file
  (trait_declaration
    name: (identifier)
    body: (class_body
      (method_declaration
        name: (identifier)
        parameters: (formal_parameters)
        body: (block (string_literal (string_fragment)))))))

=========================================================
dekobon #247 — exclusive range
=========================================================

for (i in 0..<n) { f(i) }

---

(source_file
  (for_in_statement
    variable: (identifier)
    value: (range_expression
      start: (number_literal)
      end: (identifier))
    body: (block (expression_statement (method_invocation function: (identifier) arguments: (argument_list (identifier)))))))
```

### 8.3 Stress / real-world corpus

A folder of public-domain Groovy from Jenkinsfile examples, Gradle
build scripts, and Apache Groovy's own test suite. Acceptance bar:
zero `ERROR` nodes on the entire corpus. Files vendored under
`test/stress/` to avoid network flakiness in CI.

### 8.4 Test execution

`make test` runs:

1. `tree-sitter test` over `test/corpus/`.
2. A Rust integration test in `bindings/rust/tests/parse_stress.rs`
   that walks `test/stress/`, parses each file, and asserts no
   `ERROR` nodes and no `MISSING` nodes are present in the parse
   tree. (The MISSING-node assertion is the *direct* anti-regression
   for issue #246's failure mode.)

---

## 9. Highlight, locals, injection queries

### 9.1 highlights.scm

Standard tree-sitter highlighting captures:

- `@keyword` — `def`, `class`, `trait`, `enum`, `record`, `interface`,
  `if`, `else`, `for`, `while`, `do`, `switch`, `case`, `default`,
  `return`, `throw`, `try`, `catch`, `finally`, `break`, `continue`,
  `import`, `package`, `new`, `as`, `in`, `instanceof`, `assert`,
  `yield`, `pipeline`, `sealed`, `non-sealed`, `permits`.
  Fixes murtaza64 #36 (enum unhighlighted).
- `@operator` — every operator node in §3.2.
- `@string`, `@string.regex` (slashy strings), `@string.escape`.
- `@type` — class names in declaration position and after `:`.
- `@function` — identifier in method declaration and at call site.
- `@variable` — bare identifiers in expression position.
- `@variable.parameter` — the names of method, closure and lambda
  parameters, the one name of `x -> …` included.
- `@comment` — `block_comment`, `line_comment`, `groovydoc_comment`.

### 9.2 injections.scm

- Slashy-string and dollar-slashy bodies inject as `regex`. Solves
  the "slashy is a regex" expectation in Jenkins users' editors.
- A call on the bare name `sql` or `Sql` whose first argument is a
  `"…"` or `"""…"""` string injects that string as `sql`, with or
  without parentheses (`sql.execute """…"""`, `sql.rows("…")`). Later
  string arguments are bind parameters and are not injected. No other
  embedded language is shipped.
- Groovydoc comments inject as `javadoc`.

### 9.3 locals.scm

Standard Groovy scope rules:

- `method_declaration.parameters`, `closure.parameters`, `for_loop`,
  `for_in_statement`, `catch_clause.parameter` are scopes, and so is
  `lambda_expression`, whose parameters (a `formal_parameter`, or the
  one `identifier` of `x -> …`) are definitions.
- `formal_parameter`, `variable_declarator`, `rest_declarator`,
  `multi_assignment_declaration` define identifiers. The names of a
  `multi_assignment_expression` are references, as it declares
  nothing.
- `identifier` references resolve to nearest enclosing definition.

---

## 10. Issue resolution map

How each filed bug closes against this spec.

| Issue | Where this spec addresses it |
|---|---|
| dekobon/big-code-analysis **#246** (Elvis short-circuit) | §3.2 — `elvis_expression` as a distinct named node. §8.1 — pinned corpus test reproducing the issue's `def pick(a, b, c) { … }` snippet with the expected S-expression. The downstream `get_op_type()` impl can now `matches!(id.into(), AMPAMP \| PIPEPIPE \| ELVIS_EXPRESSION)`. No `MISSING` child anywhere. |
| dekobon/big-code-analysis **#247** umbrella (all operator gaps) | §3.2 row-by-row: every symbol in the issue's three tables maps to a distinct node kind. The "critical" rows (`?:`, `?.`, `as`, `trait`, `..<`) all have dedicated nodes. The "important" rows (`=~`, `==~`, slashy, `===`, `!==`, `<=>`, `!instanceof`, `!in`) likewise. The "niche" rows (`?=`, `?[]`, `.&`, `.@`, `$/…/$`, `==>`) likewise. §8 anchors each with a pinned test. §3.2.1 explicitly handles the subscript-yielding-callable composition (`arr[i](args)`). §3.2.2 documents the lexer trailing-context rule for `!instanceof` / `!in` to avoid mis-tokenisation when followed by an identifier character. |
| amaanq #2 / #3 (maintenance / license) | The prior grammar is treated as inspiration only — no code is incorporated. This grammar is an independent project, dual-licensed under `Apache-2.0 OR MIT`, with no dependency on the upstream project's release cadence. |
| murtaza64 **#5** (juxt_function_call ambiguity) | §5.5 — command chains restricted to statements, to the values of declarations and assignments, to the content of parentheses and interpolations and to a parenthesized first argument of a command chain, the receiver a name or a name with member accesses, conflicts with `binary_expression` resolved by precedence. A name as the first argument (`foo bar`) is told from a declaration (`Foo bar`) by Apache's `isInvalidLocalVariableDeclaration`, which the scanner applies before the receiver (§6.14). §8.2 — pinned test on `f y:b ? … : …` resolving to `f(y:b)` then ternary. |
| murtaza64 **#16** (empty block comments) | §6.6 — block comments and groovydoc comments are external-scanner tokens that consume the entire `/* … */` or `/** … */` regardless of content. §8.2 — pinned test with `/**/`, `/***/`, `/* */`. |
| murtaza64 **#22** (multiple assignment) | §4 — `multi_assignment_declaration` rule. §8.2 — pinned test on `def (a, b) = [1, 2]`. |
| murtaza64 **#27** (publish to npm) | Out of grammar scope. The package layout in §2.1 includes `package.json` and `bindings/node/`; npm publishing is part of release tooling, not the grammar. |
| murtaza64 **#36** (enum + switch arrow) | §4 — `enum_declaration` with constants. `switch_arrow_case` rule. §9.1 — `enum` as `@keyword`. §8.2 — pinned regression. |
| murtaza64 **#37** (pipeline followed by def) | §4 — `pipeline_statement` is a normal statement, not a file-trailer-only production. §6.4 — AUTOMATIC_SEMICOLON ensures the newline between `pipeline {}` and `def foo = 5` is a statement terminator. §8.2 — pinned regression. |
| murtaza64 **#38** (TS typing on node bindings) | Out of grammar scope; lives in `bindings/node/index.d.ts`. The fix is to declare `language: Language` to match the upstream `tree-sitter` types. Documented in §11. |
| murtaza64 **#39** (multi-catch) | §4 — `catch_formal_parameter` accepts a `multi_type` (`Type1 \| Type2`). §8.2 — pinned regression. |

---

## 11. Bindings and packaging

Each binding file is generated from `grammar.js` via `tree-sitter
generate` and committed; the C scanner is hand-authored.

- `bindings/node/`: includes `binding.gyp`, `index.js`, `index.d.ts`.
  The `index.d.ts` declares `language: Language` to match the
  upstream `tree-sitter` types (fixes murtaza64 #38).
- `bindings/python/`: `pyproject.toml` + `setup.py` for
  `tree-sitter-groovy` PyPI package.
- `bindings/rust/`: a `lib.rs` exposing the `LANGUAGE` constant, in
  the `dekobon-tree-sitter-groovy` crate published on crates.io. It
  builds with `cc` and has no Cargo features.
  **This is the integration target for `dekobon/big-code-analysis`.**
- `bindings/go/`: a Go module exposing `Language()`.
- `bindings/swift/`: `Package.swift` for SwiftPM consumers.

`tree-sitter.json` declares grammar metadata for editor integration
per the tree-sitter CLI's expected schema (file types `.groovy`,
`.gradle`, `.gvy`, `.gy`, `Jenkinsfile`).

---

## 12. Versioning and release plan

- v0.1.0 — Initial grammar with all of §3 (operator coverage) and §4
  (statement/declaration coverage). Resolves dekobon #246, #247 in
  full and all murtaza64 open issues except #27 / #38 / #36's
  "enum highlighted" sub-bug (latter is a query-file change in v0.1.1).
- v0.1.1 — Highlight query refinements; node-binding TS typing fix.
- v0.2.0 — Groovy 4 record/sealed support (already in §4; promoted
  to GA when Groovy 4's spec stabilises for both).

semver: the rules are in `AGENTS.md` ("Semantic Versioning") and are
not repeated here. In short, a change to the tree of input that parsed
is breaking, a new rule, field or capture is additive, and a fix that
changes no existing tree is a patch.

The `dekobon/big-code-analysis` pin (currently
`tree-sitter-groovy = "=0.1.2"` from amaanq) will be moved to this
crate at v0.1.0 publication. The dekobon-side bump is tracked by
**this spec** acting as the published artifact; the actual bump PR
on dekobon's side is a follow-up.

---

## 13. Out-of-scope but worth noting

- **Tree-sitter incremental reparsing** correctness for slashy
  strings: editing inside a slashy can change whether subsequent
  `/` is a string or division. This is a known tree-sitter wart; we
  mitigate via the external scanner's strict state model but cannot
  fully fix it without parser-API changes upstream.
- **Groovy `as` for closure coercion** (`{ it } as Comparator`) is
  the same `cast_expression` as `x as String` — the RHS type can be
  any `_type`, including a single type identifier or a generic type.
- **Stub generation / IDE support beyond editor highlighting** —
  language servers can consume this grammar but are not in scope.
- **Performance budget**: parsing the Apache Groovy codebase
  (~250k LOC of `.groovy` test files) under 5 seconds on a modern
  laptop is the target. Tree-sitter's incremental nature makes
  steady-state editing free. Four shapes of input that parse with an
  `ERROR` take time above linear in their length, where v0.6.0 was
  linear: nested `foo 1, %a = …` calls (400 levels take about 3
  seconds), runs of `@C(,` and a space, of `foo(case,` and a space and
  `x ? y :` followed by a line with `case`, repeated, the last two with
  the reserved `case` in expression position;
  `docs/divergences-from-spec.md` §29 has the measurements, and
  `bindings/rust/tests/error_recovery_cost.rs` pins their cost.
  `case 1:` lines outside a `switch` were another. They are
  linear, and so are `case A:` and `x = case` lines, with `case`
  reserved and a `case` label where a statement starts an error
  production (§5.2); reserving `case` alone made the last two
  quadratic. A run of `, b) =` after the `=` of a multi-assignment
  declaration (`def (a, b) =, b) = …`) was one more, while the names
  were a rule of their own and a list of keyed pairs took no plain
  name; with the `=` and the value in the rule of the names' parentheses,
  and one list for names and pairs, it is linear, at 24 parse states.

---

## 14. Sources

Primary references used in writing this spec:

- Apache Groovy language docs:
  - `https://groovy-lang.org/operators.html`
  - `https://groovy-lang.org/syntax.html`
  - `https://groovy-lang.org/semantics.html`
  - `https://groovy-lang.org/objectorientation.html`
  - `https://groovy-lang.org/dsls.html`
  - `https://groovy-lang.org/style-guide.html`
- Apache Groovy reference grammar (ANTLR4):
  - `https://github.com/apache/groovy/blob/master/src/antlr/GroovyParser.g4`
  - `https://github.com/apache/groovy/blob/master/src/antlr/GroovyLexer.g4`
- Source grammars analysed:
  - `https://github.com/amaanq/tree-sitter-groovy/blob/master/grammar.js`
  - `https://github.com/murtaza64/tree-sitter-groovy/blob/main/grammar.js`
- Bug reports being closed:
  - `https://github.com/dekobon/big-code-analysis/issues/246`
  - `https://github.com/dekobon/big-code-analysis/issues/247`
- Sibling tree-sitter grammars in this checkout providing the layout
  template: `tree-sitter-tcl/`, `tree-sitter-irules/`.
