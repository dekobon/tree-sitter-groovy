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
| §4 Statements / decls | **Done** | class / trait / interface / enum / record / sealed / @interface / method / constructor / field / static_initializer / instance initializer / nested class, interface, trait, enum, record and @interface / multi-assignment / pipeline / try-with-resources / multi-catch / arrow switch / yield / labeled. |
| §5.1 Numeric literals | **Done** | bin / oct / hex / dec, underscores, all type suffixes. |
| §5.2 Contextual keywords | **Mostly done** | `var`, `sealed`, `non-sealed`, `permits`, `yield` parsed contextually as grammar literals. `record` is a scanner token emitted only before a record header (§6.7), so it is an identifier everywhere else. Groovy-5 candidates `val`/`async`/`await`/`defer` attempted but reverted: tree-sitter cannot fall back from a matched keyword literal to identifier when the keyword rule's RHS shape fails, so `async()`, `val.x()`, etc. error. See divergence #10. |
| §5.3 Label vs map vs ternary | **Done** | grammar-precedence resolution; pinned in `expressions-quoted-identifier.txt`. |
| §5.4 Slashy vs division | **Done** | scanner context guards + same-line lookahead. |
| §5.5 Command chains | **Mostly done** | multi-arg and named-argument forms supported. String keys (`foo 'a': 1`) are not; `case` / `default` are not accepted as keys as the first argument, a deliberate divergence from Apache; an argument after a comma continues onto a following line that can continue an expression (see spec §6.8). |
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
| §6.5 LABEL_COLON | **Equivalent** | grammar precedence on `labeled_statement` covers spec-named cases. A scanner-token implementation was attempted earlier but coupled to the then-missing named-argument command chain (`archiveArtifacts artifacts: 'x'`). Named-argument command chains now use their own scanner token, `_command_named_key` (an identifier followed by `:` on the same line, excluding `case` / `default`), and labels keep grammar precedence. |
| §6.6 Block-comment scanner | **Done** | single-token emit, `/**/` and `/***/` parse correctly. |
| §6.7 RECORD_KEYWORD | **Done** | `record` emitted only when a name follows on the same line and `(` or `<` after it, comments and a line break before `(` allowed; `record.key()`, `private Record record`, `public void record()` are identifiers. Residuals listed in §6.7. |
| §6.8 COMMAND_NAMED_KEY | **Done** | an empty token, emitted only before a name followed by `:` on the same line, never during error recovery; the in-grammar lexer reads the key, so a non-ASCII key (`foo 名前: 1`) is an identifier by the grammar's own class. Limitations are in the §5.5 row above and spec §6.8. |
| §6.9 SUBSCRIPT_OPEN | **Done** | a `[` after a newline starts a list or map literal, not a subscript. A newline before `[` ends the expression everywhere, including inside `(…)`, `[…]`, `${…}` and conditions such as `if (a` + newline + `[0])`. Differs from Groovy inside `(…)`, `[…]` and `?[…]`, where Groovy ignores newlines: `foo(a` + newline + `[0])`, `[a` + newline + `[0]]`, and the array-type forms `(String` + newline + `[]) x` and `def m(String` + newline + `[] a) { }` are valid Groovy and an `ERROR` here (stateless scanner; a faithful fix needs a paren/bracket/brace stack). A block comment at the start of the `[` line hides the newline, so `a` + newline + `/* c */ [0]` is a subscript here. |
| §6.10 STRING_BODY | **Done** | never emitted; valid only where a segment of a `"…"`, `"""…"""`, `/…/` or `$/…/$` body may start. The scanner emits nothing there, so `//` and `/*` in a string are text. A comment inside `${…}` is still a comment. |
| §6.11 CAST_CLOSE | **Done** | the `)` of a cast, emitted only when something follows it on its line (spaces, tabs and line escapes skipped; a line comment ends the line). `x = (a)` followed by `foo()` is two statements, as in Groovy. A parenthesized expression closes with this token or a plain `)`. |
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
`final Type x` is the exception: a blank final is accepted, with a
single declarator.

Because a newline is whitespace (#3), the tokens of a typed
declaration also parse as two statements: `String s = 'a'` as the
expression `String` followed by `s = 'a'`. The typed forms carry a
dynamic precedence, so the declaration wins whenever both parse;
without it a comment before the declaration changed the result. The
cost is the reverse case: a bare `x` followed by `y = 1` on the next
line is the declaration `x y = 1`, and `final a` followed by
`String b = 'y'` is `final a String` followed by `b = 'y'`.

A method declaration with a body carries a dynamic precedence for the
same reason: `String f() { }` also parses as the expression `String`
followed by the call `f() { }`, and `def f(a) { }` as a variable `f`
followed by the cast `(a) { }`. Here too the reverse case loses: a
bare `x` followed by `foo() { }` on the next line is the method
`x foo() { }`. The method's weight is above the blank final's, because
`final String f(a) { }` is also `final String f` followed by the cast
`(a) { }`. That costs the blank final one case: `final String name`
followed by `foo() { }` on the next line is `final String` followed by
the method `name foo() { }`.

In a class body the weights rank a typed field above a method with a
body, and give a field with modifiers only a negative weight. Without
the first, `private final int v` followed by a constructor was
`private final int` and a method `v C() { }`. Without the second, the
same field followed by a comment and a member that starts with a
modifier (`private String w`, `public A() { }`, `static { }`) was
`private final int`, a field of type `v` named after that modifier,
and the rest: a keyword is an identifier wherever it has no action.

A method declaration without a body carries a negative dynamic
precedence, so `x` followed by `foo()` on the next line is two
statements, as is `println foo()` on one line. Outside a class body a
method needs a body, so only the statement reading is valid Groovy.
The cost is on invalid input: `String foo()` with no body, which
Groovy rejects in a script, parses as `String` followed by `foo()`
with no `ERROR`. In a class body nothing competes with the method, so
abstract and interface methods are unaffected.

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
   which point the keyword rules can be unconditionally active, or
3. Each word gets the treatment `record` has (spec §6.7): an
   external-scanner token emitted only when the construct's own
   shape follows (`async {`, `await <expr>`, `defer {`), so the word
   lexes as an identifier everywhere else. The scanner decides by
   look-ahead, which is exactly the fallback the lexer layer lacks.

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
constructor with a body, another initializer, a nested type), after
a field with no initialiser (`int x`), and after a `;`. A block
after a bodiless method (`abstract void f()`) becomes that method's
body, which Apache's `(nls methodBody)?` allows as well.

Two places in a member list do differ from Apache:

- **A field with no initialiser.** A `{ … }` on the *same* line is
  an instance initializer here too, so `int x { … }` and
  `void run { … }` (parentheses forgotten) parse without an `ERROR`;
  Apache's `classBody` wants a `sep` between members. In a record
  body this hides an unsupported construct: the compact canonical
  constructor `public Name { … }` parses as a modifier-only field
  `Name` followed by an initializer block.
- **An enum constant.** `enum_body` takes the initializer wherever
  its member list repeats — after `;`, after a first member, or
  with no constants — but not directly after a constant. With the
  line break invisible, `A` followed by `{ … }` on the next line
  (in Apache a constant, then an initializer) cannot be told from
  `A { … }` (a constant with an anonymous class body, which has no
  production), and both stay errors. Write `A;` first.

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

One `new` form never reaches `object_creation_expression`, so a `{`
after it is not a class body either. Qualified creation,
`def i = outer.new Inner(...) { … }` (Apache `pathElement`:
`DOT nls NEW creator[1]`), has no production. `.new` reads as a
property access and `Inner(...) { … }` as a separate call with a
trailing closure, with no `ERROR`, with or without a body. As a call
argument the form is an `ERROR`. This predates #68.

---

## 12. `sealed` and `trait` are keywords where a member can start

**Apache `GroovyParser.g4`** lets both words name things: its
`identifier` rule lists `SEALED` and `TRAIT` next to `Identifier`,
so `private boolean sealed` and `String trait` are ordinary fields.
**Spec §5.2** asks for the same through an
`identifier_or_contextual` fallback.

**Current implementation**: the declaration rules spell the words
as literals, and with `word: $.identifier` the lexer hands back the
keyword wherever the parser could accept it, with no fallback to
`identifier`. That was already so where a statement can start:
`boolean sealed = true` in a method body, and a statement that
opens with a variable named `trait`, are errors at v0.4.0 too
(`def sealed = 1` is not).

Nested type declarations (#60, #68) extend it to member lists. A
nested `class`, `interface`, `trait`, `record` or `@interface` has
to be a member, because otherwise its keyword lexes as an identifier
and `class Inner { … }` parses, with no `ERROR`, as a field `Inner`
of type `class` followed by an instance initializer block. With the
productions in place `sealed` and `trait` are keywords wherever a
member can start:

| Input in a class body | Result |
|---|---|
| `boolean sealed`, `String trait`, `def sealed = 1` | field, as before |
| `int x, trait`, `List<String> sealed`, `@Lazy List sealed` | field, as before |
| `void sealed() { }`, parameters and property accesses named `sealed` / `trait` | as before |
| `private boolean sealed`, `private String trait = "x"` | `ERROR` (a field at v0.4.0) |
| `public static final int sealed = 1` | `ERROR` (a field at v0.4.0) |
| `static trait = 1`, `private trait`, `sealed() { }` | `ERROR` (parsed at v0.4.0) |
| `enum E { trait, B }`, `enum E { sealed }`, `enum E { A, sealed }` | constants (the first two an `ERROR` at v0.5.0) |

The rows that fail share one shape. A field needs no type after a
modifier, so `private boolean` is already a complete field named
`boolean`, a new member may start right after it, and the lexer
prefers the keyword. Without a modifier the type cannot stand
alone, the next word can only be the name, and it stays an
identifier.

An enum constant sits where a member could start too: the first one,
and since #89 every one after a `,`. `enum_constant` therefore takes
the two keywords as a name, aliased to `identifier`, at a lower
precedence than the declarations, so `sealed class` and `trait T {`
are still declarations there. One shape stays an `ERROR`, as at
v0.5.0: a constant named `trait` followed, with no `;` or `,`, by a
member that starts with a name (`trait` then `int f() { }` on the
next line), which reads as a trait declaration.

`record` escapes the table because its keyword is not a grammar
literal: the external scanner emits it only when a name follows on
the same line and `(` after that (spec §6.7), so `private Record record`,
`public void record() { }` and `enum E { record, B }` all parse and
a nested `record R(int x) { }` is a `record_declaration`.

**Action**: give `sealed` and `trait` the same treatment — a scanner
token emitted only when the word opens a declaration (`sealed class`,
`sealed interface`, `trait Name {`), so the literal disappears from
the lexer and the word is an identifier elsewhere. The alternative,
accepting the words as names through an aliased keyword
(`alias('sealed', $.identifier)`), is in place for enum constants,
where it needs no declared conflict. In the declarator position it
was tried for `record` and rejected: after
`private boolean` it is a genuine LR(1) conflict that needs a
declared conflict, and it still leaves `public void record() { }` an
error because the method name position wants `_property_name`.

---

## 13. A member directly after an enum's trailing comma

**Apache `GroovyParser.g4`**, on the 3.0 and 4.0 branches, has
`enumConstants (nls COMMA)? sep?` and then the member list, so any
member may follow the trailing comma with no `;`. The master branch
(Groovy 5) requires the `;`.

**Current implementation** (#88, #89): after the comma, the first
member is accepted only when it starts with something that cannot be
an enum constant: a modifier, `def`, a declaration keyword (`class`,
`interface`, `enum`, `trait`, `record`, `@interface`), or an
annotation followed by one of those. `enum_constant` has a static
precedence that makes a bare name after the comma a constant; its
comment in `grammar.js` says why that is not a GLR conflict.

Results for the member after `A,`, with Groovy 4.0.32 as the
reference:

| After `A,` | Groovy 4.0.32 | This grammar |
|---|---|---|
| `private int x`, `def x = 1`, `static { }`, `class C { }` | member | member |
| `private E(int w) { }` | constructor | `constructor_declaration` |
| `static f() { }` | method | `method_declaration` |
| `List<String> xs`, `String[] xs`, `a.b.C x` | field | field: the type cannot be a constant |
| `String x`, `int x = 1` | field | `ERROR` |
| `E(int w) { }`, `@Inject E(int w) { }` | constructor | `ERROR`: reads as a constant with an anonymous class body |
| `{ init() }` | initializer | `ERROR` |
| `void f() { }` | method | a constant `void` and a constructor `f`, with no `ERROR` |

The last row is the mirror case that the `enum_constant` comment
describes for the start of the body: a newline is whitespace, and
nothing else tells a constant followed by a constructor from a typed
method.

**Action**: accept every member and an initializer after the comma,
and prefer the constant only when it parses. That needs a GLR
conflict limited to this position. Tracked in #TBD-enum-comma.
