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
| §3 Operators (matrix) | **Done** | Every operator in §3.2 has its named node kind. The type after `as`, `instanceof` and `!instanceof` takes a whole dotted name with array dimensions and type arguments (`x as java.util.List<String>`); its `<` opens type arguments only where they close (CAST_TYPE_ARGUMENTS_OPEN, spec §3.2.2), and a quoted name or `new` after one of its dots is an `ERROR` (see #5 below). |
| §4 Statements / decls | **Mostly done** | class / trait / interface / enum / record / sealed / @interface / method / the `default` modifier on methods and type declarations (see #20 below) / constructor / a record's compact constructor (see #17 below) / field / static_initializer / instance initializer / nested class, interface, trait, enum, record and @interface / multi-assignment with and without `def` / pipeline / try-with-resources / multi-catch / arrow switch, also with several values in a case (`case 1, 2 ->`) / yield / labeled / `synchronized` statement. `def f(a, b) = …` is accepted (see #3 below). A multi-assignment declaration takes typed names, `final`, `final var` and `var` alone (`final (int a, b) = …`, `var (a, b) = …`), the Groovy 6 rest binding and keyed pairs (`def (a, *b) = …`, `def (name: n) = …`; one list takes names and pairs mixed, `def (name: n, b) = …`, which Groovy rejects, see #29 below), and is accepted as a `for` init and in a type body (`private var (a, b) = …` included). Not yet supported, each an `ERROR`: a multi-assignment declaration introduced by annotations alone (`@A (a, b) = …`) or by modifiers other than `final` where a statement starts (`static def (a, b) = …`). Wider than Groovy: a keyword before a name is read as its type (`def (final a, b) = …`, `def (default a, b) = …`), a reserved word is a key (`def (class: v) = m`), more than one rest binding parses, and `foo final (a, b) = c`, like `foo def (a, b) = c`, is a statement and a declaration. |
| §5.1 Numeric literals | **Done** | bin / oct / hex / dec, underscores, all type suffixes. |
| §5.2 Contextual keywords | **Mostly done** | `non-sealed` and `permits` are grammar literals, each with an action only where no name can stand. `record`, `trait` and `sealed` are scanner tokens emitted only before the declaration each opens (§6.7, §6.13), and `var`, `yield`, `in` and `as` scanner tokens emitted only before what each opens or takes (§6.16), so each is an identifier everywhere else (see #12 and #22 below). The Groovy 6 keywords `async`, `await`, `defer` and `val` have no construct and parse as names; grammar literals for them were attempted and reverted. See divergence #10. |
| §5.3 Label vs map vs ternary | **Done** | grammar-precedence resolution; pinned in `expressions-quoted-identifier.txt`. |
| §5.4 Slashy vs division | **Done** | scanner context guards + look-ahead to the closing `/`, on the same line where a division may follow. |
| §5.5 Command chains | **Mostly done** | multi-arg and named-argument forms supported, as a statement, as the value of a declaration or an assignment (#95), in parentheses, also those of a condition, in an interpolation, and in parentheses as the first argument of another (`println(foo 1)`; see #24 below). A key may be a string (`foo 'a': 1`, `def m = foo 'a': 1`; where a `:` may also follow a call that is a value, as in a ternary's consequence, the string stays an argument, see spec §6.8, but in a list element or a call's argument the call takes the key, `[x = foo 'a': 1]`, see spec §6.27), or a number, which takes the string key's rule (`foo 1: 'a'`, `foo a: 1, 2: 'b'`), but not, after a comma, an expression in parentheses, a list or a map (`foo x: 1, (k): 2`, `foo x: 1, [1]: 2`); `default` is not accepted as a key as the first argument, a deliberate divergence from Apache; a spread argument or a spread map is accepted after a comma (`foo 'a', *args`, `foo a: 1, *:m`), not first (`foo *a, 1`, `foo *:m`); an argument after a comma continues onto a following line that can continue an expression, a spread too (`foo 1, *a` then `+ 1` is `*(a + 1)`, where Groovy reads `foo(1, *a)` and the statement `+1`), and a `*` that ends a line or a `,` that opens the next one go on with the call without an `ERROR` (`foo 1, *` then `a`; `foo 1, *a` or `foo 1, a` then `, 2`), where Groovy rejects both (see spec §6.8). An object or array creation (`println new Date()`) or an array type value (`foo String[].class`) is a first argument on the receiver's line, in every form; see #23 below. A name, `!` or `~` is a first argument too (`println x`, `obj.m arg`, `def r = foo bar`), told from a declaration (`String s`) by Apache's rule; see #25 below. Not accepted where Groovy takes one: as the argument of such an argument (`println(bar(foo 1))`), a name after a receiver that may name a type in a value (`def x = Foo bar`), the chain continuations (`foo a bar`), with a receiver that is not a name or names joined by member accesses written without spaces (`foo().bar 1`, `obj . m 1`); see #18, #19, #23 and #25 below. A receiver may start with a call of a name with a closure that closes on its line (`foo { }.bar 1`, see #30 below). |
| §5.6 Optional parentheses | **Done** | command_chain as a statement, as the value of a declaration or an assignment, in parentheses and in an interpolation. |
| §5.7 GString interpolation | **Done** | structured `gstring_dollar_interpolation` + `gstring_brace_interpolation` for `"…"`, `"""…"""`, `/…/`, `$/…/$`. The lazy form `"${-> x}"` takes its `->` as a `closure_parameters` leaf; a closure with parameters (`"${w -> w << x}"`, which Groovy passes the writer) is an `ERROR` (spec §5.5 item 1). |
| §5.8 Closure params / it | **Mostly done** | typed and untyped, with annotations, `final`, `def`, `var`, varargs and a default; no synthetic `it`. The shape is the one of a method parameter (`parameter()` in `grammar.js`); the orders it does not accept are in #14 below. |
| §5.9 Annotations | **Mostly done** | `@`, `@Foo(args)`, `@a.b.Foo`, stacked; on type, member, parameter, local variable, `import` and `package` declarations, and on `for`, `while` and `do` loops, as in Groovy 6 (Groovy 4 and 5 reject an annotated loop). A for-in variable takes annotations before and after `final` (`for (@A final @B x in xs)`), as children of the `for_in_statement` after the loop's own, and after `def` or `var` (`for (def @A x in xs)`, `for (var @A x in xs)`), as a local does (`def @A x = 1`, `var @A x = 1`; §5.12 below). Not supported, each an `ERROR`: an annotation on a `catch` parameter (`catch (@A Exception e)`), a record component (`record R(@A int a)`) or a type parameter (`class C<@A T>`). An annotated typed local with no initializer parses (`@Field String x`, `@Field int a, b`), as one without annotations does (see #27 below). Annotations and modifiers come in any order (`private @Autowired Foo foo`, `public @interface X { }`, #92); after a modifier, parentheses with two or more names, or names and then `=`, are a multi-assignment's names, not an annotation's arguments (`private @B(a, b) = l`; Groovy's `elementValues` takes no list of names), see spec §6.26, and after annotations alone a local takes the name `_`, with a value or without, and a local or a field takes another name with a value (`@Field FOO = [a: 1]`, `@Subject svc = new Svc()`), not one without (`@A x`; in a type body `@A _` is the type of a field with a missing name); a lowercase type after annotations takes no name either (`@A foo bar`, Groovy's declaration). An annotation's arguments, the value of each `name = value` in them, and an `@interface` element's `default` (#101) take Apache's `elementValue`: an expression, an annotation, or a list of annotations, nested to any depth (`@Foo(x = [@Bar(@Baz)])`). A list that mixes annotations and expressions (`[@A, 1]`) or holds a list of annotations inside a list (`[[@A]]`) is an `ERROR`: Apache's parser takes both, and Groovy rejects them when it checks the annotation. Outside those three places an annotation is not a value, so `foo(@A)` and `x = [@A]` stay `ERROR`s. The arguments also take the forms of a call's arguments that Apache's one value or list of pairs does not (`@Foo(1, 2)`, `@Foo(x: 1)`, a trailing comma), and an annotation goes wherever an expression does among them (`@A(@B, @C)`, `@A(@B, a = 1)`, `@A(@B,)`), except as the value of `x:`, which stays an expression. One call form is not taken: in a call, `x = a: 1` is a named argument whose key is the assignment `x = a`, while in an annotation `name =` starts a pair, so `@Foo(x = a: 1)` is an `ERROR`, as it is in Groovy. Groovy needs `default` on the line of the `()`; here a line break before it is accepted. |
| §5.10 Statement terminators | **Equivalent** | newline-as-terminator via `repeat($._statement)` rather than a scanner token (see #3 below). |
| §5.11 Spread (three positions) | **Done** | `spread_arguments`, also as a list literal's element (`[*a, *b]`) and a subscript's index (`y[1, *a]`), `spread_dot_expression`, `spread_map_entry`, also as an argument (`foo(*:m, x: 1)`). The first argument of a parenthesis-free call is neither (`foo *a, 1`, `foo *:m`; see §5.5 above). |
| §5.12 `def` positions | **Mostly done** | a typed local without an initializer (`String x`, `int a, b`) is a declaration by Apache's rule (see #27 below); `def` before a type declares a typed local or field (`def String s`, `private def Foo foo`), where the scanner reads the declaration after `def` (spec §6.21). Modifiers and annotations follow `def` and `var` in any order, as after any other modifier (`def final String x`, `def static next() { }`, `def @A x = 1`, `def f(def @A x)`, `for (def @A x in xs)`, `var @A x = 1`). Not taken: a lowercase type with no `<` or `[` (`def foo bar`), a return type after `def` (`def String m() { }`), and `def` with a type in a parameter (§14 below), each read as before. `var` and `final var` before a type declare as `def` does (`var String s = 'a'`). |
| §5.13 Trailing commas | **Done** | accepted in list / map / argument / enum-constant lists. A parameter list takes none, as in Apache's `formalParameterList`. |
| §5.14 Method reference `::` | **Done** | identifier and `new` targets. |
| §5.15 Parenthesized type cast | **Done** | distinct `parenthesized_type_cast` node kind. |
| §5.16 Lambda expressions | **Mostly done** | `lambda_expression` with `parameters` (`formal_parameters` or one `identifier`) and `body` (a block, an expression or a parenthesis-free call), each form where Apache takes it. Not accepted where Groovy takes one: a lambda as a call's parenthesis-free or appended argument (`foo (x) -> x`), a lambda without parentheses as a named argument of a parenthesis-free call (`foo a: x -> x`), and a lowercase parameter type (`(foo a) -> a`). A `{` that is the body of `if`, `else`, a loop, an arrow case or a lambda, or that opens a statement, is a block before a closure, as in Groovy, also when it holds a lambda (`if (x) { a = b -> c }`, `{ a = b -> c }`), and so is each block of a nest of lambdas at any depth, but for a `{` that opens a statement and holds a lambda whose body is a closure (`{ a = b -> { p -> p } }`), which is the closure with the parameter `a`. An assignment in a closure parameter's default or a case value, and `var` called in parentheses there, read as a lambda before `->`; see #26 and #28 below. |
| §6.2 Slashy scanner | **Done** | context guards + look-ahead to the closing `/`, which stops at 64 escaped slashes (spec §6.2). Where no division may follow, after `in` and on a line after the `}` of an `if` or loop body included, the body may start with whitespace and span lines (`x = /a` then `b/`, `/ in [ \dms]+/`, `x in / a/`), and a string not closed yet runs on to the next `/` in the file, as in Groovy's lexer; on the line of the `)` of a condition or of a method with no body and of the `}` of a block or a type's body the one-line rule holds (`if (x) / 2`, `def f() { a } / 2`), through SLASH_DIVIDES, which is never emitted; where one may, and during error recovery, a `/` starts a string only on a later line, with a body that closes on that line. A `/` after an operand on its line is division, after `class` and a primitive type too, as Groovy 6's lexer reads it (`x = A.class /b/` and `x = a as int /b/` are an `ERROR`), where Groovy 3 reads a string there (`A.class("b")`). |
| §6.3 GString scanner | **Equivalent** | grammar-level; spec proposes scanner state, grammar achieves the same children. |
| §6.4 AUTOMATIC_SEMICOLON | **Equivalent** | grammar-boundary resolution covers every spec-named case (see #3 below; #11 for a `{` on the next line). |
| §6.5 LABEL_COLON | **Equivalent** | grammar precedence on `labeled_statement` covers spec-named cases. A scanner-token implementation was attempted earlier but coupled to the then-missing named-argument command chain (`archiveArtifacts artifacts: 'x'`). Named-argument command chains now use their own scanner token, `_command_named_key` (an identifier followed by `:` on the same line, excluding `default`), and labels keep grammar precedence. |
| §6.6 Block-comment scanner | **Done** | single-token emit, `/**/` and `/***/` parse correctly. A `/*` with no `*/` is not a comment; the first one emits an empty hidden `_unterminated_comment` extra that carries the scanner's "no `*/` from here on" bit, one of its four bits of state, so later ones do not read to the end of input again (#75). Error recovery near that first `/*` can differ from v0.6.0, for worse as well as better; see #16 below. |
| §6.7 RECORD_KEYWORD | **Done** | `record` emitted only when a name follows on the same line and `(` or `<` after it, comments and a line break before `(` allowed; `record.key()`, `private Record record`, `public void record()` are identifiers. Residuals listed in §6.7. |
| §6.8 COMMAND_NAMED_KEY | **Done** | an empty token, emitted only before a name followed by `:` on the same line, block comments between them aside (`foo a /* c */ : 1`), never during error recovery; the in-grammar lexer reads the key, so a non-ASCII key (`foo 名前: 1`) is an identifier by the grammar's own class. Limitations are in the §5.5 row above and spec §6.8. |
| §6.9 SUBSCRIPT_OPEN | **Done** | a `[` after a newline starts a list or map literal, not a subscript. A newline before `[` ends the expression everywhere, including inside `(…)`, `[…]`, `${…}` and conditions such as `if (a` + newline + `[0])`. Differs from Groovy inside `(…)`, `[…]` and `?[…]`, where Groovy ignores newlines: `foo(a` + newline + `[0])`, `[a` + newline + `[0]]`, and the array-type forms `(String` + newline + `[]) x` and `def m(String` + newline + `[] a) { }` are valid Groovy and an `ERROR` here (the scanner tracks no brackets; a faithful fix needs a paren/bracket/brace stack). A block comment at the start of the `[` line hides the newline, so `a` + newline + `/* c */ [0]` is a subscript here. |
| §6.10 STRING_BODY | **Done** | never emitted; valid only where a segment of a `"…"`, `"""…"""`, `/…/` or `$/…/$` body may start. The scanner emits nothing there, so `//` and `/*` in a string are text. A comment inside `${…}` is still a comment. |
| §6.11 CAST_CLOSE, RELATIONAL_CLOSE | **Mostly done** | a `)` with something after it on its line (spaces, tabs, line escapes and same-line block comments skipped; a line comment ends the line). A cast that closes with it outranks a parenthesized expression; a cast whose operand is on a later line ranks below one. `x = (a)` followed by `foo()` is two statements, as in Groovy, and `foo((String)` followed by `x)` is still a cast. The ranking is not the bracket-aware rule Groovy has; see #15 below. A `)` that ends its line before a line that opens with `(` is RELATIONAL_CLOSE unless a `)` or `]` around both lines comes within 256 characters, so a run of parenthesized lines parses in linear time; past the bound a cast inside a call is an `ERROR` or another reading (#15). A `)` before `instanceof`, `!instanceof`, `!in`, or `as` or `in` and what it takes, block comments on the line included, is RELATIONAL_CLOSE, which closes no cast, so `(x) instanceof Foo` tests `(x)`; parentheses that can only hold a type before one of them are an `ERROR` (see #5 below). |
| §6.12 NOT_IN | **Done** | `!in`, emitted only before whitespace or `[`, `(`, `{`; the scanner reads that character and leaves it, so `a !in[1, 2]` parses. |
| §6.13 TRAIT_KEYWORD, SEALED_KEYWORD | **Done** | `trait` and `sealed`, emitted only before the declaration each opens: `trait` before a name on its line and then `{`, `<`, `extends`, `implements` or `permits`; `sealed` before `class`, `interface` or `trait`, or before another modifier, an annotation, `record` or `enum` on its line. Anywhere else the word is a name (see #12 below). |
| §6.14 COMMAND_VALUE_START, COMMAND_PATH_START, COMMAND_STATEMENT_START | **Done** | an empty token before the receiver of a call that is a value, emitted only when a literal, a key, a `new`, an array type value or a name follows the receiver on its line (a name only after a receiver that may not name a type, see #25 below), so `def x = foo` followed by `1` on the next line stays two statements. The second token sits between the first name and the first member access of a statement's dotted receiver (#52), with the same condition, so `result.size` followed by `2` stays two statements; a value's dotted receiver is read by the first token. A third, `_command_statement_start`, sits before a statement's one-name receiver whose first argument is a `new`, an array type value, a name (`println x`) or a literal that the argument goes on past (`println "a" + b`), so `foo` followed by `new Bar()` or `bar`, and `foo 1` followed by `-x`, stay two statements; it is decided before the name, which costs a second read of every statement's first name (see #23 below). A first argument may also be a call in parentheses (`println(foo 1)`), which all three tokens read into, one parenthesis deep (see #24 below). A keyword Groovy reserves is no receiver of a value (`def x = return 1`); `super`, `permits`, `pipeline` and `_` are, and start a statement's call before a call in parentheses (`super(foo 1)`). A comment between the receiver and the argument on one line is not read past (see #18, #19 and #23 below). |
| §6.15 ARRAY_VALUE_START, TYPE_START | **Mostly done** | an empty token before an array type that is a value (`String[].class`, `int[]::new`, `@A(String[])`), emitted when a primitive or capitalized name and `[]` follow; where a type may also start, only when `.name`, `.&` or `::` follows the dimensions. `TYPE_START` is never emitted and marks where a type may start. Package-qualified, lowercase and generic element types, and a bare `String[]` where a type may start, are not taken (see #21 below). |
| §6.16 VAR, YIELD, IN and AS keywords | **Mostly done** | `var` before a declared name or a multi-assignment's names, `yield` and `in` before an expression, on its line for `yield`, `as` before a type, and such a word as a named argument's key (CONTEXTUAL_KEY); everywhere else the word is a name. `for (var in xs)`, `for (String in in xs)` and, where a statement starts, a method named `in` with a return type and typed parameters (`int in(int x) { x }`) are `ERROR`s (see #22 below). |
| §6.17 LAMBDA_START, BARE_LAMBDA_START | **Done** | empty tokens before a lambda's `(` and before its one name without parentheses, emitted only when `->` follows; the `(` is read where an expression may start and none ends, within 256 items. See #26 below. |
| §6.18 STATEMENT_PAREN | **Mostly done** | a `(` that opens a line after an expression, emitted where a statement may start and the parentheses can open one, so that it opens the next statement as in Groovy (`foo()` followed by `(a, b) = l`); a call's arguments do not take it. Inside parentheses and brackets the `(` still calls. Parentheses that cannot open a statement (`(a, b)`, `(a: 1)`, `()`) stay a call's arguments, and a block comment that opens the line hides the line break; see #3 below. |
| §6.19 LAMBDA_BODY_END | **Mostly done** | an empty token after the `}` of a lambda's brace body that no expression continues, so that braces an expression goes on with are a closure that starts the body (`def v = a -> { y -> y }()`); `_lambda_body_reread`, which no rule takes, makes a reparse decide a reused body's `}` again. The scan reads at most eight comments after the `}`, and an assignment of a lambda that an operator follows reads otherwise than Groovy (see #26 below). |
| §6.20 MULTI_ASSIGNMENT_COMMA | **Done** | the first `,` of a multi-assignment without `def` (`(a, b) = l`), turned down where the parentheses go on as a method's parameters (`def f(a, b) { }`), reading at most 16 names, so that a script method's head parses its untyped parameters once. |
| §6.21 DECLARATION_START | **Mostly done** | an empty token before the type of a statement that is a typed declaration by Apache's `isInvalidLocalVariableDeclaration` (`String s`, `foo.Bar q`, `foo<T> x`, `int a, b`, `String s = 'a'`), emitted where a statement starts, after annotations and as a `for` init, when the declaration ends on the line, the next line does not go on with it, or an initializer follows. Decided from the same reading of the first name as `_command_statement_start`, so `foo bar` stays a call. See #27 below. |
| §6.22 STATEMENT_BLOCK_END | **Mostly done** | an empty token after the `}` of a block that opens a statement, in a file, a block, a closure, a colon case or after a label, decided by the scan of §6.19, so that braces an expression continues are a closure (`{ foo() }()`, `{ foo() }.call()`, `{ }` then `.call()` on the next line). Read otherwise than Groovy: a word or a literal after the `}` on its line, `as` or `in` on the next line, a brace on the line after a command whose last argument is a closure, a brace that holds a lambda whose body is a closure, and two lines of braces after a lambda where the readings differ by a step (`def c = { a = (b) -> (d) -> e` then `{ z }` twice); see #28 below. |
| §6.23 JUMP_STATEMENT_END | **Done** | an empty token after `return`, `break` or `continue` where a line break or the end of input follows on the keyword's line, so that the value or the label must start on that line, as in Groovy: `if (c) continue` then `process(f)` is a `continue` and the call. A line comment, and a block comment that only spaces and tabs follow before the line break, end the statement before them, as Groovy's lexer reads either as the line break; a block comment that the label or the value follows is skipped. `throw` and `assert` still take a value on the next line, which Groovy rejects. |
| §6.24 CLOSURE_CALL_START, CLOSURE_COMMAND_START | **Mostly done** | empty tokens before the first name of a statement whose `{` follows on its line, closes on it and is followed by a member access, past line breaks and comments, so that the call with the closure is the access's object (`foo { }.bar()`, Groovy's `foo({ }).bar()`), and the receiver of a parenthesis-free call where a first argument follows the accesses (`foo { }.bar 1`). The braces are read on Groovy's tokens within 128 characters of the `{`. Read as before: braces that span lines or pass the bound, contextual words and keywords as the name, and an operator other than an access after the `}` (`foo { }[0]`, `foo { } as X`); see #30 below. |
| §7 Conflicts | **Done** | none of the spec's predicted conflicts was needed; the real list is the `conflicts` array in `grammar.js` (see #1). |
| §8 Tie-break corpus | **Done** | all five spec-mandated tie-breaks pinned in `operators-precedence-tiebreaks.txt`. |
| §9.1 highlights.scm | **Done** | keywords (the contextual ones included), operators, literals, comments, annotations, labels, types and the names of declarations, calls, fields and parameters have captures; structural nodes (statements, blocks, argument lists, expressions as such) have none. |
| §9.2 injections.scm | **Done** | groovydoc → javadoc; each `string_fragment` of a slashy or dollar-slashy string → regex, its delimiters and interpolations not (§7); named-call DSL → sql. |
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
  statement position, the value of a declaration or an assignment,
  parentheses (those of a condition included) and an interpolation,
  where a scanner token starts it, and a generic type always carries
  explicit angle brackets.
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

The lazy form `"${-> x}"` (Apache's `gstringValue` takes a `closure`)
holds a `closure_parameters` node before its `value`, a leaf whose text
is `->`, where a closure's `closure_parameters` holds an anonymous `->`
child (`{ -> x }`). With no value, `"${-> }"`, which Groovy 3 takes as a
closure with no statements, the `value` is a `MISSING` `true`. A
closure with parameters there (`"${w -> w << x}"`) is an `ERROR`. And
`"$-> x"`, which Groovy 3's lexer rejects at the `-`, parses with no
`ERROR` as two string fragments, the `$` the literal one of the fallback
above, as at v0.6.0.

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
| `return\nfoo()`, `break\nlabel`, `continue\nlabel` | parse as a bare `return`, `break` or `continue` then the next statement, as in Groovy; the scanner ends the statement at the line break (spec §6.23) |
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

  That precedence acts at every dot, the first included, so the type
  takes the whole dotted name, whatever the case of its parts
  (`x as a.b.c`), as Apache's `qualifiedClassName` does. The array
  and generic forms of `_cast_type` are built over `_dotted_type`
  too (`_cast_array_type` and `_cast_generic_type` in `grammar.js`),
  so a dotted name with array dimensions or type arguments is one
  type (`o instanceof JTextComponent.KeyBinding[]` in the JDK's
  `BasicTextUI.java`, `x as java.util.List<String>`,
  `x as a.b.C<D>[]`). The array's element is a rule of its own, which
  forks with the whole cast type at the `[` until the `]`
  (`x as a.b[0]` is a subscript of the cast). In the parenthesized
  cast the type is `_type`, and `(a.b.C[]) x` and
  `(java.util.List<String>) x` were already one type.

  Before, the first dot either continued the type or ended it, so
  that `x as java` was followed by field accesses, and both readings
  parsed to the end of the name. `DOTTED_CAST_TYPE_WEIGHT` on
  `_dotted_type` ranked the type first; before the weight,
  tree-sitter's last resort decided, by the order of the rule
  definitions where the two trees met in a node and by the age of the
  parse stacks where they met as two links. Array dimensions after a
  dotted name were an `ERROR`, and type arguments after a name of
  three or more parts read comparisons: the `qualified_type` under
  `array_type` and `generic_type` lost to `_dotted_type` by
  precedence at the second dot and at the `[`. With type arguments,
  the field accesses went on as comparisons,
  `(x as java).util.List < T > …`, whose `>` took the next line as
  its operand and forked again there, so a run of N lines
  `l = x as java.util.List<T>` took time quadratic in N: 0.40, 1.16,
  3.76 and 13.25 billion instructions for 1,000, 2,000, 4,000 and
  8,000 lines, and 0.049, 0.099, 0.197 and 0.394 billion now; with
  `util.List<T>` 0.43 to 16.3 billion before dotted names took type
  arguments, 0.044 to 0.349 billion now; `l = x as List<T>` takes
  0.036 to 0.289 billion. The reading kept a stack version per line,
  1,001 at once for 1,000 lines and 8,001 for 8,000, and one now;
  `bindings/rust/tests/dotted_cast_type_cost.rs` counts them in the
  parser's log for such runs.

  The weight stays: it ranks the cast against other readings of the
  same lines, as below. It is half a step: every other weight is a
  number of steps of `WEIGHT_UNIT` (`weighted` in `grammar.js`), so it
  decides only between readings that the steps leave tied, as long as
  they differ in fewer than `WEIGHT_UNIT` dotted cast types. A whole
  step would tie readings that a step ranks now: `(x) as` at the end
  of a line casts the name `as`, as Groovy does, and a dotted name on
  the next line is a statement, a reading that outweighs `(x) as a.b`
  across the line break by `CAST_WEIGHT`.

  The `<` after the name of a cast's type opens type arguments only
  where they close (spec §3.2.2): `x as a.B<c`, `x as B<c` and
  `v as java.lang.Integer<max && min>v` compare the cast with what
  follows, as Groovy reads them. Before, a plain name made them an
  `ERROR`, and a dotted one field accesses and comparisons on
  `x as a`, which Groovy does not read.

  When the weight came, with the fork at the first dot, the ties it
  decided had been decided by the last resort. A fuzz of about 1.97
  million inputs around these operators
  changed 2,903 clean trees and turned none into an `ERROR`; the
  corpus, the Groovy files and 20,910 Java files keep every clean
  tree. Most of the changes read as Groovy does:

  - `x`, then `f() as a.B` (or `instanceof`, `!instanceof`), then
    `y = 1` at the end of the input is the cast and an assignment
    (about four in five of the changes). It read a bodiless method
    `x f()`, the name `as` and a typed local `a.B y = 1`, and still
    does with a plain type (`f() as B`), which has no weight.
  - A generic type with a dotted base followed by another `as`,
    `instanceof` or `!instanceof` and a dotted type
    (`y as a.B<C> instanceof c.D` after a line) is the cast and the
    test. It read field accesses and comparisons.
  - `(x) as instanceof x.a` then `x = T`, and
    `(x) in (x) instanceof x.a`, test the cast `(x)` of the name
    `as` or of the call `in(x)`. They read a cast to the type
    `instanceof` and a typed local `x.a x = T`, and a cast `(x)` of a
    name `instanceof` followed by `x.a`.
  - A record after a field whose value is a cast to a dotted type
    (`def f = (T) x as a.T`, then `record R(int a) { R { } }`) is
    that field and the record. The field split into fields and a
    constructor.

  Two kinds moved away from Groovy's reading:

  - `def v = (x) as a.b.c y`, then `y = 1` and another statement,
    reads `(x) as a.b.c` and a typed local `y y = 1`
    (`operators-as-cast.txt`). It read the cast `(x)` of the name
    `as`, the call `a.b.c y` and an assignment, closer to Groovy 3's
    `def v = ((x) as)(a.b.c).y` and `y = 1`. The new reading is the
    one those lines have without the statement after them, and that
    `(x) as a.b y` then `y = 1` had before the weight. Since the
    `)` before `as` and a name closes no cast (spec §6.11), no
    weight takes part.
  - Type arguments after a dotted name of three or more parts at the
    start of a line (`x //`, then `f() as a.b.C<T>`, then `y = 1`)
    read the cast to `a.b.C` and comparisons,
    `(f() as a.b.C) < T > y = 1`. A tie kept them in a typed local
    `a.b.C<T> y = 1` after a bodiless method `x f()`. Groovy reads
    the cast to `a.b.C<T>` and `y = 1`, and so does the grammar now
    that such a name takes type arguments.

  Against the grammar before these changes, a truth table of 63,648
  inputs (`as`, `instanceof`, `!instanceof` and the parenthesized
  cast, 39 types, six left sides, fourteen followers, twelve
  contexts) changed 3,621 clean trees and 8,883 `ERROR`s into clean
  trees; 7,549 inputs now read as Groovy 3 does and 489 no longer do.
  A table of 8,640 inputs with a `<` right after the type
  (`x as a.B<c`, `x as a.b.C<y && z`, `x as T<c> y`) changed 1,998
  clean trees and 1,620 `ERROR`s into clean trees; 3,089 now read as
  Groovy and 136 no longer do. Those that no longer do are one kind:

  - Type arguments after a dotted name of three or more parts where
    Groovy reads comparisons with the name instead. It does so where
    an expression must stand (after `return`, as an argument, in a
    list or a map, as a branch of a ternary, a `case` or an `assert`)
    and what follows the `>` cannot go on from a type there:
    `return x as a.b.C<T> y`, `foo(x as a.b.C<T>[0], 1)`,
    `[x as a.b.C<T>` then `foo()]` are `(x as a.b.C) < T > y` and the
    like, which the grammar read when such a name took no type
    arguments. Now the type arguments are read, as after a plain name
    and a name of two parts: after `return` the cast is followed by a
    statement `y`, a `[0]` is a subscript of the cast, and inside
    brackets a name or a line after the `>` is an `ERROR`. Where
    a statement starts, as a declaration's value and in a condition,
    Groovy takes the type arguments too, and calls the cast with what
    follows: `x as a.b.C<T> y` is `(x as a.b.C<T>)(y)` and
    `x as a.b.C<T>[0]` is `(x as a.b.C<T>)([0])`, which the grammar
    reads as the cast and a statement, or a subscript of the cast.

  Some input that parsed, and that Groovy accepts, is an `ERROR` now,
  against the grammar before these changes and v0.6.0 alike. The
  corpus, the Groovy files and the 24,548 Java files have none. The
  kinds, with their counts in the three tables (the third holds the
  parentheses of the next paragraphs) and in the fuzz of 1,000,000
  inputs around the operators and of 500,000 around the `)`:

  - A quoted name or `new` after a dot of a cast's dotted type
    (`x as a.b."q"`, `x as T.new`). Groovy reads a command chain whose
    argument is a variable named `as`, `x(as).a.b."q"`, where a
    command may stand, and rejects the input elsewhere; the grammar
    read field accesses on the cast, `(x as a.b)."q"`, which the first
    dot no longer allows. 4,587 of the fuzz inputs.
  - Type arguments after a dotted cast type that close, followed
    inside brackets, a condition or an interpolation by what Groovy
    reads as comparisons with the type's base or as a call of the
    cast: `foo(x instanceof a.b<T> y, 1)`, `if (x as a.b<T> y) { }`,
    `[x as a.b.C<T>` then `foo()]`. The grammar read field accesses
    and comparisons on `x as a`, and for a name of three or more
    parts Groovy's comparisons (the kind above). 1,731, 324 and 476
    table inputs, 370 fuzz inputs.
  - Parentheses that can only be a cast's type, a generic or an array
    type, before `instanceof`, `!instanceof`, `!in`, or `as` or `in`
    and what they take (`(List<String>) instanceof Foo`,
    `(String[]) as Foo`, `(a.B<T>) in y`). Groovy tests a class
    literal, or calls the cast of a variable `as` or `in`; the
    grammar has no value reading of a generic type, nor of an array
    type in parentheses. It read the cast of the operator word and
    another statement. 647 table inputs, 1,140 fuzz inputs.
  - A call of the cast of a variable `as` or `in` with a command
    argument where no statement may follow on the line: in a field
    (`class K { def f = (x) as Foo y }`) and before a `,`
    (`def v = (x) as T y, z = 1`). Groovy reads `((x) as)(Foo).y`; the
    grammar reads the operator and leaves the `y` over. It read two
    fields, or two declarations. 43 table inputs, 6 fuzz inputs.
  - `!in` before an array type in parentheses, `(a.b) !in(Foo[])`,
    which Groovy tests; `x !in (Foo[])` is an `ERROR` too. 9 fuzz
    inputs.
  - `x[0] as C.d.e()..(x) instanceof T` followed by `foo()[]`, where
    the reading Groovy has no longer hides that the empty subscript
    `foo()[]` is an `ERROR` here. 1 fuzz input.

  Read otherwise than Groovy: `x as a.b[0]` is a subscript of the
  cast, where Groovy calls the cast with the list `[0]` (also after a
  plain name, `x as B[0]`); a space before the `<` makes it a
  comparison, where Groovy's lexer drops the space and reads type
  arguments where they close (`x as List <String>` is an `ERROR`;
  `return x as a.B < c > d` reads the comparisons Groovy reads there,
  `def v = x as a.B < c > d` too, where Groovy calls the cast to
  `a.B<c>` with `d`); type arguments longer than 256 items are read as
  comparisons.

  The `)` of parentheses before `instanceof`, `!instanceof`, `!in`,
  and before `as` or `in` and what they take, closes no cast (spec
  §6.11), so `return (x) instanceof Foo`, `def ok = (x) instanceof Foo`
  and `(x) !in y` are the tests of `(x)` that Groovy reads, and
  `(x) as Foo` and `(x) in y` the operators that Groovy 6.0.0 reads
  (GROOVY-10355; Groovy 3 reads a call of the cast of a variable `as`
  or `in` where a statement starts, as a declaration's value or as a
  condition, and the operator elsewhere, and 4.0 to 5.1 have no
  rewrite of that call). They read the
  cast `(x)` of a name `instanceof`, of `!` applied to the name `in`,
  or of the name `as` or `in`, followed by the next word as another
  statement. A truth table of 34,920 inputs (ten contents of the
  parentheses, the five operators with 42 right sides, seven
  followers, twelve contexts) changed 2,440 clean trees, 980 of them
  to Groovy 3's reading and none away from it, and turned 3,030 into
  an `ERROR`, all but 1,143 of them rejected by Groovy (the kinds
  above). Block comments may stand on the line before the operator
  and after `as` or `in`, as spaces may; one that runs past the end of
  the line after `as` or `in` counts as no operand.

  Read otherwise than Groovy 6.0.0, whose rewrite takes a call with
  one argument and no further command arguments, after parentheses
  that hold a name, possibly dotted, that is no primitive type, and
  for `as` an argument whose leftmost operand at relational precedence
  spells a type, and also reads `(Foo) in [..]` as `Foo in [..]` in any
  context: `(int) as Foo` coerces `(int)`, where Groovy casts the name
  `as` and calls it with `Foo`; `(x) in y, z` is the test and an
  `ERROR`, where Groovy calls the cast with two arguments;
  `(Foo) in [0]` casts the subscript `in[0]`, as Groovy 3 does, since
  the scanner cannot tell `(Foo)` from `(byte)` in `(byte) in[i]`,
  while `(Foo) in [1, 2]` is the test: a subscript takes several
  indices too, so the scanner reads the brackets for the `,` of a list
  of several elements, and `(byte) in[i, j]` is the test as well;
  `(x) in !y` casts the name `in`, as `!` counts as no operand; `(x) as foo()` calls the coercion `(x) as foo`, where
  Groovy calls the cast of `as` with `foo()`; and `(x) as T y` and
  `(x) in y z` read the operator and a statement, where Groovy calls
  the cast of `as` or `in` with a command argument.
- Generics (`List<String>`) ✓ via `generic_type`, with bounded
  wildcards (`? extends Foo`, `? super Bar`), nested arguments
  (`Map<String, List<Integer>>`), qualified bases
  (`java.util.List<String>`), class / interface / trait
  `type_parameters`, and method-level `method_type_parameters`
  (`<T> T identity(T x)`). The opening `<` of `type_arguments` and
  `type_parameters` is `token.immediate` so it only binds to a
  base name without intervening whitespace — preserving `a < b` as
  a binary comparison.

**Action**: the comparisons Groovy reads after type arguments it
cannot take (`return x as a.b.C<T> y`) and the call of a cast with a
list (`x as a.b[0]`) are open. `x as (java.util.Map)` is no cast here:
it reads two statements, `x` and a call of `as`, where Groovy 3 reads
one, the command call `x(as(java.util.Map))`, and Groovy 6 the cast
(`coercionType` takes a `castParExpression`).

Ties that tree-sitter's last resort still decides here: a
parenthesized cast whose operand is on the line of its `)`, followed by
a line that assigns (`(T) x` then `y = 1`), weighs as much as `(T)`
followed by the typed local `x y = 1`, and the cast is kept, as Groovy
reads it. Readings that hold as many dotted cast types can tie, as the
weight counts them; the tables and the fuzz above show no such tie.

---

## 6. Typed local variable declarations and the weights of their readings

**Spec §4 / §5.12** show typed declarations of all shapes,
including the no-initializer form (`String x`).

**Current implementation** accepts `Type x = expr` and
`Type x = expr, y = expr2` (via `_initialized_declarator`), after the
first declarator also one without an initializer (`Type x = expr, y`),
and the bare `Type x`, `Type x, y` and `final Type x, y`. The bare form
is grammatically indistinguishable from the expression `Type` and the
statement `x`, which a line break between them makes in Groovy, so the
scanner decides it (spec §6.21; see #27 below).

Because a newline is whitespace (#3), the tokens of a typed
declaration also parse as two statements: `String s = 'a'` as the
expression `String` followed by `s = 'a'`. Where a statement starts, the
scanner's token opens the declaration (spec §6.21), so the two readings
no longer fork at its type. After modifiers, and where the token is not
emitted, the typed forms carry a dynamic precedence, so the declaration
wins whenever both parse; without it a comment before the declaration
changed the result. The cost is the reverse case: a bare `x` followed by
`y = 1` on the next line is the declaration `x y = 1`, and `String`
followed by `s = 1` the declaration `String s = 1`, where Groovy reads
two statements. A declaration with a type and a name on the line after
`final a` is one of its own: `final a` followed by `String b`,
`String b = 'y'` or `String b = x -> y` declares `a` and `b`, as Groovy
does (it was `final a String` followed by `b = 'y'`, and until the
scanner learned where closure parameters may start, spec §6.21, by
`b = x -> y` with a lambda for a value).

A method declaration with a body carries a dynamic precedence for the
same reason: `String f() { }` also parses as the expression `String`
followed by the call `f() { }`, and `def f(a) { }` as a variable `f`
followed by the cast `(a) { }`. Here too the reverse case loses: a
bare `x` followed by `foo() { }` on the next line is the method
`x foo() { }`. The method's weight is above the blank final's:
`final String f(a) { }` is also `final String f` followed by the cast
`(a) { }`. With the cast's weight that reading weighs as much as the
method, and tree-sitter's last resort keeps the method by comparing
symbol numbers. The weight costs the blank final one case:
`final String name` followed by `foo() { }` on the next line is
`final String` followed by the method `name foo() { }`.

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

The weights of class members, in steps (`WEIGHT_UNIT` each as a
dynamic precedence):

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
statements. Outside a class body a
method needs a body, so only the statement reading is valid Groovy.
The cost is on invalid input: `String foo()` with no body, which
Groovy rejects in a script, parses as `String` followed by `foo()`
with no `ERROR`. In a class body nothing competes with the method, so
abstract and interface methods are unaffected.

**Action**: none for the declarations the token opens. The reverse
case would need the token to read the line before (`x` then `y = 1`),
which it does not.

---

## 7. Slashy-regex injection is one document per fragment

**Spec §9.2** wants `queries/groovy/injections.scm` to inject
slashy and dollar-slashy bodies as `regex`.

**Current implementation** injects each `string_fragment` of such a
string as its own `regex` document. The delimiters (`/`, `$/`, `/$`)
and the `${…}` and `$name` interpolations are not injected. Every
string flavour is a `string_literal`; the query tells the two regex
flavours apart by a delimiter child rather than the source text: only
a slashy string has an anonymous `/` child (its closer; the opener is
a hidden scanner token) and only a dollar-slashy string has `$/`.

**Divergence**: a body that an interpolation splits is several
documents, and so is one with a literal `$`, which is a fragment of
its own: `/^a+$/` is `^a+` and then `$`, and a group that spans the
split (`/(a$|b)/`) does not parse as one. Capturing the string itself
does not avoid this: by default a node's children are left out of its
injection, so only the opening `/` was injected (nothing of a
dollar-slashy string), and `injection.include-children` would pass the
delimiters and interpolations as regex text. `tree-sitter-highlight`
injects one content node per match, and `injection.combined` would
join every regex in the file into one document.

**Action**: none. One document per string needs a body whose literal
text is not child nodes, an AST change.

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

## 10. Groovy 6 contextual keywords (`val` / `async` / `await` / `defer`)

**Spec §5.2** lists `val`, `async`, `await`, and `defer` as
contextual keywords — keywords in specific syntactic positions,
identifiers elsewhere (`val` is conditionally enabled in real
Groovy via `'val' {isValEnabled()}?`). They are Groovy 6 syntax: the
6.x grammar has `async` closures, `await`, `for await`, `defer` and
`val` declarations, and the 5.0.x grammar none of them. Groovy 6.x is
the grammar this one targets (spec §1.2 item 5).

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
   Groovy 6 mode (e.g. via shebang or a leading directive), at
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
parses with these tokens as plain identifiers, as Groovy 5 and earlier
read them; Groovy 6 reads them as keywords where their constructs
follow, so its `async { … }`, `await x`, `defer foo()` and
`val x = 1` read as calls and a typed declaration here.

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
opening the next line). After a `;` or a block the `{` opens a
statement and is a block (#28). After a parenthesis-free call whose
last argument is a closure it is a block as well (`foo { a }` then
`{ b }`), where Groovy passes it to the call; after a literal or a name
argument it is that argument's closure, as in Groovy (`foo 1` then
`{ y }` is `foo(1({ y }))`). After a postfix update, an
`instanceof` test or an `as` cast it joins the expression (`x++`,
`x instanceof Foo` or `x as Foo`, then `{ y }`, calls the result with a
closure), where Groovy reads a block. Groovy 3 runs the block after
`def b = x instanceof String`, `x as String`, `x as List<String>` and
`[1] as int[]`, as a type takes no closure (Apache `matchingType`,
`coercionType`), and calls `foo(x)` or a name with it; a `{ -> … }`,
`{ a, b -> … }` or `{ … }()` there is a closure statement. A scanner
token for a `{` that opens such a line after the name of a cast's type,
taken by a block where a statement may start, cost 4 states but turned
42 of 4,000 shape inputs (`z = (x) as Foo` then `{ -> … }`) into an
`ERROR`; reading the closure statement as well cost 32 states, and a
type that ends in `>` or `]` needs a marker of its own, 8 more. The
corpus test "A block on the line after an instanceof test or an as cast
is a closure of the result" pins today's reading. In a class body the same holds after a field
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

Groovy takes the class modifiers in any order, and so does the
grammar: `sealed abstract class S { }`, `sealed @A class S { }` and
`sealed static class C { }` in a class body are declarations (#92);
they were an `ERROR` at `sealed` at v0.6.0. `sealed record` and
`sealed enum`, which Groovy rejects too, stay an `ERROR`.

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
| `private sealed` then `def run() { }` or `int x = 1` on the next line, in a class body | Groovy 4 to 6: that member with the modifiers `private sealed`, as `modifiers` takes line breaks between modifiers (read off the 4.0.x, 6 and master grammars); Groovy 3, where `sealed` is no keyword: field `sealed` and the member (observed) | field `sealed` and the member, Groovy 3's reading, as the grammar takes `sealed` only before `class`, `interface` or `trait`; at v0.6.0 the member with `sealed` an `ERROR` in it |
| `private boolean sealed` then `static class D { }` on the next line | field `sealed` and a class (read off the 4.0.x grammar) | the same; `ERROR` at v0.6.0 |

Across a line break the scanner leaves the word a name before a
modifier, so the last row keeps its field rather than folding it into a
sealed class. Looking past the modifiers to `class`, `interface` or
`trait` would read the `sealed` then `public class` row as Groovy does,
but the last row then as one sealed class too, where Groovy reads a
field; nothing in the look-ahead tells the two apart.

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
`final` and annotations after it, then `def`, `var` or a type, the
name, and a default.

| Parameter | Groovy 4.0.32 | This grammar |
|---|---|---|
| `@A final String x = 1`, `def x`, `var x`, `final def x`, `String... xs` | accepted | accepted |
| `def String x` | accepted | `ERROR` |
| `final @A x`, `@A final @B String x`, in a method, a closure or a lambda | accepted | accepted; `final @A x` was an `ERROR` at v0.6.0 (#92) |
| `... xs`, `def ... xs` | accepted | `ERROR` |

`def` and `var` are keywords in a parameter, so `def f(def x)` has no
`type`. `catch` parameters and `try` resources take `final` and `def`,
`var` or a type, with no annotations.

**Action**: accept `def` before a type and `...` with no type.
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
| `x = (a) - b`, `x = (a) + b`, `x = (foo.bar) - c` on one line | cast of `-b` (Groovy 3 to 5); from 6.0.0 the difference or the sum, as `AstBuilder.visitCastExprAlt` rewrites a cast whose type is a bare name with a last segment that starts with no upper-case letter (GROOVY-10355), while `(A) - b` stays the cast | the cast, as before: the scanner decides at the `)`, past the name (spec §5.15) |
| `foo((a)` then `(b))` | cast of `(b)` | call `(a)(b)`, as at v0.5.0 |
| `[(Foo)` then `(y)(z)]`; `a[(Foo)` then `(y)(z)]`, `a?[(int)` then `(a)(b)]` | the cast of the cast `(Foo) ((y) z)` (Groovy 3) | as a list's first element, the same, as at v0.6.0; as a subscript's index, the call `(Foo)(y)(z)`, where v0.6.0 read the cast of the cast; as a later list element, a map value or a call's argument, the call, as at v0.6.0. The two readings weigh the same (the later-line cast's `-CAST_WEIGHT` and the same-line cast's `+CAST_WEIGHT` cancel). Reduced as one hidden rule between the same stack nodes, two such trees are merged ahead of time and the runtime keeps the first (`stack_node_add_link` in tree-sitter's `stack.c`), the call; trees of different node types are compared when their parent is reduced, and the cast wins. A list's element is inlined into the list for that (3 parse states); the subscript's indices, one hidden rule shared by both subscripts, would cost about 40 |
| `x = (a)` then `(b) y` | `x = a`, then the cast `(b) y` | the same: the `(` opens the next statement (spec §6.18); before that, one assignment, `(a)` cast of `(b) y` |
| `(Foo)` then `(x)`, `(a) = (b)` then `(c) = (d)` | two statements (Groovy 3) | the same; the `)` that ends the first line closes no cast (spec §6.11), so a run of such lines parses in linear time |
| `foo((Foo)` then `(Bar) y)`, `foo((Foo)` then `(Bar) ')')`, `foo((Foo)` then `(Bar) y /* ) */)` | cast of the cast (Groovy 3) | the same: the scanner finds the call's `)` after the next line's `(`, on Groovy's tokens, so a bracket in a string or a comment counts for none |
| `foo((Foo)` then `(Bar) y, '…')`, the call's `)` more than 256 characters after the `(` | cast of the cast (Groovy 3) | an `ERROR`: the scanner reads 256 characters for a bracket around both lines (ENCLOSING_BRACKET_LOOKAHEAD_CHARACTERS, spec §6.11); finding none, it closes no cast with the `)` after `Foo`, and `(Foo)` calls `(Bar)` before an `ERROR` at `y` |
| `foo((Foo)` then `(bar) + 1, '…')` or `(bar)[0], '…')`, the call's `)` more than 256 characters after the `(` | cast of the cast (Groovy 3), as at v0.6.0 | with no `ERROR`, the sum of the call `(Foo)(bar)` and `1`, or a subscript of that call: the cast is turned down as in the row above, and the rest of the line goes on from the call |
| `(List<String>)` then `(x)` | `List`, then `x` (Groovy 3) | the cast `(List<String>) (x)`, as before: an array or generic type is no expression, and the cast is the only reading of its parentheses |
| `(Foo)` then `(x) -> x` | the call `(Foo)((x) -> x)` (Groovy 3) | the cast of the lambda: the scanner keeps the cast where `->` follows the next line's parentheses (spec §6.11), as after an expression the lambda look-ahead does not run (spec §6.17) and the call would end in an `ERROR` at `->`; at v0.6.0 a cast of a cast with an `ERROR` at `->` |
| `class C { def k = (Foo) x as Bar` then `def m() { } }`; `… (Foo) x in y` then `private int z` | one field with the cast, and a method (a field) (Groovy 3) | the field `k = (Foo)`, the fields `x as` (`x in`) and `Bar def` (`y private`), and a constructor `m` (the field `int z`), with no `ERROR`, in every build since v0.6.0: see the type-body paragraph below |
| `"${(Foo)` then `(Bar) y}"` | two statements in the interpolation's closure, `(Foo)` and the cast `(Bar) y` (Groovy 3) | an `ERROR`: the interpolation's `}` comes before any bracket around both lines, so the `)` after `Foo` closes no cast (spec §6.11), and `(Foo)` calls `(Bar)` before an `ERROR` at `y`; at v0.6.0 the cast of a cast, with no `ERROR` |

At v0.5.0 the two readings were unranked and the stack order decided:
`x = (a)` followed by `foo()` was a cast of the call.

In a type body a cast's own line decides too. `def k = (Foo) x`
followed by `void m() { }` or `private int n` on the next line is the
field `k` with the cast and a method or a field, as in Groovy 3. Until
the `)` before a name on its line became `_cast_word_close` (spec
§6.11) it was the field `k` with the value `(Foo)`, a field `x void`
(`x private`) and the constructor `m() { }` (the field `n`), with no
`ERROR`: the second reading holds a typed field, whose weight
(`TYPED_FIELD_WEIGHT`, 4 steps) outweighed the cast's (`CAST_WEIGHT`, 1
step). Parentheses that hold no type keep the split where a name
follows: `def k = (1) x` then `def m() { }` is still the field `k = (1)`,
a field `x def` and a constructor, where Groovy calls `(1)` with `x`
(a command expression whose receiver is in parentheses, which this
grammar does not read). Where the name is followed on its line by `as`,
`in` or `instanceof` and another name, the split stays, in every build
since v0.6.0: `def k = (Foo) x as Bar` then `def m() { }` is the field
`k = (Foo)`, the fields `x as` and `Bar def` and a constructor `m`, and
`def k = (Foo) x in y` then `private int z` the fields `x in`,
`y private` and `int z`, with no `ERROR`, where Groovy 3 reads one field
and a method (a field). The split holds two typed fields there, and the
penalty of `_cast_word_close` offsets one (`TYPED_FIELD_WEIGHT`); the
scanner decides on the same tokens either way, so a larger penalty for
that case would need a token of its own. A `;` after the value, or the
`}` of the body after it, keeps the cast.

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

Runs of lines that end in a parenthesized name or a cast of one still
take time quadratic in their length where the next line does not open
with `(`, as at v0.6.0: 4,000 lines of `x.y = z * (w)` take 2.0 s, and
of `this.total = base + (int) (offset)` 4.1 s. The `)` that ends each
line may close a cast whose operand is on the next line, the reading
ranked last, which stays alive; the scanner reads the next line for it
only where that line opens with `(`.

**Action**: a scanner that tracks bracket depth, which would also give
the grammar a statement terminator where a line break is significant
(#90, #96). Tracked in #99.

## 16. The marker of an unterminated block comment

**Spec §6.6**: a `/*` with no `*/` is not a comment, and the first one
emits an empty hidden extra token, `_unterminated_comment`, that
carries the scanner's "no `*/` from here on" bit (#75), one of its
four bits of state; the second is set after the `}` of a lambda's
brace body or a statement's block (spec §6.19, §6.22), and the third
after `return`, `break` or `continue` before a block comment that
something follows on the line (spec §6.23). The fourth is carried by
a comment after a newline before a slashy string (spec §6.2).
Valid input never sees the marker that sets the first. Two effects
on other input are accepted.

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
emits only when a literal, a key, a `new`, an array type value or a
name follows the receiver on its line (spec §6.14). Before #95 the
forms below were a declaration of `foo`
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
| `def x = foo bar`, `def x = foo $/a/$`, `def x = Foo $/a/$` | `foo(bar)`, `foo('a')`, `Foo('a')` | `command_chain` value (#25 below), as a dollar-slashy string is a first argument after any receiver (spec §5.7); before, `def x = foo` (`Foo`) then the argument |
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
a name followed by member accesses (`.`, `?.`, `??.`, `*.`, `.&`, `.@`,
`?.@`, `??.@`, `*.@`) written without spaces, built of the same nodes
as in an expression. A statement with such a receiver holds
`_command_path_start`, an empty token between the first name and the
first access that the scanner emits only when a literal, a key, a `new`, an array type value or a
name follows the receiver on its line (spec §6.14); the value form's
`_command_value_start` reads the same receivers before them. The
arguments are those of the value form: the first is a whole expression
that starts with one of those, or a named argument. Before #52 each
form below with a
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
| `def a = foo .bar 1`, `x = (foo ?.bar 2)`, `def c = foo .Bar y`, also with a tab | a call | a call: where a lambda may start in a value, the spaces after the first name are skipped for the arrow (spec §6.14); not intended, kept as it agrees with Groovy |
| `def b = Foo .bar baz` | `Foo.bar(baz)` | `Foo .bar` and `baz`: the first name, not the last, decides whether a name is a first argument there |
| `def d = foo .new Bar()` | `new Bar(foo)` | a call of a method `new` with `Bar()`; without the space, `foo.new` and `Bar()` |
| `x = "${foo .bar 1}"`, `(p, q) = foo .bar 3`, `def f = foo /* c */.bar 5` | a call | the expression and the number, an `ERROR` in the interpolation |
| `foo /* c */.bar 4` | a call | a call: the parser skips the comment before the statement's token; with a space after the comment, the expression and `4` |
| `foo().bar 1`, `foo[0].bar 1`, `'s'.m 1`, `(obj).m 1`, `obj.'name' 1`, `obj::m 1` | a call | the expression and `1`, as before |
| `true.x 1`, `null.x 2`, `def a = true.x 3` | `true.x(1)`, a call | the field access and the number as two statements, as before: a receiver starts with a name, and `true` and `null` are literals |
| `a.b c` | `a.b(c)` | `command_chain` (#25 below); before, `a.b` then `c` |
| `a.b.C x = 1`, `a.b c = 1` | a declaration | a declaration, as before |
| `obj.m { }`, `obj.m -1`, `obj.m [1]`, `obj.m (1)` | a call with a closure; a subtraction; a subscript; a call | the same, as before |
| `obj.m 'a' other 'b'` | `obj.m('a').other('b')` | the chains `obj.m 'a'` and `other 'b'` (spec §5.5 item 4) |
| `x.y a.b 'c'`, `foo a.b 'x'`, `foo 1 a.b 'c'`, `foo 'a' obj.m 'b'`, `x() a.b 'c'` | a chain continuation (`x.y(a.b).c`) | the first statement and the call `a.b 'c'`; before, the first statement, `a.b` and `'c'` |
| `return obj.m 'x'` | rejected | `return obj.m` and `'x'`, as before |
| `return` then `obj.m 'x'` on the next line | `return`, then a call | the same (spec §6.23); before, `return obj.m` and `'x'` |
| `java.util.List 'foo'() { }`, `java.util.List '''foo''' () { }` | a method | a method, as before |
| `obj.m 'a'() { }`, `obj.m 'a'(b)`, `java.util.list 'a'() { }`, `java.util.List "$x"() { }` | a call with the argument `'a'(…)` | the same; before, a method declaration, or `obj.m` and `'a'(b)` |
| `java.util.List 'foo'(1)` | a call | `java.util.List` and `'foo'(1)`, as before |
| `x.y 1` then `-1` on the next line | two statements | one call with the argument `1 - 1`; before, `x.y` and `1 - 1` |
| `x.y 2` then `++i` on the next line | `x.y(2)`, then `++i` | the call with the argument `2++`, then `i`: the line goes on with the first argument |
| `obj.Bar $/$a/$() { }`, or `"$a"`, `"${a}"` or `"""$a"""` in place of the string, with `def` typed on the line before and deleted again | a call with the argument `$/$a/$(…)` | a call from scratch; the reparse after the deletion reads a method that returns `obj.Bar` and is named by the string, neither with an `ERROR`: the runtime ignores an empty token during the error recovery of the text with `def`, and the reparse reuses the `.` after `obj` without calling the scanner, so `_command_path_start` is not emitted (spec §6.14). v0.6.0 reads the method from scratch too. `bindings/rust/tests/incremental_reparse.rs` pins both trees |
| `obj.m 1, *xs` | a call | a call; before, an `ERROR` |
| `obj.m 'a': 1`, `def r = obj.m 'a': 1` | a call | a call; before, an `ERROR` |

**Action**: none planned for the spaced and multi-line receivers. A
receiver that ends in a call or a subscript needs the scanner to read
balanced brackets. Turning the spaced value receivers above down, as the
rule says, was tried: of 2,900 inputs (three receivers, no separator,
a space, a tab or a comment before each of `.`, `?.`, `*.`, `.&`, `::`
and `.@`, two member names, no first argument or one of three kinds,
five contexts), 300 trees that
agreed with Groovy 3 moved away from it and none moved toward it, and
`Map .Entry[].class` became a subscript with a missing index. Taking
the name rule from the last name instead moved 30 toward Groovy and 30
away. The test 'Spaces before the first access of a value's receiver'
in `test/corpus/expressions-command-chain.txt` pins the trees.

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
`default`, and annotations after the first of them, as they come in
any order among themselves. After them the return type is optional, and a member with none is a method, where
after other modifiers it is a constructor (`public m() { }`): the
modifier is meaningless on a constructor. The modifier is an anonymous
`default` child before the `name`; an `@interface` element's default
is the `default` field after the parameters, and no method has both.
A class, interface, trait, enum or record declaration takes `default`
among its modifiers too, at the top level or nested, through the
hidden `_type_modifiers`, and so does an `@interface` declaration
(`public @interface X { }`, `default @interface Z { }`; #92). Fields, constructors, compact constructors, initializers
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
4 + 3 steps against −1 + 4 + 3. With a modifier-only field between them
(`private String y`, `static count`, then `default void b() { }`) the
split `private String`, `y static`, `count default`, `void b() { }`
weighs 10 steps against 6 and wins. That is the weakness of modifier-only
fields that §6 describes: `private String y`, `static count` and
`void b() { }` on three lines split the same way before this rule, as
a field `count void` and a constructor `b`.

In an `@interface` body a `default` after the `()` is the element's
default, by a static precedence on `_annotation_type_element`, so a
line break before it is accepted, as before (§5.9 row above).

Error recovery changed at an unclosed `switch` whose last branch is a
colon case at the end of the input (`switch (x) {`, `case 1:` and
`foo()`): the whole input is one `ERROR`, where v0.6.0 inserted the
missing `}`. At the end of the input the runtime inserts the first
missing token, in token order, after which the parser can reduce, and
`identifier` comes before `}`. Before this rule the state after a name
in a case body had no action at the end of the input, so a missing
name was turned down and `}` was taken; since this rule the table
shares that state with the one a name reaches in a script, where the
input may end, so a missing name is inserted and the parse stops
there. An arrow case and an empty switch body still get the missing
`}`. `if (x) {` followed by `foo()` was such an `ERROR` at v0.6.0
already. Giving the case body a state of its own would take a token
that acts differently there than in a script, and was not tried; the
corpus test 'An unclosed switch with a colon case at the end of the
input' pins the tree.

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
| `default @Override void b() { }` | a method | a method; before, an `ERROR`, as was any annotation after a modifier (#92) |
| `default = 1`, `default.x()`, `return default`, `break default`, `String default`, `final default = 1`, `void default() { }`, `{ default -> 1 }`, `println default`, `if (x) a() else default`, `static default = 1` and `private default` in a class, `enum E { default }` | rejected: `default` is reserved | `ERROR`; before, `default` was a name |
| `x = default`, `foo(default)`, `[default]`, `throw default`, `assert default`, `def default = 1`, `def default() { }`, `def f(default) { }`, `for (default in xs) { }`, `void default() { }` in a class | rejected | `default` is a name, as before |
| `foo.default`, `[default: 1]`, `foo(default: 1)`, `foo 'a', default: 1` | a property, a key | the same, as before |
| `foo default: 1`, `x = foo default: 1`, `a.b default: 1` | a call with the named argument `default` | the statement, then the label `default:`, as before (spec §6.8) |
| `switch (x) { case 1: foo()` then `default: bar() }` on the next line, `case 1: default: c()`, and the other switch forms | the switch | the same tree as before |
| `switch (x) { case 1:` or `default:`, with or without statements after it, at the end of the input, also inside a method | rejected | an `ERROR` over the input; before, the switch with a missing `}`, inside a method the method and an `ERROR` around the switch |
| `switch (x) {` or `switch (x) { case 1 -> foo()` at the end of the input | rejected | the switch with a missing `}`, as before |
| `lbl: default void b() { }`, `if (x) default void b() { }` | rejected | a label or an `if` around a `method_declaration`; before, around the expression `default`, then a method |
| `private String y`, `default void a() { }`, `static count` and `default void b() { }` in a class, then the `lt` of the second `default` deleted | rejected (`defau void b()`) | from scratch the field `y`, the method `a`, a field `defau` of type `count` and the method `void b() { }`; a reparse after the edit reads `private String`, a field `y default` and `void a() { }` instead, neither with an `ERROR`: the runtime checks a reused member only while one stack version exists (spec §6.18), and `static count` is reused though a name follows it now. The next edit of the word heals it; `bindings/rust/tests/incremental_reparse.rs` pins it |

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
| `def c = String /* c */ [].class` | a class | a `MISSING` index, as before (an `ERROR` where a statement starts): only spaces, tabs and line escapes may come before the `[` |
| `def c = _Foo[].class`, `$Foo[].class`, `Été[].class` | a class when the name is one (`_Foo` observed) | a `MISSING` index, as before (an `ERROR` where a statement starts): the name must start with `A` to `Z` |
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
| `enum E { A(v -> v) }`, `A(val -> val)`, `A(value -> value)` as an enum's first constant | lambdas | `ERROR` | lambdas |
| `enum E { A(var -> var) }` | a lambda with the parameter `var` | `ERROR` | `ERROR`: the arguments are also a constructor's parameters (`A(var x) { }`), where `var` is decided as the keyword before the arrow is seen; the keyword ends at the word, the lambda's empty token before it, and the scanner cannot mark both once the spaces after the word are read |
| `enum E { A(var(1)) }` | a call of a method named `var` | `ERROR` | the call: where a parameter and an expression may both start, `var` before `(` is a name (spec §6.16) |
| `void var() { }`, `int as(int x) { x }` where a statement starts | methods | `ERROR` | methods |
| `for (String in : xs)` | a loop over `in` | `ERROR` | a loop over `in` |
| `for (var x in xs)`, `for (var x : xs)` | loops | `ERROR` | loops |
| `var (a, b) = l`, and `private var (a, b) = l` in a class body | declarations | `ERROR` | declarations |
| `var(a, b)`, `var (a, b).foo()` in a method | calls of `var` | `ERROR` | calls |
| `import a.B` then `as = 1` on the next line | an import and an assignment | `ERROR` | the same as Groovy |
| `println var`, `println in`, `foo as` | calls | `ERROR` | `println var` a call, as `println x` (#25 below); `println in` and `foo as` two statements, as `in` and `as` go on with the receiver as operators |
| `var x = 1` in a class body | a field `x`, `var` a modifier | a field `x` of type `var` | as Groovy: a field `x`, `var` an anonymous modifier as `def` is; `var String s` a field of type `String` |
| `var m() { }`, `interface I { var m() }` in a type body | rejected (Groovy 3: "var cannot be used for method declarations"; Groovy 6 parses a method and its AST builder rejects it) | a method with the return type `var` | the same as before: before a name and `(`, `var` is no modifier |
| `var` then `x = 1` on the next line | a declaration of `x` | the same | the same |
| `var true`, `var super`; `var` then `null` on the next line | the call `var(true)` (Groovy 3), …; the name `var`, then `null` | declarations of locals named `true`, `super` and `null`, with no `ERROR` | the same as before: before a keyword that starts an operand `var` stays the keyword (spec §6.16); `var new Foo()` is an `ERROR` |
| `return var` then `x = 1` on the next line, `if (c) return var` then `else x`, `break var` then `x` | `return var`, then an assignment; the `if` with its `else`; `break` to the label `var` | `return`, then a declaration of `x`; a declaration of `else`; `break`, then a declaration of `x` | as Groovy: `var` after the keyword on its line is the value or the label |
| `yield` then `1`, `yield // c` then `1`; `yield` then `-1` on the next line | the name `yield`, then the statement (Groovy 3) | a `yield_statement` of `1` (`-1`), also at v0.6.0 | the `command_chain` `yield 1`; the difference `yield - 1`, with no `ERROR`: a line break after `yield` makes it a name (spec §6.16), and a name takes a literal on the next line as its call's argument (`foo` then `"a" + b`, #25 below) and a sign there as a binary operator (`a` then `+ b`, #3 above). `yield` then `{ }` is the call `yield({ })`, as in Groovy |
| `yield x`, `yield (x)`, `yield -1`, `yield [1]` outside a switch expression | a call, a call, a subtraction, an index (Groovy 4 and later: the same, read off the `.g4`) | `yield_statement` | the same as before |
| `yield` then `x` on the next line, also after `// c` or `/* c */` on its line | two names (Groovy 4 and later: the same, as `yieldStatement` takes no `nls` before its value) | a `yield_statement` of `x`, also at v0.6.0 | two names |
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
| `return` then `foo new Foo()` on the next line | `return null`, then a call | the same (spec §6.23); before, `return foo`, then the creation (spec §5.5 item 2) |
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
| `"${a; b}"`, `"${def x = 1}"`, `"${foo 1; 2}"`, `"${if (c) 1 else 2}"`, `"${return 1}"`, `"${throw new E()}"` | a closure, called | an `ERROR`, as before: an interpolation holds one expression, and statements in it are not parsed (the lazy form `"${-> x}"` parses, #2 above) |
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
| `Foo bar`, `int x`, `foo.Bar x` | a declaration with no value | a `local_variable_declaration` (#27 below); before, two statements |
| `void x` | `void(x)` | two statements, as before: `void` takes no name argument and names no type of a variable |
| `foo.Bar this`, `String super`, `String int` | `foo.Bar(this)`, `String(super)`, `String(int)` | two statements, as before: a receiver that may name a type takes no name argument, and a reserved word declares no variable |
| `Été x`, `Ωmega x`, `Жук x` | a declaration with no value | a `local_variable_declaration` (#27 below); before, two statements: a capital of Latin-1, Greek or Cyrillic counts as upper case; one of another script reads as lower case, and such a receiver takes a name argument |
| `foo bar = 1`, `foo.bar baz = 1`, `myType x /* c */ = compute()`, `foo bar /* c` then `*/ = 1` | a declaration | the same, as before: comments after the name are trivia |
| `foo a /* c */ b`, `println x // c` | `foo(a).b`; a call | the call `foo a`, the comment and `b`; the call and the comment |
| `foo a /* c */ : 1`, `def v = foo a /* c */ : 1`, `foo a /* c` then `*/ : 1` | `foo(a: 1)` | the same: a block comment before the `:` is trivia, as Groovy's lexer hides it; before, the receiver, or the declaration, and the label `a: 1` |
| `foo in /* c */ : 1`, `foo bar baz /* c */ : 1` | `foo(in: 1)`, `foo.bar(baz: 1)` | `foo` and the label `in: 1`; the call `foo bar` and the label `baz: 1`: a key spelled `in` takes no comment before its `:` (spec §6.8), and the chain does not go on (spec §5.5 item 4); before, an `ERROR`, and `foo`, `bar` and the label |
| `foo bar =~ x` | `foo(bar =~ x)` | the same; before, the declaration of `bar` with the value `~x` |
| `foo bar` then `= 1` on the next line; `a.b.c x` then `y = 1` | `foo(bar = 1)`; `a.b.c(x)`, then an assignment | the same; before, a declaration across the line break |
| `def x = Foo bar`, `(Foo bar)` | `Foo(bar)` | the value `Foo` and the statement `bar`, and an `ERROR`, as before: the receiver may name a type |
| `in x`, `as x` where a statement starts | `in(x)`, `as(x)` | two statements, as before: `var`, `yield`, `in` and `as` take no name argument as receivers where a statement starts |
| `as "a" + b`, `as 1 + 2`, `as (foo 1)` where a statement starts or on the line after an expression | `as("a" + b)`, `as(1 + 2)`, `as(foo(1))` | `as "a"` and `+ b`, `as 1` and `+ 2`, and an `ERROR`, as before: `as` is a receiver only before `new` or an array type value (`as new Foo()`), so that a reparse reads what a parse from scratch does (spec §6.14) |
| `def x = in y`, `(in y)` | `in(y)` | `command_chain`; before, the value `in` and `y`, and an `ERROR` |
| `record x`, `trait x`, `sealed x` where a statement starts | `record(x)`, … | two statements, as before: the scanner decides each word's keyword there |
| `foo bar` then `baz` on the next line, `foo /* c */ bar` | a call, then `baz`; a call | the call and `baz`; two statements and the comment, as before (spec §6.14) |
| `if (x) foo bar else baz qux` | an `if` with an `else` | the same; before, the `if` and four statements |
| `then: thrown IllegalStateException`, `{ foo bar }` | a call | `command_chain` |
| `foo bar() { }`, `foo bar(a) { }`, `task copy(type: Copy) { }` | `foo(bar({ }))` | the same; before, a `method_declaration` with return type `foo`, or two statements for the last |
| `foo bar(int a) { }`, `foo bar(String a, b) { }`, `foo bar(final a) { }`, `foo bar(a, int b) { }`, `foo bar(String... a) { }`, `foo bar(int` then `a) { }`, `foo bar(Map<K, List<V>> a, int b) { }`, `foo bar(String[] a, List<? extends T> l) { }`, `foo bar(InputStream in, String... names) { }` | rejected | a `method_declaration`, as before |
| `foo bar(baz qux)` with no body | `foo.bar(baz(qux))` | `foo`, then the call `bar((baz qux))`; before, a `method_declaration` with no body |
| `println max(VALUES[i])`, `println format(DEBUG ? a : b)`, `println range(MIN..MAX)`, `println foo(MAX as long)`, `println foo(X in list)`, `println foo(Foo instanceof Bar)`, `println valid(MIN < x && MAX > x)`, `(foo x(B[i]))` | a call whose argument is the call | `command_chain`: only what a type holds may follow a capitalized name in a parameter, and `instanceof`, and `as` or `in` before an operand, name none (spec §6.14); before, `println` and the call, or an `ERROR` for the last |
| `println foo(A<b, c> d)` | `println(foo(A < b, c > d))` | `println` and the call `foo(…)`, as before: `A<b, c> d` reads as a parameter whose type has type arguments |
| `println replace(text, /a+/)`, `def x = foo bar(/re/)` | a call whose argument is the call | `command_chain`: a slashy string in the parentheses is no parameter; before, the receiver, or the value `foo`, and the call |
| `bar sh(println` then `List)` on the next line | `bar.sh(println(List))` | a `method_declaration` with no body, as before |
| `{ foo a -> a }`, `{ foo a, b -> a }`, `{ foo a` then `-> a }` on the next line, `{ foo a, b = 1 -> a }`, `{ foo a, @A(x) b -> a }`, `{ foo a, b /* c */ -> a }`, `{ foo a, b // c` then `-> a }`, `{ foo a, final` then `b -> a }`, `{ foo a,` then `@A` then `String b -> a }`, `{ String a = 1, var` then `b -> a }`, `{ foo a, Map<K, V> b -> a }`, `{ foo a, b = i++` then `* 2 -> a }`, `{ foo a, b = /re/ -> a }`, `{ foo a, b = (1) -> b }`, `{ foo a, b = $/x` then `y/$ -> a }` | closure parameters, the first of type `foo` | the same, as before: a dollar-slashy string is read whole, across lines, within 1,024 characters (spec §5.7) |
| `{ foo a, b0, b1, …, b127 -> a }`, 128 or more names after a lowercase-typed first parameter | closure parameters | `ERROR`: the look-ahead for `->` reads 256 items (a comma and a name are two), then answers that no parameters follow; 127 names parse |
| `{ foo a, b = '->' }`, `{ foo a, b = /->/ }`, `{ foo a, b = { x -> x } }`, `println msg, "${path.join(" -> ")}"` | a call with two arguments | the same: strings are read whole, a slashy one where an operand may start and a `${…}` with the strings in it too, and an `->` inside brackets ends no list |
| `assertThrows IllegalArgumentException, () -> foo()`, `foo x, (y) -> y`, `foo x, k: () -> b`, `list.each { foo x, () -> b }` | a call whose last argument is the lambda | the same: an element that starts with `(` or a key starts no closure parameter, so the `->` is the lambda's |
| `foo x, y = () -> b`, `foo x, 'k': () -> b` | a call whose last argument holds the lambda | an `ERROR`: the look-ahead reads closure parameters up to the `->` |
| `foo a, i++` then `def f = x -> x`; `println a, /re/` then `def g = x -> x`; `case 1 -> inc counter, i--` then `case 2 -> …` | a call, then the next statement or case | the same: a postfix update and a slashy string end an operand, and at a line break after an element the list goes on only where `->` or an operator that may open a line follows |
| `foo a, i-->0`, `tokenize text, /->/`, `println a, $/x->y/$` | a call | the same: `--` is one token, and a slashy string where an operand may start and a dollar-slashy one are read whole, so none opens an `->` |
| `foo a, b as List<String>` then `def f = x -> x`; `foo a, x in /->/` | a call, then the declaration; `foo(a, x in /->/)` | an `ERROR`: the `>` of the type arguments reads as an operator that carries the list over the line break, and after a word the look-ahead opens no slashy string, so each reaches an `->` |
| `foo a bar`, `a b c d`, `foo bar baz qux` | `foo(a).bar`, `a(b).c(d)`, `foo(bar).baz(qux)` | the calls `foo a` and the statement `bar`, the calls `a b` and `c d` (spec §5.5 item 4); before, one statement per name |
| `foo bar 1`, `foo bar baz: 1`, `compile group as: 1, name: 2`, `foo bar new Baz()`, `foo bar(baz 1)`, `foo x.y 'a'`, `foo bar $/a/$` | `foo(bar).1`, `foo.bar(baz: 1)`, `foo.bar(new Baz())`, `foo.bar(baz(1))`, `foo(x.y).a`, `foo(bar).a` | `foo` and the call after it, as before (spec §5.5 item 4) |
| `foo bar x = 1`, `foo bar baz(int a) { }`, `foo bar true` | `foo.bar(x = 1)`, `foo(bar).baz(…)`, `foo(bar).true` | `foo` and the declaration, the method or the call after it, as before (spec §5.5 item 4) |
| `foo var x`, `foo var x, y`, `foo yield x`, `foo record x`, `obj.m var x` | `foo(var).x`, `foo.var(x, y)`, … | the receiver, then `var x` (`var x, y`, `yield x`, …), as before: after a name argument that starts declarations, any name leaves the input as it was |
| `def x = foo bar baz`, `class C { def r = a b c }` | `foo(bar).baz`, a field `a(b).c` | `def x = foo` and the call `bar baz`; the field `r` and a field `c` of type `b`, as before: in a value a further name is a continuation |
| `foo xs[0] 1`, `foo x.y 1` | `foo(xs[0]).1`, `foo(x.y).1` | the call `foo xs[0]` and `1`; `foo` and the call `x.y 1`, as before: the look-ahead does not read past a subscript |
| `foo bar` then `(1)` or `-1` on the next line | a call, then the expression | with `(1)` the same, as the `(` opens the next statement (spec §6.18); with `-1` one call with the argument `bar - 1` (§3); before, `foo` and `bar(1)` or `bar - 1` |
| `foo bar` then `{ }` on the next line | `foo(bar({ }))` | the same; before, `foo` and the chain `bar { }` |
| `return foo bar`, `throw foo bar`, `assert foo bar` | rejected | the statement with `foo`, then `bar`, as before |
| `foo class`, `foo def`, `foo default` | rejected | the same as before |
| `foo var`, `foo yield`, `foo record`, `foo super`, `foo pipeline { }` | a call | `command_chain`; before, two statements, and `foo` and a `pipeline_statement` for the last |
| `obj.m $/x/$`, `def x = foo $/a/$`, `obj.Bar $/a/$` | a call | `command_chain`: a dollar-slashy string is a first argument after any receiver, as a quoted one is (spec §5.7); before, the receiver and the string |
| `java.util.List $/foo/$() { }`, `obj.Bar $/$a/$() { }` | a method `foo`; a call | the same, as for a quoted name (spec §6.14); before, a `method_declaration` for the second |
| `this.new Inner()`, `x.new Inner()` | a creation | the access `x.new` and the call `Inner()`, as before (#23 above) |
| `foo true + 1`, `foo 1 + 2`, `println "a".size()`, `timeOutInMs 30 * 60 * 1000`, `foo "a" { }` | `foo(true + 1)`, `foo(1 + 2)`, `println("a".size())`, the product, `foo("a"({ }))` | the same: the scanner opens the call before the receiver when the argument goes on past a literal (spec §6.14); before, the call of the literal and `+ 1` (`+ 2`), an `ERROR` for the next two and a block for the last |
| `foo 1` then `-x`, `++x`, `(x)`, `[0]`, `bar` or `"b"` on the next line | the call, then the statement | the same, as before: Groovy lets none of them open a line within an expression |
| `foo "a"` then `.trim()`, `?: b`, `* 2`, `{ }`, `as T`, `in b` or `instanceof T` on the next line | one call whose argument goes on | the same; before, an `ERROR`, or the call and statements for the last four |
| `foo "a" + b` then `-c` on the next line | `foo("a" + b)`, then `-c` | one call with the argument `"a" + b - c`, as after a name argument (the `foo bar` then `-1` row) |
| `foo "<a string of 257 characters or more>" + b`, or `.trim()` or `?: c` after the string | `foo("…" + b)`, `foo("…".trim())`, `foo("…" ?: c)` | the call `foo "…"` and `+b`; an `ERROR` for the other two, as at v0.6.0: the scanner reads at most 256 characters of the string (LITERAL_ARGUMENT_LOOKAHEAD_CHARACTERS, spec §6.14), so that a long string, which is most often the whole argument, is not read twice |
| `foo "a"` then `(x) -> x` on the next line | `foo("a"((x) -> x))` | the call and a lambda statement, as before: the `(` opens the next statement |
| `foo "a" (x) -> x`, `foo 1 (x) -> x` | `foo("a"((x) -> x))`, `foo(1((x) -> x))` | the call and a lambda statement, as before: the scanner leaves the literal alone before a lambda's parameters (spec §6.14) |
| `sh "a" { }` or `foo "a" (x)`, then `(x) -> x` on the next line | the lambda one more argument of the call of `a` | an `ERROR`, as in 0.6.0 and as after a name argument (`sh foo { }` then `(x) -> x`): the `(` stays the call's arguments (spec §6.18) |
| `foo 1..3` or `foo 1.abs()` then `++`, `--`, `, b` or `, k: 1` on the next line; `foo 1..3 ~b`, `foo 1..3 !b` | rejected | one call, or the call and `~b` (`!b`), with no `ERROR`, as after a name argument (`foo a..b` then `++`): the scanner reads no further than a number's `.` (spec §6.14); before, an `ERROR` or a missing name |
| `goto "a" + b`, `const 1 + 2`, `threadsafe "a" + b` | rejected: the words are reserved | a call with the sum, as a name receiver takes it: the words are names in this grammar (`goto x`, `def y = goto`); before, the call of the literal and `+b` |
| `foo` then `"a" + b` on the next line, `foo /* c */ "a" + b`, `var "a" + b`, `record "a" + b`, `trait "a" + b`, `sealed "a" + b`, `permits "a" + b`, `super "a" + b`, `_ "a" + b` | `foo`, then the sum; a call with the sum | the call of the literal and `+b`, as before: the scanner reads no literal after a line break or a comment, nor after a word whose own keyword it decides there or a keyword of the grammar |
| `foo "a" = b`, `foo "a" += b`, `foo "a"` then `= b` or `** b` on the next line | rejected | a call whose argument is the assignment or the power, with no `ERROR` |

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
| `def f = (a, b) -> a + b`, `() -> 42`, `(int a, String b = 'x') -> { a }`, `(final x) -> x`, `(@A a, def b, var c, String... d) -> a`, `(List<? extends Number> a) -> a`, `(m = new HashMap<String, Integer>()) -> m`, `(a = "${x.join(")")}") -> a`, `(a = /x,y/) -> a` | a lambda | `lambda_expression`; before, an `ERROR` |
| `x -> x + 1` as a value, an argument (`list.map(x -> x * 2)`), in parentheses or as a map entry's or named argument's value (`foo(k: x -> x)`) | a lambda | the same |
| `return x -> x`, `[x -> x]`, `c ? x -> x : y`, `foo 1, x -> x`, `a -> b -> c`, `x -> x` as a statement | rejected | an `ERROR`, as before |
| `foo (x) -> x`, `list.each (x) -> println x`, `foo()` then `(a) -> a` on the next line | `foo((x) -> x)` | the call `foo(x)` and an `ERROR`, as before: the scanner does not read the arguments of a call (spec §6.17) |
| `if (c) { a() }`, `if (c) { a() } else { b() }`, `while (c) { a() }` or `for (i in l) { a() }` then `(x) -> x` on the next line | the statement and a lambda statement (Groovy 3) | the same (spec §6.17); before, an `ERROR` from scratch and the lambda statement on a reparse that reused the statement |
| `if (c) { a() } (x) -> x` on one line | one `if` statement (Groovy 3) | the `if` and a lambda statement: after the `}` of a body's block the scanner reads a `(` as at a statement's start, on its line too (spec §6.17); before, the `if` and an `ERROR` |
| `foo a: x -> x` | rejected by Groovy 3, a call in 5.0.x and later | an `ERROR`, as before: in a parenthesis-free call's named argument only a lambda with parentheses is taken (`foo a: (x) -> x`). With it, the grammar grew by about 50 states |
| `(foo a) -> a`, `def f = (foo bar) -> 1`, `(x x) -> x`, `foo((foo bar) -> 1)` | a lambda with the parameter type `foo` (`x`), observed with Groovy 3 | an `ERROR`, as before: the parentheses read as the call `foo a` (spec §6.14); a capitalized or dotted type (`(Foo bar) -> 1`, `(foo.Bar b) -> 1`) is a lambda |
| `foo(null -> 1)`, `foo(int -> 1)`, `(int) -> 1`, `(if) -> 1` | rejected | an `ERROR`, as before: a reserved word, a primitive type or `void` names no parameter |
| `(a = X<b, int) -> a`, `(Foo<super> g) -> g` | rejected | a `lambda_expression`, the first with a parameter named `int`: in a default a `<` right after a capitalized name opens type arguments, whose `,` ends no parameter, and in type arguments `super` and `extends` bound a wildcard (spec §6.17); before, an `ERROR` |
| `(_) -> 1`, `_ -> 1`, `var -> var`, `(String this) -> 1` | a lambda | the same |
| `x = $/* c */ -> 1` | a lambda with the parameter `$` | a dollar-slashy string with a `MISSING` `/$`, as before: `$/` always opens a dollar-slashy string here, where Groovy's lexer needs a `/$` that closes it (spec §5.7) |
| `def c = { a = (b) -> c }`, `{ a = b -> c }`, `{ String a = b -> c }`, `{ a = 1 + (b) -> c }`, `{ String a = (T) (b, c) -> d }`, `{ a = (b) -> { } }`, as a value or an argument | the closure parameter `a` with a default | the same, as before |
| `{ @A String a = b -> c }`, `{ @A String a = 1 -> a }`, `{ @A(x) String a, b = 1 -> a }` | the annotated closure parameter `a` with a default | the same, as at v0.6.0: `_closure_parameters_start` is valid after a parameter's annotations too (spec §6.21) |
| `{ final String a, b = (T) (x, y) -> z }`, `{ final String a, b = (x, y) -> (z) -> w }` | the closure parameters `a` and `b`, the second with the default `(T)` called or the lambda `(x, y) -> (z)` | the same: a blank final with several names weighs one step (spec §6.21) |
| `if (x) { a = b -> c }`, `else { a = (b) -> c }`, `while (x) { String a = b -> c }`, `for (…) { def g = t -> 1 }`, `do { … } while (x)`, `case 1 -> { a = b -> c }`, `default -> { … }` | a block that holds the lambda | the same; **before, the closure with the parameter `a`** (the BREAKING class below) |
| `f(x -> { Consumer<String> c = s -> foo(s); c.accept(x) })`, `(x) -> { b = (t) -> 1 }` | a block that holds the lambda | the same |
| `if (x) { a = b -> { p -> p } }`, `while (x) { cb = v -> { -> v } }`, `if (x) { a = b -> { c = d -> e } }`, `f(x -> { a = b -> { c = d -> e } })` | blocks that hold the lambdas | the same, for nests of any depth, each in the block of the one before (before #28, of up to ten lambdas); **before, the closure with the parameter `a`** where such a block held a lambda whose body is a closure or two lambdas, or a lambda's block two lambdas |
| `x -> { a = b -> { p -> p } }`, `if (x) { a = b -> { c = d -> { p -> p } } }`, a nest of more than ten lambdas | blocks that hold the lambdas | the same (#28): a block that ends a lambda outweighs the lambda in it by a step; before, the closure with the parameter `a` (`c`) |
| `{ T a = (U) (b, c) -> (d) -> e }`, `{ T a = (b, c) -> (d) -> e }` | the default `(U)` cast of the lambda `(b, c) -> (d)`, or that lambda, and the body `e` | the same: a parenthesized expression as a lambda's whole body weighs one step below a lambda (`PARENTHESIZED_BODY_WEIGHT`), below the reading of `{ a = (b) -> (d) -> e }` that Groovy has (a row below) and above a typed local that holds two lambdas. With the cast the weights do not decide: its step makes up the parenthesized body's, so the default `(U)` called with `b` and `c` and the body `(d) -> e` weighs the same, and tree-sitter's last resort keeps the cast by comparing symbol numbers; before, the default `(U)` called with `b` and `c` and the body `(d) -> e`, and no parameter and a body that declares `a` |
| `{ T a = (U) (b, c) -> (d) -> (e) -> f }` | the default `(U)` cast of `(b, c) -> (d)` and the body `(e) -> f` | the default `(U)` cast of `(b, c) -> (d) -> (e)` and the body `f`: the two readings weigh the same, and tree-sitter's last resort keeps this one; before, the default `(U)` called with `b` and `c` and the body `(d) -> (e) -> f` |
| `{ a = (String b) -> (d) -> e }` | the default `String(b)`, a call, and the body `(d) -> e` | the default `(String b) -> (d)` and the body `e`: `String b` in parentheses is no call here (spec §6.14); before, no parameter and a body that assigns `a` the lambda |
| `{ a = b -> c }` that opens a statement, `lbl: { a = (b) -> c }`, `case 1: { a = b -> c }` | a block that holds the lambda | the same (#28); before, the closure parameter `a` |
| `{ foo() }` that opens a statement, a labeled statement or a colon case's statement | a block | the same (#28); before, a closure |
| `case (A) -> 1`, `case 1 + (a) -> 1` | a case with the value `(A)` or `1 + (a)` | the same, as before |
| `{ p = a = b -> c }`, `{ a, b = x += y -> z }`, `case x.y = it -> 1` | the parameter with the assignment as its default; the case with the assignment as its value | the closure body `p = (a = (b -> c))`; an `ERROR` for the second and third: the lambda without parentheses is taken after an assignment |
| `{ a = (var b) -> c }`, `case (var a) -> 1` | the default or value `var(b)` | the lambda `(var b) -> c` assigned to `a`; an `ERROR` for the case: the scanner reads `var` before a name as the keyword (spec §6.16) |
| `case (A) -> (b) -> b`, `{ a = (b) -> (d) -> e }` | the value `(A)` and the body `(b) -> b`; the default `(b)` and the body `(d) -> e` | the same; before, an `ERROR` |
| `def c = { v = (A<B> a) -> a }`, `{ v = (String[] a) -> a }`, `{ a = (int a) -> 1 }` | the parameter `v` (`a`) with the default `A<B>(a)`, `String[](a)` or `int(a)`, a call | `v = …` assigned the lambda; before, an `ERROR`: those parentheses are no expression here |
| `def v = a -> { y -> y }()`, `a -> { y -> y }[0]`, `a -> { y -> y } { z }`, `a -> { } + 1`, `(x) -> { y -> y }.call()`, `foo((x) -> { y -> y } + 1)`, `(x) -> { y -> y }` then `.call()` on the next line | the closure that starts the body called, subscripted, given a closure, added | the same (spec §6.19); before, an `ERROR`, two statements, a lambda and a list, a block and `+ 1` as a statement, and the lambda called or added |
| `def v = (x) -> { foo() }.call()`, `x -> { foo() }` then `+ 1`, `(1)` or `<T> T m(T t) { t }` on the next line, `def f = x -> { foo() }` in a class body then `{ z }` or a generic method on the next line | the lambda called; a lambda and a statement, a method or an instance initializer | the same, as before |
| `v = a -> { foo() } == 1`, `v = a -> { } + 1`, `v += a -> { foo() }` then `!= x` on the next line | the assignment compared or added: Groovy continues an assignment past a lambda without parentheses | the closure that starts the body compared or added, as in a declaration (`def v = a -> { foo() } == 1`); before, the same for the first, a block and the statement `+1` for the second, and the assignment compared for the third, as Groovy reads it |
| `def v = (a) -> { b = (c) -> { } }.call()`, `(x) -> { T t = (u) -> v }.call()` | the lambda's block, then the lambda called | the closure with the parameter `b` (`t`) called in the body; before, the block. The closure reading holds a lambda less; a weight on the lambda's block that outranked it also made `def c = { a = (b) -> { }.call() }` assign a lambda (spec §5.16) |
| `def v = a -> { y -> y }` then `{ z }` on the next line; `def v = a -> { foo() }` then `as T` or `in l` on the next line | the closure given `{ z }`; the block and a statement `as(T)` or `in(l)` | the lambda and a block (a closure statement before #28); the closure cast or tested, as before. Groovy's reading depends on whether the braces parse as a block, which the scanner does not know |
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
  position of a `{`, 2,439 changed so. The block's weight beat the
  closure reading only for nests of up to ten lambdas: one more lambda
  in the block cost the block's reading `LAMBDA_WEIGHT` and the closure
  reading nothing, and a lambda's block had no weight to spend, until a
  lambda's block got the weight of #28. When
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

Of 116,000 mutations of the corpus, real and Java inputs, the fourth
kind accounts for the only 2 changed trees that had no `ERROR`.

Two limits are known (spec §6.17). Nested lambdas whose bodies are
expressions, `x = (a) -> (a) -> … a`, parse in time quadratic in the
nesting: 1.1 seconds at 4,000 levels and 28.6 at 20,000 (140 KB),
where block bodies take a fraction of that; a cast's operand
(`(T) (a) -> …`), a sum, a ternary's branch and nests in a closure
parameter's default, a closure's first statement and a case value do
the same. Nests with block bodies, in arguments and of closures are
linear, and real code nests a few lambdas. A grammar fix that left
some of the nests quadratic was not taken. And a reparse can read a
closure parameter's default otherwise than a parse from scratch:
typing `{` before `T a = (b, c) -> (d) -> e }` reuses the lambda
`(d) -> e`, so the closure's body declares `a`, where a parse from
scratch reads the parameter `T a` with the default `(b, c) -> (d)`, as
Groovy 3 does; neither tree has an `ERROR`
(`bindings/rust/tests/incremental_reparse.rs` pins both).

**Action**: the call forms in the fourth row would need the scanner to
read every call's arguments; measured on code made of calls, the
reading the scanner does now costs about 1.5% more instructions.

A `{` that opens a statement, a labeled statement or a colon case's
statement is a block now, and a block that ends a lambda weighs a step
more than the lambda in it; see #28 below.

---

## 27. Typed local declarations without an initializer

**Apache `GroovyParser.g4`** (Groovy 3 to 6): a statement is a
`localVariableDeclaration` unless
`SemanticPredicates.isInvalidLocalVariableDeclaration` turns it down
(#25 above), and a declarator needs no initializer, so `String s` and
`int a, b` declare. Where the declaration does not parse, the statement
is read as a command expression (`String s` then `.trim()` is
`String(s.trim())`).

**Current implementation**: the scanner emits an empty token before the
type when a declared name follows by the same rule and the declaration
ends on its line, or the next line does not go on with it, or a list of
names or an initializer follows (spec §6.21). The token opens a form
of `local_variable_declaration` with the fields of the typed form with
an initializer: one `type`, a `name` per variable, and a `value` after
the name it belongs to. The same reading of the statement's first name
decides `_command_statement_start` (spec §6.14), so a statement that
starts with two names is a call or a declaration, never both.

The Groovy column was observed by running Groovy 3.

| Input | Groovy | This grammar |
|---|---|---|
| `String s`, `int i;`, `List<String> xs`, `String[] a`, `foo.Bar q`, `java.util.List<String> l` | a declaration | a `local_variable_declaration`; before, two statements, a field access and a name, comparisons or an `ERROR` |
| `String a, b`, `String a, b = 1`, `String a = 1, b`, `final String a, b`, `@Field int a, b`, `String a,` then `b` | declarations | one `local_variable_declaration` with a `name` for each; before, an `ERROR` |
| `foo<T> x`, `foo[] x`, `foo [] x`, `a<b>c`, `foo.bar.Baz q`, `Été x` | a declaration | the same; before, an `ERROR` or two statements |
| `a < b > c`, `List <String> xs` | a declaration | comparisons, as before: the grammar's type arguments follow the name directly |
| `foo bar`, `foo.bar x`, `foo.Bar.baz q`, `println Foo`, `assertEquals Expected, actual` | a call | a `command_chain`, as before |
| `void x`, `String this`, `String int` | `void(x)`, `String(this)`, `String(int)` | two statements, as before |
| `String s; x`, `{ String s }`, `String s // c`, `String s /* c */`, `if (c) String s else String t`, `while (c) String s` | a declaration | the same; before, two statements, the last with `else` read as a name |
| `String s` then `x`, `(a, b) = l`, `{ }`, `[1]`, `+ 1`, `!x` or `/x/` on the next line | the declaration, then a statement | the same; before, `String` and the line read with `s` (`s { }`, `s + 1`, the call `s /x/`) or after it |
| `String s` then `(a) { }` | the declaration and a cast | the same; before, the method `s` |
| `String s` then `(foo 1)`; `Foo f` then `(foo bar) { }` | the declaration and a call | the same: a call in parentheses on the next line opens a statement of its own; before, the method `s` with an `ERROR`, and the method `f` |
| `int total` then `(MIN..MAX).each { }`; `List<String> names` then `(ITEMS as List).each { }`; `Foo foo` then `(/x/).each { }` | the declaration and a call | the same: those parentheses hold no parameters (spec §6.14); before, `int` and the call `total(MIN..MAX).each { }`, the method `names` with an `ERROR`, and `Foo` and the call `foo(/x/).each { }` |
| `Foo f` then `(int a) { }` | the declaration and the call `int(a, { })` | the method `f`, as before: parameters on the next line go on with the name |
| `String s` then `(int x) -> x + 1` | the declaration and a lambda statement | the method head `String s(int x)` and an `ERROR` at `->`, as before: typed parameters on the next line go on with the name, and the look-ahead does not read on to the `->` (an untyped `(x) -> x + 1` is the declaration and the lambda) |
| `foo()` then `in.gov.Foo x`, `as.b.Foo<T> y` or `in<T> q` on the next line | the call and a declaration | the same: `.` and `<` start no operand of `in` or `as`, so the word is read on as a type's first name (spec §6.16); before, an `ERROR` |
| `String s` then `(Foo new Bar())` | the declaration and the call `Foo(new Bar())` | the same: a word that names no variable is no parameter's name (spec §6.21); before, `String`, `s` and the parenthesis as three statements |
| `String s` then `<T> T m(T t) { }`; in a type body `def String s` then `<T> T m(T t) { }` | the declaration, or the field, and a generic method | the same: a `<` that opens type parameters starts the next statement (spec §6.21); before, an `ERROR`, as at v0.6.0 |
| `if (x) { Runnable a, b = y -> z }`, the braces of a body whose first statement declares several names, a later one with a lambda | a block that declares `a` and `b` | the closure with the parameters `Runnable a` and `b = y` as the body, as before: where closure parameters may start, a list that `->` ends is theirs, and without the scanner's token only a declaration whose first name has a value parses (`if (x) { Runnable a = y -> z }` is the block). `def c = { Runnable a, b = () -> z }` has a `MISSING` `->` |
| `String s` then `= 1` | a declaration with the value | the same, as before |
| `String s` then `.trim()`, `?.x()`, `* 2`, `== 1`, `instanceof T` or `/ 2` | `String(s.trim())`, … | `String` and `s.trim()`, …, as before |
| `String s` then `/= 2 // c` | the declaration and the slashy string `/= 2 /` divided by `c` | `String` and `s /= 2`, as before: the scanner starts no slashy string with `=` |
| `String s` then `in xs` or `as T` | the declaration and `in(xs)`, `as(T)` | the declaration and then `in` and `xs`, or `as` and `T`, as statements; before, `String` and `s in xs`, `s as T` |
| `InputStream in` then `open()` or `in.close()` | the declaration and a call | the same; before, `InputStream in open()` and `InputStream in in.close()` |
| `Foo<<Baz>> result`, `this.Foo x` | `(Foo << Baz) >> result`, `this.Foo(x)` | the same; the field access and the name, as before |
| `Foo f =~ /x/` | `Foo(f =~ /x/)` | the declaration of `f` with the value `~/x/`, as before |
| `String /* c */ s`, `int/*unsigned*/ mask;`, `String /* a` then `b */ s` | a declaration | the same; before, two statements and the comment |
| `foo /* c */ bar`, `Foo /* c */ new Bar()` | a call | two statements and the comment, as before (spec §6.14) |
| `Foo /* c */ x.y`, `foo.bar /* c */ x` | `Foo(x.y)`, `foo.bar(x)` (Groovy 3) | two statements and the comment, as at v0.6.0: a comment between the receiver and the first argument is not read past (spec §6.14) |
| `List</* c */String> l`, `Map<K,` then `V> m` on the next line | declarations | an `ERROR`: the look-ahead reads type arguments on one line, without comments |
| `{ String a, b -> }`, `{ String a, b = 1 -> }`, `{ String a` then `-> }`, `{ String a, b -> }` with `b` on the next line | closure parameters | the same, as before |
| `{ String a, b }`, `{ String a = 1, b }` | declarations | the same; before, closure parameters with a missing `->` |
| `for (int i; i < n; i++)`, `for (int i, j; …)` | a declaration as the init | the same; before, an `ERROR` |
| `for (String s : xs)`, `for (String s` then `: xs)`, `for (String s in xs)` | a for-in loop | the same, as before |
| `return` then `String s` on the next line | `return`, then the declaration | the same (spec §6.23); before, `return String` and `s`, as `return`, `break` and `continue` took the token |
| `return String s`; `return java.util.List<String> s` | rejected; `return` with comparisons | `return String` and `s`; `return` with comparisons, as before |
| `foo() String s`, `x = 1 Foo bar`, `move left By steps`, `Foo T name` | `foo().String(s)`, `x = 1(Foo).bar`, `move(left).By(steps)`, `Foo(T).name` | the statement and the declaration; before, three statements, or `move left` and two |
| `final x` then `String s` or `String s = 'a'`; `String s = y -> z`, `Runnable r = () -> { }` | two declarations | the same; before, the blank final `final x String` and `s` or `s = 'a'`; a lambda value kept that reading until the scanner told closure parameters apart (spec §6.21) |
| `if (c) Foo f` then `as T` then `else { }` | `Foo(f as T)` in the `if`, as the declaration does not parse there | the declaration, `as`, `T` and the call `else { }`; before, `Foo`, the cast and the call |
| `def String s`, `def String s = 'a'`, `final def List<String> xs = []`, `private def Foo foo = mock(Foo)` in a type body | a declaration of `s` with the type `String`, …, a field | the same, as `def` before the type opens the declaration (spec §6.21); before, the declaration of `String` and the statement `s` (`s = 'a'`, an `ERROR` for the third), and an `ERROR` in a type body |
| `def String` then `s = 1` on the next line; `def` then `String s = 1` | `def String`, then the assignment; one declaration | the same |
| `def final String x = 1`, `def static String x = 1` and `def static next() { }` in a type body, `def @A x = 1`, `def @A foo() { }`, `def f(def @A x)`, `for (def @A x in xs)`, `var @A x = 1` | a declaration of `x`; a field `x`; a method `next`; …, as `def` and `var` are modifiers and modifiers and annotations come in any order | the same; before, `def final` and the declaration of `x`, the fields `def static` and `String x = 1`, the field `def static` and a constructor `next() { }`, an `ERROR` for each annotation after `def`, and the statement `var` and a local `x` |
| `def foo bar`, `def String m() { }` | a declaration of `bar`; a method `m` | `def foo` and `bar`; `def String` and the call `m() { }`, or a field and a constructor in a type body, as before |
| `var String s = 'a'`, `var String t`, `final var List<String> xs = []` | a declaration of `s` (`t`, `xs`) with the type, as `var` is a modifier like `def` (Apache's `modifier` lists `VAR`) | the same; before, `var String` and the statement `s = 'a'` (`t`), with no `ERROR` |
| `var String` then `v = 1` on the next line | `var String`, then the assignment | the same |
| `def <T> T m() { }`, also in a type body (`def <T> T unwrap(Class<T> c)`) | a method with type parameters (Groovy 3) | an `ERROR` at `def` and a method without it, as at v0.6.0: the method's head takes type parameters after modifiers but not after `def`. The two files of 266 real ones that have an `ERROR` both hold this form. Not fixed here, as an open pull request edits the same rule |
| `for (var String x : xs)`, `for (def String x : xs)`, `try (var String r = open())`, `try (def String r = open())` | a loop variable, a resource, with the type (Groovy 3) | an `ERROR` at the type in each, as at v0.6.0, where the `for` with `var` was one `ERROR` for the whole loop: `def` and `var` take a type only where a statement or a member starts (spec §6.21) |
| `@A foo bar` | a declaration | an `ERROR`, as before: after annotations a lowercase type needs `<` or `[` |
| `try (String s) { }` | a resource declaration | an `ERROR`, as before |
| `static String s`, `private String s` | a declaration | an `ERROR`, as before |
| `"${String s}"` | a closure that declares `s`, called | an `ERROR`, as before |
| `class C { String s; int a, b }` | fields | `field_declaration`s, as before |
| `String f()`, `String f() { }` | rejected; a method | `String` and the call `f()`; a method, as before |
| `Foo a, 1`, `Foo a, b()`, `String a, b c` | `Foo(a, 1)`, `Foo(a, b())`, `String(a, b).c` | an `ERROR`, as before |
| `Integer _` then `{ }, b` | `Integer(_({ }), b)` | the declaration, a block and an `ERROR` (the closure before #28); before, `Integer` and the call `_ { }, b` |
| `do long in` then `[1] while (c)` | a `do` loop whose body is `long in [1]` | the declaration in an `ERROR`; before, the membership test |

The rows with "before" changed trees that had no `ERROR`; spec §6.21 lists
the kinds and how each was checked. Where Groovy declares only because the
rest parses (the `Integer _` and `do` rows), this grammar has no such
fallback, and of 394,000 fuzzed inputs 10 that Groovy accepts went from
no `ERROR` to one (spec §6.21).

**Action**: the forms Groovy takes that stay as before need the
statement after another on its line to continue it (#25 above), a
lowercase type after
annotations, a resource without an initializer, and type arguments
across lines or after a space, each of which the scanner could read at
more cost.

---

## 28. Blocks that open a statement

**Apache `GroovyParser.g4`** (Groovy 3 to 6): `statement` lists `block`
first, and its alternatives are decided by ANTLR's prediction, which
takes the first one of those that match. A `{ … }` that opens a
statement is a block where the statement can end at its `}`, before a
separator, the `}` of the list or the end of the input, and a closure
where an expression goes on: `{ }()`, `{ }.call()` and `{ }` followed by
`.call()` on the next line call it, as `pathElement` takes `nls` before a
`.`, while `{ }` followed by `(1)`, `[0]`, `{ }` or `+ 1` on the next line
is a block and a statement. Groovy 2's parser, which Groovy 3 still runs
with `-Dgroovy.antlr4=false`, rejects an unlabeled block there
("Ambiguous expression could be either a parameterless closure
expression or an isolated open code block … giving it a label, e.g.
L:{...}"); the default parser of Groovy 3 accepts it through class
generation.

**Current implementation**: spec §6.22. The lists of statements and a
label take a block that an empty scanner token, `_statement_block_end`,
ends; where the scanner does not emit it, the braces are a closure, as
they were in every statement position before. The bodies of `if`,
`else`, the loops and an arrow case keep taking their block directly.

The Groovy column was observed by running Groovy 3.

| Input | Groovy | This grammar |
|---|---|---|
| `{ }`, `{ a }`, `{ it }`, `{ int x = 1; x }`, `{ { b } }` that open a statement, in a script, a method, a closure or a colon case, and after a label (`lbl: { … }`, also on the next line) | a block | the same; before, a closure statement |
| `{ a: 1 }` that opens a statement | a block that holds the labeled statement `a: 1` | the same; before, a closure that holds it |
| `{ a = b -> c }`, `{ a = b -> { c = d -> e } }` that open a statement | blocks that hold the lambdas | the same; before, closures with the parameters `a` and `c` |
| `{ }` then `(1)`, `[0]`, `+ 1`, `- 1` or `foo()` on the next line; `{ }; foo()`; `{ } // c` | a block, then a statement | the same; before, a closure statement, the closure minus or plus one for `- 1` and `+ 1` |
| `{ a }` then `{ b }` on the next line | two blocks | the same; before, the closure `{ a }` called with `{ b }` |
| `{ }()`, `{ }.call()`, `{ } as Runnable`, `{ } + 1`, `{ }[0]`, `{ } { b }`, `{ }` then `.call()`, `* 2` or `?.x` on the next line | the closure called, cast, added, subscripted, given a closure, multiplied | the same, as before |
| `{ a -> }`, `{ -> 1 }`, `{ a, b -> a }` that open a statement | a closure | the same, as before |
| `foo()`, `foo` or `x = 1` then `{ a }` on the next line | `foo({ a })`, `x = 1({ a })` | the same, as before (#11 above) |
| `{ } foo`, `{ } 1`, `{ } !x`, `{ } ~x`, `{ } new A()` | the closure called with `foo`, `1`, `!x`, `~x`, `new A()` | a block and a statement; before, a closure and a statement |
| `if (a) { } [1].each { }`, a `[` on the line of an `if`'s block | the body `{ }[1].each { }`, a closure subscripted | the block and the statement `[1].each { }`, as in 0.6.0 |
| `foo { a }` then `{ b }` on the next line, `foo { a } { b }` | `{ b }` one more argument of the call | a block; before, a closure statement |
| `println 1` then `{ b }` on the next line | `println(1({ b }))`, the literal called with the closure | the same, as the literal argument goes on (spec §6.14); before, a closure statement |
| `def c = { a = (b) -> (d) -> e` then two or more lines of `{ z }` and `}`; `{ a = b -> c` then two or more lines of `{ z }` and `}` | the parameter `a = (b)` and the lambda `(d) -> e` called with each `{ z }`; a block that holds `a = (b -> c({ z }, …))` | the parameter `a` with the default `(b) -> (d)`, the call `e { z }` and blocks; the closure with the parameter `a = b`, the call `c { z }` and blocks. Before, the same readings with closure statements for the blocks in the second, and Groovy's reading in the first: each block weighs a step, the call `e { z }` takes no second closure (the row above), and the readings differ by one step (spec §5.16) |
| `{ a = b -> { p -> p } }` that opens a statement, also after a label and in a colon case | a block that holds the lambda | the closure with the parameter `a`, as before: a statement block's step does not make up a lambda whose body is a closure (spec §5.16) |
| `def c = { a = b -> { p -> p }` or `if (x) { a = (b) -> { }` then eight lines of `{ z }` and `}` | the parameter `a` and the closure called with each `{ z }`; the block of the `if` and the lambda called with each `{ z }` | the same, up to 75 such lines (`LAMBDA_NESTING_DEPTH` 100); at a depth of 10 eight lines read the lambda and a block per line, or a closure for the `if`'s body |
| `{ }` then `as T` or `in l` on the next line | a block and `as(T)`, `in(l)` | the closure cast or tested, as before |
| `return` then `{ }` on the next line | `return`, then a block | the same (spec §6.23); before, `return { }`, as a line break was whitespace |
| `{ }` then `()`, `(a, b)` or `++` on the next line | rejected | a block and an `ERROR`; before, the closure called or incremented |
| `{ }` then nine comments and `.call()` | the closure called | a block, then an `ERROR`: the scanner reads at most eight comments after the `}` (spec §6.19); before, the closure called |
| `def c = { a = a -> { foo() }` then `+ 1 }` on the next line | the parameter `a`, a block and the statement `+1` | the same; before, the parameter `a` and the closure plus one |

The rows with "before" changed trees that had no `ERROR` (BREAKING); in
the rows of `()` and of nine comments the new tree has one, and in the
second Groovy has none.
Measured on 8,448 inputs of brace shapes in statement positions
(spec §6.22): 1,360 trees changed, 1,080 to Groovy's reading and 280 from
a reading Groovy does not have to another (the rows of `{ } foo` and of a
brace after a command); no input of that set that read as Groovy does
reads otherwise. Outside it some do: the two lambda templates below, and
of 5,600 inputs of a lambda before lines of braces, calls or closures, 12,
each the parameter `a = (b)` before `(d) -> e` and lines of braces (the
row above); with the depth at 10 it was 116.
The 11 corpus trees and 152 of the 24,548 Java files that changed with no
`ERROR` are each a brace that opens a statement, now a block, two of them
on consecutive lines included; no real Groovy file changes. Of 231,760
mutations, 41 gained an `ERROR` (40 with the depth at 100), each the
`()`, `(a, b)` or `++` row above, which Groovy rejects.

A block that ends a lambda's brace body weighs as much as one that opens
a statement, and the lambda in it a step less, so the nests of #26 read
as blocks at any depth, but for the row of `{ a = b -> { p -> p } }` above:
`if (x) { a = b -> { c = d -> { p -> p } } }`,
`x -> { a = b -> { p -> p } }` and nests of more than ten lambdas, which
read as closures with parameters before (the rows of #26 that name
#28), are Groovy's blocks now. Of the 61,246 lambda template inputs of
spec §6.19, 5,516 without an `ERROR` changed. Groovy 3 rejects 1,247 of
them; of the 4,269 it accepts, the new trees have its closures and
lambdas in 3,748 and the old ones in 2:
`l: { def f = x -> def a = b -> { p -> p } }` and
`{ def f = (x) -> def a = b -> { p -> p } }` before `foo()`, where Groovy
takes `def f = x` for a closure parameter, as the block fails at
`x -> def`, and this grammar reads a block that holds a lambda whose body
is the name `def`. The other 519 match in neither; 3 of them
hold a closure in a method parameter's default, which the comparison does
not see in Groovy's tree.

Two JDK test files whose class holds fields with Java array
initializers, one empty and the others with a trailing comma
(`String[] n = { };` then `OpenType[] t = { SimpleType.LONG,
SimpleType.STRING, };`), had an `ERROR` at each comma in the class and
are one `ERROR` for the whole file now; no variant of the grammar or its
weights tried avoids it.

**Action**: the rows that read otherwise than Groovy with no
`ERROR` would need the scanner to know the call before the brace (a
command's closure argument) or whether the braces parse as a block
(`as`, `in`), which it does not read. A command with a closure that took
the closures after it, as Groovy does, was tried for the call row and its
step-only consequence: it read `foo { a }` then `{ }()`, `{ }.x` or
`{ } * 2` on the next line as an `ERROR`, as a call without parentheses
cannot be continued, in 646 of 66,076 mutated inputs. The lambda whose
body is a closure would need a block weight above the demoted lambda's,
which would add up over the blocks of a file. The row of nine comments is the
bound of spec §6.19; a closure past the bound would cost 31 bytes read
per byte of input on runs of 255 comments, against 1.2 (spec §6.22).

---

## 29. Parse time on input this grammar rejects

Four shapes of input that parse with an `ERROR` take time above linear
in their length, where v0.6.0 was linear. Each appeared with a feature
of this release. The times are the minimum of three parses, without a
logger, by the runtime of `tree-sitter` 0.27.2; the counts are from its
log.

| Input | Groovy 3 | Here | v0.6.0 | From |
|---|---|---|---|---|
| `foo 1, %a = ` repeated, then `1` | rejected | 71, 416 and 2,970 ms at 100, 200 and 400 levels; reductions grow with the square of the levels and stack versions linearly (213 at 100) | 0.6, 1.2 and 2.4 ms; 4 versions | a call as a value (849c0ab): each level keeps a version alive past the `ERROR` of its argument |
| `@C(, ` repeated | rejected | 214, 845 and 3,625 ms at 1,000, 2,000 and 4,000; reductions and log messages linear | 2.0, 4.1 and 8.3 ms | annotations as annotation values (d23d53f): recovery inserts a missing name, which nests each `@C(` in the one before, and the runtime rebuilds the growing `ERROR` node, work its log does not show |
| `foo(case, ` repeated | rejected | 177, 694 and 2,892 ms at 1,000, 2,000 and 4,000; reductions linear (2,997 at 1,000), at most 5 versions | 0.55, 1.09 and 2.16 ms | the reserved `case` (36aaef8): no argument starts with it, and the run costs what `foo(], ` repeated costs, here and at v0.6.0 (below) |
| `x ? y :`, a line of `case`, repeated | rejected | 529, 2,493 and 10,694 ms at 1,000, 2,000 and 4,000; about n²/2 reductions (431,993 at 1,000), at most 4 versions | 0.94, 1.95 and 3.82 ms, without an `ERROR` (`case` was a name) | the reserved `case` (36aaef8): no ternary's alternative starts with it |

Three more shapes were in this table. `foo 1, *a = ` repeated, which
Groovy reads as `foo(1, *(a = foo(…)))`, cost as much as the first row
while a command chain took no spread argument after a comma. It takes
one now, and that nest parses without an `ERROR` in one stack version:
0.13, 0.26 and 0.49 ms at 100, 200 and 400 levels. `case 1:` lines
outside a `switch`, which Groovy rejects, took 237, 963 and 3,873 ms at
1,000, 2,000 and 4,000 lines (about n²/2 reductions) after the
`default` modifier (fa9986c, §20): while `case` was a name, one version
nested the lines as labels and closed the whole nest at each line.
Reserving `case` (36aaef8), as in Groovy, made those lines linear but
not `case A:` lines: recovery skipped the reserved word, read `A:` as
the label, and the same nest formed (743, 3,013 and 11,890 ms), as it
did for `x = case` lines (554, 2,246 and 9,291 ms), `case A: // c`,
`case A: ;`, `if (x)` or `A:` before `case B:`, and `case A:` lines
under a mistyped `switch` header (`swit (x) {`: 724, 2,910 and
12,160 ms). A `case` label where a statement starts is now an error
production that recovery completes with a missing `switch` (spec §5.2),
so each line recovers on its own: 3.1, 6.3 and 12.3 ms for `case A:`
lines, 3.0, 6.1 and 12.4 ms for `case 1:` lines (5.9, 11.5 and 23.3 ms
at v0.6.0), 4.2, 8.5 and 16.6 ms for `x = case` lines and 3.2, 6.3 and
13.0 ms under the mistyped header. In a switch body the lines cost what
they did: the reductions of `case A:` and `case 1:` lines there are the
same before and after the production at 1,000, 2,000 and 4,000 lines.

The production does not reach a `case` where an expression goes on,
which leaves the third and fourth rows of the table. Runs of the same
kind are quadratic at v0.6.0 as well, and unchanged here: `A:` followed
by a line of `}`, repeated (727, 2,939 and 12,224 ms at 1,000, 2,000 and
4,000 repeats here; 760, 3,045 and 11,829 ms at v0.6.0), `x =` followed
by a line of `}`, repeated (543, 2,121 and 8,865 ms; 543, 2,174 and
8,789 ms), and `foo(], ` repeated (166, 653 and 2,721 ms; 168, 651 and
2,757 ms).

A run of `, b) =` after the `=` of a multi-assignment declaration was a
shape of this table as well: `def (a, b) =`, then `, b) =` repeated,
took 224, 1,013 and 4,421 ms at 100, 200 and 400 repeats (0.19, 0.38
and 0.75 ms at v0.6.0), with about n²/2 reductions (4,957 at 100), once
the forms of the declaration (6f0ae10) had made its names a rule of
their own. The parser reduced that rule before the `=`, so at the `,`
recovery could no longer go back into the list, as it did at v0.6.0.
The parentheses, the `=` and the value stand in one rule now, which
keeps the list on the stack until the value ends: the run takes 0.21,
0.43 and 0.82 ms, with 5 reductions a repeat (506 at 100) in at most 4
versions, 2 of them the list's and 3 those of a version that ends the
declaration at each `=,` (below) and is dropped at the `)`. So it does
after `final` or `var`, with typed names or a rest
binding, in a type body, as a `for` init, and after an annotation
(`@A def (a, b) =`), where v0.6.0 took 217, 995 and 4,378 ms (4,864
reductions at 100). No tree without an `ERROR` changes, of the corpus
or of the 118 real files at hand, and the real files take the same
instructions.

That left the run after a list of keyed pairs (764de3b), which took no
plain name, so `, b` did not continue it: `def (k: a, j: b) =`, then
`, b) =` repeated, took 229, 1,035 and 4,276 ms (0.20, 0.39 and 0.76 ms
at v0.6.0, where each pair was a name whose key was an `ERROR`). One
list takes names and pairs alike now, and the run takes 0.21, 0.41 and
0.81 ms. Groovy's `typeNamePairs` holds names or pairs, not both, so
`def (name: n, b) = m` and `def (name: n, *rest) = m`, which Groovy
rejects, parse with no `ERROR`; no other tree of the corpus that had
none changes.
An error production for a plain name among pairs, which would keep the
`ERROR`, was tried: recovery by a missing token costs what skipping
does, and the run stayed quadratic.

With the list on the stack, a declaration whose `=` does not come lost
its recovery: `def (a, b)` then `foo()` on the next line, a declaration
while it is typed, was one `ERROR` over every statement after it, as at
v0.6.0. The names as a rule of their own had let recovery wrap that one
node and go on. An error production takes the case now. Its names are
such a rule, and after it a token that matches no input stands for the
value; the parser reduces the rule only in recovery, which tries every
reduction. Where the token after the `)` can start a statement,
recovery completes the declaration with a missing `identifier`, and the
statements after it stay statements (`def (a, b)` then `foo()` is the
declaration and the call; in a type body, `private (e, f)` then a
method is the declaration and the method). Where it cannot
(`def (a, b) : l`, `def (a, b).x`, `def (a, b) -> l`), the runtime
tries no missing token while another stack version can go on, and
recovery wraps the names in an `ERROR` and reads on after `def`, as it
did while they were a rule before the `=`: `def (a, b) : l` then
`foo()` is a declaration of `l` with that `ERROR`, and the call.

One more recovery was lost with the list on the stack. Recovery from
`def (a, b) =,` went back into the list at the `,`, as the run above
needs, and took the next line as names (`def c = 1`, or `class D { }`,
where a keyword before a name is its type), which left both lines an
`ERROR`. The error productions take a `,` after them, and a second one
is a declaration whose `=` comes and whose value does not:
`def (a, b) =,` then `def c = 1` is the declaration with a missing
value and the declaration of `c`, and so is `def (a, b),` before it.
On one line the list still goes on (`def (a, b) =, c) = 1` declares
three names), as that version is dropped at the `)` after the next
name. A `,` after a value stays an `ERROR`.

The cost was measured on 1,248 texts, 26 unfinished or wrong
declarations before 8 continuations in 6 places, by the bytes under
`ERROR` nodes. Against the names as a rule before the `=`, 270 texts
have fewer and 74 more (245 and 27 by more than two bytes); against
v0.6.0, 565 and 126 (403 and 107). The 27 are:

- 12 lists that are not closed before a line with an `=` (`def (a,`
  then `def c = 1`). With the names as a rule the runtime inserted a
  missing `)` there, and the two lines were one declaration of `a` and
  `c`; they are one `ERROR` and the statement `1`. The runtime inserts
  a missing token only where the next token then reduces a rule, and
  after a `)` in the rule of the value the `=` is shifted. A rule for
  the `)` gave the missing `)` back and was not kept: it made `for`
  headers with such a list one `ERROR` with the statement after them
  (`for(def(a,b b=`, then `x)o();;){}`, then `bar()`; 22 of 25,500
  texts of another probe got a larger `ERROR`), and a reparse reused a
  declaration with a missing `)` where a parse from scratch found a
  cheaper recovery.
- 14 `for` headers with a line break after an unfinished declaration
  (`for (def (a, b)`, then `foo(); ;) { }`), and a `,` after the value
  in a type body before lines that are no members. In each a later `)`
  or `,` goes on with the list at less cost than the recovery that
  keeps the statement, and the runtime drops the dearer one before the
  cheaper one fails. A rule of the names before the `=` has no list to
  go back into; that is the quadratic run above.

The names are a rule of their own inside the parentheses, so that a
list of one name and a list of more end in one parse state. Written
out where they are used, a name and a repeat make two rules, `(a)` and
`(a, b…)`, with a state each. Two recoveries of one cost from an error
in the list (`def (a::, b) = l` is the name `a` before two skipped
tokens, and the key `a` before two others) then met only after the
value, at the first token of the next statement, and a reparse that
reused that statement kept the other recovery, at every later edit.
With one state they meet at the `)`, where the parser keeps one. Over
the single edits of 244 texts with a wrong multi-assignment between
two statements (each offset, 0 to 3 bytes removed, one of 10
insertions), 1,441 reparses differed from a parse from scratch with the
names written out; 7 do now, and 5 did with the names before the `=`.
Over those of 3,168 texts (11 lists with 4 ends, 8 tokens in place of
the `=`, 3 values and 3 next lines), 2 do, as before.

Once the rule is reduced, a `,` cannot go on with it, so the rules that
use it take names after it, which no text without an error reaches: the
rule takes every `,` after a name (`prec.right`). They are what the run
above goes on with. For a few commits the rule was left-recursive
instead (`list , name`), which takes a `,` after it by itself. It
nested a node for each name, where the runtime balances the nodes of a
repeat, and `ts_node_child`, `ts_node_next_sibling` and
`ts_tree_get_changed_ranges` took time quadratic in the names: 2 s to
walk the siblings of 8,000 names and 35 s for the changed ranges of
100,000 after an edit, against 0.02 s each.
`bindings/rust/tests/multi_assignment_comma_cost.rs` reads the shape of
the rule from the parser's log.

The rule costs a reparse difference of its own. Its node is one that a
reparse reuses as a whole, also when it holds an `ERROR` and the parser
recovers from another error typed before it, where a parse from scratch
may find a cheaper recovery. In `def(a,l(a,b)` the `(` after `l` is an
`ERROR` in a typed name, `l a`; with `,` typed after `def` the reparse
keeps the list with that name, and a parse from scratch has one `ERROR`
over the comma and the names up to `l`. With the left-recursive list
the runtime reused none of the list's nodes after an error in it (its
log names them fragile), so the reparse recovered as a parse from
scratch does. Over the single edits of 232 texts with an error inside
the list, 238 reparses differ, 18 with the left-recursive list and none
at v0.6.0; each of the 220 more types `,`, `:`, `=` or `)` between the
keyword and the `(`. `bindings/rust/tests/incremental_reparse.rs` pins
the first.

One tie is left, between the error production and skipping. A word of
ten characters before the `=` (`def (a, b) abcdefghij = l`) costs as
much to skip as the missing value does. The production weighs half a
step less (`MISSING_VALUE_WEIGHT` in `grammar.js`), so the word is
skipped, from scratch and on reparse: of the 1,998 single edits of
`x = foo new Foo()`, `def (a, b) abcdefghij = f new Pair()` and
`def s = 1`, 773 reparsed otherwise without the weight and 1 does with
it. Both recoveries are alive until the statement after the line, and
while two stack versions are alive the runtime does not break down a
node that it reused when the token after it has changed. So where the
value is a parenthesis-free call, an edit right after that statement
can reparse otherwise: ` instanceof ` typed after `def s = 1` goes on
with the value from scratch and is a statement of its own on reparse.
`bindings/rust/tests/incremental_reparse.rs` pins it.

The rules take 24 parse states more than the names as a rule before
the `=` did (STATE_COUNT 4,859 to 4,883, LARGE_STATE_COUNT 1,860 to
1,865, `parser.c` 9.97 to 10.00 MB), 18 of them the error
productions'.

Where no such list takes it, that run is quadratic at v0.6.0 as well:
`, q) =` repeated with nothing before it takes 222, 1,043 and
4,395 ms at 100, 200 and 400 repeats (223, 1,035 and 4,509 ms at
v0.6.0), and after `x =` as much. After the names of a multi-assignment
without `def` the run went the other way: `(a, b) =`, then `, b) =`
repeated, took 233, 1,072 and 4,379 ms at v0.6.0 and takes 0.32, 0.63
and 1.24 ms, as `(a, b) = …` is a `multi_assignment_expression` now,
whose list recovery goes back into.

**Action**: none planned. `bindings/rust/tests/error_recovery_cost.rs`
pins today's reductions and stack versions of each row, so that they
grow no further, and bounds those of the spread nest, of the run after
a declaration's list of names and of the case lines linearly
(`case 1:`, `case A:` with and without a comment or an
empty statement after it, and `x = case`, alone and under a mistyped
header, with the steps of the stack versions as well, as recovery that
inserts a missing token logs none of its reductions); for the second
and third rows those counts are linear and their time is not pinned.

---

## 30. A call with a closure that a member access follows

**Apache `GroovyParser.g4`** (4.0.x to 6.0.x): `pathExpression` is
`primary (pathElement)*`, and a `pathElement` is a
`closureOrLambdaExpression` or a member access, with `nls` before it and
after its operator, so `foo { }.bar()` is `foo({ }).bar()` and
`foo { }` followed by `.bar()` on the next line is too. `commandExpression`
takes an `argumentList` after a path that ends in a name, so
`foo { }.bar 1` is `foo({ }).bar(1)`.

**Current implementation**: the statement `foo { }` is a `command_chain`,
and the scanner emits an empty token before the name where the braces
close on its line and a member access follows them (spec §6.24): the
call is then a `method_invocation`, the object of the access, or with a
first argument after the accesses the first object of a
`command_chain`'s receiver.

The Groovy column was observed by running Groovy 3.

| Input | Groovy | This grammar |
|---|---|---|
| `foo { }.bar()`, `foo { } .bar()`, `foo { }` then `.bar()` or `?.bar()` on the next line, `foo { }*.bar()`, `foo { }??.bar()`, `foo { }.&bar`, `foo { }.@bar`, `foo { }::bar`, `pom {}.writeTo("x")` | the access on `foo({ })` | the same; before, the chain `foo { }`, an `ERROR` and the rest |
| `l: foo { }.bar()`, `if (c) foo { }.bar() else baz { }`, `a { }.b { }.c()`, `foo { }.bar { }`, `foo { }.bar = 1`, `foo { }.bar + 1`, `foo { }.bar[1]`, `Foo { }.bar()`, `in { }.bar()` | the same | the same; before, an `ERROR` |
| `foo { }.bar` then `1` on the next line | `foo({ }).bar`, then `1` | the same |
| `foo { }.bar 1`, `foo { }.bar "a" + b`, `foo { }.bar x: 1`, `foo { }.bar x`, `foo { }.Bar x`, `foo { }?.bar.baz 1`, `foo { }` then `.bar 1` on the next line | `foo({ }).bar(1)`, … | a `command_chain` with that receiver; before, an `ERROR` |
| `foo { }.bar(baz 1)` | `foo({ }).bar(baz(1))` | a `command_chain` with that receiver and the argument `(baz 1)`, as `obj.m (foo 1)` (#24 above); before, an `ERROR` |
| `foo { }.bar().baz 1`, `a { }.b { }.c 1`, `a { }.b(1).c 2`, `foo { }.bar(1,` then `2).baz 1`, `foo { }.bar(a in b).qux 1` (also `as`, `instanceof`), `foo { }.bar[0] 1`, `foo { }.bar() x` | `foo({ }).bar().baz(1)`, `a({ }).b({ }).c(1)`, …, `foo({ }).bar((a in b)).qux(1)`, `foo({ }).bar[0].call(1)`, `foo({ }).bar().x` | an `ERROR`, as before: no receiver of the grammar takes brackets (`foo().baz 1` is `foo().baz` and `1`, #19 above), so the scanner reads past the brackets and emits no token where a first argument follows them or a later access |
| `foo { }.new X`, `foo { }.new X()` | `foo({ }).new(X)`, `new X(foo({ }))` | an `ERROR`, as before: the grammar reads `obj.new X` as `obj.new` and the statement `X`, so no token is emitted where a name follows `new` |
| `foo { }.bar(…)` whose brackets pass 128 characters from the `{`, then `.baz 1` | `foo({ }).bar(…).baz(1)` | the accesses, then the statement `1`, as for `foo().baz 1`: the bound cuts nothing in brackets, so that the call itself stays a call |
| `foo { }::bar 1`, `foo { }.'bar' 1` | `foo({ })::bar` called with `1`; `foo({ }).bar(1)` | an `ERROR`, as before: the receiver rule takes neither `::` nor a quoted name, so no token is emitted |
| `foo {` then `}.bar()`, braces and accesses past 128 characters from the `{` | `foo({ }).bar()` | an `ERROR`, as before: the scanner reads the braces on their line, within the bound |
| `foo { return /}/ }.bar()`, `foo { assert /}/ }.bar()`, `foo { a in /}/ }.bar()`, `foo { yield / 2 }.bar()`, `foo { x::return / 2 }.bar()` | `foo({ }).bar()` | the same: the scanner reads a slashy string after a keyword, and after `in` or `as` after an operand, as Groovy's lexer does, and a division after a keyword that an access, `::` included, names and after `yield` before a `/` and whitespace, which the grammar reads as a name; before, an `ERROR` |
| `(a) { }` then `foo { yield / 2 }.bar()` (or `foo { x++ / 2 }.bar()`) then `(b) { }`, and the `b` deleted | rejected after the deletion (Groovy 3: `()` on the next line) | from scratch the cast of `{ }` and one call `foo { … }.bar()() { }`; a reparse after the deletion reads the call of `(a)` with the closure instead, neither with an `ERROR`: `(a) { }` keeps both readings alive past the statement after it, which the runtime then reuses whole and checks only while one stack version exists (spec §6.18). `bindings/rust/tests/incremental_reparse.rs` pins it |
| `record { }.bar()`, `var { }.bar()`, `trait { }.bar()`, `sealed { }.bar()`, `super { }.bar()` | a call of the name with the closure, then `bar()` | an `ERROR`, as before: the scanner decides their own tokens at the name |
| `as { }.bar()`, `as { }.bar 1`, where a statement starts or on the line after an expression | a call of `as` with the closure, then `bar()` or `bar(1)` | an `ERROR`, as before: after an expression the scanner reads `as` for the operator and can no longer put a token before the word, and a reparse that reuses the statement before must read the same (spec §6.14) |
| `yield { }.bar()` | the same | a `yield_statement` of `{ }.bar()`, as before |
| `foo { }[0]`, `foo { } { }`, `foo { } + 1`, `foo { } as X`, `foo { } instanceof X` | the call subscripted, given a closure, added, cast, tested | the chain `foo { }` and statements of their own (`[0]`, a block, `+1`, `as` and `X`, `instanceof` and `X`), as before |
| `return` then `foo { }[0]` or `foo { } + 1` on the next line | `return`, then the call subscripted or added | a bare `return`, then the same as the statement (spec §6.23); at v0.6.0 the return of the subscripted or added call |
| `foo { }()`, `foo { }?[0]`, `foo { }..x`, `foo { } ? a : b`, `foo { } == 1`, `foo { } / 2`, `foo { } * 2`, `foo { } ?: 2` | the call called, subscripted, in a range, a conditional, compared, divided, multiplied, an Elvis operand | an `ERROR`, as before |
| `return foo { }.bar 1`, `break foo { }.bar()` | rejected | `return foo { }.bar` and `1`; `break foo` and `{ }.bar()`, as before |
| `x = foo { }.bar 1`, `def x = foo { }.bar 1` | `x = foo({ }).bar(1)` (Groovy 3) | the assignment of `foo { }.bar` and the statement `1`, with no `ERROR`, as at v0.6.0: the tokens of spec §6.24 are emitted only where a statement starts, so a value's parenthesis-free call (spec §6.14) takes no receiver that starts with a closure call |
| `"${foo { }.bar 1}"` | `foo({ }).bar(1)` in the interpolation | an `ERROR` at `1`, as at v0.6.0, for the same reason: the interpolation's value is `foo { }.bar`, and nothing takes the `1` before the `}` |

No tree that parsed without an `ERROR` at v0.6.0 changes: of the corpus,
Java and shape inputs none changes, and of the 266 real files one, from
an `ERROR` to none.

**Action**: an operator other than a member access after the `}` needs
the scanner to read on past it as here, at a cost on every statement
`foo { }`; braces that span lines need the read to go past the line, which
the bound would then cut on every block.

## 31. Several values in a switch case

**Apache `GroovyParser.g4`** (4.0.x to master): `switchExpressionLabel`
is `(CASE expressionList[true] | DEFAULT) (ARROW | COLON)`, and
`expressionList` is `expressionListElement (COMMA nls
expressionListElement)*`, so a line break may follow a value's comma but
not come before it, and `default` is no value.

**Current implementation**: a case takes one or more values, a `value`
field each, separated by commas (spec §4); a line break is whitespace
there, and `default` after a comma is read as a name.

Groovy 3 has no switch expressions and rejects each input below; the
Groovy column is read off the 4.0.x to master grammars.

| Input | Groovy | This grammar |
|---|---|---|
| `case 1, 2 -> a`, `case 1, 2: a`, `case 1,` then `2 -> a` on the next line | a case with two values | the same |
| `case 1` then `, 2 -> a` on the next line | rejected: no `nls` before the comma | a case with two values |
| `case 1, default -> a`, `case 1, default: a` | rejected: `default` is no expression | a case with the values `1` and the name `default` |
| `case *xs -> a` | a case with a spread value | an `ERROR` |

**Action**: none planned; rejecting the two accepted forms needs a line
break before the comma to end the case's values and `default` to be
turned down there, for input that Groovy rejects anyway.

## 32. A sign in a cast's operand before a power

**Apache `GroovyParser.g4`** (3.0.x to master): the cast alternative of
`expression` is `castParExpression castOperandExpression`, and
`castOperandExpression` is another cast, a `postfixExpression`, or `!`,
`~`, `++`, `--`, `+` or `-` before another `castOperandExpression`: no
binary operator, so every one applies to the cast.

**Current implementation**: `parenthesized_type_cast` has a precedence
above `**` (`PREC.CAST`), so a power after the operand applies to the
cast, as in Groovy. A prefix `+`, `-`, `++` or `--` in the operand has
the precedence of those operators elsewhere, below `**`, and takes the
power after it.

| Input | Groovy | This grammar |
|---|---|---|
| `(int) a * b`, `(int) a + b`, `(int) a ** b`, `(T) (U) a ** b`, `-(T) a ** b` | the product, sum or power of the cast; Groovy 3 evaluates `(int) 2.9 ** 2` to 4 and `-(int) 2.5 ** 2` to -4 | the same; before 0.7.0 `(int) a ** b` was `(int) (a ** b)` |
| `(int) -a ** b`, `(int) ++a ** b` | `((int) (-a)) ** b`; Groovy 3 evaluates `(int) -2.9 ** 2` to 4 | `(int) (-(a ** b))` |

**Action**: a sign that binds tighter than `**` inside a cast's operand
only, which needs a second unary rule for that operand; not attempted.

## 33. Keywords and strings as the name of a method with no return type

**Apache `GroovyParser.g4`** (`GROOVY_6_0_X`): `methodName` is
`identifier | stringLiteral`, and `identifier` takes `Identifier`,
`CapitalizedIdentifier` and the contextual keywords `as`, `async`,
`await`, `defer`, `in`, `module`, `permits`, `record`, `sealed`,
`trait`, `val`, `var` and `yield`. Every other keyword is no method
name, `this`, `void` and the primitive types included. `stringLiteral`
is the `StringLiteral` token, a string with no interpolation: a `$` in
`"…"` or `"""…"""` always starts one, and in a slashy or
dollar-slashy string one does where a letter or `{` follows it.

**Current implementation**: the name of a method with no `def` and no
return type (`_untyped_method_name`) reserves the
`untyped_method_name` set, every keyword that is a token of this
grammar and no Apache `identifier` (spec §5.2). A parse state takes
the reserved set of one of its positions, so after `private` or `@A`
the set also covers the type of a field or a method and the name of a
field or a local declared by modifiers alone (`private case = 1` is an
`ERROR`, as in Groovy). `this`, `void` and the primitive types are
identifiers in this grammar, and `goto`, `const` and `threadsafe` are
no tokens of it, so none can be reserved: tree-sitter takes only tokens
as reserved words, and making them tokens would take a token for each
wherever it stands today, `this` in every expression, key and receiver
position and the types in every type position. The scanner turns them
down instead: before one of them and a `(` on its line it emits
`_excluded_method_name` (spec §6.25), the word is the return type and
the name is missing, the tree `@A this(x) { }` had at v0.6.0. Past a
line break or a comment before the `(` they are still names.

A quoted name there is `_constant_string_literal`, `string_literal`
with no interpolation and no fallback `$`, which would take the one of
`${x}` as text, and no slashy string. Groovy 6's lexer reads a `/`
after a name, a `)` or a modifier as division (`REGEX_CHECK_SET` in
`GroovyLexer.g4`), so only the `>` of type parameters could come
before a slashy name, and that is not taken. A dollar-slashy name
takes a literal `$` where no letter, `_`, `$`, `{` or `/` follows it,
any character past ASCII counted as a letter. After `def` or a return
type the name is any `string_literal`, as before.

| Input | Groovy | This grammar |
|---|---|---|
| `private case(x) { }`, `@A true(x) { }`, `private <T> static(x) { }` | rejected | an `ERROR` |
| `@A in(x) { }`, `private record(x) { }`, `@A var(x) { }` | a method | the same |
| `@A this(x) { }`, `private int(x) { }`, `@A void(x) { }`, `private goto(x) { }` | rejected | a return type before a missing name |
| `@A this` then `(x) { }` on the next line, `private int /* c */ (x) { }` | rejected | a method named `this` or `int` |
| `def case() { }`, `def true() { }` | rejected | a method named `case` or `true`: after `def` or a return type no word is reserved (spec §5.2, #93) |
| `@A "a${x}"(n) { }`, `private "a$x"(n) { }` | rejected | an `ERROR` |
| `@A 'a b'(n) { }`, `@A "a b"(n) { }`, `@A $/a b/$(n) { }` | a method | the same |
| `@A /a b/(n) { }`, `private /a b/(n) { }` | rejected: the `/` is division | an `ERROR` |
| `@A <T> /a/(n) { }` | a method | an `ERROR` |
| `private $/a $ b/$(n) { }`, `@A $/a $1 b/$(n) { }` | a method | the same |
| `@A $/a $— b/$(n) { }` | a method: no interpolation starts at `—` | an `ERROR`: a character past ASCII after the `$` is read as a letter |
| `def "a${x}"(n) { }` | rejected | a method whose name holds the interpolation |

**Action**: none planned for a word before a line break or a comment,
or for a slashy name after type parameters; a keyword after `def` or a
return type is #93.

## 34. Method declarations in a block or a closure

**Apache `GroovyParser.g4`** (`GROOVY_6_0_X`): `scriptStatement` takes
`methodDeclaration[3, 9]`, and `classBodyDeclaration` takes a method as
a member. `blockStatement` is `statement`, which takes no method
declaration, so a block, a closure and a method's body declare no
method. AstBuilder's `validateMethodDeclaration` rejects a script
method without a body.

**Current implementation**: `method_declaration` is a `_statement`, so
a method with `def`, a return type or modifiers is taken in any list of
statements, a block's and a closure's included, with a body or
without. A method with no `def` and no return type and annotations
alone or type parameters (`untypedMethodHead`) is taken only at a
script's top level, with a body (`_script_method_declaration`), and
in a type body (`_untyped_method_declaration`).

| Input | Groovy | This grammar |
|---|---|---|
| `{ def f() { } }`, `if (c) { private g(x) { } }` | rejected | a method |
| `private foo(x)` in a script | rejected: a script method needs a body | a method without a body |
| `{ @A f(x) { } }`, `def m() { private <T> f(x) { } }` | rejected | an `ERROR` or a missing node, as at v0.6.0 |
| `@A f(x)` or `private <T> f(x)` in a script | rejected | an `ERROR` |

**Action**: none planned here. Taking the older forms out of blocks
and closures, or requiring their body in a script, changes trees that
parse with no `ERROR` today.

## 35. Space between a member access and `@`

**Apache `GroovyParser.g4`** (master): `pathElement` takes `NL*`
between the operator and the `@` (`DOT NL* (AT | …)?` and
`(SPREAD_DOT | SAFE_DOT | SAFE_CHAIN_DOT) NL* (AT | …)?`), and the
lexer hides spaces and comments between tokens, so Groovy takes
spaces, comments and line breaks between the operator and the `@`, and
spaces after the `@`.

**Current implementation**: `.@`, `?.@`, `??.@` and `*.@` are each one
token, so nothing may stand between the operator and the `@`. A space
after the `@` is trivia, as after any other token.

| Input | Groovy | This grammar |
|---|---|---|
| `obj.@f`, `obj?.@f`, `obj??.@f`, `obj*.@f`, `obj?.@ f` | a direct field access | the same |
| `obj. @f`, `obj?. @f`, `obj*. @f`, `obj?.` then `@f` on the next line | a direct field access | an `ERROR` at the `@` |

**Action**: none planned. Reading the `@` apart from the operator
would need the scanner to tell it from an annotation that opens the
next line; real code writes the operator whole (none of the 21,667
files of the real-code comparison has a space or a line break there).
