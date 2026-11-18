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
   Groovy 2.x–4.x source files without `ERROR` nodes for the constructs
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
5. **No tracking of Groovy 5+ syntax** until released. We target the
   Groovy 4.x grammar; Groovy 3.x and 2.x are subsets and parse with
   the same productions.

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
797–908). The numbering follows Apache's "level" comments
(1 = highest, 15 = lowest); the tree-sitter PREC constants we use
are simply 21 minus this number so higher = tighter — but the
*relative* order is what matters and it matches Apache exactly.

| Apache level | Name | Operators | Associativity | Notes |
|---:|---|---|---|---|
| 1 | UNARY_NOT | `~` `!` | right | |
| 2 | POWER | `**` | right | `1 ** 2 ** 3` → `1 ** (2 ** 3)` |
| 3 | UNARY_ADD | prefix `++` `--`, unary `+` `-` | right | |
| 4 | MUL | `*` `/` `%` | left | |
| 5 | ADD | `+` `-` (binary) | left | |
| 6 | SHIFT_OR_RANGE | `<<` `>>` `>>>` `..` `..<` `<..` `<..<` | left | **Apache puts ranges and shifts at the same level**, with left-associative resolution. `0..n<<1` parses as `(0..n)<<1`. |
| 7 | REL | `<` `<=` `>` `>=` `in` `!in` `instanceof` `!instanceof` `as` | left | `as` and `!instanceof` take a *type* on the right (not an expression); `in` / `!in` / `<` etc. take an expression. |
| 8 | EQ | `==` `!=` `<=>` `===` `!==` `=~` `==~` | left | Apache's comment calls regex "level 8.5" but the rule places them in the same alternative as `==` family. We treat them as one level. |
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
  has `op=(AS | NOT_INSTANCEOF) nls coercionType` versus
  `op=INSTANCEOF nls matchingType` versus
  `op=(LE | GE | …) nls right=expression`. The grammar must
  thread this through — the RHS of `as` is `_type`, RHS of `<` is
  `_expression`. See §3.2 cast_expression and
  instanceof_expression fields.

### 3.2 Operator → node kind map

Every cell maps to a distinct named node so that
`get_op_type()` in big-code-analysis can do exact node-kind matching
instead of fishing through tree shape. This is the explicit ask in
issue #246's "Suggested fix" path 1.

| Symbol | Node kind | Field shape | Notes |
|---|---|---|---|
| `+` `-` `*` `/` `%` `<<` `>>` `>>>` `&` `^` `\|` `&&` `\|\|` `<` `<=` `>` `>=` `==` `!=` | `binary_expression` | `left`, `operator`, `right` | Java-shaped; precedence per §3.1. |
| `**` | `power_expression` | `left`, `right` | Right-associative; separate node so metric counts power-ops distinctly. |
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
| `*` (prefix in arg) | `spread_arguments` | `value` | Only valid inside argument lists / list literals. |
| `*:` (prefix in map) | `spread_map_entry` | `value` | Only valid inside map literals. |
| `?[]` | `safe_subscript_expression` | `object`, `index` | Distinct from regular `subscript`. |
| `[]` | `subscript_expression` | `object`, `index` | |
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

#### 3.2.2 `!instanceof` and `!in` — lexer trailing-context rule

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
| Method declaration | port from murtaza64 | `def` or type, optional throws clause |
| Method definition (with body) | port from murtaza64 | |
| Multi-assignment | **new** | `def (a, b) = expr` is a `multi_assignment_declaration` (murtaza64 #22). `(a, b) = expr` is a `multi_assignment_expression` (#94), after Apache `#multipleAssignmentExprAlt`: two or more plain names, then `=` and no other operator, with line breaks allowed around the `=`. It is an expression, so it nests: `x = (a, b) = list`, `foo((a, b) = list)`, a `for` init, after `return`. Its names are `identifier`s rather than `variable_declarator`s, as nothing is declared. With one name, `(a) = x` is an `assignment_expression` of a parenthesized expression, as in Apache. A `(` that opens a line continues an expression that ends the line before as a call, where Groovy ends the statement (`docs/divergences-from-spec.md` §3), with no `ERROR`: two multi-assignments on consecutive lines, `(a, b) = [1, 2]` then `(c, d) = [3, 4]`, are one statement, `(a, b) = ([1, 2](c, d) = [3, 4])`, and in the swap `def b = 2` then `(a, b) = [b, a]` the call `2(a, b)` is assigned. A `;` after the previous statement avoids it. The reverse also holds: `def f(a, b) = [1, 2]`, which Groovy rejects, is a declaration of `f` followed by a multi-assignment. The value of either form may be a parenthesis-free call, as Apache's `statementExpression` and `variableInitializer` allow: `(a, b) = f 3` and `def (a, b) = f 3` (§5.5, #95). The declaration form follows Apache's `variableDeclaration` with `typeNamePairs`: annotations, then `def`, `final def`, `final var` or `final` alone (`final (a, b) = …`, `@Field def (a, b) = …`). A name may have a type (`typeNamePair`): `def (int a, String b) = …`, `final (List<String> xs, b) = …`. A typed name is a `variable_declarator` with a `type` field before its `name`; an untyped name keeps the `variable_declarator` with a `name` alone. Two Groovy 6 forms are accepted, after Groovy 6's `GroovyParser.g4` and `AstBuilder`; the Groovy 3 this grammar is checked against rejects both. A rest binding, `def (a, *b) = list` or `def (h, List<Integer> *t) = list`, has its `*` in the `rest` field of its `variable_declarator`. A keyed pair, `def (name: n, age: int a) = map`, is a `variable_declarator` with a `key` field before the optional `type` and the `name`. A list holds names or keyed pairs, not both. The declaration is also a `for` init (`for (def (i, n) = [0, 5]; i < n; i++)`) and a member of a type body, Apache's `fieldDeclaration`, with the annotations and modifiers of a field and `def`, or modifiers alone (`static final def (a, b) = …`, `private (int a, b) = …`). Groovy parses the member and then drops it: it declares no field (Groovy 3 to 6). Not accepted, each an `ERROR`: `var` alone (`var (a, b) = …`), which Groovy accepts. `var` is a keyword where a statement starts, so `var` as a name there (`int var = (x)`, `var = (x)`) is an `ERROR`, and with a `(` valid after `var` the recovery from that error resumed at the `(` and could take in the whole method. `final var (a, b) = …` is accepted, so the same still happens after `final`: in a method, `final var = (1)` can turn the method and what follows into one `ERROR`, where without `final` the `ERROR` stays on the line. Both go once `var` is accepted as a name. Also not accepted: annotations alone (`@A (a, b) = …`), where the `(` would open the annotation's arguments or the names; other modifiers where a statement starts (`static def (a, b) = …`), which Groovy 3 takes and ignores where it rejects `static def x = 1`; `var` in a type body, where `var x = 1` is a field of type `var`; and a list that mixes names and keyed pairs. Wider than Groovy: `def (a = 1, b) = …` parses. A keyword or `val` before a name is its type: `def (def a, b)` has the type `def`, and so do `var`, `val`, `final` and `default` in that place, where Groovy 6 reads `def`, `var` and `val` as no type and Groovy rejects `final` and `default`. A reserved word is a key (`def (class: v) = m`, `def (if: v) = m`; `new` is not), as it is a name in `def (a, if) = l`: a keyword with no action lexes as an identifier. More than one rest binding parses (`def (*a, *b) = …`), which Groovy rejects after parsing. As a line break is whitespace, `foo final (a, b) = c` and `foo def (a, b) = c` are a statement `foo` followed by a declaration, where Groovy rejects them. A line that ends in a parenthesized name, dotted name or primitive type (`def y = (x)`, `x = (b)`, `return (x)`, `(Foo.Bar)`, `(int)`) followed by a multi-assignment declaration on a later line is two statements, as in Groovy, when the declaration starts with `final`, has a rest binding or keyed pairs, or is a member of a type body. Before those forms were accepted the second line was read with no `ERROR` as the operand of a cast, a call of a method named by its first word: `(x) def(h, *t) = l` (`docs/divergences-from-spec.md` §15). After a parenthesized generic type the cast reading remains: `def y = (List<String>)` then `def (a, *b) = l`, or `def (a, b) = l`, is still `(List<String>) def(a, *b) = l`, where Groovy reads two statements. |
| Local variable declaration | port from both | including `def` and explicit types; annotations may come first (`@Field def x = 1`), and `@Library('shared') _`, an annotation on the placeholder name `_`, is a declaration of `_`. Apache takes any name after annotations alone; only `_` is accepted here, because a newline is whitespace and `@A x` followed by `y = 1` would also be the typed local `x y = 1`. |
| Closure expression | port from murtaza64 | `{ a, b -> … }` and `{ … }`; parameters take annotations, `final`, `def` or a type, varargs and a default (§5.8). |
| `if` / `else` | port | |
| `while` / `do-while` | port | |
| `synchronized` statement | **new** | `synchronized (lock) { … }`; the body must be a block. Apache `#synchronizedStmtAlt`. |
| `for(;;)` | port | `init` accepts a single expression (`i = 0`), a `local_variable_declaration` (`int i = 0`, `def i = 0`, incl. multiple declarators) or a `multi_assignment_declaration` (`def (i, n) = [0, 5]`). Apache `forInit: localVariableDeclaration \| expressionList` also allows a comma expression list (`i = 0, j = 1`) — not yet supported. |
| `for (x in xs)` | port | |
| `switch` with `case … :` | port | |
| `switch` with `case … -> …` | **new** | arrow form. Closes murtaza64 #36 (second half). |
| `try` / `catch` / `finally` | port | |
| Multi-catch | **new** | `catch (Foo \| Bar e)`. Closes murtaza64 #39. A catch parameter takes `final` and `def`, `var` or a type. |
| Try-with-resources | **new** | `try (Foo f = …) { … }`; a resource takes `final` and `def`, `var` or a type. |
| `return`, `break`, `continue`, `yield` | port | break/continue accept an optional label. |
| `throw` | port | |
| `assert e` and `assert e : msg` | port from murtaza64 | power-assertion shape. |
| Labeled statement | port from murtaza64 | `name: statement`. Disambiguation vs. map literal in §5.3. |
| Command chain (`juxt_function_call`) | port from murtaza64 + §5.5 | |
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
needed. The same technique is available to the other contextual
words.

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
   grammar accepts a command chain as a statement, and as the value of
   a declarator (a local variable, a field, `def (a, b) = …`) or of an
   assignment with any operator (`x = foo 1`, `x.y += foo 1`,
   `(a, b) = f 3`). That covers Jenkins' `def out = sh script: 'make',
   returnStdout: true`. An assignment is an expression, so the value
   form nests where an assignment does: `println(x = foo 1)`,
   `"${x = foo 1}"`. The value form is the same `command_chain` node
   with the same fields. Not accepted here, and taken by Groovy
   (observed with Groovy 3): a chain in parentheses (`(foo 1)`,
   `if (foo 1)`); a chain as the only argument of a call (`bar(foo 1)`,
   `println(sh script: 'x', returnStdout: true)`, and `bar(foo 1, 2)`,
   which is `bar(foo(1, 2))`), though Groovy rejects one among several
   arguments (`bar(a, foo 1)`); and a chain in an interpolation
   (`"${foo 1}"`). Each is an `ERROR`. Nor is a lambda body, which the
   grammar does not have. Apache's array initializer
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
   and keeps its next-line reading (#90). Not accepted, though Groovy
   takes each as a call (observed): spaces or a line break inside the
   receiver (`obj . m 1`, `obj` followed by `.m 1` on the next line), a
   receiver not made of names (`foo().bar 1`, `foo[0].bar 1`, `'s'.m 1`,
   `(obj).m 1`), a quoted name (`obj.'name' 1`), `::` (`obj::m 1`), and
   a value whose first name is `new`, `switch`, `true`, `false` or
   `null` (`def x = true.x 1`). Each parses as before, as the
   expression followed by the argument.
   A dotted receiver that starts a statement is a call also after
   another statement on the same line: `x.y a.b 'c'`, `foo a.b 'x'` and
   `foo 1 a.b 'c'` are the first statement followed by the call
   `a.b 'c'`. Groovy continues the first statement there instead
   (`x.y(a.b).c`, item 4). After `return`, on its line or the next one,
   the name is the returned value, so `return obj.m 'x'` is `return
   obj.m` followed by `'x'`, as before #52; Groovy rejects the first and
   reads the second as a `return` and a call.
   A capitalized last name before a quoted name and `(` is the return
   type of a method, as in Groovy at script and class level:
   `java.util.List 'foo'() { }` declares `foo`, while `obj.m 'a'() { }` passes `'a'({ })` to `m`. Groovy
   decides the first by trying the parameter list, so
   `java.util.List 'foo'(1)`, a call there, is the expression followed
   by `'foo'(1)` here.
3. **Argument forms allowed**: the first argument is a literal, a
   closure, or a named arg (`k: v`). After a comma an argument is a
   named arg or any expression (`assertEquals 'v', a + b`,
   `foo 'a', x ? 1 : 2`). That is Apache's `argumentListElement`
   without spread arguments: `foo 'a', *args` and `foo a: 1, *:m`
   are an `ERROR`.
   A named-arg key is an identifier; Apache also accepts string keys
   (`foo 'a': 1`), which are not implemented yet (§6.8). Disallowed:
   nested command chains, other than as the value of an assignment
   (`bar 1, x = foo 2`, item 1), and an identifier or other non-literal
   expression as the first positional argument: `foo bar` is also how
   a typed declaration starts (`Type name = …`), and the parser cannot
   tell the two apart before the `=`. A bare `foo bar` parses as two
   expression statements.
   In a value (item 1), and after a dotted receiver (item 2), the first
   positional argument is any expression that starts with a literal, as
   in Apache's `argumentList`: `def x = foo 'a' + b` and
   `log.info 'x: ' + y` pass the sum. Neither takes a closure as the
   first argument: `def c = foo { }` and `list.each { }` stay a
   `method_invocation`, which is how Apache reads a closure after a
   name. Nor does either take a name, although no declaration can start
   there: `def x = foo bar` is `def x = foo` followed by `bar`, as in
   statement position, and `a.b c` is `a.b` followed by `c` (Groovy
   calls `a.b(c)`; `a.b.C x = 1` declares `x`). After a
   value's arguments a `,` continues the call, as Apache's greedy
   argument list does: `def x = foo 1, y = 2` passes `y = 2` to `foo`
   rather than declaring `y`, and in `[x = foo a: 1, b: 2]` the `b: 2`
   is the call's named argument, not a map entry.
4. **Chain continuation**: after the first call, subsequent `name`
   tokens become property/method dispatches. The Apache grammar uses
   `commandArgument: pathElement+ argumentList?` for the chain
   continuation. It is not implemented: `foo 1 b 2` parses as the two
   chains `foo 1` and `b 2`, in a value as well, and `foo 'a' obj.m 'b'`
   as `foo 'a'` and `obj.m 'b'`.
5. **Conflicts** with `binary_expression` (where `a - b` could be
   "call `a` with `-b`") are resolved by giving `binary_expression`
   higher precedence: a command chain *only* matches when the token
   after the receiver can start a first argument (item 3), with or
   without a space before it (`foo'a'`). `foo -1` is the subtraction
   `foo - 1` and `foo [1]` a subscript, as in Groovy.

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
assignment (`def x = f 1`, §5.5 item 1). The grammar does not need a
separate rule.

### 5.7 GString interpolation

The double-quoted-string body alternates between literal fragments,
escape sequences, and interpolation segments. Two interpolation
forms:

- `$identifier` — followed by an immediate identifier (with optional
  dotted property access, no method call). The identifier follows
  §5.2, without `$`, so `"値は$名前です"` interpolates `名前です`, as
  in Groovy.
- `${ expression }` — full expression, recursively parsed.

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
  tokens (§6.7–§6.9) skip it as they skip spaces and tabs, so the next
  line continues the current one: `x \` followed by `[0]` is a
  subscript, and `foo \` followed by `a: 1` is a named-argument call.
  A `\` anywhere else outside a string is an `ERROR`. Two cases differ
  from Apache. A `\` before a lone `\r` is an `ERROR` when more input
  follows the `\r`. At the very end of the file a `\`, alone or with a
  `\r` after it, is skipped with no `ERROR`, because the lexer has
  already taken it as the start of the pattern.
- **Semicolon** `;` is an optional, anonymous element of every
  statement list (`repeat(choice($._statement, ';'))`) and member list
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
| Member access | `recv*.prop` / `recv*.method(args)` | `spread_dot_expression` |
| Map literal entry | `*: expr` | `spread_map_entry` |

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

- a local variable declaration (`def x = …`),
- a field (`def x` in a class or enum body),
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
  prec.dynamic(1, prec.right(PREC.UNARY, seq(
    '(', $._type, alias($._cast_close, ')'), $._expression))),
  prec.dynamic(-1, prec.right(PREC.UNARY, seq(
    '(', $._type, ')', $._expression))))
```

Ambiguity with parenthesized expression: `(a) b` could be a cast
(if `a` is a type) or a juxt-call. Resolved by precedence and
GLR: if `a` matches `_type`, prefer the cast; otherwise it's a
parenthesized expression followed by a juxt-call (which is a
syntax error in expression position, by §5.5).

Two rules settle it, both from Apache's
`castParExpression castOperandExpression`:

- On one line the cast wins wherever it parses, as the cast
  alternative comes first in Apache's `expression`: `(a) + b`,
  `(a) [0]` and `(a) (b)` are casts, `(a) * b` and `(a).size()` are
  not. The first alternative carries this: it closes with the scanner
  token `CAST_CLOSE` (§6.11), a `)` with something after it on its
  line, and has a positive dynamic precedence.
- The rule has no `nls` between its two parts, so where a line break
  can end a statement the operand is on the line of the `)`: `x = (a)`
  followed by `foo()` on the next line is two statements. The second
  alternative carries this: a cast whose operand is on a later line
  has a negative dynamic precedence and loses to any other complete
  reading. Inside `(…)` and `[…]` Groovy ignores line breaks and reads
  a cast. Here the cast stands there when no other reading parses
  (`foo((String)` followed by `x)`), and loses when the later line can
  also continue the expression (`foo((a)` followed by `- b)` is a
  subtraction); see `docs/divergences-from-spec.md` §15.

Without the two weights the readings were unranked and the result
depended on the surrounding code.

---

## 6. External scanner design

The external scanner is the C file `src/scanner.c`. It exists to
handle the context-sensitive lexing situations that pure tree-sitter
grammar rules cannot disambiguate. Each token below has a single
emit/no-emit decision per call. The implementation exports the
slashy opener, the three comment tokens, the contextual `record`
(§6.7), `trait` and `sealed` (§6.13) keywords, the named-argument key of a parenthesis-free call
(§6.8), the start of such a call that is a value or a statement
with a dotted receiver (§6.14), the
same-line subscript opener (§6.9), the start of an array type that is
a value (§6.15), the `)` of a cast
(§6.11), the `!in` operator (§6.12), the empty marker of an
unterminated block comment (§6.6), `STRING_BODY` (§6.10) and
`TYPE_START` (§6.15), which are never emitted and only mark a string
body and where a type may start, and
`ERROR_SENTINEL`, which no rule references and which only signals
error recovery; the other tokens listed here are met in-grammar, see
`docs/divergences-from-spec.md`.

### 6.1 Tokens exported

```c
enum TokenType {
  AUTOMATIC_SEMICOLON,    // §5.10 — newline-as-terminator
  SLASHY_STRING_START,    // §5.4 — opening `/` of slashy regex
  SLASHY_STRING_BODY,     // §5.4 — body (until unescaped `/`)
  SLASHY_STRING_END,      // §5.4 — closing `/`
  DOLLAR_SLASHY_STRING,   // §5.7 — single-token `$/.../$` when no interpolation
  GSTRING_BODY,           // §5.7 — literal fragment of a double-quoted string
  GSTRING_INTERPOLATION_START, // `${` or `$` followed by identifier
  LABEL_COLON,            // §5.3 — colon that closes a statement label
  LINE_COMMENT,           // §6.6 — `//` to end of line, single token
  BLOCK_COMMENT,          // single-token block comment incl. /**/ and /***/
  GROOVYDOC_COMMENT,      // single-token /** ... */ comment
  RECORD_KEYWORD,         // §6.7 — `record` only at the start of a record header
  TRAIT_KEYWORD,          // §6.13 — `trait` only at the start of a trait header
  SEALED_KEYWORD,         // §6.13 — `sealed` only before `class`, `interface` or `trait`
  COMMAND_NAMED_KEY,      // §5.5 — empty; before the `name` of `name:` in a parenthesis-free call
  COMMAND_VALUE_START,    // §6.14 — empty; before the receiver of such a call that is a value
  COMMAND_PATH_START,     // §6.14 — empty; after the first name of such a call's dotted receiver in a statement
  SUBSCRIPT_OPEN,         // §5.10 — `[` on the same line as the token before it
  ARRAY_VALUE_START,      // §6.15 — empty; before an array type that is a value
  CAST_CLOSE,             // §6.11 — `)` with something after it on its line
  NOT_IN,                 // §6.12 — `!in` before whitespace or an opening bracket
  UNTERMINATED_COMMENT,   // §6.6 — empty; at the first `/*` with no `*/` after it
  STRING_BODY,            // §6.10 — never emitted; valid only inside a string body
  TYPE_START,             // §6.15 — never emitted; valid only where a type may start
  ERROR_SENTINEL,         // never in a rule; valid only during error recovery
};
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
no `*/` from here on", which is the only state it has. The bit is a
property of the input and stays true at every later position, so
later `/*` openers, and the block comments the keyword look-aheads
(§6.7, §6.13) skip, fail at once while it is set.

tree-sitter stores scanner state only in a token the scanner emits,
and restores it from the last such token before every call, so a
failed scan cannot leave the bit behind. The scan that reaches the end
of input therefore emits `UNTERMINATED_COMMENT`: an empty, hidden
token, listed in `extras` because a comment may start anywhere. The
runtime accepts an empty extra token only when the state changed with
it, which holds exactly once per parse path. The state is empty while
the bit is unset and one byte once it is set.

An edit does not leave the bit stale. The marker read to the end of
input, so after any edit behind it the runtime lexes it again, and a
node lexed with the bit set is reused only where the new parse has
the bit set too. A change of included ranges is not an edit: a range
added later that contains `*/` leaves the marker in place, because
the runtime checks only a node's own look-ahead, as it does for every
other token.

One reparse differs from a parse from scratch because the marker is
empty. In `x = (a)/* n` followed by `c` on the next line, a cast and
a parenthesized expression are both possible when the parser reaches
`/*`. The parse path of the cast shifts the marker, fails on `/` and
is dropped before the other path reduces, so `(a)` is reduced with
one path alive and is not marked as the outcome of a conflict.
Inserting `*/` before `c` makes the text valid, and the reparse
reuses that `parenthesized_expression`: `x = (a)`, a comment, and `c`
as a second statement, where a parse from scratch gives one statement
with a `parenthesized_type_cast` over `c`. The reparse has no error,
and the next edit to that node corrects it. This is accepted. It was
4 in 1_000_000 random chained edits, where 3 differed before the
marker existed, from other causes.
`bindings/rust/tests/incremental_reparse.rs` checks reparses against
parses from scratch and pins this case.

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
sets it, so that costs one extra read of the input, once.

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
over a name and the same whitespace for a single `:` (a following
second `:` means a method reference, so no emit). `case` and `default`
are rejected so a switch label after a statement on the same line stays
a label.

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
identifier, and the key position reserves no word, which covers `new`.
An empty external token is ignored by the tree-sitter runtime directly
after a parse error, so a key there is not recognized until the parser
has shifted another token.

After a comma the statement cannot end, so a key on a continuation line
(`sh script: 'make',` then `returnStdout: true`) is matched in the grammar
as a plain identifier followed by `:` and needs no scanner token.

Known limitations:

- String keys (`foo 'a': 1`, `foo "$k": 1`) are an `ERROR`. Apache's
  `namedArgPrimary` accepts them.
- `case` and `default` cannot be keys as the first argument. Apache's
  `namedArgLabel` accepts every keyword, so `foo default: 1` is a
  named-argument call there and an expression statement plus a label here.
  The divergence is deliberate: the one-line
  `switch (x) { case 1: foo default: bar }` depends on it. After a comma
  both are accepted as keys (`foo 'a', default: 1`, `foo a: 1, case: 2`),
  on the same line or the next: the scanner still rejects them, and the
  grammar's continuation rule takes the keyword as a plain identifier.
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
  line reading `(a)` parses as the call `2(a)`. The same happens when the
  next line starts with `{` (a trailing closure), `-` or `+` (a binary
  operator), `++` or `--` (a postfix update: `foo 1, 2` followed by `++x`
  parses as `2++` and then `x`), `in`, `as` or `instanceof`. A next line
  that starts with `[` is not affected: a `[` after a newline never
  subscripts (§6.9), so `foo 1, 2` followed by `[a].each { }` is two
  statements. A chain whose one argument is positional (`foo 'a'` then
  `(a)`) is not affected either, because that argument is not a full
  expression; in a value it is one, and joins (§6.14). The value of a
  named argument is one, so it joins even as
  the only argument: `foo a: 1` followed by `(b)` has the value `1(b)`.
  Expressions outside command chains already behave this way:
  `def x = 'b'` followed by `(a)` is a call on `'b'`.
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

One residual does not involve a line break. A name followed by `(` is
not a trait header, so `trait T(int x) { }` is a method `T` with
return type `trait` and no `ERROR`, at statement level and in a member
list; Groovy 3 rejects it (observed), and v0.6.0 gave an `ERROR`. It
is the counterpart of the `record foo() { … }` residual of §6.7.

### 6.14 COMMAND_VALUE_START and COMMAND_PATH_START — implementation contract

A parenthesis-free call may be the value of a declarator or an
assignment (§5.5 item 1), and its receiver may have member accesses
(§5.5 item 2). A newline is whitespace, so with only the grammar
`def x = foo` followed by `1` on the next line would be one call, and
so would `result.size` followed by `2`; Groovy ends the statement at
the line break (observed with Groovy 3). Both forms therefore hold an
empty token, which the scanner emits only when the first argument
starts on the receiver's line.

```text
Emit COMMAND_VALUE_START, empty, where a value may start, when:
  - after whitespace, line breaks and line escapes (a value may start
    on the line after its `=`), a name starts, read as in §6.8; and
  - the name is not `new`, `switch`, `true`, `false` or `null`, which
    start a value as keywords; and
  - any member accesses follow it, as below; and
  - the first argument follows, as below.
The token ends before the name.

Emit COMMAND_PATH_START, empty, directly after the first name of a
statement, when:
  - a member access starts there, with nothing before it; and
  - one or more member accesses follow, as below; and
  - the first argument follows, as below; and
  - not: the last name of the receiver starts with `A` to `Z`, the
    argument is a string that closes on the line with no `$` in it,
    and `(` follows it on the line.
The token ends after the name.

Member accesses: each `.`, `?.`, `??.`, `*.`, `.&` or `.@` followed by
a name, with no space anywhere.
The first argument: after spaces, tabs and line escapes, but no line
break and no comment, a digit, `'`, `"`, the word `true`, `false` or
`null`, or a key: a name followed on the same line by `:` and not
`::`, other than `case` and `default` (§6.8).
The in-grammar lexer reads the receiver and the argument. On any other
input return false. Neither token is emitted during error recovery.
```

The two tokens are decided where nothing hides a line break from the
scanner. Comments are `extras`, so after a comment the parser is where
it was and the scanner is called again, past the line break it can no
longer see. A token placed after the receiver would read `def x = foo`
followed by `/* c */ 1` on the next line as one call.
COMMAND_VALUE_START is decided before the receiver, and
COMMAND_PATH_START between its first name and the operator after it,
where no whitespace, comment or line break may come. Every form with a
line break after the receiver parses as it did before.

The scanner reads on after a statement's first name only when a member
access follows it directly, so it reads nothing more on other
statements. Those that start with a dotted name are read once more. A
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
(§5.5 item 3). `record`, `trait` and `sealed` followed by a member
access are names (§6.7, §6.13), so `record.add 'x'` is a call.

A string after a receiver whose last name is capitalized, followed by
`(`, is the name of a method: Groovy declares `foo` in
`java.util.List 'foo'() { }` and `java.util.List '''foo''' () { }`, and
calls in `obj.m 'a'() { }`, `java.util.list 'a'() { }` and
`java.util.List "$x"() { }` (observed). The scanner reads the string,
triple-quoted or not, to its close on the line.

A slashy string is not a first argument, as in Groovy, which reads `/`
after a name as division. Known differences from Groovy:

- A comment between the receiver and its first argument on one line,
  `def x = foo /* c */ 1` or `x.y /* c */ 1`, is not a call: the
  look-ahead reads no comment, so it never scans an unterminated one to
  the end of the input. The tree is the one from before, the receiver,
  the comment and `1`. Groovy reads a call (observed with Groovy 3),
  also when the comment spans lines.
- A name as the first argument (`def x = foo bar`) is not taken
  (§5.5 item 3).
- Two literals Groovy takes as a first argument do not start with a
  digit or a quote, so the token is not emitted (observed with
  Groovy 3). `def x = foo $/a/$`, which Groovy reads as `foo('a')`, is
  `def x = foo` followed by the dollar-slashy string, with no `ERROR`.
  `def x = foo .5`, Groovy's `foo(0.5)`, is an `ERROR`: `.5` is not a
  number in this grammar at all. Both read the same before the value
  form.
- The first positional argument is an expression, and like every
  expression it continues onto a next line that can continue it. A
  next line that starts with `(` makes a call: `def x = foo 1` followed
  by `(c)` passes `1(c)`. One that starts with `-`, `+`, `++` or `--`
  makes a binary or postfix expression: `x.y 1` followed by `-1`
  passes `1 - 1`, and `x.y 2` followed by `++i` passes `2++` and leaves
  `i`. Groovy ends the statement at the line break in each case
  (observed). That is the general reading of such a line
  (`docs/divergences-from-spec.md` §3), and an argument after a comma
  does the same in statement position (§6.8). Before, the receiver was
  followed by the same expression as a statement of its own, `1(c)` or
  `1 - 1`: only where that expression is attached changed.
- The receiver is read without spaces, comments or line breaks inside
  it, so `obj . m 1` and `obj` followed by `.m 1` on the next line are
  not calls; Groovy reads both as `obj.m(1)` (observed). Both read as
  they did before #52.

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
`a[0]` costs nothing more. `java.lang.String[].class` (a package
before the class), `foo[]` with a lowercase class `foo`, and a generic
element type are not taken and read as before: the first two are a
subscript with a missing index, and `List<String>[].class` is a chain
of comparisons with no error, which is kept. In `.Name` the names are
read without spaces, as in §6.14.

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
- `operators-power.txt` (** including `2 ** 3 ** 2` right-assoc test)
- `operators-safe-chain-dot.txt` (??.) — Groovy 4+; v2 addition
- `operators-precedence-tiebreaks.txt` — explicit tests that pin
  the Apache precedence ordering on the tie-break cases:
  - `0..n<<1` → `(0..n)<<1` (range/shift same level, left-assoc)
  - `a ? b ?: c : d` → `a ? (b ?: c) : d`
    (Elvis nests inside ternary's middle slot)
  - `a ?: b ?: c` → `a ?: (b ?: c)` (right-assoc)
  - `a ==> b ==> c` → `a ==> (b ==> c)` (right-assoc)
  - `1 ** 2 ** 3` → `1 ** (2 ** 3)` (right-assoc)

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
  `for_in_statement`, `catch_clause.parameter` are scopes.
- `formal_parameter`, `variable_declarator`, `multi_assignment_declaration`
  define identifiers. The names of a `multi_assignment_expression` are
  references, as it declares nothing.
- `identifier` references resolve to nearest enclosing definition.

---

## 10. Issue resolution map

How each filed bug closes against this spec.

| Issue | Where this spec addresses it |
|---|---|
| dekobon/big-code-analysis **#246** (Elvis short-circuit) | §3.2 — `elvis_expression` as a distinct named node. §8.1 — pinned corpus test reproducing the issue's `def pick(a, b, c) { … }` snippet with the expected S-expression. The downstream `get_op_type()` impl can now `matches!(id.into(), AMPAMP \| PIPEPIPE \| ELVIS_EXPRESSION)`. No `MISSING` child anywhere. |
| dekobon/big-code-analysis **#247** umbrella (all operator gaps) | §3.2 row-by-row: every symbol in the issue's three tables maps to a distinct node kind. The "critical" rows (`?:`, `?.`, `as`, `trait`, `..<`) all have dedicated nodes. The "important" rows (`=~`, `==~`, slashy, `===`, `!==`, `<=>`, `!instanceof`, `!in`) likewise. The "niche" rows (`?=`, `?[]`, `.&`, `.@`, `$/…/$`, `==>`) likewise. §8 anchors each with a pinned test. §3.2.1 explicitly handles the subscript-yielding-callable composition (`arr[i](args)`). §3.2.2 documents the lexer trailing-context rule for `!instanceof` / `!in` to avoid mis-tokenisation when followed by an identifier character. |
| amaanq #2 / #3 (maintenance / license) | The prior grammar is treated as inspiration only — no code is incorporated. This grammar is an independent project, dual-licensed under `Apache-2.0 OR MIT`, with no dependency on the upstream project's release cadence. |
| murtaza64 **#5** (juxt_function_call ambiguity) | §5.5 — command chains restricted to statements and to the values of declarations and assignments, the receiver a name or a name with member accesses, conflicts with `binary_expression` resolved by precedence. §8.2 — pinned test on `f y:b ? … : …` resolving to `f(y:b)` then ternary. |
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
  steady-state editing free.

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
