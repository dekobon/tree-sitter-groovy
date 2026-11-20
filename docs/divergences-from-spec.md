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
| §4 Statements / decls | **Mostly done** | class / trait / interface / enum / record / sealed / @interface / method / the `default` modifier on methods and type declarations (see #20 below) / constructor / a record's compact constructor (see #17 below) / field / static_initializer / instance initializer / nested class, interface, trait, enum, record and @interface / multi-assignment with and without `def` / pipeline / try-with-resources / multi-catch / arrow switch / yield / labeled / `synchronized` statement. `def f(a, b) = …` is accepted (see #3 below). A multi-assignment declaration takes typed names, `final`, `final var` and `var` alone (`final (int a, b) = …`, `var (a, b) = …`), the Groovy 6 rest binding and keyed pairs (`def (a, *b) = …`, `def (name: n) = …`), and is accepted as a `for` init and in a type body (`private var (a, b) = …` included). Not yet supported, each an `ERROR`: a multi-assignment declaration introduced by annotations alone (`@A (a, b) = …`) or by modifiers other than `final` where a statement starts (`static def (a, b) = …`). Wider than Groovy: a keyword before a name is read as its type (`def (final a, b) = …`, `def (default a, b) = …`), a reserved word is a key (`def (class: v) = m`), more than one rest binding parses, and `foo final (a, b) = c`, like `foo def (a, b) = c`, is a statement and a declaration. |
| §5.1 Numeric literals | **Done** | bin / oct / hex / dec, underscores, all type suffixes. |
| §5.2 Contextual keywords | **Mostly done** | `non-sealed` and `permits` are grammar literals, each with an action only where no name can stand. `record`, `trait` and `sealed` are scanner tokens emitted only before the declaration each opens (§6.7, §6.13), and `var`, `yield`, `in` and `as` scanner tokens emitted only before what each opens or takes (§6.16), so each is an identifier everywhere else (see #12 and #22 below). Groovy-5 candidates `val`/`async`/`await`/`defer` attempted but reverted: tree-sitter cannot fall back from a matched keyword literal to identifier when the keyword rule's RHS shape fails, so `async()`, `val.x()`, etc. error. See divergence #10. |
| §5.3 Label vs map vs ternary | **Done** | grammar-precedence resolution; pinned in `expressions-quoted-identifier.txt`. |
| §5.4 Slashy vs division | **Done** | scanner context guards + same-line lookahead. |
| §5.5 Command chains | **Mostly done** | multi-arg and named-argument forms supported, as a statement, as the value of a declaration or an assignment (#95), in parentheses, also those of a condition, in an interpolation, and in parentheses as the first argument of another (`println(foo 1)`; see #24 below). String keys (`foo 'a': 1`) are not; `case` / `default` are not accepted as keys as the first argument, a deliberate divergence from Apache; an argument after a comma continues onto a following line that can continue an expression (see spec §6.8). An object or array creation (`println new Date()`) or an array type value (`foo String[].class`) is a first argument on the receiver's line, in every form; see #23 below. A name, `!` or `~` is a first argument too (`println x`, `obj.m arg`, `def r = foo bar`), told from a declaration (`String s`) by Apache's rule; see #25 below. Not accepted where Groovy takes one: as the argument of such an argument (`println(bar(foo 1))`), a name after a receiver that may name a type in a value (`def x = Foo bar`), the chain continuations (`foo a bar`), with a receiver that is not a name or names joined by member accesses written without spaces (`foo().bar 1`, `obj . m 1`); see #18, #19, #23 and #25 below. |
| §5.6 Optional parentheses | **Done** | command_chain as a statement, as the value of a declaration or an assignment, in parentheses and in an interpolation. |
| §5.7 GString interpolation | **Done** | structured `gstring_dollar_interpolation` + `gstring_brace_interpolation` for `"…"`, `"""…"""`, `/…/`, `$/…/$`. |
| §5.8 Closure params / it | **Mostly done** | typed and untyped, with annotations, `final`, `def`, `var`, varargs and a default; no synthetic `it`. The shape is the one of a method parameter (`parameter()` in `grammar.js`); the orders it does not accept are in #14 below. |
| §5.9 Annotations | **Mostly done** | `@`, `@Foo(args)`, `@a.b.Foo`, stacked; on type, member, parameter, local variable, `import` and `package` declarations. Not supported, each an `ERROR`: an annotation on a `catch` parameter (`catch (@A Exception e)`), a for-in variable (`for (@A x in xs)`), a record component (`record R(@A int a)`) or a type parameter (`class C<@A T>`), and an annotated typed local with no initializer (`@A String x`; with `final` before the type it parses; see #6 below). An annotation must come before the modifiers (`private @Autowired Foo foo` is an `ERROR`, #92), and after annotations alone a local declaration takes only the name `_`. An annotation's arguments, the value of each `name = value` in them, and an `@interface` element's `default` (#101) take Apache's `elementValue`: an expression, an annotation, or a list of annotations, nested to any depth (`@Foo(x = [@Bar(@Baz)])`). A list that mixes annotations and expressions (`[@A, 1]`) or holds a list of annotations inside a list (`[[@A]]`) is an `ERROR`: Apache's parser takes both, and Groovy rejects them when it checks the annotation. Outside those three places an annotation is not a value, so `foo(@A)` and `x = [@A]` stay `ERROR`s. The arguments also take the forms of a call's arguments that Apache's one value or list of pairs does not (`@Foo(1, 2)`, `@Foo(x: 1)`, a trailing comma), and an annotation goes wherever an expression does among them (`@A(@B, @C)`, `@A(@B, a = 1)`, `@A(@B,)`), except as the value of `x:`, which stays an expression. One call form is not taken: in a call, `x = a: 1` is a named argument whose key is the assignment `x = a`, while in an annotation `name =` starts a pair, so `@Foo(x = a: 1)` is an `ERROR`, as it is in Groovy. Groovy needs `default` on the line of the `()`; here a line break before it is accepted. |
| §5.10 Statement terminators | **Equivalent** | newline-as-terminator via `repeat($._statement)` rather than a scanner token (see #3 below). |
| §5.11 Spread (three positions) | **Done** | `spread_arguments`, `spread_dot_expression`, `spread_map_entry`. |
| §5.12 `def` positions | **Mostly done** | typed-bare `Type x` (no init) deferred (see #6 below). |
| §5.13 Trailing commas | **Done** | accepted in list / map / argument / enum-constant lists. A parameter list takes none, as in Apache's `formalParameterList`. |
| §5.14 Method reference `::` | **Done** | identifier and `new` targets. |
| §5.15 Parenthesized type cast | **Done** | distinct `parenthesized_type_cast` node kind. |
| §5.16 Lambda expressions | **Mostly done** | `lambda_expression` with `parameters` (`formal_parameters` or one `identifier`) and `body` (a block, an expression or a parenthesis-free call), each form where Apache takes it. Not accepted where Groovy takes one: a lambda as a call's parenthesis-free or appended argument (`foo (x) -> x`), a lambda without parentheses as a named argument of a parenthesis-free call (`foo a: x -> x`), and a lowercase parameter type (`(foo a) -> a`). A `{` that is the body of `if`, `else`, a loop, an arrow case or a lambda is a block before a closure, as in Groovy, also when it holds a lambda (`if (x) { a = b -> c }`). An assignment in a closure parameter's default or a case value, and `var` called in parentheses there, read as a lambda before `->`, and a `{` that opens a statement stays a closure; see #26 below. |
| §6.2 Slashy scanner | **Done** | context guards + same-line lookahead. |
| §6.3 GString scanner | **Equivalent** | grammar-level; spec proposes scanner state, grammar achieves the same children. |
| §6.4 AUTOMATIC_SEMICOLON | **Equivalent** | grammar-boundary resolution covers every spec-named case (see #3 below; #11 for a `{` on the next line). |
| §6.5 LABEL_COLON | **Equivalent** | grammar precedence on `labeled_statement` covers spec-named cases. A scanner-token implementation was attempted earlier but coupled to the then-missing named-argument command chain (`archiveArtifacts artifacts: 'x'`). Named-argument command chains now use their own scanner token, `_command_named_key` (an identifier followed by `:` on the same line, excluding `case` / `default`), and labels keep grammar precedence. |
| §6.6 Block-comment scanner | **Done** | single-token emit, `/**/` and `/***/` parse correctly. A `/*` with no `*/` is not a comment; the first one emits an empty hidden `_unterminated_comment` extra that carries the scanner's one bit of state, so later ones do not read to the end of input again (#75). Error recovery near that first `/*` can differ from v0.6.0, for worse as well as better; see #16 below. |
| §6.7 RECORD_KEYWORD | **Done** | `record` emitted only when a name follows on the same line and `(` or `<` after it, comments and a line break before `(` allowed; `record.key()`, `private Record record`, `public void record()` are identifiers. Residuals listed in §6.7. |
| §6.8 COMMAND_NAMED_KEY | **Done** | an empty token, emitted only before a name followed by `:` on the same line, never during error recovery; the in-grammar lexer reads the key, so a non-ASCII key (`foo 名前: 1`) is an identifier by the grammar's own class. Limitations are in the §5.5 row above and spec §6.8. |
| §6.9 SUBSCRIPT_OPEN | **Done** | a `[` after a newline starts a list or map literal, not a subscript. A newline before `[` ends the expression everywhere, including inside `(…)`, `[…]`, `${…}` and conditions such as `if (a` + newline + `[0])`. Differs from Groovy inside `(…)`, `[…]` and `?[…]`, where Groovy ignores newlines: `foo(a` + newline + `[0])`, `[a` + newline + `[0]]`, and the array-type forms `(String` + newline + `[]) x` and `def m(String` + newline + `[] a) { }` are valid Groovy and an `ERROR` here (the scanner tracks no brackets; a faithful fix needs a paren/bracket/brace stack). A block comment at the start of the `[` line hides the newline, so `a` + newline + `/* c */ [0]` is a subscript here. |
| §6.10 STRING_BODY | **Done** | never emitted; valid only where a segment of a `"…"`, `"""…"""`, `/…/` or `$/…/$` body may start. The scanner emits nothing there, so `//` and `/*` in a string are text. A comment inside `${…}` is still a comment. |
| §6.11 CAST_CLOSE | **Mostly done** | a `)` with something after it on its line (spaces, tabs, line escapes and same-line block comments skipped; a line comment ends the line). A cast that closes with it outranks a parenthesized expression; a cast whose operand is on a later line ranks below one. `x = (a)` followed by `foo()` is two statements, as in Groovy, and `foo((String)` followed by `x)` is still a cast. The ranking is not the bracket-aware rule Groovy has; see #15 below. |
| §6.12 NOT_IN | **Done** | `!in`, emitted only before whitespace or `[`, `(`, `{`; the scanner reads that character and leaves it, so `a !in[1, 2]` parses. |
| §6.13 TRAIT_KEYWORD, SEALED_KEYWORD | **Done** | `trait` and `sealed`, emitted only before the declaration each opens: `trait` before a name on its line and then `{`, `<`, `extends`, `implements` or `permits`; `sealed` before `class`, `interface` or `trait`, or before another modifier, an annotation, `record` or `enum` on its line. Anywhere else the word is a name (see #12 below). |
| §6.14 COMMAND_VALUE_START, COMMAND_PATH_START, COMMAND_STATEMENT_START | **Done** | an empty token before the receiver of a call that is a value, emitted only when a literal, a key, a `new`, an array type value or a name follows the receiver on its line (a name only after a receiver that may not name a type, see #25 below), so `def x = foo` followed by `1` on the next line stays two statements. The second token sits between the first name and the first member access of a statement's dotted receiver (#52), with the same condition, so `result.size` followed by `2` stays two statements; a value's dotted receiver is read by the first token. A third, `_command_statement_start`, sits before a statement's one-name receiver whose first argument is a `new`, an array type value or a name (`println x`), so `foo` followed by `new Bar()` or `bar` stays two statements; it is decided before the name, which costs a second read of every statement's first name (see #23 below). A first argument may also be a call in parentheses (`println(foo 1)`), which all three tokens read into, one parenthesis deep (see #24 below). A keyword Groovy reserves is no receiver of a value (`def x = return 1`); `super`, `permits`, `pipeline` and `_` are, and start a statement's call before a call in parentheses (`super(foo 1)`). A comment between the receiver and the argument on one line is not read past (see #18, #19 and #23 below). |
| §6.18 STATEMENT_PAREN | **Mostly done** | a `(` that opens a line after an expression, emitted where a statement may start and the parentheses can open one, so that it opens the next statement as in Groovy (`foo()` followed by `(a, b) = l`); a call's arguments do not take it. Inside parentheses and brackets the `(` still calls. Parentheses that cannot open a statement (`(a, b)`, `(a: 1)`, `()`) stay a call's arguments, and a block comment that opens the line hides the line break; see #3 below. |
| §6.17 LAMBDA_START, BARE_LAMBDA_START | **Done** | empty tokens before a lambda's `(` and before its one name without parentheses, emitted only when `->` follows; the `(` is read where an expression may start and none ends, within 256 items. See #26 below. |
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
AUTOMATIC_SEMICOLON is a v2 improvement. Groovy reads `a` and the unary
`+b` (observed on Groovy 3), and `-` behaves the same. A `++` or `--`
that opens a line makes the line above a postfix update here: `a`
then `++b` is `a++` and `b`, where Groovy reads `a` and `++b`. `**` and
`?[` at the start of a line continue the expression too, where Groovy
rejects the line. Of 24,548 Java files, 386 hold 5,593 `+` or `-` that
open a line after an expression outside brackets, mostly in string
concatenations, which splitting would turn into unary operators; the
266 real Groovy files hold none, and neither set a line-start `++` or
`--` after an expression.

The other is a `{` that opens the next line: it joins the previous
expression where the §6.4 contract as written would end the
statement. After a call that is Apache's reading too; after
`new T(...)` it is not (see §11 below).

A `(` that opens the next line after an expression opens the next
statement where one may start, as Apache's `arguments` has no `nls`
before it: `foo()` followed by `(a, b) = [1, 2]` is a call and a
multi-assignment, so are the swap `def b = 2` followed by
`(a, b) = [b, a]` and two multi-assignments on consecutive lines, and
`println x` followed by `(1..3).each { }` is a call and a second
statement (observed on Groovy 3). The scanner emits that `(` as
`_statement_paren` (spec §6.18), which a call's arguments do not take.
Where no statement may start, inside parentheses and brackets, the `(`
still continues the expression, as Groovy's lexer drops line breaks
there (`bar(foo` followed by `(1))`). These differences remain:

- Parentheses that cannot open a statement are a call's arguments, as
  before: empty ones, ones that hold a spread or a named argument with a
  key of any kind (`a: 1`, `'a': 1`, `"$a": 1`, `1: 2`, `(a): 1`), ones
  that a `,`, `->` or `else` follows, and ones with a `,` inside, outside
  brackets and type arguments, that hold neither a multi-assignment's
  names nor a parenthesis-free call (`foo` followed by `(a, b);`, as Java
  code is often written). Groovy rejects each such line, except before
  `->`, where it appends the lambda to the expression, which this
  grammar does not take (spec §5.16). Parentheses the look-ahead cannot
  read to their end are judged as if they closed where it stopped, so
  `(a, b` while it is typed is a call's arguments, as `(a, b)` is.
- Two lists with a `,` that Groovy reads as statements stay arguments,
  as this grammar takes neither as a statement: `(a in b, c)`, a call
  `a.in(b, c)` to Groovy, and `(Map<K, V> m, x)`, a call of
  `Map<K, V>` with `m` and `x`.
- A block comment that opens the `(` line hides the line break, as for
  `[` (spec §6.9): `foo` followed by `/* c */ (1)` is a call.
- Where no statement may start but Groovy's lexer does not drop the
  line break, the `(` continues the expression: in a class body and in
  a ternary's middle (`def x = foo` then `(1)`, `a ? foo` then
  `(1) : 2`), which Groovy rejects, and in an interpolation
  (`"${foo` then `(1)}"`), which Groovy reads as a closure of two
  statements.
- An enum constant's arguments, a method's parameters and the `(`
  after `if`, `while`, `catch` and the like may still follow a line
  break, where Groovy rejects one (a script method's parameters on the
  next line are two calls in Groovy). A creation's and an annotation's
  arguments may follow one in Groovy too.

The opposite also happens, one line read as two statements. After
`def f`, a `(` may open a parameter list or the next statement, and
the parser keeps both readings so that `def f` followed by
`(a, b) = list` on the next line is two statements. A line break is
whitespace here, so nothing requires one between those statements:
`def f(a, b) = [1, 2]`, on one line or with the `=` on the next, and
`@A def f(a, b) = …` parse with no `ERROR` as a declaration of `f`
followed by a multi-assignment. Groovy rejects them. The scanner ends the
second reading at the first `,` where the parentheses go on as a
method's parameters, before a `{` or `throws` (spec §6.20), so it costs a
method's head with a body no more parse steps than before
multi-assignments.

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
   `var`, `yield`, `in` and `as` take this route now (spec §6.16,
   #22 below); `await <expr>` could reuse the look-ahead that `yield`
   and `in` use.

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
| `x = (a)` then `(b) y` | `x = a`, then the cast `(b) y` | the same: the `(` opens the next statement (spec §6.18); before that, one assignment, `(a)` cast of `(b) y` |

At v0.5.0 the two readings were unranked and the stack order decided:
`x = (a)` followed by `foo()` was a cast of the call.

A multi-assignment declaration on the next line is a case where only
one reading used to be complete. After a line that ends in a
parenthesized name, dotted name or primitive type (`def y = (x)`,
`x = (b)`, `(b)`, `return (x)`, `assert (x)`, `(Foo.Bar)`, `(int)`),
with blank lines and comments allowed between, these were read with
no `ERROR` as the operand of the cast, a call of a method named
`final`, `def`, `private` and so on, assigned the value
(`(x) final(a, b) = m`):

- a declaration that starts with `final` (`final (a, b) = m`; in
  `final def (a, b) = m` the operand was `final` alone);
- a declaration with a rest binding or keyed pairs
  (`def (h, *t) = l`, `def (k: v) = m`);
- in a type body, a multi-assignment member after a field
  (`def (a, b) = m`, `private (a, b) = m`).

Now that the declaration parses, it is the second statement or member,
as in Groovy 3 for the forms it knows. `def (a, b) = l` at a statement
start was already two statements. After a parenthesized generic type
the cast stays the only reading here: `def y = (List<String>)` then
`def (a, *b) = l`, or `def (a, b) = l`, is
`(List<String>) def(a, *b) = l`, where Groovy 3 reads two statements.

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

**Incremental reparse**: because the marker is empty, one reparse was
not the parse from scratch, until the look-ahead for a lambda (spec
§6.17) made the runtime lex the `(` again.

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
differed at v0.6.0 from other causes. The same happened with
`foo((String)/* why` then `arg)` when `*/` is inserted before `arg`.
The scanner's look-ahead at the `(` of `(a)` and `(String)` now reads
through the `)` and the comment, and the runtime counts it against the
`(`, so the edit makes it lex the `(` again and both reparses are the
parse from scratch. It reads on through a type that names no parameter
as well (`(int)`, `(a.b.C)`, `(String[])`), so a primitive or dotted
type, which the look-ahead first turned down before the `)` and the
same edit left reused, is lexed again too.
`bindings/rust/tests/incremental_reparse.rs` holds the texts with a
name, a primitive and a dotted type among those it edits.

A change of included ranges with no edit is a third case, shared with
every other token: a range added later that contains `*/` does not
make the runtime lex the marker again.

**Action**: none planned for error recovery. A marker with a width
would need a token the grammar accepts wherever `/` can appear.

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
| `def x = foo 1` then `(c)` on the next line | two statements | two statements: the `(` opens the next (spec §6.18); before that, the argument was `1(c)` |
| `def x = foo bar`, `def x = foo $/a/$` | `foo(bar)`, `foo('a')` | `command_chain` value (#25 below); before, `def x = foo` then the argument |
| `def x = foo .5` | `foo(0.5)` | an `ERROR`, as before (spec §6.14) |
| `def x = foo default: 1` | a named argument `default` | `def x = foo` then the label `default:`, as for the statement form (spec §6.8) |
| `def r = obj.method 1` | a call | `command_chain` value whose receiver is a `field_access` (#52; see #19) |
| `def x = (foo 1)`, `if (foo 1) { }`, `"${foo 1}"`, `bar(foo 1)` | a call; `bar(foo 1, 2)` is `bar(foo(1, 2))`, and `bar(a, foo 1)` is rejected | a call, and `bar(a, foo 1)` an `ERROR` (#24 below) |
| `def x = foo 1 b 2` | `foo(1).b(2)` | the chains `foo 1` and `b 2`, as in statement position |
| `def c = foo { }` | a call with a closure | `method_invocation`, as before |
| `return foo 1`, `throw foo 1`, `assert foo 1`, `def s = 'a' + foo 1` | rejected | two statements, as before |
| `def f(a = foo 1) { }` at script or block level | rejected | no `ERROR`: after `def f` a `(` may open the next statement (§3), so this is the declaration `def f`, then a call on `(a = foo 1)` with a trailing closure; before, a `method_declaration` with a local `ERROR` in its parameters. In a class body it is an `ERROR`, as before |

**Action**: a comment between the receiver and
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
| `a.b c` | `a.b(c)` | `command_chain` (#25 below); before, `a.b` then `c` |
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
| `short[].class`, `def m() { short[].class }`, `{ short[]::new }`, `if (x) short[].class` | a class | `array_type`; before, an `ERROR` |
| `foo String[].class`, `foo int[]::new`, `obj.m String[].class`, `def x = foo String[].class` | a call with the class as its argument | a `command_chain` with the `array_type` value as its first argument (#23 below); before that, two statements, the name or the access and then the class, with no `ERROR` |
| `foo String[]`, `obj.m String[]` | a call with the class as its argument | a `command_chain` with the `array_type` as its first argument, the name `String` being a first argument (#25 below); before, an `ERROR` |
| `enum E { A(String[]) }`, `case 1 -> String[]` in a switch | an argument, an arrow body (the second read off the 4.x `.g4`; Groovy 3 has no switch expression) | an `ERROR`, as before: a constructor's parameter and a statement may start there |
| `String /* c */ [].class` | a class | a `MISSING` index, as before: only spaces, tabs and line escapes may come before the `[` |
| `_Foo[].class`, `$Foo[].class`, `Été[].class` | a class when the name is one (`_Foo` observed) | a `MISSING` index, as before: the name must start with `A` to `Z` |
| `String[][0]` | an index on the class | an `ERROR`; before, a `MISSING` index |

**Action**: none planned. Taking a package-qualified name would mean
reading every dotted chain in an expression a second time, and a
lowercase class name is rare enough not to pay for. A generic element
type would turn trees without an error into other trees.

---

## 22. `var`, `yield`, `in` and `as`: keyword or name

**Apache `GroovyParser.g4`** lists `AS`, `IN`, `VAR` and `YIELD` in
its `identifier` rule (4.0.x to 6.0.x), so the four words name
variables, fields, parameters, methods and labels. **Spec §5.2** asks
for that through an `identifier_or_contextual` fallback.

**Current implementation**: the keywords come from the external
scanner, as `record`, `trait` and `sealed` do (spec §6.16): `var`
before a declared name, or before the `(…) =` of a multi-assignment
when it stands alone; `yield` and `in` before an expression; `as`
before a type. Everywhere else the word is an `identifier`. Before,
each was a grammar literal and a keyword wherever it had an action,
which was after any statement that can end (`var`, `yield`) or after
any expression (`in`, `as`). The Groovy column was observed with
Groovy 3, in which `yield` is not a keyword; what it says of Groovy 4
and later is read off `GroovyParser.g4`.

| Input | Groovy | Before | Now |
|---|---|---|---|
| `int var = 1`, `String yield = 'a'`, `InputStream in = open()`, `int as = 1` | declarations | `ERROR` | declarations |
| `final var = (1)` in a method | a declaration of `var` | `ERROR`, which could take in the rest of the method | a declaration of `var` |
| `var = 2`, `var.foo()`, `var[0]`, `var++`, `var()`, `return var`, `yield.foo()`, `yield = 1`, `yield()`, `yield++` | names | `ERROR` | names |
| `foo()` then `in.read()` on the next line; `x = 1` then `in = 2` | two statements | `ERROR` | two statements |
| `{ var -> var }`, `{ yield -> yield }`, `var: foo()`, `yield: foo()` | names, labels | `ERROR` | names, labels |
| `void var() { }`, `int as(int x) { x }` where a statement starts | methods | `ERROR` | methods |
| `for (String in : xs)` | a loop over `in` | `ERROR` | a loop over `in` |
| `for (var x in xs)`, `for (var x : xs)` | loops | `ERROR` | loops |
| `var (a, b) = l`, and `private var (a, b) = l` in a class body | declarations | `ERROR` | declarations |
| `var(a, b)`, `var (a, b).foo()` in a method | calls of `var` | `ERROR` | calls |
| `import a.B` then `as = 1` on the next line | an import and an assignment | `ERROR` | the same as Groovy |
| `println var`, `println in` | calls | `ERROR` | `println var` a call, as `println x` (#25 below); `println in` two statements, as `in` goes on with the receiver as an operator |
| `var x = 1` in a class body | a field `x`, `var` a modifier | a field `x` of type `var` | the same as before |
| `var` then `x = 1` on the next line | a declaration of `x` | the same | the same |
| `return var` then `x = 1` on the next line | `return var`, then an assignment | `return`, then a declaration of `x` | the same as before |
| `yield x`, `yield (x)`, `yield -1`, `yield [1]` outside a switch expression | a call, a call, a subtraction, an index (Groovy 4 and later: the same, read off the `.g4`) | `yield_statement` | the same as before |
| `var in xs`, `var as T` | a declaration of `xs` of type `in` (`T` of type `as`) | a declaration of `in` (`as`), then `xs` (`T`) | the same as before |
| `var instanceof T`, `yield instanceof T`, `yield !instanceof T`; `x` then `in instanceof T` on the next line | a test of `var`, `yield`, `in` | a declaration of `instanceof`, a `yield_statement`, a membership test, each followed by `T` | a test of the name |
| `var instanceof = 1` | rejected | a declaration of `instanceof` | the same as before: no type follows `instanceof` |
| `yield++` then `foo()` on the next line; `x = 1`, then `in++`, then `foo()` | updates of the name, then a call | `yield ++foo()`; `x = 1 in ++foo()` | the update, then the call |
| `x in ++y`, `a in` then `++b` on the next line | a membership test of `++y`, `++b` | the same | the same |
| `yield ++y` | `(yield++)` called with `y` | `yield_statement` | the same as before |
| `for (String in in xs)`, `for (int as in xs)` | loops over `in`, `as` | `ERROR` | `ERROR`: the first `in` is the operator, as a name follows it |
| `yield(1, 2)` | a call | `ERROR` | `ERROR`: before `(` the word is the keyword |
| `for (var in xs)` | a loop over `var` | `ERROR` | `ERROR`: `var` before a name is the keyword |
| `int in(int x) { x }` where a statement starts | a method | `ERROR` | `ERROR`: before `(` the word is the operator, as in `a in (b)` |
| `final var(x) { }` in a script | a method `var` | `ERROR` | `ERROR`: after `final`, `var (` is the keyword |
| `x = (b)` then `in` on the next line | two statements | a cast of `in` | two statements |
| `def y = (x)` then `var (a, b) = l` on the next line | two statements | a cast of the call `var(a, b)`, assigned | two statements |

The last two rows, the `instanceof` row and the `++` row changed trees
that had no `ERROR`. Spec §6.16 describes the kinds: where the old
keyword ended one of two readings, the new tree is the one an ordinary
name gets; a multi-assignment opened by `var` alone reads as the one
opened by `def`; and `instanceof` with a type, or a line break after
`++` or `--`, makes the word the name it is in Groovy.

**Action**: none planned. `for (var in xs)` and `for (String in in xs)`
need the scanner to know it is in a `for` header, and taking `var` as
a name before `in` and `as` everywhere would change the trees of
`var in xs` and `var as T`, which parse without an `ERROR` and which
Groovy declares too. A
method named `in` with a return type would need the operator to look
past the parentheses to a body, where `a in (b) { … }` is a membership
test of a call with a closure in Groovy too.

---

## 23. A creation or an array type as a parenthesis-free call's first argument

**Apache `GroovyParser.g4`** (4.0.x to 6.0.x): `commandExpression` is
`expression argumentList? commandArgument*`, and the first element of
`argumentList` is any `expressionListElement` or a named argument. So
`println new Date()`, `foo new int[3]` and `foo String[].class` are
calls whose first argument is the whole expression after the receiver,
and `NEW nls creator` lets a line break follow `new`.

**Current implementation**: a first argument that starts with `new`
(an object or array creation) or is an array type value (spec §6.15)
is taken on the receiver's line in every form: a statement whose
receiver is one name, a statement with a dotted receiver, and a value
(spec §5.5 item 3). The statement form with one name opens with
`_command_statement_start`, an empty token that the scanner emits
before the name (spec §6.14); the other two hold the tokens they had.
The argument is the node it is anywhere else and the whole expression:
`foo new Foo().bar()` passes the result of `bar()`.

The Groovy column was observed by running Groovy 3.

| Input | Groovy | This grammar |
|---|---|---|
| `println new Date()`, `fail new RuntimeException('x')`, `foo new int[3]`, `foo new int[] {1, 2}`, `foo new Foo() { }`, `x.y new int[2]`, `log.info new Foo()`, `obj?.m new Foo()`, `def r = foo new Foo()`, `x = foo new Foo()`, `def (a, b) = f new Pair()` | a call | `command_chain`; before, the receiver, or a declaration whose value is the receiver, then the creation as a statement |
| `foo new Foo(1), 2`, `foo new Foo(), bar: 1`, `println new Date(), 'a'` | a call | `command_chain`; before, an `ERROR` |
| `foo new Foo().bar()`, `foo new Foo() + 1`, `foo new Foo() as Bar` | the expression is the argument | the same |
| `foo String[].class`, `foo int[]::new`, `obj.m String[].class`, `def x = foo String[].class` | a call | `command_chain` (#21 above) |
| `foo` then `new Foo()` on the next line, also with `/* c */` opening that line | two statements | two statements, as before |
| `foo \` then `new Foo()` on the next line (a line escape), `foo new` then `Foo()` on the next line, `foo new /* c */ Foo()` | a call | a call |
| `foo /* c */ new Foo()` | a call | `foo`, the comment and the creation, as before (spec §6.14) |
| `foo new Foo()` then `(c)` on the next line | a call, then `c` | the same: the `(` opens the next statement (spec §6.18); before that, the argument was `new Foo()(c)` |
| `foo new Foo()` then `{ }` on the next line | `foo(new Foo().call({ }))` | `{ }` is the creation's anonymous class body (§11); before, the body of the creation as a statement of its own |
| `if (x) foo new Foo() else bar new Bar()` | an `if` with an `else`, each branch a call | the same; before, the `if`, the creation, and `else` and `bar` read as names |
| `foo 1 bar new Baz()`, `foo bar new Baz()`, `foo new Foo() bar 1` | a chain: `foo(1).bar(new Baz())`, `foo.bar(new Baz())`, `foo(new Foo()).bar(1)` | two calls, or the statement `foo` and a call (spec §5.5 item 4); before, `bar` and the creation were statements of their own |
| `return new Foo()`, `throw new Foo()`, `assert new Foo()` | the statement | the same: a keyword of the grammar is not a receiver |
| `return foo new Foo()`, `return foo String[].class`, `break l new X()`, `continue l new X()` | rejected | `return foo` (`break l`, `continue l`), then the argument as a statement, as before and as `return foo 1` |
| `final String s new Foo()` | rejected | a declaration of `s`, then the creation, as before |
| `return` then `foo new Foo()` on the next line | `return null`, then a call | `return foo`, then the creation, as before (spec §5.5 item 2) |
| `this new Foo()`, `it new Foo()`, `String new Foo()`, `int new Foo()` | calls (`this.this(new Foo())`) | calls; `this` is a name in this grammar |
| `super new Foo()`, `permits new Foo()`, `record new Foo()`, `trait new Foo()`, `sealed new Foo()` | calls | the name, then the creation, as before |
| `var new Foo()` | a call | `ERROR`, as before: `var` before a name is the keyword (spec §6.16) |
| `yield new Foo()` | a call (Groovy 4 yields in a switch expression's arm) | `yield_statement`, as before (spec §6.16) |
| `foo new: 1` | a named argument `new` | the same, as before |
| `foo.new Inner()` | `new Inner(foo)` | `foo.new`, then the call `Inner()`, as before |
| `foo new @A Foo()` | a call | `ERROR`, as before: a creation takes no annotation here |
| `foo String[]` | a call | a call (#21 above, #25 below) |
| `foo java.lang.String[].class` | a call | `ERROR`, as before (#21 above) |
| `"${foo new Foo()}"` | a call | a call (#24 below) |
| `[a new B()]` | rejected | `ERROR`, as before (#18 above) |
| `x.y new` then `(c)` on the next line | rejected | a method `new` with no body, as before: `new` before `(` is no creation |
| `foo`, a no-break space (U+00A0) and `new Foo()` | rejected | `ERROR`, as before: the scanner reads the space as part of the name (spec §6.8), and the grammar's lexer does not skip it |

Each change of a tree with no `ERROR` is one of the first rows, the
`else` row or the chain row: a call that was its receiver followed by
its first argument as statements of their own. Spec §6.14 describes how
this was checked.

**Action**: none planned for the keyword receivers, which real code
does not name methods after. A comment between the receiver and `new`
needs the look-ahead to read comments, as in #18 and #19. A qualified
inner class creation needs a `new` after `.`, which the grammar does not
have in any expression.

---

## 24. A parenthesis-free call in parentheses or an interpolation, and as a first argument in parentheses

**Apache `GroovyParser.g4`** (4.0.x to 6.0.x): `expressionInPar` is
`LPAREN enhancedStatementExpression rparen`, and
`enhancedStatementExpression` is a `statementExpression`, which is
`commandExpression`. So `(foo 1)` is a call, and as a `parExpression`
it is a `primary`, which may stand wherever an expression may
(`return (foo 1)`, `[(foo 1)]`). The parentheses after `if`, `while`,
`do … while`, `switch` and `synchronized` are `expressionInPar` as
well. A `${…}` interpolation is a `closure` (`gstringValue`), and
`AstBuilder` takes the one expression statement in it as the value, so
`"${foo 1}"` is a call too; any other content is called as a closure.

**Current implementation**: the content of a `parenthesized_expression`
may be a `command_chain`, and so may the condition of `if`, `while` and
`do`, a `switch` value, a `synchronized` lock and the `value` of a
`gstring_brace_interpolation`. It is the value form
(spec §5.5 item 1), which opens with `_command_value_start`, emitted
only when the first argument starts on the receiver's line (spec
§6.14). The scanner is unchanged: the token is now also valid after
`(` and `${`.

Measured against the grammar before: STATE_COUNT 4298 to 4330,
LARGE_STATE_COUNT 1577 to 1575, `parser.c` 8,188,269 to 8,207,368
bytes, and no conflict, precedence or external token added. Parse steps
and steps with more than one stack version are unchanged on real files,
on Java files and on code made only of parenthesized expressions, casts,
conditions or interpolations; a condition takes one more reduction, for
the hidden rule below. Real Groovy and Gradle files take 0.3%
more instructions and Java files 0.6%, with no wall-clock change above
the noise.

In a parenthesized expression and an interpolation the choice between an
expression and a call is inline, for 22 and 8 states. Code made only of
interpolations takes 1.4% more instructions, as the scanner now reads
the name after each `${`. A condition takes the choice through the
hidden rule `_statement_expression`: inline, it gave each alternative
its own copy of the states for the rest of the `if` or loop, about 70
states. The rule costs a reduction per condition, and code made only of
conditions takes about 5% more instructions. A reduction in every
parenthesis as well cost about 4% more instructions on code made of
parenthesized expressions.

The Groovy column was observed by running Groovy 3.

| Input | Groovy | This grammar |
|---|---|---|
| `(foo 1)`, `def x = (foo 1, 2)`, `(sh script: 'x').trim()`, `((log.info 'x'))`, `(foo new Bar())` | a call | a `parenthesized_expression` holding a `command_chain`; before, an `ERROR` |
| `return (foo 1)`, `c ? (foo 'a') : 2`, `[(foo 1)]`, `!(foo 1)`, `(foo 1) + (bar 2)` | the call as an operand | the same |
| `if (fileExists 'pom.xml') { }`, `while (foo 1) { }`, `do { } while (foo 1)`, `switch (foo a: 1) { }`, `synchronized (lock 'a') { }` | a call | the `command_chain` is the `condition`, `value` or `lock`; before, an `ERROR` |
| `"${foo 1}"`, `"v: ${sh script: 'x', returnStdout: true}"`, `"""${foo 'a', 2}"""`, `/${log.info 'x'}/` | a call | the `command_chain` is the interpolation's `value`; before, an `ERROR` |
| `(foo) 1`, `(Foo) 'x'`, `(int) 1`, `(a) - 1`, `(a, b) = foo 1` | casts; a multi-assignment | the same, as before: the name is followed by `)` or `,` |
| `"${foo} 1"`, `"$foo 1"`, `"${foo}1"` | the name, then the text | the same, as before |
| `"${a; b}"`, `"${def x = 1}"`, `"${foo 1; 2}"`, `"${if (c) 1 else 2}"`, `"${return 1}"`, `"${throw new E()}"`; `"${-> x}"` | a closure, called; a closure | an `ERROR`, as before: an interpolation holds one expression, and statements in it are not parsed |
| `(int 1)`, `(String 'x')` | calls of the methods `int` and `String` | the same |
| `(foo` then `1)` on the next line | a call: inside parentheses Groovy's lexer drops line breaks | an `ERROR`, as before (spec §6.9) |
| `(foo /* c */ 1)` | a call | an `ERROR`, as before (spec §6.14) |
| `foo` then `(bar 1)` on the next line | `foo`, then the call | the same: the `(` opens the next statement (spec §6.18); before that, an `ERROR` |
| `(foo 1) 2` | `foo(1).call(2)` | the parenthesis, then `2`: a receiver is a name (spec §5.5 item 2); before, an `ERROR` |
| `(return 1)`, `if (throw new Foo()) { }` | rejected | an `ERROR`: a keyword Groovy reserves is no receiver of a value (spec §6.14) |
| `def x = return 1`, `x = class 'a'`, `v.w += if 1` | rejected | the declaration or assignment with the keyword as a name, then the argument as a statement; before, a call of a method named `return` (`class`, `if`). An argument that is no statement makes an `ERROR` (`x = throw 1, 2`) |
| `def x = super.foo 1`, `def x = static.foo 1`, `def x = permits 'x'`, `e = pipeline 'x'`, `(_ 1)`, `(super 'a')` | calls (`static` before `.` is a name to Groovy) | the same, as before |
| `def foo(bar 1)`, `def x (foo 1) { }` at script or block level | rejected | no `ERROR`: the declaration of `foo` (`x`), then the parenthesis as the next statement (§3), as `def foo(1)` before; before, a `method_declaration` with an `ERROR` in its parameters. In a class body it is an `ERROR`, as before |
| `def f(Foo.Bar true)`, `final def g(int null)`, with no body | rejected | the declaration of `f` (`g`), then the parenthesis as the next statement; before, a `method_declaration` whose parameter is named `true` (`null`). With a body it is that `method_declaration`, as before |

The `def x = return 1` row and the last row are changes of a tree with
no `ERROR`; the other is in the table below.

**A first argument in parentheses.** Apache's `commandExpression` is
`expression argumentList? commandArgument*`, and the first element of
`argumentList` may be an `expressionInPar` like any other primary. So
`println(foo 1)`, which is no `expression` as `foo 1` is no argument of
a call, is the command `println` whose first argument is `(foo 1)`, and
an expression may go on after the parenthesis: `println(foo 1).trim()`
is `println((foo 1).trim())`. It is a command, so it is accepted only
where one is.

**Current implementation**: the same. The first argument of a
`command_chain` may start with a `parenthesized_expression` that holds a
`command_chain`, and the scanner decides it with the token it emits
before the receiver (`_command_statement_start` or
`_command_value_start`) or after the receiver's first name
(`_command_path_start`): `(`, Groovy's trivia (line breaks and comments
included, as inside parentheses), then a receiver and a first argument
as `_command_value_start` reads them after the `(` (spec §6.14). The
look-ahead does not go into a second parenthesis, which keeps it
linear. No state, conflict, precedence or external token is added. On
real Groovy and Gradle files the scanner advances over 3.4% more
characters and the parse takes 0.3% more instructions; code made only of
calls with parentheses (`foo(a, b)`) takes 1.6% more. Parse steps and
steps with more than one stack version are unchanged.

| Input | Groovy | This grammar |
|---|---|---|
| `println(foo 1)`, `println (sh script: 'x', returnStdout: true)`, `bar(foo 1, 2)`, `println(foo 1), 2` | `println` called with `foo(1)`, `bar(foo(1, 2))`, `println(foo(1), 2)` | a `command_chain` whose first argument is the `parenthesized_expression`; before, an `ERROR` |
| `println(foo 1).trim()`, `bar(foo 1) + 2`, `bar(foo 1)[0]`, `bar(foo 1) { }` | the expression is the argument: `println(foo(1).trim())`, and `bar(foo(1, { }))` for the last | the same, the closure a trailing closure of the parenthesis |
| `x.add(foo 'a')`, `System.out.println(log.info 'x')`, `def y = bar(foo 1)`, `y = obj.m(foo a: 1)`, `"${bar(foo 1)}"`, `(bar(foo 1))` | a call | the same |
| `println(` then `foo 1)` on the next line, `println(/* c */ foo 1)` | a call | the same |
| `foo(bar a: 1)`, `foo(bar a: 1, 2)` | `foo(bar([a:1]))`, `foo(bar([a:1], 2))` | the same; `foo(a: 1)` stays a `method_invocation` with a named argument |
| `println(x)`, `foo(a, b)`, `foo(new Bar())`, `foo(String[].class)` | calls | `method_invocation`, as before |
| `println(bar(foo 1))` | a call | an `ERROR`, as before: the look-ahead stops at the second parenthesis |
| `assertEquals(3, foo 1)`, `bar(a, foo 1)`, `a + bar(foo 1)`, `[bar(foo 1)]` | rejected | an `ERROR`, as before |
| `return bar(foo 1)` | rejected | `return bar`, then `(foo 1)`, as `return foo 1` is `return foo` and `1`; before, an `ERROR` |
| `println` then `(foo 1)` on the next line | `println`, then the call | the same: the `(` opens the next statement (spec §6.18); before that, an `ERROR` |
| `foo(1).bar(baz 2)` | a call | an `ERROR`, as before: a receiver is made of names (spec §5.5 item 2) |
| `bar(foo 1) 2` | `bar(foo(1)).2` | the call, then `2` (spec §5.5 item 4); before, an `ERROR` |
| `foo bar(baz 1)`, `println foo(bar 1)`, `void foo(bar 1) { }`, `String foo(bar 1) { }` | the chain goes on: `foo.bar(baz(1))`, `println.foo(bar(1))`, `void.foo(bar(1, { }))` | the first name as a statement, then the call: `foo`, then `bar((baz 1))`; `void`, then `foo((bar 1) { })` (spec §5.5 item 4); before, an `ERROR` |
| `super(foo 1)`, `permits(foo 1)`, `pipeline(foo 1)`, `_(foo 1)` at the start of a statement, `super(foo 1)` in a constructor | calls | the same; before, an `ERROR`. `super new Foo()` stays `super`, then the creation (#23 above) |
| `foo bar(baz true)`, `println this(it null)`, with no body | `foo.bar(baz(true))`, or rejected | the statement `foo` (`println`), then the call; before, a method `bar` (`this`) with return type `foo` (`println`) and a parameter named `true` (`null`) |
| `x` then `foo(it null, a, b)` on the next line; `x` then `C(Foo.Bar true) {` and `}` on the lines after | `x`, then `foo(it(null, a, b))`; rejected | `x`, then the call; before, a method `foo` (`C`) with return type `x`, also with the body. With the return type on the header's line and a body it is that method, as before, also `void f(Foo.Bar true) { }`, which Groovy reads as `void.f(Foo.Bar(true, { }))` |

The last two rows are the other change of a tree with no `ERROR`. Of
120,000 inputs a fuzzer built around parentheses, conditions,
interpolations and calls with parentheses, and 120,000 it made by
changing corpus inputs and lines of real files, every tree with no
`ERROR` that changed is one of the changes named here, and only the
keyword rule made any an `ERROR`. A second fuzzer put
each keyword of grammar.js and some names before a first argument, in
every value position, in 80,000 inputs: every tree with no `ERROR` that
the rule for keywords changed (10,891, of which 4,989 now hold an
`ERROR`) has a keyword Groovy reserves as the receiver, and Groovy 3
rejects each of 600 of them taken at random. No tree changed for the
corpus, the highlight, tag and stress tests, or the real Groovy, Gradle
and Jenkins files, except by error recovery in two corpus inputs and 41
Java files parsed as Groovy.

---

## 25. A name as a parenthesis-free call's first argument

**Apache `GroovyParser.g4`** (4.0.x to 6.0.x, and Groovy 3):
`commandExpression` is `expression argumentList? commandArgument*`, and
the expression is greedy, so the first argument is whatever cannot
continue it: a name, `!`, `~`, a literal, `new`. A statement tries
`localVariableDeclaration` first unless
`SemanticPredicates.isInvalidLocalVariableDeclaration` turns it down,
which it does when the first name (for a dotted name, the last) is
neither a primitive type nor a modifier, starts with no upper-case
letter (`Character.isUpperCase`) and is followed by neither `<` nor `[`,
and no `=` follows the name after it, comments hidden. So `foo bar` is `foo(bar)`, while `Foo bar`, `int x`,
`foo.Bar x` and `foo bar = 1` declare. The predicate is the same in
Groovy 3, 4, 5 and 6; 6.0 adds a check for annotated loops only.

**Current implementation**: the scanner applies the same rule before
the receiver (spec §6.14), with no change to the grammar: after
`_command_statement_start`, `_command_path_start` and
`_command_value_start` the first argument is any expression. A
receiver that may name a type takes no name argument in a value either,
and a few forms where Groovy reads something else are turned down
(spec §5.5 items 3 and 4). The look-ahead reads each list after the
name (a closure's parameters, a parenthesis's elements) for at most 256
items, so malformed input that restarts a statement at every element
stays linear (spec §6.14). STATE_COUNT, LARGE_STATE_COUNT and
`parser.c` are unchanged; files whose trees do not change parse with
0.1% more instructions.

The Groovy column was observed by running Groovy 3.

| Input | Groovy | This grammar |
|---|---|---|
| `println x`, `foo Bar`, `foo this`, `obj.m arg`, `a.b c.d`, `x?.y z` | a call | `command_chain`; before, the receiver and the argument as two statements |
| `println x + 1`, `foo x.y`, `foo x()`, `foo xs[0]`, `foo x ? 1 : 2`, `foo bar as T`, `foo bar == baz` | the expression is the argument | the same |
| `foo !x`, `foo ~x`, `foo!x` | a call with the negation | the same |
| `foo != x`, `foo !in xs`, `foo !instanceof T` | the operator | the same, as before |
| `assertEquals expected, actual`, `foo x, y: 1` | a call | `command_chain`; before, an `ERROR` |
| `def r = foo bar`, `x = foo bar`, `def y = a.b c`, `(foo bar)`, `println(foo bar)`, `"${foo bar}"`, `if (foo bar) { }` | a call | `command_chain`; before, the value `foo` and the statement `bar`, or an `ERROR` |
| `foo -1`, `foo [1]`, `foo *x`, `foo (1)`, `foo in xs`, `foo as T`, `foo instanceof T`, `foo ? a : b`, `foo++` | the operator goes on with `foo` | the same, as before |
| `Foo bar`, `int x`, `void x`, `foo.Bar x` | a declaration with no value | two statements, as before (#6 above) |
| `Été x`, `Ωmega x`, `Жук x` | a declaration with no value | two statements, as before: a capital of Latin-1, Greek or Cyrillic counts as upper case; one of another script reads as lower case, and such a receiver takes a name argument |
| `foo bar = 1`, `foo.bar baz = 1`, `myType x /* c */ = compute()`, `foo bar /* c` then `*/ = 1` | a declaration | the same, as before: comments after the name are trivia |
| `foo a /* c */ b`, `println x // c` | `foo(a).b`; a call | the call `foo a`, the comment and `b`; the call and the comment |
| `foo bar =~ x` | `foo(bar =~ x)` | the same; before, the declaration of `bar` with the value `~x` |
| `foo bar` then `= 1` on the next line; `a.b.c x` then `y = 1` | `foo(bar = 1)`; `a.b.c(x)`, then an assignment | the same; before, a declaration across the line break |
| `def x = Foo bar`, `(Foo bar)` | `Foo(bar)` | the value `Foo` and the statement `bar`, and an `ERROR`, as before: the receiver may name a type |
| `in x`, `as x` where a statement starts | `in(x)`, `as(x)` | two statements, as before: `var`, `yield`, `in` and `as` take no name argument as receivers where a statement starts |
| `def x = in y`, `(in y)` | `in(y)` | `command_chain`; before, the value `in` and `y`, and an `ERROR` |
| `record x`, `trait x`, `sealed x` where a statement starts | `record(x)`, … | two statements, as before: the scanner decides each word's keyword there |
| `foo bar` then `baz` on the next line, `foo /* c */ bar` | a call, then `baz`; a call | the call and `baz`; two statements and the comment, as before (spec §6.14) |
| `if (x) foo bar else baz qux` | an `if` with an `else` | the same; before, the `if` and four statements |
| `then: thrown IllegalStateException`, `{ foo bar }` | a call | `command_chain` |
| `foo bar() { }`, `foo bar(a) { }`, `task copy(type: Copy) { }` | `foo(bar({ }))` | the same; before, a `method_declaration` with return type `foo`, or two statements for the last |
| `foo bar(int a) { }`, `foo bar(String a, b) { }`, `foo bar(final a) { }`, `foo bar(a, int b) { }`, `foo bar(String... a) { }`, `foo bar(int` then `a) { }`, `foo bar(Map<K, List<V>> a, int b) { }` | rejected | a `method_declaration`, as before |
| `foo bar(baz qux)` with no body | `foo.bar(baz(qux))` | `foo`, then the call `bar((baz qux))`; before, a `method_declaration` with no body |
| `bar sh(println` then `List)` on the next line | `bar.sh(println(List))` | a `method_declaration` with no body, as before |
| `{ foo a -> a }`, `{ foo a, b -> a }`, `{ foo a` then `-> a }` on the next line, `{ foo a, b = 1 -> a }`, `{ foo a, @A(x) b -> a }`, `{ foo a, b /* c */ -> a }`, `{ foo a, b // c` then `-> a }`, `{ foo a, final` then `b -> a }`, `{ foo a, Map<K, V> b -> a }` | closure parameters, the first of type `foo` | the same, as before |
| `{ foo a, b0, b1, …, b127 -> a }`, 128 or more names after a lowercase-typed first parameter | closure parameters | `ERROR`: the look-ahead for `->` reads 256 items (a comma and a name are two), then answers that no parameters follow; 127 names parse |
| `{ foo a, b = '->' }`, `{ foo a, b = { x -> x } }` | a call with two arguments | the same: strings are read whole, and an `->` inside brackets ends no list |
| `foo a bar`, `a b c d`, `foo bar baz qux` | `foo(a).bar`, `a(b).c(d)`, `foo(bar).baz(qux)` | the calls `foo a` and the statement `bar`, the calls `a b` and `c d` (spec §5.5 item 4); before, one statement per name |
| `foo bar 1`, `foo bar baz: 1`, `foo bar new Baz()`, `foo bar(baz 1)`, `foo x.y 'a'` | `foo(bar).1`, `foo.bar(baz: 1)`, `foo.bar(new Baz())`, `foo.bar(baz(1))`, `foo(x.y).a` | `foo` and the call after it, as before (spec §5.5 item 4) |
| `foo bar x = 1`, `foo bar baz(int a) { }`, `foo bar true` | `foo.bar(x = 1)`, `foo(bar).baz(…)`, `foo(bar).true` | `foo` and the declaration, the method or the call after it, as before (spec §5.5 item 4) |
| `foo var x`, `foo var x, y`, `foo yield x`, `foo record x`, `obj.m var x` | `foo(var).x`, `foo.var(x, y)`, … | the receiver, then `var x` (`var x, y`, `yield x`, …), as before: after a name argument that starts declarations, any name leaves the input as it was |
| `def x = foo bar baz`, `class C { def r = a b c }` | `foo(bar).baz`, a field `a(b).c` | `def x = foo` and the call `bar baz`; the field `r` and a field `c` of type `b`, as before: in a value a further name is a continuation |
| `foo xs[0] 1`, `foo x.y 1` | `foo(xs[0]).1`, `foo(x.y).1` | the call `foo xs[0]` and `1`; `foo` and the call `x.y 1`, as before: the look-ahead does not read past a subscript |
| `foo bar` then `(1)` or `-1` on the next line | a call, then the expression | with `(1)` the same, as the `(` opens the next statement (spec §6.18); with `-1` one call with the argument `bar - 1` (§3); before, `foo` and `bar(1)` or `bar - 1` |
| `foo bar` then `{ }` on the next line | `foo(bar({ }))` | the same; before, `foo` and the chain `bar { }` |
| `return foo bar`, `throw foo bar`, `assert foo bar` | rejected | the statement with `foo`, then `bar`, as before |
| `foo class`, `foo def`, `foo default` | rejected | the same as before |
| `foo var`, `foo yield`, `foo record`, `foo super`, `foo pipeline { }` | a call | `command_chain`; before, two statements, and `foo` and a `pipeline_statement` for the last |
| `obj.m $/x/$`, `def x = foo $/a/$` | a call | `command_chain`, as `$` starts a name to the scanner; before, the receiver and the string |
| `this.new Inner()`, `x.new Inner()` | a creation | the access `x.new` and the call `Inner()`, as before (#23 above) |
| `foo true + 1`, `foo 1 + 2` | `foo(true + 1)`, `foo(1 + 2)` | the call `foo true` (`foo 1`) and `+ 1` (`+ 2`), as before: a literal first argument of a statement is matched by the grammar's own form, which is no whole expression |

The `foo bar` then `-1` row is the general reading of a line that
starts with what continues an expression (#3 above), which a name
argument now takes as well. The rows with "before" changed trees that
had no `ERROR`; spec §6.14 lists the kinds and how each was checked.
Of 235,000 fuzzed inputs, 101 that had no `ERROR` have one, and Groovy 3
rejects all but 14: six where a `(` on the line after a name argument
continues it, four where a method header with no return type follows a
name argument on the next line, two with a generic type as the name
argument and two of keywords Groovy reads as chains (spec §6.14).

**Action**: the chain continuation (spec §5.5 item 4) would make the
chain forms one call; measured, it grows the parse table by about 30%
for the one real file of 266 that has the form. A name argument after a
capitalized receiver in a value would need to tell `(Foo bar)` from
`(Foo bar) -> …` and parameter lists.

---

## 26. Lambda expressions

**Apache `GroovyParser.g4`** (4.0.x to 6.0.x, and Groovy 3):
`lambdaExpression`, with its parameters in parentheses, is a `primary`;
`standardLambdaExpression` also takes one name with no parentheses and
stands only in `enhancedStatementExpression`,
`enhancedArgumentListElement` and `enhancedExpression` (the last from
5.0.x). A closure's optional parameter list is tried first, so where
an expression may be followed by `->`, the end of a closure
parameter's default and of a case value, Groovy keeps that reading
where only a closure can stand. Where a block can stand too, the body
of `if`, `else`, a loop, an arrow case and a lambda, `statement` and
`lambdaBody` try the block first.

**Current implementation**: spec §5.16 and §6.17. Before the
`lambda_expression` node every lambda was an `ERROR`. Measured over the
corpus inputs, 266 real Groovy and Gradle files and 24,548 Java files:
no tree that had no `ERROR` changed; 308 Java files went from an
`ERROR` to none, and the lines with `->` in an `ERROR` fell from 4,253
to 171, each a form Groovy rejects too (`return t -> t`,
`(Function<A, B>) x -> …`, `p -> s -> …`) or Java syntax Groovy
lacks (`(A & B)` casts, `case String s ->`). Real Groovy files hold no
lambda and are unchanged.

The Groovy column was observed by running Groovy 3, which has no arrow
cases; for those it is the reading of the 4.0.x to 6.0.x grammars.

| Input | Groovy | This grammar |
|---|---|---|
| `def f = (a, b) -> a + b`, `() -> 42`, `(int a, String b = 'x') -> { a }`, `(final x) -> x`, `(@A a, def b, var c, String... d) -> a` | a lambda | `lambda_expression`; before, an `ERROR` |
| `x -> x + 1` as a value, an argument (`list.map(x -> x * 2)`), in parentheses or as a map entry's or named argument's value (`foo(k: x -> x)`) | a lambda | the same |
| `return x -> x`, `[x -> x]`, `c ? x -> x : y`, `foo 1, x -> x`, `a -> b -> c`, `x -> x` as a statement | rejected | an `ERROR`, as before |
| `foo (x) -> x`, `list.each (x) -> println x`, `foo()` then `(a) -> a` on the next line | `foo((x) -> x)` | the call `foo(x)` and an `ERROR`, as before: the scanner does not read the arguments of a call (spec §6.17) |
| `foo a: x -> x` | rejected by Groovy 3, a call in 5.0.x and later | an `ERROR`, as before: in a parenthesis-free call's named argument only a lambda with parentheses is taken (`foo a: (x) -> x`). With it, the grammar grew by about 50 states |
| `(foo a) -> a` | a lambda with the parameter type `foo` | an `ERROR`, as before: the parentheses read as the call `foo a` (spec §6.14) |
| `foo(null -> 1)`, `foo(int -> 1)`, `(int) -> 1`, `(if) -> 1` | rejected | an `ERROR`, as before: a reserved word, a primitive type or `void` names no parameter |
| `(_) -> 1`, `_ -> 1`, `var -> var`, `(String this) -> 1` | a lambda | the same |
| `def c = { a = (b) -> c }`, `{ a = b -> c }`, `{ String a = b -> c }`, `{ a = 1 + (b) -> c }`, `{ String a = (T) (b, c) -> d }`, `{ a = (b) -> { } }`, as a value or an argument | the closure parameter `a` with a default | the same, as before |
| `if (x) { a = b -> c }`, `else { a = (b) -> c }`, `while (x) { String a = b -> c }`, `for (…) { def g = t -> 1 }`, `do { … } while (x)`, `case 1 -> { a = b -> c }`, `default -> { … }` | a block that holds the lambda | the same; **before, the closure with the parameter `a`** (the BREAKING class below) |
| `f(x -> { Consumer<String> c = s -> foo(s); c.accept(x) })`, `(x) -> { b = (t) -> 1 }` | a block that holds the lambda | the same |
| `if (x) { a = b -> { p -> p } }`, `while (x) { cb = v -> { -> v } }`, `if (x) { a = b -> { c = d -> e } }`, `f(x -> { a = b -> { c = d -> e } })` | blocks that hold the lambdas | the same, for nests of up to ten lambdas, each in the block of the one before (`LAMBDA_NESTING_DEPTH`); **before, the closure with the parameter `a`** where such a block held a lambda whose body is a closure or two lambdas, or a lambda's block two lambdas |
| `x -> { a = b -> { p -> p } }`, `if (x) { a = b -> { c = d -> { p -> p } } }`, a nest of more than ten lambdas | blocks that hold the lambdas | the closure with the parameter `a` (`c`), as before: a block in a lambda's body has no weight of its own, and one that outweighed the lambda in it would make `def c = { a = (b) -> { } }` a closure that assigns `a` a lambda |
| `{ T a = (U) (b, c) -> (d) -> e }`, `{ T a = (b, c) -> (d) -> e }` | the default `(U)` cast of the lambda `(b, c) -> (d)`, or that lambda, and the body `e` | the same: a parenthesized expression as a lambda's whole body weighs one step below a lambda (`PARENTHESIZED_BODY_WEIGHT`), below the reading of `{ a = (b) -> (d) -> e }` that Groovy has (a row below) and above a typed local that holds two lambdas; before, the default `(U)` called with `b` and `c` and the body `(d) -> e`, and no parameter and a body that declares `a` |
| `{ T a = (U) (b, c) -> (d) -> (e) -> f }` | the default `(U)` cast of `(b, c) -> (d)` and the body `(e) -> f` | the default `(U)` cast of `(b, c) -> (d) -> (e)` and the body `f`: the two readings weigh the same, and the parser keeps this one; before, the default `(U)` called with `b` and `c` and the body `(d) -> (e) -> f` |
| `{ a = (String b) -> (d) -> e }` | the default `String(b)`, a call, and the body `(d) -> e` | the default `(String b) -> (d)` and the body `e`: `String b` in parentheses is no call here (spec §6.14); before, no parameter and a body that assigns `a` the lambda |
| `{ a = b -> c }` that opens a statement, `lbl: { a = (b) -> c }`, `case 1: { a = b -> c }` | a block that holds the lambda | the closure parameter `a`, as before: such a `{` is a closure here (the next row) |
| `{ foo() }` that opens a statement, a labeled statement or a colon case's statement | a block | a closure, as before lambdas |
| `case (A) -> 1`, `case 1 + (a) -> 1` | a case with the value `(A)` or `1 + (a)` | the same, as before |
| `{ p = a = b -> c }`, `{ a, b = x += y -> z }`, `case x.y = it -> 1` | the parameter with the assignment as its default; the case with the assignment as its value | the closure body `p = (a = (b -> c))`; an `ERROR` for the second and third: the lambda without parentheses is taken after an assignment |
| `{ a = (var b) -> c }`, `case (var a) -> 1` | the default or value `var(b)` | the lambda `(var b) -> c` assigned to `a`; an `ERROR` for the case: the scanner reads `var` before a name as the keyword (spec §6.16) |
| `case (A) -> (b) -> b`, `{ a = (b) -> (d) -> e }` | the value `(A)` and the body `(b) -> b`; the default `(b)` and the body `(d) -> e` | the same; before, an `ERROR` |
| `def c = { v = (A<B> a) -> a }`, `{ v = (String[] a) -> a }`, `{ a = (int a) -> 1 }` | the parameter `v` (`a`) with the default `A<B>(a)`, `String[](a)` or `int(a)`, a call | `v = …` assigned the lambda; before, an `ERROR`: those parentheses are no expression here |
| `def v = a -> { y -> y }()`, `a -> { y -> y }[0]`, `a -> { y -> y } { z }`, `a -> { } + 1`, `(x) -> { y -> y }.call()`, `foo((x) -> { y -> y } + 1)`, `(x) -> { y -> y }` then `.call()` on the next line | the closure that starts the body called, subscripted, given a closure, added | the same (spec §6.19); before, an `ERROR`, two statements, a lambda and a list, a block and `+ 1` as a statement, and the lambda called or added |
| `def v = (x) -> { foo() }.call()`, `x -> { foo() }` then `+ 1`, `(1)` or `<T> T m(T t) { t }` on the next line, `def f = x -> { foo() }` in a class body then `{ z }` or a generic method on the next line | the lambda called; a lambda and a statement, a method or an instance initializer | the same, as before |
| `v = a -> { foo() } == 1`, `v = a -> { } + 1`, `v += a -> { foo() }` then `!= x` on the next line | the assignment compared or added: Groovy continues an assignment past a lambda without parentheses | the closure that starts the body compared or added, as in a declaration (`def v = a -> { foo() } == 1`); before, the same for the first, a block and the statement `+1` for the second, and the assignment compared for the third, as Groovy reads it |
| `def v = (a) -> { b = (c) -> { } }.call()`, `(x) -> { T t = (u) -> v }.call()` | the lambda's block, then the lambda called | the closure with the parameter `b` (`t`) called in the body; before, the block. The closure reading holds a lambda less; a weight on the lambda's block that outranked it also made `def c = { a = (b) -> { }.call() }` assign a lambda (spec §5.16) |
| `def v = a -> { y -> y }` then `{ z }` on the next line; `def v = a -> { foo() }` then `as T` or `in l` on the next line | the closure given `{ z }`; the block and a statement `as(T)` or `in(l)` | the lambda and a closure statement; the closure cast or tested, as before. Groovy's reading depends on whether the braces parse as a block, which the scanner does not know |
| `def v = (a) -> { y -> y }` then nine comments and `.call()` | the closure called | the lambda called: the scanner reads at most eight comments after the `}` (spec §6.19); before, the same, and with fewer comments too |
| `x = "${(a) -> { y -> y }` then `(1)}"` on the next line | the interpolation a closure that holds the lambda and the statement `(1)` | the closure that starts the body called: an interpolation holds one expression here; before, the lambda called |
| `(y.y) if (z) { a() } else { b() }` on one line | rejected | the parenthesized expression and the `if` statement; **before, the cast of a call named `if`** and a call named `else` (a BREAKING class below) |

Four kinds of trees that had no `ERROR` changed. No input from the
corpus, the 266 real files or the 24,548 Java files is among them:

- **A lambda's brace body that something continues** (the rows from
  `def v = a -> { y -> y }()` above): the scanner ends the body at the
  `}` only where no expression goes on (spec §6.19). Measured from the
  grammar with the weights of the next item: of 61,246 template inputs,
  2,928 without an `ERROR` changed; Groovy 3 accepts 2,070 and reads
  1,669 of them the new way and 90 the old way, the assignments and the
  blocks of lambdas in parentheses in the rows above. Of 60,319
  mutations of the corpus and lambda probes, 1,559 changed: 1,230 of the
  1,429 Groovy accepts the new way and 50 the old way. One mutation gained
  an `ERROR`, `def v = a -> { foo() }[]`, an empty subscript, which no
  expression takes here, and 151 mutations and 45 template inputs lost
  one.

- **A brace body that holds a lambda** (`if (x) { a = b -> c }` and
  the other bodies in the table). Before lambdas the block reading
  failed and the closure with the parameter `a` was the only one; now
  the block wins, as in Groovy, which agreed in 60 of 60 sampled
  inputs (Groovy 3). Of 27,957 inputs built with such bodies in every
  position of a `{`, 2,439 changed so. The block's weight beats the
  closure reading only for nests of up to ten lambdas: one more lambda
  in the block costs the block's reading `LAMBDA_WEIGHT` and the closure
  reading nothing, and a lambda's block has no weight to spend. When
  the nest bound was raised from what one or two lambdas need to ten,
  1,018 of 61,246 template inputs and 915 of 60,319 mutations of the
  corpus and lambda probes changed between trees without an `ERROR`.
  Groovy 3 read 655 of the 759 template inputs it accepts the new way
  (75 of them up to the statement count of a class body or an
  interpolation's closure), none the old way. Of the 838 mutations it
  accepts, 442 read the new way and 12 the old way: a body block
  followed on its line by what continues the closure in Groovy
  (`if (x) { a = b -> { p -> p } } - 1`), which now reads like every
  other such block, a block and a statement of its own.
  On nests of lambdas generated against Groovy 3, the mismatches fell
  from 932 of 4,000 to 254.
- **An assignment in a closure parameter's default or a case value,
  and `var` called in parentheses there** (the two rows above). Of
  120,000 inputs built from lambda-shaped templates, 116 changed so.
  Taking the lambda in a nested assignment the same way in every
  context is what keeps the grammar small: deciding it by context gave
  every expression state in a closure parameter's default and a case
  value a copy of its own, 8% more states.
- **A keyword call that is a cast's operand** (`(y.y) if (z) { … }` on
  one line): the weight of an `if`'s block makes the statement outrank
  the cast of a call named `if`, which Groovy, reserving `if`, never
  reads. 3 of 236,000 fuzzed inputs, none of them valid Groovy.

Of 116,000 mutations of the corpus, real and Java inputs, the third
kind accounts for the only 2 changed trees that had no `ERROR`.

**Action**: the call forms in the fourth row would need the scanner to
read every call's arguments; measured on code made of calls, the
reading the scanner does now costs about 1.5% more instructions.

A `{` that opens a statement, a labeled statement or a colon case's
statement would be a block, as in Groovy, if `block` were one of
`_statement`'s alternatives with `BODY_BLOCK_WEIGHT`, the body
positions then reading their `{` through it. A prototype needed one
precedence changed (`array_initializer` above `block` and `closure` in
a nested initializer) and kept the table size (4,510 states). It
changes trees that have no `ERROR`: 3 corpus inputs (`{ }`, `{ x }`
and a closure of several statements at the start of a statement), no
real Groovy file, and 139 of the 19,257 Java files without an
`ERROR`, whose nested blocks (`{ int x = 1; … }` in a method) are
closures now; on code made of conditions it takes about 3% more
instructions. That is a change of the AST of valid input, left to a
release that may break it.
