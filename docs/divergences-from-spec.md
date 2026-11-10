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

## Spec implementation status at a glance

| Spec section | Status | Notes |
|---|---|---|
| §3 Operators (matrix) | **Done** | Every operator in §3.2 has its named node kind. |
| §4 Statements / decls | **Done** | class / trait / interface / enum / record / sealed / @interface / method / constructor / field / static_initializer / instance initializer / multi-assignment / pipeline / try-with-resources / multi-catch / arrow switch / yield / labeled. |
| §5.1 Numeric literals | **Done** | bin / oct / hex / dec, underscores, all type suffixes. |
| §5.2 Contextual keywords | **Mostly done** | `var`, `record`, `sealed`, `non-sealed`, `permits`, `yield` parsed contextually. Groovy-5 candidates `val`/`async`/`await`/`defer` attempted but reverted: tree-sitter cannot fall back from a matched keyword literal to identifier when the keyword rule's RHS shape fails, so `async()`, `val.x()`, etc. error. See divergence #10. |
| §5.3 Label vs map vs ternary | **Done** | grammar-precedence resolution; pinned in `expressions-quoted-identifier.txt`. |
| §5.4 Slashy vs division | **Done** | scanner context guards + same-line lookahead. |
| §5.5 Command chains | **Done** | multi-arg form supported. |
| §5.6 Optional parentheses | **Done** | command_chain at statement level. |
| §5.7 GString interpolation | **Done** | structured `gstring_dollar_interpolation` + `gstring_brace_interpolation` for `"…"`, `"""…"""`, `/…/`, `$/…/$`. |
| §5.8 Closure params / it | **Done** | typed and untyped, no synthetic `it`. |
| §5.9 Annotations | **Done** | `@`, `@Foo(args)`, `@a.b.Foo`, stacked. |
| §5.10 Statement terminators | **Equivalent** | newline-as-terminator via `repeat($._statement)` rather than a scanner token (see #3 below). |
| §5.11 Spread (three positions) | **Done** | `spread_arguments`, `spread_dot_expression`, `spread_map_entry`. |
| §5.12 `def` positions | **Mostly done** | typed-bare `Type x` (no init) deferred (see #6 below). |
| §5.13 Trailing commas | **Done** | accepted in list / map / arg / param / enum-constant lists. |
| §5.14 Method reference `::` | **Done** | identifier and `new` targets. |
| §5.15 Parenthesized type cast | **Done** | distinct `parenthesized_type_cast` node kind. |
| §6.2 Slashy scanner | **Done** | context guards + same-line lookahead. |
| §6.3 GString scanner | **Equivalent** | grammar-level; spec proposes scanner state, grammar achieves the same children. |
| §6.4 AUTOMATIC_SEMICOLON | **Equivalent** | grammar-boundary resolution covers every spec-named case (see #3 below; #11 for a `{` on the next line). |
| §6.5 LABEL_COLON | **Equivalent** | grammar precedence on `labeled_statement` covers spec-named cases. A scanner-token implementation was attempted in this session but coupled to a not-yet-implemented named-argument command-chain (`archiveArtifacts artifacts: 'x'`), which the workaround grammar path quietly absorbed. Reverted to grammar precedence; v2 work is named-args + scanner token together. |
| §6.6 Block-comment scanner | **Done** | single-token emit, `/**/` and `/***/` parse correctly. |
| §7 Conflicts | **Done** | actual conflicts list smaller than spec predicted (see #1). |
| §8 Tie-break corpus | **Done** | all five spec-mandated tie-breaks pinned in `operators-precedence-tiebreaks.txt`. |
| §9.1 highlights.scm | **Done** | every node kind has a capture. |
| §9.2 injections.scm | **Done** | groovydoc → javadoc; slashy + dollar-slashy → regex; named-call DSL → sql. |
| §9.3 locals.scm | **Done** | scopes, definitions, references. |
| §10 Issue resolution | **Done** | all eleven listed issues closed by corresponding rules and corpus regressions. |

The sections marked **Equivalent** below are cases where the spec
proposes an external-scanner implementation strategy, but the
observable behaviour the spec requires is already delivered by an
in-grammar mechanism. The "deliverable contract" — every example in
§3, §4, §5, and §10 parses without `ERROR` or `MISSING` nodes — is
fully met. A v2 refactor to true scanner tokens is optional polish.

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

## 2. GString interpolation — implemented in grammar (no scanner)

**Spec §5.7 / §6.3** require double-quoted and triple-double
strings to expose interpolation structure (`$identifier` and
`${expr}`) as parse-tree children rather than as flat text.

**Current implementation** (no external scanner): the
`string_literal` rule routes double-quoted and triple-double
bodies through structured `_gstring_part` / `_triple_gstring_part`
alternatives. Each body is a sequence of:

- `string_fragment` (literal text or escape, never containing `$`),
- `gstring_dollar_interpolation` with a `value` field whose
  `identifier` captures the `$name.path.chain` shape, and
- `gstring_brace_interpolation` with a `value` field whose
  `_expression` carries any full Groovy expression inside `${…}`.

A bare `$` not followed by an identifier-start or `{` (e.g. the
trailing `$` in `"^a.c$"`) is preserved as a single-char
`string_fragment` via the `_gstring_literal_dollar` fallback
token (`prec(-1)` so a real `$name` still wins).

Limitation: tree-sitter regex tokens cannot do look-ahead, so the
two-token shape (`text` stopping before `$`, plus the
`literal_dollar` fallback) is necessarily slightly noisier than a
scanner-driven state machine would produce — but downstream
tooling sees the same children either way.

---

## 3. AUTOMATIC_SEMICOLON — realised via statement-boundary structure

**Spec §6.4** wants the external scanner to emit an automatic
semicolon when a newline follows a statement-ending token and the
next token doesn't continue the expression.

**Current implementation**: `\n` is in the `extras` set and the
top-level `source_file` / `block` / `class_body` rules all use
`repeat($._statement)`, so the parser starts a fresh statement
after every `\n` that doesn't keep the current statement alive
through a pending operand. This gives the same observable behaviour
as a true automatic-semicolon token for every case `SPECIFICATION.md`
calls out:

| Spec-named case | Actual result |
|-----------------|---------------|
| `pipeline {}\ndef foo = 5` (murtaza64 #37) | parses as two statements; pinned in `test/corpus/regressions.txt` |
| `return foo\nbar` | parses as `return foo` then `bar` |
| `a\nb` | parses as two expression statements |
| `foo()\n.bar()` (leading-dot chain) | parses as one method invocation (line-continuation) |
| `a\n+ b` (leading-operator) | currently parses as one binary expression — see edge case below |

One observable difference from the spec's scanner-token design
is the leading-operator edge case: `a\n+ b` greedily joins as
`a + b` rather than splitting. Idiomatic Groovy keeps continuation
operators at the end of the previous line (`a +\n  b`), so the
case rarely appears in real code; a stricter scanner-driven
AUTOMATIC_SEMICOLON is a v2 improvement.

The other is a `{` that opens the next line: it joins the previous
expression where the §6.4 contract as written would end the
statement. After a call that is Apache's reading too; after
`new T(...)` it is not (see #11).

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
- Generics (`List<String>`) ✓ via `generic_type`, with bounded
  wildcards (`? extends Foo`, `? super Bar`), nested arguments
  (`Map<String, List<Integer>>`), qualified bases
  (`java.util.List<String>`), class / interface / trait
  `type_parameters`, and method-level `method_type_parameters`
  (`<T> T identity(T x)`). The opening `<` of `type_arguments` and
  `type_parameters` is `token.immediate` so it only binds to a
  base name without intervening whitespace — preserving `a < b` as
  a binary comparison.

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
The match still works because slashy bodies now have
`string_fragment` and interpolation children (the regex match runs
against the captured source range, which still starts with `/`).

If a future quoted-string flavour also began with `/`, the match
anchor would mis-route. The fully robust fix is distinct node
kinds (`slashy_string_literal`, `dollar_slashy_string_literal`),
which a v0.2.0 AST-shape change can introduce alongside any other
breaking renames. Until then the current source-anchor predicate
covers every shipping flavour without ambiguity.

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

---

## 10. Groovy-5 contextual keywords (`val` / `async` / `await` / `defer`)

**Spec §5.2** lists `val`, `async`, `await`, and `defer` as
contextual keywords — keywords in specific syntactic positions,
identifiers elsewhere (`val` is conditionally enabled in real
Groovy via `'val' {isValEnabled()}?`).

**Current implementation**: not exposed as keyword tokens. An
in-session attempt added `defer_statement`, `await_expression`,
`async_closure`, and a `val` alternative to
`local_variable_declaration`, but every contextual keyword form
produced an ERROR node when the same identifier appeared in an
expression-start position not matching the keyword rule's
required shape:

- `async()` — caller wants `async` as a function name; the rule
  required `{ … }` to follow, so the parse errored.
- `val.x()` — caller wants `val` as a variable; the rule
  required `name = value`, so the parse errored.
- `await.method()` / `defer.y()` — same problem.

The root cause is tree-sitter's lexer: once a rule consumes the
literal `'async'` as a keyword token, there is no fallback to
match it as `$.identifier` even when the rest of the rule fails.
`prec.dynamic(-1, …)` and conflict declarations both proved
insufficient because the decision is made at the lexer layer, not
the parser layer.

**Action**: re-enable when one of these lands:

1. Tree-sitter exposes a per-rule "soft keyword" mechanism that
   lets the lexer fall back to identifier when the keyword rule
   fails, or
2. The grammar gains a feature flag that opts the file into
   Groovy-5 mode (e.g. via shebang or a leading directive), at
   which point the keyword rules can be unconditionally active.

Until then, code containing `val` / `async` / `await` / `defer`
parses with these tokens as plain identifiers, which matches the
behaviour of Groovy 2.x–4.x compilers.

---

## 11. A `{` on the next line continues the previous expression

**Apache `GroovyParser.g4`** allows a line break before a trailing
closure but not before an anonymous class body (rules quoted from
the 4.0.x branch; `master` spells `nls` as `NL*`):

- `pathExpression` ends in `(pathElement)*`, and `pathElement` is
  `nls ( … | closureOrLambdaExpression ) | arguments | …`, with the
  comment "Can always append a block, as foo{bar}". The `nls` lets
  the closure start on a later line.
- `creator` is
  `createdName ( nls arguments anonymousInnerClassDeclaration[0]? | … )`.
  Nothing separates the arguments from the class body, so the body
  cannot follow an `NL` token.
- `GroovyLexer.g4` hides `NL` while the innermost open bracket is
  `(`, `[` or `?[` ("Inside (...) and [...] but not {...}, ignore
  newlines"; the resource list of `try (…)` is excepted), so there a
  line break before either `{` does not count.

The Groovy 3.0 release notes ("Nested code blocks") say the same of
calls: a block after a method call is its closure argument "even
after a new line", and the previous statement may need a terminating
`;` when a separate block is meant. Parsed by Groovy 4.0.33,

```groovy
def list = new ArrayList()
{ list.add(1) }
```

is one statement, `def list = new ArrayList().call({ ... })`, and no
anonymous class is created; `new Foo() { }`, with the `{` on the
same line, creates one. Those two results were observed. Everything
else this section says about Groovy is read off `GroovyParser.g4`.

**Spec §6.4** would end the statement instead: its
AUTOMATIC_SEMICOLON contract emits the terminator at a line break
outside `(` and `[` unless the next token is a binary operator, a
dot, `?` or `:`, and `{` is not on that list. For a trailing closure
it is the contract that departs from Apache, not the grammar.

**Current implementation**: newlines are `extras` (§5.10), so the
parser cannot see the line break and a `{` binds the same way on
either line.

After a call, a name or a literal the `{` joins that expression,
which is Apache's reading. `foo()` with `{ x }` on the next line is
one call with a trailing closure, and so is an Allman-style
Jenkinsfile or Gradle block (`node('linux')` with its `{ … }`
opening the next line). In a class body the same holds after a field
initialiser:

```groovy
class A {
    int x = 5
    {
        x = 6
    }
}
```

parses without an `ERROR` as a field whose value is the call
`5 { x = 6 }`, not as a field followed by an instance initializer.
The instance initializer (a bare `block` in `class_body`) is
recognised where nothing before it can take the `{`: at the start
of the body, after a member that ends in `}` (a method or
constructor with a body, another initializer, a nested enum), after
a field with no initialiser (`int x`), and after a `;`. A block
after a bodiless method (`abstract void f()`) becomes that method's
body, which Apache's `(nls methodBody)?` allows as well.

Three places in a member list do differ from Apache:

- **A field with no initialiser.** A `{ … }` on the *same* line is
  an instance initializer here too, so `int x { … }` and
  `void run { … }` (parentheses forgotten) parse without an `ERROR`;
  Apache's `classBody` wants a `sep` between members. In a record
  body this hides an unsupported construct: the compact canonical
  constructor `public Name { … }` parses as a modifier-only field
  `Name` followed by an initializer block.
- **A nested `class`, `interface` or `trait`.** These have no
  production in a member list yet, so the keyword lexes as an
  identifier there and `class Inner { … }` takes the same reading: a
  field `Inner` of type `class` followed by an initializer block
  that holds the members, often with no `ERROR` at all (an
  `extends Base` clause reads as one more field). v0.4.0, which had
  no initializer block, reported an `ERROR` at the `{`. Nested enums
  have a production and are not affected.
- **An enum body.** `enum_body` takes no instance initializer at
  all: `enum E { A, B; { init() } }` is an error, where Apache's
  `classBody` accepts the block.

The larger divergence is after `new T(...)`: every `{` that follows
the arguments is the anonymous class body. Apache instead has a
closure called on the new object when

- the `{` starts a later line, outside parentheses and brackets.
  Here the `ArrayList` input above gets `{ list.add(1) }` as the
  class body of an anonymous `ArrayList` subclass, with an `ERROR`
  for the statement inside it; or
- the `{` is on the same line but its content cannot be a class
  body, as in `new T() { x -> x }` (ANTLR skips the optional
  `anonymousInnerClassDeclaration` when it cannot match). Here that
  is a `class_body` with an `ERROR` in it.

v0.4.0 parsed both as a call with a trailing closure. A next-line
block made of members,

```groovy
def r = new Runnable()
{
    void run() { }
}
```

is accepted as an anonymous class without an error, although
`creator` cannot produce one there. Directly inside parentheses or
brackets (`foo(new T()` with the body opening the next line) the
line break changes nothing in either parser.

Workaround: where the closure reading after `new T(...)` is wanted,
wrap the creation in parentheses (`(new ArrayList())`, then the
`{`) or spell the call out (`new ArrayList().call { … }`). A `;`
after `new T(...)` makes the block a separate statement, which is
not Groovy's reading of the original. For an instance initializer
after `int x = 5`, end the field with `;` or put the block where
nothing before it can take the `{`, as in Groovy.

**Action**: none for the trailing closure. A newline-aware token
for that `{` would move the grammar away from Apache and break the
Allman-style calls above, and so would an AUTOMATIC_SEMICOLON built
to the §6.4 contract as written; the contract has to treat a `{` as
continuing the expression first.

For `new T(...)` two routes are open, neither taken yet. A declared
conflict on `object_creation_expression` with `prec.dynamic` on the
body restores both closure forms, but any body with an error in it
(a member `class_body` cannot parse, or one still being typed) then
falls back without an `ERROR` to the call-with-closure tree that #68
reported. A newline-aware scanner token would refuse the body after
a line break; it has to track the enclosing bracket, because
Apache's lexer hides the break inside `(` and `[`, and it does
nothing for the same-line form.

Nested `class` / `interface` / `trait` declarations need their own
productions in `_class_member`. Once the keyword is valid at member
start it no longer lexes as an identifier, and the field-plus-block
reading goes away.

Two `new` forms never reach `object_creation_expression`, so a `{`
after them is not a class body either. Both predate #68, and in the
positions shown neither reports an `ERROR`, with or without a body:

- Qualified creation, `def i = outer.new Inner(...) { … }` (Apache
  `pathElement`: `DOT nls NEW creator[1]`), has no production.
  `.new` reads as a property access and `Inner(...) { … }` as a
  separate call with a trailing closure. As a call argument the
  form is an `ERROR`.
- In a member list, after another member, a cast written directly
  in front of `new` loses to a reading in which `new` is an
  identifier, provided the arguments can pass for parameters (none,
  or bare names): `def f = (Foo) new T() { … }` becomes a field
  `f = (Foo)` followed by a method `T()` with return type `new`.
  `new T() as Foo` and `(Foo) (new T())` parse as intended.
