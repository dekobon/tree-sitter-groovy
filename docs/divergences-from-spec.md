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
| §4 Statements / decls | **Mostly done** | class / trait / interface / enum / record / sealed / @interface / method / the `default` modifier on methods and type declarations (see #20 below) / constructor / a record's compact constructor (see #17 below) / field / static_initializer / instance initializer / nested class, interface, trait, enum, record and @interface / multi-assignment with and without `def` / pipeline / try-with-resources / multi-catch / arrow switch / yield / labeled / `synchronized` statement. Multi-assignment without `def` at the start of a line continues an expression that ends the line before, as a call, and `def f(a, b) = …` is accepted (see #3 below). Not yet supported, each an `ERROR`: typed names (`def (int a, String b) = …`), `final` (`final (a, b) = …`, `final def (a, b) = …`) and `var (a, b) = …`. |
| §5.1 Numeric literals | **Done** | bin / oct / hex / dec, underscores, all type suffixes. |
| §5.2 Contextual keywords | **Mostly done** | `var`, `non-sealed`, `permits`, `yield` parsed contextually as grammar literals. `record`, `trait` and `sealed` are scanner tokens emitted only before the declaration each opens (§6.7, §6.13), so each is an identifier everywhere else (see #12 below). Groovy-5 candidates `val`/`async`/`await`/`defer` attempted but reverted: tree-sitter cannot fall back from a matched keyword literal to identifier when the keyword rule's RHS shape fails, so `async()`, `val.x()`, etc. error. See divergence #10. |
| §5.3 Label vs map vs ternary | **Done** | grammar-precedence resolution; pinned in `expressions-quoted-identifier.txt`. |
| §5.4 Slashy vs division | **Done** | scanner context guards + same-line lookahead. |
| §5.5 Command chains | **Mostly done** | multi-arg and named-argument forms supported, as a statement and as the value of a declaration or an assignment (#95). String keys (`foo 'a': 1`) are not; `case` / `default` are not accepted as keys as the first argument, a deliberate divergence from Apache; an argument after a comma continues onto a following line that can continue an expression (see spec §6.8). Not accepted where Groovy takes one: in parentheses (`(foo 1)`), as the only argument of a call (`bar(foo 1)`), in an interpolation (`"${foo 1}"`), with a name as the first argument, with a receiver that is not a name or names joined by member accesses written without spaces (`foo().bar 1`, `obj . m 1`); see #18 and #19 below. |
| §5.6 Optional parentheses | **Done** | command_chain as a statement and as the value of a declaration or an assignment. |
| §5.7 GString interpolation | **Done** | structured `gstring_dollar_interpolation` + `gstring_brace_interpolation` for `"…"`, `"""…"""`, `/…/`, `$/…/$`. |
| §5.8 Closure params / it | **Mostly done** | typed and untyped, with annotations, `final`, `def`, `var`, varargs and a default; no synthetic `it`. The shape is the one of a method parameter (`parameter()` in `grammar.js`); the orders it does not accept are in #14 below. |
| §5.9 Annotations | **Mostly done** | `@`, `@Foo(args)`, `@a.b.Foo`, stacked; on type, member, parameter, local variable, `import` and `package` declarations. Not supported, each an `ERROR`: an annotation on a `catch` parameter (`catch (@A Exception e)`), a for-in variable (`for (@A x in xs)`), a record component (`record R(@A int a)`) or a type parameter (`class C<@A T>`), and an annotated typed local with no initializer (`@A String x`; with `final` before the type it parses; see #6 below). An annotation must come before the modifiers (`private @Autowired Foo foo` is an `ERROR`, #92), and after annotations alone a local declaration takes only the name `_`. An annotation's arguments, the value of each `name = value` in them, and an `@interface` element's `default` (#101) take Apache's `elementValue`: an expression, an annotation, or a list of annotations, nested to any depth (`@Foo(x = [@Bar(@Baz)])`). A list that mixes annotations and expressions (`[@A, 1]`) or holds a list of annotations inside a list (`[[@A]]`) is an `ERROR`: Apache's parser takes both, and Groovy rejects them when it checks the annotation. Outside those three places an annotation is not a value, so `foo(@A)` and `x = [@A]` stay `ERROR`s. The arguments also take the forms of a call's arguments that Apache's one value or list of pairs does not (`@Foo(1, 2)`, `@Foo(x: 1)`, a trailing comma), and an annotation goes wherever an expression does among them (`@A(@B, @C)`, `@A(@B, a = 1)`, `@A(@B,)`), except as the value of `x:`, which stays an expression. One call form is not taken: in a call, `x = a: 1` is a named argument whose key is the assignment `x = a`, while in an annotation `name =` starts a pair, so `@Foo(x = a: 1)` is an `ERROR`, as it is in Groovy. Groovy needs `default` on the line of the `()`; here a line break before it is accepted. |
| §5.10 Statement terminators | **Equivalent** | newline-as-terminator via `repeat($._statement)` rather than a scanner token (see #3 below). |
| §5.11 Spread (three positions) | **Done** | `spread_arguments`, `spread_dot_expression`, `spread_map_entry`. |
| §5.12 `def` positions | **Mostly done** | typed-bare `Type x` (no init) deferred (see #6 below). |
| §5.13 Trailing commas | **Done** | accepted in list / map / argument / enum-constant lists. A parameter list takes none, as in Apache's `formalParameterList`. |
| §5.14 Method reference `::` | **Done** | identifier and `new` targets. |
| §5.15 Parenthesized type cast | **Done** | distinct `parenthesized_type_cast` node kind. |
| §6.2 Slashy scanner | **Done** | context guards + same-line lookahead. |
| §6.3 GString scanner | **Equivalent** | grammar-level; spec proposes scanner state, grammar achieves the same children. |
| §6.4 AUTOMATIC_SEMICOLON | **Equivalent** | grammar-boundary resolution covers every spec-named case (see #3 below; #11 for a `{` on the next line). |
| §6.5 LABEL_COLON | **Equivalent** | grammar precedence on `labeled_statement` covers spec-named cases. A scanner-token implementation was attempted earlier but coupled to the then-missing named-argument command chain (`archiveArtifacts artifacts: 'x'`). Named-argument command chains now use their own scanner token, `_command_named_key` (an identifier followed by `:` on the same line, excluding `case` / `default`), and labels keep grammar precedence. |
| §6.6 Block-comment scanner | **Done** | single-token emit, `/**/` and `/***/` parse correctly. A `/*` with no `*/` is not a comment; the first one emits an empty hidden `_unterminated_comment` extra that carries the scanner's one bit of state, so later ones do not read to the end of input again (#75). Error recovery near that first `/*` can differ from v0.6.0, for worse as well as better, and one reparse differs from a parse from scratch; see #16 below. |
| §6.7 RECORD_KEYWORD | **Done** | `record` emitted only when a name follows on the same line and `(` or `<` after it, comments and a line break before `(` allowed; `record.key()`, `private Record record`, `public void record()` are identifiers. Residuals listed in §6.7. |
| §6.8 COMMAND_NAMED_KEY | **Done** | an empty token, emitted only before a name followed by `:` on the same line, never during error recovery; the in-grammar lexer reads the key, so a non-ASCII key (`foo 名前: 1`) is an identifier by the grammar's own class. Limitations are in the §5.5 row above and spec §6.8. |
| §6.9 SUBSCRIPT_OPEN | **Done** | a `[` after a newline starts a list or map literal, not a subscript. A newline before `[` ends the expression everywhere, including inside `(…)`, `[…]`, `${…}` and conditions such as `if (a` + newline + `[0])`. Differs from Groovy inside `(…)`, `[…]` and `?[…]`, where Groovy ignores newlines: `foo(a` + newline + `[0])`, `[a` + newline + `[0]]`, and the array-type forms `(String` + newline + `[]) x` and `def m(String` + newline + `[] a) { }` are valid Groovy and an `ERROR` here (the scanner tracks no brackets; a faithful fix needs a paren/bracket/brace stack). A block comment at the start of the `[` line hides the newline, so `a` + newline + `/* c */ [0]` is a subscript here. |
| §6.10 STRING_BODY | **Done** | never emitted; valid only where a segment of a `"…"`, `"""…"""`, `/…/` or `$/…/$` body may start. The scanner emits nothing there, so `//` and `/*` in a string are text. A comment inside `${…}` is still a comment. |
| §6.11 CAST_CLOSE | **Mostly done** | a `)` with something after it on its line (spaces, tabs, line escapes and same-line block comments skipped; a line comment ends the line). A cast that closes with it outranks a parenthesized expression; a cast whose operand is on a later line ranks below one. `x = (a)` followed by `foo()` is two statements, as in Groovy, and `foo((String)` followed by `x)` is still a cast. The ranking is not the bracket-aware rule Groovy has; see #15 below. |
| §6.12 NOT_IN | **Done** | `!in`, emitted only before whitespace or `[`, `(`, `{`; the scanner reads that character and leaves it, so `a !in[1, 2]` parses. |
| §6.13 TRAIT_KEYWORD, SEALED_KEYWORD | **Done** | `trait` and `sealed`, emitted only before the declaration each opens: `trait` before a name on its line and then `{`, `<`, `extends`, `implements` or `permits`; `sealed` before `class`, `interface` or `trait`, or before another modifier, an annotation, `record` or `enum` on its line. Anywhere else the word is a name (see #12 below). |
| §6.14 COMMAND_VALUE_START, COMMAND_PATH_START | **Done** | an empty token before the receiver of a call that is a value, emitted only when a literal or a key follows the receiver on its line, so `def x = foo` followed by `1` on the next line stays two statements. The second token sits between the first name and the first member access of a statement's dotted receiver (#52), with the same condition, so `result.size` followed by `2` stays two statements; a value's dotted receiver is read by the first token. A comment between the receiver and the argument on one line is not read past (see #18 and #19 below). |
| §6.15 ARRAY_VALUE_START, TYPE_START | **Mostly done** | an empty token before an array type that is a value (`String[].class`, `int[]::new`, `@A(String[])`), emitted when a primitive or capitalized name and `[]` follow; where a type may also start, only when `.name`, `.&` or `::` follows the dimensions. `TYPE_START` is never emitted and marks where a type may start. Package-qualified, lowercase and generic element types, and a bare `String[]` where a type may start, are not taken (see #21 below). |
| §7 Conflicts | **Done** | none of the spec's predicted conflicts was needed; the real list is the `conflicts` array in `grammar.js` (see #1). |
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

**Actual `grammar.js` conflicts:** the `conflicts` array in
`grammar.js` is the only list. Each entry has a comment that says which
two readings it keeps alive and which token decides. This file does not
repeat it, because a copy here went out of date: it listed three
entries when the grammar had nine, and fourteen after #83.

The spec's list was an educated guess written before any rule was
implemented; once the grammar landed, none of the predicted
conflicts actually fired. Two reasons:

- Several entries (`command_chain`, `_type_arguments`) were avoided
  by how those features were built: a command chain is limited to
  statement position and to the value of a declaration or an
  assignment, where a scanner token starts it, and a generic type
  always carries explicit angle brackets.
- Other entries (`closure` vs `map_literal`, `trait_declaration` vs
  `class_declaration`, `label` vs `identifier`, `method_reference`
  vs `ternary`) were avoided by using `prec` and structural
  differences instead of explicit conflict declarations. For
  example, `closure` and `map_literal` use different brackets
  (`{}` vs `[]`) so they never compete at the same point;
  `labeled_statement` uses `prec(1)` to win at statement-start
  position because map-entries live inside `[…]` so cannot
  collide there.

The conflicts that exist come from positions the spec had no rule
for: a name in parentheses that may be a type or an expression, a
name after `def` that may be a variable or a method, the first tokens
of a closure that may be parameters or a statement.

**Action**: keep the spec text as historical record; the
authoritative list is `grammar.js`.

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
`new T(...)` it is not (see §11 below).

A `(` that opens the next line joins the previous expression as a
call's arguments. Apache's `arguments` has no `nls` before it, so
outside brackets Groovy ends the statement. The multi-assignment
without `def` (#94) is the form this hits, with no `ERROR`:

- Two on consecutive lines, `(a, b) = [1, 2]` then
  `(c, d) = [3, 4]`, are one statement here,
  `(a, b) = ([1, 2](c, d) = [3, 4])`. Before #94 that input had an
  `ERROR`.
- After a statement that ends in an expression, the `(` calls that
  expression and the call is assigned, as before #94: `foo()` then
  `(a, b) = [1, 2]` assigns `foo()(a, b)`, and the swap `def b = 2`
  then `(a, b) = [b, a]` assigns `2(a, b)`.

Groovy reads two statements in each case (observed on Groovy 3). A
`;` at the end of the previous statement avoids it.

The opposite also happens, one line read as two statements. After
`def f`, a `(` may open a parameter list or the next statement, and
the parser keeps both readings so that `def f` followed by
`(a, b) = list` on the next line is two statements. A line break is
whitespace here, so nothing requires one between those statements:
`def f(a, b) = [1, 2]`, on one line or with the `=` on the next, and
`@A def f(a, b) = …` parse with no `ERROR` as a declaration of `f`
followed by a multi-assignment. Groovy rejects them.

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
- `qualified_type` (`java.util.List`) — works in every type
  position, including `cast_expression` (`x as java.util.Map`) and
  `instanceof_expression` (`x instanceof java.lang.Number`). In
  those two slots the token sequence is the same as a
  `field_access` chain, and `PREC.RELATIONAL` is looser than
  `PREC.ACCESS`, so a plain `_type` there read
  `(field_access (cast x as java.util) Map)`. They take `_cast_type`
  instead, whose dotted form (`_dotted_type`, spec §3.2.2) shifts
  each dot at a precedence tighter than `field_access`.
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

In a record body, `static count` followed by `R { }` is a
modifier-only field and a compact constructor, and also the typed
field `static count R` followed by an instance initializer. The
compact constructor's weight is one more than the typed field's less
the modifier-only field's, so the first reading wins (§17);
`grammar.js` computes it from the other two.

The same fork splits a type nested in a record after a typed field
with modifiers. A keyword is an identifier wherever it has no action,
so `private String y` followed by `class C extends B implements I { }`
is also the fields `String`, `y class`, `C extends` and `B implements`
and a compact constructor `I`. A type header splits into at most two
fields after the one its keyword names, so a nested type in a record
carries the weight of a modifier-only field, two typed fields and a
compact constructor, plus one. `grammar.js` computes it from those.
Before the compact constructor the split died at `I {`. With it, the
split won whenever the header had `extends` or `implements`, and once
the constructor had its weight, with neither (§17).

The dynamic precedences of class members:

| Member | Weight |
|---|---|
| type declaration nested in a record body | 14 (a nested record 15) |
| `compact_constructor_declaration` (record body only) | 6 |
| typed `field_declaration` (`def` or a type) | 4 |
| constructor after an enum's trailing comma | 4 |
| `method_declaration` with a body | 3 |
| `record_declaration` | 1 |
| modifier-only `field_declaration` | −1 |
| `method_declaration` without a body | −1 |

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
  body a modifier-only field followed by a block is a compact
  constructor instead, on either line (§17).
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

## 12. `sealed` and `trait`: keyword or name

**Apache `GroovyParser.g4`** lets both words name things: its
`identifier` rule lists `SEALED` and `TRAIT` next to `Identifier`,
so `private boolean sealed` and `String trait` are ordinary fields.
**Spec §5.2** asks for the same through an
`identifier_or_contextual` fallback.

**Current implementation** (#73): as for `record`, the keywords come
from the external scanner, which emits each only before the
declaration it opens (spec §6.13): `trait` when a name follows on the
same line and then `{`, `<`, `extends`, `implements` or `permits`;
`sealed` when `class`, `interface` or `trait` follows, or, on the same
line, another modifier, an annotation, `record` or `enum`. Everywhere
else the word is an `identifier`.

Groovy takes the class modifiers in any order. The grammar accepts no
modifier after `sealed`, so `sealed abstract class S { }`,
`sealed @A class S { }` and `sealed static class C { }` in a class body
are an `ERROR` at `sealed`, as at v0.6.0, until modifiers are accepted
in any order. `sealed record` and `sealed enum`, which Groovy rejects
too, stay an `ERROR` as well.

v0.5.0 and v0.6.0 spelled both words as grammar literals. With
`word: $.identifier` the lexer returns a keyword wherever the parser
has an action for it, and the nested type declarations of #60 and #68
gave them one at every member start. After `private String`, which is
already a complete field named `String`, `trait` then opened a trait
declaration that took the following members as its name and body.

| Input | v0.6.0 | Now |
|---|---|---|
| `boolean sealed`, `String trait`, `def sealed = 1`, `void sealed() { }` in a class body | field, method | the same |
| `static count` then `void sealed() { }` (or `void trait() { }`) on the next line, in a class body | field `count`, method | a typed field `count void` and a constructor `sealed`, with no `ERROR`: the split of a modifier-only field and a typed method (#96), which the keyword hid at v0.6.0. Groovy reads a field and a method |
| `private boolean sealed`, `private String trait = "x"`, `public static final int sealed = 1` in a class body | `ERROR`; after `trait` the next members inside a `trait_declaration` | field |
| `private String trait() { }` | `ERROR` | method |
| `static trait = 1`, `private sealed`, `sealed() { }` | `ERROR` | field, field, constructor |
| `return trait`, `trait = x`, `String sealed = 'x'` in a method body | `ERROR` | statement, local variable |
| an enum constant `trait`, then `int f() { }` on the next line | `ERROR` | constant and method |
| `trait T { }`, `sealed class C permits A { }`, `static count` then `trait T { }` on the next line | declaration | the same |

A line break is whitespace to the grammar. The scanner sees the line
breaks after the word, in its look-ahead, but not the ones before it,
so where Groovy relies on those the readings differ:

| Input | Groovy | This grammar |
|---|---|---|
| `trait` then `T { }` on the next line, at statement level | name `trait`, then the call `T { }` (observed, Groovy 3) | the same; a `trait_declaration` at v0.6.0 |
| the same in a member list | rejected (observed, Groovy 3) | field `T` of type `trait` and an instance initializer; a `trait_declaration` at v0.6.0 |
| `private String trait` then `T { }` on the next line, in a record body | field `trait` and a compact constructor (read off the 4.0.x grammar) | the same; a field `String` and a `trait_declaration` at v0.6.0 |
| `private boolean sealed` then `class Inner { }` on the next line | field `sealed` and a class (read off the 4.0.x grammar) | field `boolean` and a sealed class, as at v0.6.0 |
| `trait T` then `{ }` on the next line, in a member list | field `T` of type `trait` and an initializer, which then fails to resolve (observed, Groovy 3) | `trait_declaration` |
| `sealed` then `public class C { }` on the next line | sealed class (read off the 4.0.x grammar) | name `sealed` and a class at statement level; field of type `sealed` named `public` and a class in a member list; `ERROR` at v0.6.0 |
| `private boolean sealed` then `static class D { }` on the next line | field `sealed` and a class (read off the 4.0.x grammar) | the same; `ERROR` at v0.6.0 |

A modifier after `sealed` is an `ERROR` while the modifier order is
fixed, so across a line break the scanner leaves the word a name and
the last row keeps its field. Once modifiers are accepted in any order,
looking past them to `class`, `interface` or `trait` would read the
`sealed` then `public class` row correctly as well.

`trait T(int x) { }` is a method `T` with return type `trait` and no
`ERROR`; Groovy 3 rejects it (observed), and v0.6.0 gave an `ERROR`.
It is the counterpart of the `record foo() { … }` residual of spec
§6.7.

**Action**: none planned. The `private boolean sealed` then
`class Inner { }` row needs what precedes the word on its line, which
the scanner cannot see.

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
conflict limited to this position. Tracked in #97.

---

## 14. Parameter modifiers in a fixed order

**Apache `GroovyParser.g4`** gives a method parameter and a closure
parameter one rule, `formalParameter`:
`variableModifiersOpt type? ELLIPSIS? variableDeclaratorId (ASSIGN expression)?`.
`variableModifier` is an annotation, `final`, `def` or `var`, in any
number and order.

**Current implementation**: `formal_parameter` and `closure_parameter`
share the helper `parameter()` in `grammar.js`: annotations, then
`final`, then `def`, `var` or a type, the name, and a default.

| Parameter | Groovy 4.0.32 | This grammar |
|---|---|---|
| `@A final String x = 1`, `def x`, `var x`, `final def x`, `String... xs` | accepted | accepted |
| `def String x` | accepted | `ERROR` |
| `final @A x` | accepted | `ERROR` |
| `... xs`, `def ... xs` | accepted | `ERROR` |

`def` and `var` are keywords in a parameter, so `def f(def x)` has no
`type`. `catch` parameters and `try` resources take `final` and `def`,
`var` or a type, with no annotations.

**Action**: accept the modifiers in any order and `...` with no type.
A closure body can open with the same words, so each added form has to
be checked against the forks listed in §5.8 of the spec. Tracked in
#98.

---

## 15. A cast whose operand is on a later line

**Apache `GroovyParser.g4`**: `castParExpression castOperandExpression`
has no `nls` between its parts. Where a line break is a token, a `)`
at the end of a line ends the expression. Inside `(…)` and `[…]` the
lexer drops line breaks, so a cast may continue on the next line
there.

**Current implementation**: a newline is `extras` whitespace and the
scanner tracks no brackets, so it cannot know which brackets enclose a
cast. It reports only whether something follows the `)` on its line
(`_cast_close`, spec §6.11), and `parenthesized_type_cast` is ranked
by that: above a parenthesized expression when the operand is on the
line, below every other complete reading when it is not.

| Input | Groovy 4.0.32 | This grammar |
|---|---|---|
| `x = (a)` then `foo()` | two statements | two statements |
| `x = (a) /* c */` then `foo()` | two statements | two statements |
| `x = (int) \` then `y` (a line escape) | cast | cast |
| `foo((String)` then `x)` | cast | cast: nothing else parses |
| `foo((a)` then `- b)`, or `+ b)` | cast of `-b` | subtraction `(a) - b` |
| `foo((a)` then `(b))` | cast of `(b)` | call `(a)(b)`, as at v0.5.0 |
| `x = (a)` then `(b) y` | `x = a`, then the cast `(b) y` | one assignment, `(a)` cast of `(b) y`, as at v0.5.0 |

At v0.5.0 the two readings were unranked and the stack order decided:
`x = (a)` followed by `foo()` was a cast of the call.

**Action**: a scanner that tracks bracket depth, which would also give
the grammar a statement terminator where a line break is significant
(#90, #96). Tracked in #99.

## 16. The marker of an unterminated block comment

**Spec §6.6**: a `/*` with no `*/` is not a comment, and the first one
emits an empty hidden extra token, `_unterminated_comment`, that
carries the scanner's one bit of state (#75). Valid input never sees
it. Two effects on other input are accepted.

**Error recovery**: the marker is one more node on the parse stack, so
recovery near the first unterminated `/*` can choose a different
repair than it did at v0.6.0, worse as well as better. Over 13_245
single `/*` insertions into the corpus inputs, 233 trees (1.8%)
changed: 86 with more bytes inside `ERROR`, 59 with fewer, 88 with the
same. No input that parsed without an `ERROR` is affected.

**Incremental reparse**: because the marker is empty, one reparse is
not the parse from scratch.

| Step | Text | Tree |
|---|---|---|
| Before the edit | `x = (a)/* n` then `c` | `x = (a)` with `ERROR`s after it |
| Insert `*/` before `c`, reparse with the old tree | `x = (a)/* n` then `*/c` | `x = (a)` as `parenthesized_expression`, `block_comment`, `c` as a second statement |
| The same text parsed from scratch | | one statement, `parenthesized_type_cast` over `c` |

At `/*` a cast and a parenthesized expression are both possible. The
parse path of the cast shifts the marker, fails on `/` and is dropped
before the other path reduces, so `(a)` is reduced with one path
alive, is not marked as the outcome of a conflict, and is reused. The
reparse has no error and the next edit to that node corrects it. In
1_000_000 random chained edits this happened 4 times; 3 reparses
differed at v0.6.0 from other causes. The same happens with
`foo((String)/* why` then `arg)` when `*/` is inserted before `arg`.
`bindings/rust/tests/incremental_reparse.rs` pins the first case and
fails if the two parses start to agree.

A change of included ranges with no edit is a third case, shared with
every other token: a range added later that contains `*/` does not
make the runtime lex the marker again.

**Action**: none planned. A marker with a width would need a token the
grammar accepts wherever `/` can appear.

---

## 17. A record's compact constructor

**Apache `GroovyParser.g4`** (4.0.x branch): `memberDeclaration` has
`modifiersOpt (classDeclaration | compactConstructorDeclaration)`, and
`compactConstructorDeclaration` is `methodName nls methodBody`, so a
line break may come before the `{`. The parser takes it in every class
body. `AstBuilder` then rejects it outside a record, and when its name
is not the record's. `methodName` may also be a string literal, and
`modifiersOpt` includes `def`. `AstBuilder` also rejects any instance
initializer in a record, and any instance field.

**Current implementation** (#72): `compact_constructor_declaration`
(annotations, modifiers, `def`, `name`, `body`) is a member of a record
body and of no other. `def` comes after the modifiers, as on a field or
a method. The body node is still `class_body`: the rule is a hidden
`_record_body` aliased to it. A type nested in a record has its own
body rule, so the member does not reach it, and a record nested in any
body brings it. The name is not compared with the record's, so any
`Name { … }` in a record body is a compact constructor.

After modifiers or `def` and a name, the same tokens are a field
followed by an instance initializer. A static precedence on
`compact_constructor_declaration` settles it at the `{`, with no GLR
fork. A line break is whitespace here, so a `{` on the next line reads
the same. Groovy's other reading is never valid in a record, as it
ends in an instance initializer.

After modifiers and two names, the fork between a field's name and a
type (§6) keeps two readings to the end of the block: a typed field
followed by an initializer, and a modifier-only field followed by a
compact constructor. The constructor's dynamic precedence is one more
than the typed field's less the modifier-only field's, so the second
wins. That is Groovy's reading when the line break follows the first
name (`static count`, then `Name { … }`). With no line break between
the names Groovy rejects either reading.

The fork also reaches a type nested after a typed field with
modifiers. Where a field name is expected the type keyword is an
identifier, so `private String y` followed by `class C { }` is also
the field `String`, a field `class` of type `y` and a compact
constructor `C`, and `extends` and `implements` in the header split
the same way. A type nested in a record carries a dynamic precedence
above any such split (§6), so it keeps the reading it has elsewhere.

The Groovy column is read off `GroovyParser.g4` and `AstBuilder.java`
on the 4.0.x branch, not run. Where two alternatives of a rule parse
the whole input, ANTLR takes the first, here `fieldDeclaration`.

| In a record body | Groovy 4 | This grammar |
|---|---|---|
| `public Name { … }`, `Name { … }`, `@A Name { … }`, `def Name { … }` | compact constructor | `compact_constructor_declaration` |
| `Name` then `{ … }` on the next line | compact constructor | `compact_constructor_declaration` |
| `public Name` or `def Name` then `{ … }` on the next line | field and initializer, rejected | `compact_constructor_declaration` |
| `static count` then `{ … }` on the next line | field and initializer, rejected | `compact_constructor_declaration` named `count` |
| `public Name;` then `{ … }` | field and initializer, rejected | `field_declaration` and `block` |
| `static count` then `Name { … }` on the next line | field `count`, compact constructor | field `count`, `compact_constructor_declaration` |
| `static String NAME` then `Name { … }` on the next line | field `NAME`, compact constructor | field `NAME`, `compact_constructor_declaration` |
| `static Foo bar { … }`, or `static Foo bar` then `{ … }` on the next line | rejected | field `Foo`, `compact_constructor_declaration` named `bar` |
| `String bar { … }` | rejected | `field_declaration` and `block` |
| `private String y` then `class C extends B { }`, or another nested type, on the next line | field, nested type | `field_declaration`, then the type's declaration |
| `static count`, then `Foo y`, then `class C extends B { }`, each on its own line | an instance field in a record, which Groovy rejects | fields `count Foo`, `y class` and `C extends`, then a compact constructor `B`, with no `ERROR`; v0.6.0 read two fields and the class. The nested type's weight covers one field before it, not a modifier-only field followed by a typed field without modifiers |
| `'Name' { … }` | compact constructor | `ERROR` |

In a class, interface, trait, enum or `@interface` body nothing
changes: `public Name { … }` is a field and an initializer, which
Groovy rejects, and `Name { … }` is an `ERROR`.

The words that Apache's `modifier` takes and `_modifier` does not
(`var`, `default`, `sealed`, `non-sealed`) do not start a compact
constructor, nor does `def` before a modifier: `def public Name { … }`
is a field named `public` and a compact constructor `Name`, as a
keyword is an identifier where it has no action.

**Action**: none planned. A string name can join the rule if real
code uses it.

---

## 18. A parenthesis-free call as a value

**Apache `GroovyParser.g4`** (4.0.x branch): `variableInitializer` is
`enhancedStatementExpression`, and the right side of
`#assignmentExprAlt` and of `#multipleAssignmentExprAlt` is a
`statementExpression`, which is `commandExpression`. So
`def out = sh script: 'make', returnStdout: true` declares `out` as the
result of a call, and so do `x = foo 1` and `(a, b) = f 3`.
`expressionInPar` takes one as well; `return`, `throw`, `assert` and
`yield` take an `expression`.

**Current implementation** (#95): `command_chain` is also the value of
a declarator (a local variable, a field, `def (a, b) = …`) and of an
assignment with any operator. A newline is whitespace, so the value
form opens with `_command_value_start`, an empty token that the scanner
emits only when a literal or a key follows the receiver on its line
(spec §6.14). Before #95 the forms below were a declaration of `foo`
followed by a statement, or an `ERROR` once a comma followed.

The Groovy column was observed by running Groovy 3.

| Input | Groovy | This grammar |
|---|---|---|
| `def out = sh script: 'make', returnStdout: true`, `def x = foo 1, 2`, `x += foo 'a'`, `(a, b) = f 3`, `static x = foo 1` in a class body | a call | `command_chain` value |
| `def x = foo` then `1`, `'a'` or `where: 1` on the next line, also with a comment opening that line | two statements | two statements, as before |
| `def x = foo 'a' + b`, `def x = foo 1 as int` | the expression is the argument | the same |
| `def x = foo 1, y = 2` | `y = 2` is the second argument | the same |
| `true ? x = foo 1, b : 3` | the chain takes `b`, the ternary the `: 3` | `ERROR`: `b : 3` reads as a named argument |
| `def x = c ? y = foo a : 2` | `c ? (y = foo(a)) : 2` | `ERROR` over the whole ternary: the scanner takes `a :` as a key, so the call takes the ternary's `: 2`; before, a local `ERROR` at `a` |
| `def x = foo /* c */ 1` | a call | `def x = foo`, the comment and `1`, as before |
| `def x = foo 1` then `(c)` on the next line | two statements | the argument is `1(c)` (§3); before, `def x = foo` then `1(c)` |
| `def x = foo bar` | `foo(bar)` | `def x = foo` then `bar`, as before and as in statement position |
| `def x = foo $/a/$`, `def x = foo .5` | `foo('a')`, `foo(0.5)` | as before: `def x = foo` then the string; an `ERROR` (spec §6.14) |
| `def x = foo default: 1` | a named argument `default` | `def x = foo` then the label `default:`, as for the statement form (spec §6.8) |
| `def r = obj.method 1` | a call | `command_chain` value whose receiver is a `field_access` (#52; see #19) |
| `def x = (foo 1)`, `if (foo 1) { }`, `bar(foo 1)`, `"${foo 1}"` | a call; `bar(foo 1, 2)` is `bar(foo(1, 2))`, and `bar(a, foo 1)` is rejected | `ERROR`, as before |
| `def x = foo 1 b 2` | `foo(1).b(2)` | the chains `foo 1` and `b 2`, as in statement position |
| `def c = foo { }` | a call with a closure | `method_invocation`, as before |
| `return foo 1`, `throw foo 1`, `assert foo 1`, `def s = 'a' + foo 1` | rejected | two statements, as before |
| `def f(a = foo 1) { }` at script or block level | rejected | no `ERROR`: after `def f` a `(` may open the next statement (§3), so this is the declaration `def f`, then a call on `(a = foo 1)` with a trailing closure; before, a `method_declaration` with a local `ERROR` in its parameters. In a class body it is an `ERROR`, as before |

**Action**: a chain in parentheses or as a call's only argument are
the forms real code is likely to need. A comment between the receiver and
the argument needs the look-ahead to read comments without scanning an
unterminated one twice.

---

## 19. A parenthesis-free call with a dotted receiver

**Apache `GroovyParser.g4`** (4.0.x branch): `commandExpression` is
`expression (argumentList)? commandArgument*`, and the expression may
be any path, so `obj.method 1`, `a?.b.c 'x'`, `foo().bar 1` and
`foo.bar[0] 1` are calls. `SemanticPredicates.isInvalidLocalVariableDeclaration`
reads `a.b c` as a call unless the last name of the path starts with
an upper-case letter or `c` is followed by `=`: `a.b.C x` and
`a.b c = 1` are declarations.

**Current implementation** (#52): the receiver of `command_chain` may be
a name followed by member accesses (`.`, `?.`, `??.`, `*.`, `.&`, `.@`)
written without spaces, built of the same nodes as in an expression. A
statement with such a receiver holds `_command_path_start`, an empty
token between the first name and the first access that the scanner
emits only when a literal or a key follows the receiver on its line
(spec §6.14); the value form's `_command_value_start` reads the same
receivers before them. The arguments are
those of the value form: the first is a whole expression that starts
with a literal, or a named argument. Before #52 each form below with a
call in the Groovy column was the expression followed by the argument
as a separate statement, or an `ERROR` once a comma followed.

The Groovy column was observed by running Groovy 3.

| Input | Groovy | This grammar |
|---|---|---|
| `obj.method 1`, `obj?.m 'a', 2`, `a.b.c x: 1`, `this.foo 1`, `System.out.println 'x'`, `def r = obj.m 1` | a call | `command_chain`, the receiver a `field_access` or the node of its operator |
| `log.info 'x: ' + y`, `obj.m 1 as int` | the expression is the argument | the same |
| `result.size` then `2`, `'a'` or `where: 1` on the next line, also with a comment or a blank line between | two statements | two statements, as before |
| `x.y \` then `1` on the next line (a line escape) | a call | a call |
| `x.y /* c */ 1` | a call | `x.y`, the comment and `1`, as before |
| `x.y` then `/* c */ a.b 'c'` on the next line | two statements, the second a call | the same |
| `obj . m 1`, `obj` then `.m 1` on the next line | a call | the expression and `1`, as before |
| `foo().bar 1`, `foo[0].bar 1`, `'s'.m 1`, `(obj).m 1`, `obj.'name' 1`, `obj::m 1` | a call | the expression and `1`, as before |
| `a.b c` | `a.b(c)` | `a.b` then `c`, as before (a name is not a first argument, spec §5.5 item 3) |
| `a.b.C x = 1`, `a.b c = 1` | a declaration | a declaration, as before |
| `obj.m { }`, `obj.m -1`, `obj.m [1]`, `obj.m (1)` | a call with a closure; a subtraction; a subscript; a call | the same, as before |
| `obj.m 'a' other 'b'` | `obj.m('a').other('b')` | the chains `obj.m 'a'` and `other 'b'` (spec §5.5 item 4) |
| `x.y a.b 'c'`, `foo a.b 'x'`, `foo 1 a.b 'c'`, `foo 'a' obj.m 'b'`, `x() a.b 'c'` | a chain continuation (`x.y(a.b).c`) | the first statement and the call `a.b 'c'`; before, the first statement, `a.b` and `'c'` |
| `return obj.m 'x'` | rejected | `return obj.m` and `'x'`, as before |
| `return` then `obj.m 'x'` on the next line | `return`, then a call | `return obj.m` and `'x'`, as before |
| `java.util.List 'foo'() { }`, `java.util.List '''foo''' () { }` | a method | a method, as before |
| `obj.m 'a'() { }`, `obj.m 'a'(b)`, `java.util.list 'a'() { }`, `java.util.List "$x"() { }` | a call with the argument `'a'(…)` | the same; before, a method declaration, or `obj.m` and `'a'(b)` |
| `java.util.List 'foo'(1)` | a call | `java.util.List` and `'foo'(1)`, as before |
| `x.y 1` then `-1` on the next line | two statements | one call with the argument `1 - 1`; before, `x.y` and `1 - 1` |
| `obj.m 1, *xs`, `obj.m 'a': 1` | a call | `ERROR` |

**Action**: none planned for the spaced and multi-line receivers. A
receiver that ends in a call or a subscript needs the scanner to read
balanced brackets.

---

## 20. The `default` modifier

**Apache `GroovyParser.g4`** (4.0.x branch): `DEFAULT` is a
`classOrInterfaceModifier` ("interface only"), so `modifier` takes it,
and with it `modifiersOpt` before a method, a constructor, a nested
type or a compact constructor, `modifiers` before a field or a local
variable, and `classOrInterfaceModifiersOpt` before a top-level type.
`modifiers` allows a line break between modifiers and after the last.
`default` is a keyword (`keywords`), so it is never a name. Groovy 3
gives the modifier a meaning only on an interface method, and ignores
it elsewhere.

**Current implementation**: a method may have `default` among its
modifiers, in a type body or as a statement. The rule,
`_default_method_declaration`, is aliased to `method_declaration` and
has its weights (§6). Other modifiers may come before and after
`default`, as they come in any order among themselves. After them the
return type is optional, and a member with none is a method, where
after other modifiers it is a constructor (`public m() { }`): the
modifier is meaningless on a constructor. The modifier is an anonymous
`default` child before the `name`; an `@interface` element's default
is the `default` field after the parameters, and no method has both.
A class, interface, trait, enum or record declaration takes `default`
among its modifiers too, at the top level or nested, through the
hidden `_type_modifiers`. An `@interface` declaration takes no
modifiers. Fields, constructors, compact constructors, initializers
and local variables do not take it, and each is an `ERROR` with it.

`default` is therefore a keyword wherever a member or a statement can
start, as in Groovy, where it is reserved, and after anything that can
end before one, such as `return`, `break` or an expression. Before,
it was a keyword only in a switch body and after the `()` of an
`@interface` element, and an identifier elsewhere: `default void b()
{ }` in a type body was a field of type `default` named `void` and a
constructor `b`. As a name in those positions it is now an `ERROR`.
Where only an expression or a name can follow it is still an
identifier (`x = default`, `foo(default)`, `def default = 1`).

At the start of a statement outside a switch body, `default:` is a
label by its own alternative of `labeled_statement`, an `identifier`
node as before, so `foo default: 1` is still a statement and a label
(spec §6.8). In a switch body, `default` after a case's statements
could also start a statement in them; `switch_case` and
`switch_default` are left-associative, so it ends the case and is the
next branch, as before. No statement in a Groovy switch body starts
with `default`.

A member or a statement that can end before `default` ends there, so a
line break before it separates the two, as in Groovy. Where a fork
between a field's name and a type comes before, the reading that takes
`default` as a name survives. `private String y` followed by
`default void b() { }` is also `private String`, a field `y default`
and the method `void b() { }`; the method weights decide it,
4 + 3 against −1 + 4 + 3. With a modifier-only field between them
(`private String y`, `static count`, then `default void b() { }`) the
split `private String`, `y static`, `count default`, `void b() { }`
weighs 10 against 6 and wins. That is the weakness of modifier-only
fields that §6 describes: `private String y`, `static count` and
`void b() { }` on three lines split the same way before this rule, as
a field `count void` and a constructor `b`.

In an `@interface` body a `default` after the `()` is the element's
default, by a static precedence on `_annotation_type_element`, so a
line break before it is accepted, as before (§5.9 row above).

The Groovy column was observed by running Groovy 3, which does not
parse records; the rows marked `.g4` are read off the grammar.

| Input | Groovy | This grammar |
|---|---|---|
| `default void b() { }` in an interface, trait, class, enum or anonymous class body, with annotations, other modifiers on either side, type parameters, `def` or no return type | a method | `method_declaration`; before, a field of type `default` named after the next word, then the rest |
| the same in a record body | a method (`.g4`) | `method_declaration` |
| `default void b() { }` at the start of a statement | a script method | `method_declaration`; before, the expression `default`, then a method |
| any member or statement, then `default void b() { }` on the next line | two | two |
| `default` alone on a line, then `void b() { }` | a method | a method |
| `default C() { }` in `class C` | a constructor | a method `C` |
| `default class C { }`, and `interface`, `trait`, `enum` and `record` declarations with `default`, at the top level or nested | the type | the type's declaration; before, the expression `default` and the declaration at the top level, and nested an `ERROR`, or for a record a field `record` of type `default` and a constructor |
| `default String value()` in an `@interface` | an element | `method_declaration` |
| `default String value() default 'x'` in an `@interface` | an element with a default | `ERROR`, as before |
| `String value()` then `default 'x'` on the next line, in an `@interface` | rejected | the element's default, as before |
| `String value()` then `default String b()` on the next line, in an `@interface` | two elements | `value` defaulting to `String`, then an `ERROR`, as before |
| `String a() default 'x'` then `default String b()` on the next line, in an `@interface` | two elements | two elements; before, an `ERROR` |
| `void a() default 1`, `Class<?> type() default Object` in an interface | an element default | `ERROR`; before, an `ERROR`, or a field `Object` of type `default` |
| `default String NAME = 'x'`, `default X = 1`, the compact constructor `default R { }` | a field, a compact constructor (`.g4`) | `ERROR`; before, an `ERROR`, or for `default X = 1` and `default R { }` a field of type `default` |
| `final default int X = 1`, `default final int X = 1` | a field `X` | `ERROR`; before, a field `default`, or one of type `default`, and the field `X` |
| `default static { }` | rejected | `ERROR`; before, a field `static` of type `default` and an initializer |
| `default int x = 1` in a block | a local variable | `ERROR`; before, the expression `default` and a declaration |
| `default @Override void b() { }` | a method | `ERROR`, as is any annotation after a modifier (#92) |
| `default = 1`, `default.x()`, `return default`, `break default`, `String default`, `final default = 1`, `void default() { }`, `{ default -> 1 }`, `println default`, `if (x) a() else default`, `static default = 1` and `private default` in a class, `enum E { default }` | rejected: `default` is reserved | `ERROR`; before, `default` was a name |
| `x = default`, `foo(default)`, `[default]`, `throw default`, `assert default`, `def default = 1`, `def default() { }`, `def f(default) { }`, `for (default in xs) { }`, `void default() { }` in a class | rejected | `default` is a name, as before |
| `foo.default`, `[default: 1]`, `foo(default: 1)`, `foo 'a', default: 1` | a property, a key | the same, as before |
| `foo default: 1`, `x = foo default: 1`, `a.b default: 1` | a call with the named argument `default` | the statement, then the label `default:`, as before (spec §6.8) |
| `switch (x) { case 1: foo()` then `default: bar() }` on the next line, `case 1: default: c()`, and the other switch forms | the switch | the same tree as before |
| `lbl: default void b() { }`, `if (x) default void b() { }` | rejected | a label or an `if` around a `method_declaration`; before, around the expression `default`, then a method |

**Action**: none planned. Fields, constructors and local variables
could take the modifier through rules of their own, as methods do;
Groovy 3 ignores it on all of them.

---

## 21. Array types as values

**Apache `GroovyParser.g4`** (4.0.x branch) has no rule for an array
type as a value. `String[]` in an expression is a `primary` (or a
dotted name) followed by a `pathElement` `indexPropertyArgs` whose
`expressionList` is empty, an index with no arguments, which Groovy's
compiler turns into a class when the name before it is one.

**Current implementation**: such a value is the `array_type` that a
declaration has, under the node that `String.class`, `Foo::new` or
`String.&new` has (spec §6.15). The scanner decides at the name, by
its spelling, so the grammar recognizes fewer forms than Groovy. The
Groovy column was observed with Groovy 3.

| Input | Groovy | This grammar |
|---|---|---|
| `def c = String[].class`, `foo(int[][].class)`, `x == byte[].class`, `case String[]:`, `return String[].class`, `[String[], int[]]`, `[k: int[]]`, `x in String[]`, `c ? String[] : int[]` | a class | `array_type` in the value; before, a subscript with a `MISSING` index |
| `def f = String[]::new`, `xs.toArray(int[]::new)`, `String[].&new` | a method reference or pointer on the class | `method_reference_expression` or `method_pointer_expression` with an `array_type` target; before, a `MISSING` index |
| `@A(String[])`, `@A(value = int[])`, `@A([String[], int[].class])`, `Class c() default String[]` | a class | `array_type`; before, a `MISSING` index |
| `def e = Map.Entry[].class` | a class | `array_type` with a `qualified_type`; before, a `MISSING` index |
| `String[ ].class`, `String[` then `].class` on the next line, `String[/* c */].class` | a class | `array_type`; before, a `MISSING` index |
| `String[].class`, `int[]::new` as a statement, in a closure body, in `(…)` or as an enum constant's argument | a class | `array_type`; before, an `ERROR` |
| `return String[]`, `(String[])`, `{ -> int[] }` | a class | an `ERROR` or a `MISSING` node, as before: a type may start there, and only `.name`, `.&` or `::` after the dimensions makes the name a value |
| `def i = java.lang.String[].class` | a class | a `MISSING` index, as before: the name starts with a package |
| `def a = foo[].class` | an index with no arguments on `foo`, or a class if `foo` is one | a `MISSING` index, as before: the name is lowercase |
| `def q = List<String>[].class` | a class | comparisons and a list, with no `ERROR`, as before |
| `String[]` then `xs = foo()` on the next line | a class, then an assignment | a declaration of `xs`, as before |
| `String[][0]` | an index on the class | an `ERROR`; before, a `MISSING` index |

**Action**: none planned. Taking a package-qualified name would mean
reading every dotted chain in an expression a second time, and a
lowercase class name is rare enough not to pay for. A generic element
type would turn trees without an error into other trees.
