# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `array_creation_expression` for `new T[n][m]`, `new T[n][]` and
  `new T[] { … }`, after Apache `creator`, with `type`, `dimensions`
  (a `dimensions_expr` per `[n]`, one `dimensions` node for the `[]`s)
  and `value` (an `array_initializer`). After one `[]` a `{` among the
  elements is a closure, as in Groovy 3 and 4; after two or more a
  nested `{ … }` is an initializer, as in Groovy 5. Each form was an
  `ERROR` that swallowed its declaration. Folds and indents cover the
  initializer. (#102)
- `multi_assignment_expression` for `(a, b) = expr` without `def`,
  with a `name` field per identifier and a `value` field; it nests
  (`x = (a, b) = list`). It was an `ERROR`. Two on consecutive lines
  are two statements, as in Groovy, since a `(` that opens a line
  opens a statement (see Changed), and `def f(a, b) = list`, which
  Groovy rejects, parses; see `docs/divergences-from-spec.md` §3.
  (#94)
- `compact_constructor_declaration` for a record's compact canonical
  constructor (`public Warning { … }`; annotations, modifiers, `def`
  or none), with `name` and `body`, in a record body only. The name is
  not compared with the record's. Highlights, tags and locals treat it
  as `constructor_declaration`. (#72)
- An `@interface` element takes a default: `String value() default
  'x'` is a `method_declaration` with an optional `default` field
  holding an expression, an annotation or a `list_literal` of
  annotations; that `default` is `@keyword`. Most such elements were
  an `ERROR`. (#101)
- A parenthesis-free call as the value of a declarator (local, field,
  `def (a, b) = …`) or of an assignment with any operator:
  `def out = sh script: 'make', returnStdout: true`, `x = foo 1, 2`,
  `(a, b) = f 3`. The value is a `command_chain` with the fields of
  the statement form. The first argument must start on the receiver's
  line with a literal, a `name:` key, or one of the first arguments
  added below (a creation, an array type value, a name, `!`, `~`, or
  a call in parentheses), so `def x = foo` followed by `1` on the
  next line stays two statements. A chain is also accepted in
  parentheses and in an interpolation (see below). (#95)
- A parenthesis-free call whose receiver is a name followed by member
  accesses (`.`, `?.`, `??.`, `*.`, `.&`, `.@`) written without spaces,
  as a statement and as a value: `obj.method 1, 2`, `log.info "x"`,
  `def r = obj?.find id: 1`. `command_chain.receiver` adds those six
  node kinds; the method name of a `field_access` receiver is
  `@function.call`. The first argument must start on the receiver's
  line, so `result.size` followed by `2` on the next line stays two
  statements. Receivers that end in a call or a subscript
  (`foo().bar 1`) are not handled. Spaces before the first access are
  read in a declarator's or an assignment's value and in parentheses
  (`def a = foo .bar 1`), where the first name decides whether a name
  is a first argument (`docs/divergences-from-spec.md` §19). (#52)
- An annotation's arguments take annotations, as Apache's
  `elementValue` does: as an argument (`@Foo(@Bar)`), as the value of
  a pair (`@Foo(x = @Bar)`) and in a list (`@Foo([@Bar, @Baz])`),
  nested to any depth (`@Foo(x = [@Bar(@Baz)])`). Each was an `ERROR`.
  A pair is an `assignment_expression`, as in a call. A list that
  mixes annotations and expressions (`[@A, 1]`) or holds a list of
  annotations (`[[@A]]`), which Groovy rejects when it checks the
  annotation, is an `ERROR`, and so is an annotation anywhere else
  (`foo(@A)`).
- The `default` modifier, which Apache's grammar takes among a
  declaration's modifiers: `default void run() { }` in an interface,
  in any other type body or as a statement is a `method_declaration`
  with an anonymous `default` child, also after annotations, among
  other modifiers, with `def` or with no return type (`default e() { }`
  is a method, never a constructor). Class, interface, trait, enum and
  record declarations take it too. The modifier is `@keyword.modifier`;
  an `@interface` element's `default` field stays `@keyword`. Fields,
  constructors, compact constructors, initializers and local variables
  do not take it (see Changed; `docs/divergences-from-spec.md` §20).
- An array type is a value where Groovy reads a class:
  `def c = String[].class`, `int[][].class`, `xs.toArray(int[]::new)`,
  `String[].&new`, `foo(String[])`, `@A(String[])`,
  `def e = Map.Entry[].class` and an element's `default int[]` hold the
  `array_type` a declaration has, under the `field_access`,
  `method_reference_expression` or `method_pointer_expression` that
  `String.class`, `Foo::new` and `String.&new` have. Each was a
  subscript with a `MISSING` index, or an `ERROR` with two dimensions,
  for `short[]` and at the start of a statement. The element must be a
  primitive or capitalized name: package-qualified, lowercase and
  generic elements (`java.lang.String[].class`, `List<String>[].class`)
  and a bare `String[]` where a type may start (`return String[]`) read
  as before (`SPECIFICATION.md` §6.15).
- `multi_assignment_declaration` takes the forms of Apache's
  `variableDeclaration` with names in parentheses: a type per name
  (`def (int a, String b) = l`, in a new `type` field of the
  `variable_declarator`), `final`, `final var`, `var` alone and
  annotations before the names (`final (a, b) = l`, `var (a, b) = l`,
  `@Field def (a, b) = l`), a `for` init, and a type body member with
  a field's modifiers (`private static def (a, b) = l`,
  `private var (a, b) = l`). It takes Groovy 6's rest binding,
  `def (a, *b) = list`, in any place of the list, as a new
  `rest_declarator` with a `name` and, when typed, a `type`
  (`def (h, List<Integer> *t) = list`), and keyed pairs,
  `def (name: n, age: int a) = map`, with the key in a `key` field; a
  list holds names or pairs, not both. A rest binding's name is a local
  definition and `@variable`, as the other names are; a key is
  `@property`. Each was an `ERROR`.
  Annotations alone (`@A (a, b) = l`) and modifiers other than `final`
  where a statement starts (`static def (a, b) = l`) are still one.
- A parenthesis-free call takes more first arguments on the receiver's
  line, as a statement, after a dotted receiver and as a value: an
  object or array creation or an array type value
  (`println new Date()`, `foo new int[3]`, `log.info new Foo()`,
  `def r = foo new Foo()`, `foo String[].class`), and a name, `!` or
  `~` (`println x`, `obj.m arg`, `def r = foo bar`,
  `assertEquals expected, actual`, `foo !x`). The argument is the
  whole expression (`foo new Foo().bar()`, `println x + 1`). Apache's
  `isInvalidLocalVariableDeclaration` decides a name on its spelling,
  so `String s`, `int x`, `foo.Bar x` and `foo bar = 1` stay
  declarations. A name is no first argument before `->` (closure
  parameters), unless an argument after a comma starts with `(` or a
  key (`assertThrows IllegalArgumentException, () -> foo()` and
  `foo x, k: () -> b` pass a lambda), before parentheses that hold
  typed parameters (`foo bar(int a) { }` stays a method), or after a
  receiver that may name a type in a value (`def x = Foo bar`). A new
  empty token, `_command_statement_start`, opens such a statement whose
  receiver is one name. Not supported: a chain that goes on after its
  first argument (`foo 1 bar 2` is two calls and `foo a bar` a call and
  a name, where Groovy reads `foo(1).bar(2)`) and a comment between the
  receiver and the argument (`SPECIFICATION.md` §5.5, §6.14).
- A parenthesis-free call takes a spread argument after a comma, as
  Apache's `argumentListElement` does: `foo 'a', *args`,
  `foo a: 1, *b, c: 2`, `x = foo 1, *a`, `obj.m 1, *xs`, and on the
  line after the comma. The argument is a `spread_arguments`, as in
  parentheses, over the whole expression after the `*`, so the nest
  `foo 1, *a = foo 1, *a = 1` is Groovy's `foo(1, *(a = foo(…)))`. It
  was an `ERROR`, and the nest took time cubic in its depth (3.0 s for
  400 levels); it is linear (0.5 ms). `foo *a` is still a product, as
  in Groovy; still an `ERROR`: a spread as the first argument
  (`foo *a, 1`), which Groovy takes, and a spread map (`foo a: 1, *:m`).
- A parenthesis-free call is accepted in parentheses
  (`(sh script: 'x').trim()`, `def x = (foo 1)`), as the condition of
  `if`, `while` and `do`, a `switch` value and a `synchronized` lock
  (`if (fileExists 'pom.xml') { }`), in a `${…}` interpolation
  (`"${sh script: 'x'}"`), and in parentheses as another call's first
  argument (`println(foo 1).trim()`, `x.add(foo 'a')`, `super(foo 1)`),
  as Apache's `expressionInPar` and `commandExpression` take it. Each
  was an `ERROR`. The look-ahead reads one parenthesis deep, so
  `println(bar(foo 1))` is still an `ERROR`, as are `bar(a, foo 1)`,
  which Groovy rejects, and more than one statement in an
  interpolation. A keyword Groovy reserves is no receiver of a value:
  `(return 1)` is an `ERROR`.
- `lambda_expression` for Groovy 3's Java-style lambdas, with
  `parameters` (`formal_parameters`, or one `identifier` without
  parentheses) and `body` (a `block`, an expression or a
  parenthesis-free call): `def f = (a, b) -> a + b`, `() -> 42`,
  `(int a, String b = 'x') -> { a }`, `list.map(x -> x * 2)`,
  `foo(k: x -> x)`, `def h = x -> foo 1`. Each was an `ERROR`. A lambda
  in parentheses stands wherever an expression does, as Apache's
  `primary` takes it; one without parentheses in a value, an argument
  in parentheses, a map entry's or named argument's value and in
  parentheses. Two empty scanner tokens, `_lambda_start` and
  `_bare_lambda_start`, open the forms only when `->` follows; the
  look-ahead reads a default value's strings whole, a triple-quoted one
  across lines (`(a = '''it's''') -> a`). A closure
  parameter's default and a case value keep their reading before `->`
  (`def c = { a = (b) -> c }`, `case (A) -> 1`), as in Groovy. A brace
  body ends at its `}` only where no expression continues it, so
  `def v = a -> { y -> y }()` calls the closure and
  `def w = a -> { } + 1` adds to it inside the body, while a line break
  before a statement or a class member ends the body
  (`SPECIFICATION.md` §6.19). A lambda statement may follow the block of
  an `if`, `else` or loop body (`if (c) { a() }` then `(x) -> x`), from
  scratch as on a reparse, through `_body_block_end`, a token the scanner
  never emits; on the block's line too, where Groovy reads one `if`
  (`SPECIFICATION.md` §6.17). A lambda is a scope, a fold and an indent
  region, and its parameter without parentheses is a parameter
  definition and `@variable.parameter`. Still an `ERROR`: a lambda as a
  parenthesis-free call's argument (`foo (x) -> x`, `foo a: x -> x`), a
  lowercase parameter type (`(foo a) -> a`), and `return x -> x`, which
  Groovy rejects too. Read otherwise than Groovy: an assignment of a
  lambda that an operator follows (`v = a -> { foo() } == 1` compares
  the closure), more than eight comments between the `}` and what
  continues the body, which end it, and the `{` of a lambda in
  parentheses that also reads as a closure with a parameter
  (`def v = (a) -> { b = (c) -> { } }.call()`); see
  `docs/divergences-from-spec.md` §26. Nested lambdas whose bodies are
  expressions (`x = (a) -> (a) -> … a`) parse in time quadratic in the
  nesting, 1.1 s at 4,000 levels (`SPECIFICATION.md` §6.17).
- Typed local declarations without an initializer: `String s`,
  `int i;`, `List<String> xs`, `foo.Bar q`, `java.util.List<String> l`,
  `String a, b`, `@Field String x` and the init of
  `for (int i; i < n; i++)` are a `local_variable_declaration` with one
  `type` and a `name` per variable, and a typed declaration takes later
  names without a value (`String a = 1, b`, `final String a, b`). Each
  was two statements, other expressions or an `ERROR` (see Changed).
  Apache's `isInvalidLocalVariableDeclaration` decides it on the
  spelling, as for a name argument above, so `foo bar` stays a call. An
  empty scanner token, `_declaration_start`, opens the declaration
  where a statement starts, after annotations and as a `for` init, when
  it ends on its line, the next line does not go on with it, or an
  initializer follows (`SPECIFICATION.md` §6.21). Not declarations, as
  in v0.6.0: a comment between the type and the name
  (`String /* c */ s`), a lowercase type after annotations
  (`@A foo bar`), a space before `<` (`List <String> xs`), modifiers
  other than `final` (`static String s` is an `ERROR`) and a `try`
  resource without an initializer.

### Changed

- **BREAKING:** a dotted name followed on its line by a literal or a
  `name:` key is a `command_chain` (see Added), which changes trees for
  input that parsed without an `ERROR` at v0.6.0. `obj.method 'x'`,
  `sql.execute "…"` and `task.dependsOn 'a'` were the expression and a
  separate statement holding the literal; `obj.method x: 1` was the
  expression and a `labeled_statement`. After another statement on the
  same line (`foo 1 a.b 'c'`) the dotted call is a second statement,
  where Groovy continues the first. `obj.m 'a'() { }` and
  `a.B "x$y"() { }` were a `method_declaration` and are now a call, as
  in Groovy; `java.util.List 'foo'() { }` is still a method. (#52)
- **BREAKING:** the value of a declarator or an assignment takes a
  parenthesis-free call (see Added), which changes trees for input that
  parsed without an `ERROR` at v0.6.0. `def x = foo 1`, `x = foo 'a'`,
  `x += foo 1` and `a.b = foo 1` had the value `foo` and then a
  statement; `x = foo a: 1` was an assignment and a
  `labeled_statement`. The argument is the whole expression
  (`def x = foo 1 ?: 2`, `foo 1 as int`, `foo 1 { }`). A line escape
  between receiver and argument now joins them, and in
  `bar 1, x = foo 2` the assignment takes `foo 2`. `def (a, b) = foo 1`
  followed by `(c, d) = bar 2` was the value `foo`, then
  `1(c, d) = bar` and `2`; it is two statements, each with a chain.
  (#95)
- **BREAKING:** in a record body, a name after modifiers, annotations
  or `def` and followed by a block is a `compact_constructor_declaration`
  (see Added), which changes trees for input that parsed without an
  `ERROR` at v0.6.0: `public Warning { … }` and `def Warning { … }`
  were a modifier-only field and an initializer `block`. A field with
  no type, or a typed field with modifiers, followed by a block on the
  next line is now a field and a compact constructor named after its
  last word (`private static String NAME` then `{ … }`: field `String`,
  constructor `NAME`); Groovy rejects an initializer in a record, and
  a `;` after the field keeps the block. Class, interface, trait, enum
  and `@interface` bodies are unchanged. (#72)
- **BREAKING:** `trait` and `sealed` are keywords only before the
  declaration each opens (see Fixed), which changes trees for input
  that parsed without an `ERROR` at v0.6.0. `trait` at the end of a
  line before its name (`trait` then `T { }`) is the name `trait` and
  a call at statement level, as in Groovy, and a field `T` of type
  `trait` and a block in a member list; both were a
  `trait_declaration`. `modifiers Type trait` at the end of a line is a
  field named `trait`, where v0.6.0 read a field `Type` and a
  `trait_declaration` that took the next member. In a class body a
  modifier-only field followed by a method named `sealed` or `trait`
  (`static count` then `void sealed() { }`) is now a typed field and a
  constructor, the misreading of #96 that the keyword hid. (#73)
- **BREAKING:** an `@interface` element whose default is a bare name
  (`Class value() default Object`) is a `method_declaration` with a
  `default` field; v0.6.0 parsed it with no `ERROR` as the element and
  a field of type `default`. (#101)
- **BREAKING:** `object_creation_expression`'s `type` field no longer
  includes `array_type` in `node-types.json`. `new int[]()` and
  `new String[]() { }`, an object creation of an array type at v0.6.0,
  are now an `ERROR`, as Groovy rejects them. (#102)
- **BREAKING:** the npm package's `engines.node` floor is now `>=22`
  (was `>=20`); Node 20 is end of life. npm warns and still installs.
  The CI lint job runs Node 24. (#62)
- **BREAKING:** a `:` after a pair in an annotation's arguments,
  `@Foo(x = a: 1)`, parsed with no `ERROR` as a named argument whose
  key is the assignment `x = a`; a pair now ends at its value (see
  Added), so it is an `ERROR`, as in Groovy. The call form
  `foo(x = a: 1)` is unchanged.
- **BREAKING:** `default` is a keyword wherever a member or a statement
  can start (see Added), which changes trees for input that parsed
  without an `ERROR` at v0.6.0. `default void b() { }` in a type body
  was a field of type `default` named `void` and a constructor `b`, and
  at the start of a statement, like `default class C { }`, the
  expression `default` and the declaration; each is now one
  declaration. `default C() { }` in `class C` is a method, where Groovy
  reads a constructor. As a name there, which Groovy rejects, the word
  is now an `ERROR`: `default = 1`, `default.x()`, `return default`,
  `println default`, `String default`, `{ default -> 1 }`,
  `static default = 1` in a class body and `enum E { default }`. So are
  `default final int X = 1` in a type body and `default int x = 1` in a
  block, a field and a local variable in Groovy. After `=`, in arguments
  and after `def` it is still a name (`x = default`, `foo(default)`,
  `def default = 1`).
- An unclosed `switch` whose last branch is a colon case
  (`switch (x) {`, `case 1:` and `foo()` at the end of the input) is
  an `ERROR` over the whole input; v0.6.0 inserted the missing `}`, and
  inside a method kept the method and put the `ERROR` around the
  switch. An arrow case or an empty switch body still gets the missing
  `}`. It came with the `default` keyword above
  (`docs/divergences-from-spec.md` §20).
- Two shapes of input that parse with an `ERROR`, here and at v0.6.0,
  take time above linear in their length, where v0.6.0 was linear:
  nested `foo 1, %a = …` calls (3.0 s for 400 levels, 4.8 KB) and runs
  of `@C(,` and a space (3.6 s for 4,000). They came with the call as a
  value and annotations as annotation values. The new
  `bindings/rust/tests/error_recovery_cost.rs` pins their cost
  (`docs/divergences-from-spec.md` §29). The same nest with `*`, which
  Groovy takes, is linear (see Added), and so are `case 1:` lines
  outside a `switch`, which took 3.9 s for 4,000 after the `default`
  modifier until `case` was reserved (below): 25 ms.
- **BREAKING:** `case` is a reserved word, as in Groovy, and no longer a
  name. Input that parsed without an `ERROR` at v0.6.0 and that Groovy
  rejects is an `ERROR`: `case = 1`, `foo(case)`, `x = case`,
  `def m(int case) { }`, `{ case -> }`, `[(case): 1]`, the label
  `case:`, `case new Foo()` outside a switch, `import a.case.B`,
  `package a.case` and `enum E { case }`. It stays a key and a member
  name, with the same trees: `[case: 1]`, `foo(case: 1)`,
  `foo a: 1, case: 2`, `@A(case = 1)`, `x.case`, `x?.case()`,
  `x.&case`, `x::case`. `foo case: 1`, which was `foo` and the label
  `case: 1`, is a call with a named argument, as in Groovy, also as a
  value, after a dotted receiver and on a `switch` case's line
  (`case 1: foo case: 2`). Still wider than Groovy: `def case = 1`,
  `def case() { }` and `class C { int case }` parse
  (`SPECIFICATION.md` §5.2).
- **BREAKING:** a line that ends in parentheses around a name, a dotted
  name or a primitive type (`def y = (x)`, `return (x)`), followed on
  the next line by a multi-assignment declaration that starts with
  `final` or `var`, has a rest binding or keyed pairs, or is a type
  body member (see Added), was the cast of a call of a method named
  `final`, `var`, `def` and so on, `(x) final(a, b) = m`, with no
  `ERROR`. It is two statements or members, as in Groovy.
- **BREAKING:** `var`, `yield`, `in` and `as` are names wherever their
  construct does not follow (see Fixed), which changes trees for input
  that parsed without an `ERROR` at v0.6.0 to the trees an ordinary
  name gets. `x = (b)` followed by `in` or `as` on the next line was a
  cast of the word, and `def y = (x)` followed by `var (a, b) = l` the
  cast of a call `var(a, b)`; each is two statements. `var instanceof T`
  was a declaration of `instanceof` and the statement `T`, and
  `yield instanceof T` a `yield_statement` and `T`; each is the test of
  the name. `yield++` followed by `foo()` on the next line was
  `yield ++foo()`, and `x = 1`, `in++` and `foo()` on three lines were
  `x = 1 in ++foo()`; each is now the update of the name and the call.
- **BREAKING:** a creation or an array type value is a parenthesis-free
  call's first argument (see Added), which changes trees for input that
  parsed without an `ERROR` at v0.6.0: `println new Date()`,
  `foo new Foo() { }`, `log.info new Foo()` and `def r = foo new Foo()`
  were the receiver, or a declaration whose value is the receiver,
  followed by the creation as a statement of its own. Each is one
  `command_chain`. After another statement on the same line,
  `foo 1 bar new Baz()` is `foo 1` and the call `bar new Baz()`, where
  Groovy continues the first.
- **BREAKING:** a method header with no body whose parameter is named
  `true`, `false` or `null` now reads the parentheses as a call in
  parentheses (see Added): `def f(Foo true)`, which Groovy rejects, is
  the declaration of `f` and the parenthesis `(Foo true)`, and
  `foo bar(baz true)`, Groovy's `foo.bar(baz(true))`, is the statement
  `foo` and the call `bar((baz true))`. Each was a `method_declaration`
  with no `ERROR`. With a body it is still the method.
- **BREAKING:** a name, `!` or `~` is a parenthesis-free call's first
  argument (see Added), which changes trees for input that parsed
  without an `ERROR` at v0.6.0. `println x`, `obj.m arg`, `foo !x`,
  `then: thrown IllegalStateException` and `def r = foo bar` were the
  receiver, or the declaration, followed by the argument as a
  statement; each is one call. A script method with a lowercase return
  type and untyped parameters (`foo bar(a) { }`) is a call of `foo`
  with `bar(a) { }`, as in Groovy; typed parameters keep the method.
  `if (x) foo bar else baz qux` was an `if` and four statements and is
  an `if` with an `else`. Closure parameters after a lowercase type are
  read for their `->` within 256 items, strings whole (a triple-quoted
  one across lines, `{ foo a, b = '''x` then `y''' -> a }`), so 128 or
  more names after the first (`{ foo a, b0, …, b127 -> a }`) are an
  `ERROR`.
- **BREAKING:** a literal first argument of a statement whose receiver is
  one name (a number, a quoted string, `true`, `false` or `null`) is the
  start of a whole expression, as in Groovy, which changes trees for
  input that parsed without an `ERROR` at v0.6.0. After the receiver and
  the literal, `+ b`, `- b`, `[0]`, `(b)`, `as T`, `in b`, `!in b`,
  `instanceof T` and `!instanceof T` on the literal's line, also past a
  block comment or a line escape, were the call and separate statements,
  and are one argument expression (`println "a" + b`,
  `src "${url}/" + f`); `foo 'a' ++b` was `foo 'a'` and `++b`, and is
  `foo('a'++)` and `b`, where Groovy goes on with `.b`. `{ }` on the
  literal's line was the call and a closure statement, a `block` since
  the change above, and is a call of the literal with the closure
  (`foo("a"({ }))`, Groovy's call of a method named `a`). On the next
  line `{ }`, `as T`, `in b`, `!in b`, `instanceof T` and
  `!instanceof T` now continue the argument, as do the operators that
  Groovy lets open a line within an expression (`foo "a"` then
  `.trim()`, `?: b`, `* 2`), which were an `ERROR`; `+`, `-`, `~`, `(`,
  `[`, a name and a literal there open the next statement, as before
  (`foo 1` then `-x`). A script-level generic method on the next line
  (`println "a"` then `<T> void m(T t) { }`), which Groovy 3 rejects as
  well, was the call and a `method_declaration`, and is an `ERROR`, as
  after `println("a")`: the `<` goes on with the argument. Of 266 real
  files, 11 that had an `ERROR` have none (`assertEquals "a " +` then
  `"b", x, y` in tests, `timeOutInMs 30 * 60 * 1000` in Gradle files).
  Read otherwise than
  Groovy: an expression that goes on past the literal and ends its line
  takes a next line that opens with `+` or `-` (`foo "a" + b` then `-c`
  is one call), as after a name argument; a comment between the receiver
  and the literal, the receivers `var`, `record`, `trait`, `sealed`,
  `permits`, `super` and `_`, a literal on the line after the
  receiver, and a lambda in parentheses after the literal on its line
  (`foo "a" (x) -> x`, the call and a lambda statement, where Groovy
  passes the lambda to a call of the literal) keep the literal alone.
  An assignment after the literal
  (`foo "a" = b`, `foo "a" += b`), and `= b` or `** b` on the next line,
  which Groovy rejects, are a call with no `ERROR`, as are a number's
  range or member access followed by `++`, `--` or `, b` on the next
  line (`foo 1..3` then `++`), and the receivers `goto`, `const` and
  `threadsafe`, which Groovy reserves (`goto "a" + b`)
  (`SPECIFICATION.md` §5.5 item 3, §6.14).
- **BREAKING:** the body of `if`, `else`, a loop or an arrow case that
  holds the assignment of a lambda is a `block` that holds the lambda
  (see Added), as in Groovy, which changes trees for input that parsed
  without an `ERROR` at v0.6.0: `if (x) { a = b -> c }`,
  `while (x) { String a = b -> c }` and nests such as
  `if (x) { a = b -> { c = d -> { p -> p } } }` were a closure with the
  parameter `a` (`String a`). On one line,
  `(y.y) if (z) { a() } else { b() }`, which Groovy rejects, is the
  parenthesis and an `if`; it was the cast of a call named `if`.
- **BREAKING:** a `(` that opens a line after an expression opens the
  next statement where one may start, as in Groovy, rather than the
  arguments of a call of that expression, which changes trees for input
  that parsed without an `ERROR` at v0.6.0: `foo` followed by `(1)` or
  `(x);`, `foo()` followed by `(a, b) = [1, 2]`, `x = (a)` followed by
  `(b) y` and `def d = load()` followed by
  `(data as Map<String, Object>).each { }` are two statements now.
  Inside parentheses and brackets the `(` still calls
  (`bar(foo` then `(1))`), and so do parentheses that cannot open a
  statement, so input Groovy rejects keeps its tree: `()`, `(a, b)`, a
  spread, a named argument with any key (`(a: 1)`, `('a': 1)`), and
  parentheses followed by `,`, `->` or `else`
  (`if (x) foo` then `(1) else bar`). Java-style wrapped calls with
  several arguments (`foo` then `(a, b);`) keep their tree. A block
  comment that opens the `(` line hides the line break, so `foo` then
  `/* c */ (1)` is a call. A new external token, `_statement_paren`,
  carries that `(` (`SPECIFICATION.md` §6.18). A `)` that ends its line
  before such a `(` closes no cast unless a `)` or `]` after the `(`,
  within 256 characters, closes brackets around both lines, so runs of
  lines like `(x)` or `(a) = (b)` parse in linear time (12 ms for 4,000
  lines of `(x)`, where the cast kept alive took 2.1 s). Past the bound
  a cast inside a call is an `ERROR` or another reading, with the call's
  `)` more than 256 characters on: `foo((Foo)` then `(Bar) y, '…')` is
  an `ERROR`, and `foo((Foo)` then `(bar) + 1, '…')`, the cast of a cast
  at v0.6.0 as in Groovy, is the sum of the call `(Foo)(bar)` and `1`
  with no `ERROR` (`(bar)[0]` a subscript of that call). An
  interpolation's `}` closes no bracket around both lines, so
  `"${(Foo)` then `(Bar) y}"`, the cast of a cast at v0.6.0, is the call
  `(Foo)(Bar)` and an `ERROR` at `y`, where Groovy 3 reads two
  statements, `(Foo)` and the cast `(Bar) y`. The cast of an
  array or generic type stays (`(List<String>)` then `(x)`); a new
  external token, `_type_only_close`, which is never emitted, marks it
  (`SPECIFICATION.md` §6.11, `docs/divergences-from-spec.md` §15).
- **BREAKING:** after `as`, `instanceof` and `!instanceof` a dotted
  name with array dimensions or type arguments is one type (see Fixed),
  which changes trees for input that parsed without an `ERROR` at
  v0.6.0: `x as a.b.C<D>[]` was comparisons with a list, and
  `l = x as java.util.List<T>` followed by `foo()` on the next line the
  comparisons `(x as java.util.List) < T > foo()`; each is the cast to
  a `generic_type` over a `qualified_type`, in an `array_type` for the
  first, and `foo()` is a statement of its own. A `<` there opens type
  arguments only where they close, so `x as a.B<c`, which was
  `(x as a).B < c`, compares the cast to `a.B` with `c`. The dotted type
  outranks field accesses by a dynamic precedence instead of
  tree-sitter's order of rules, which decides a tie the other way: `x`,
  `f() as a.B` and `y = 1` on three lines were a method `x f()` with no
  body, the name `as` and a typed local `a.B y = 1`, and are the cast
  and an assignment, as in Groovy. Now an `ERROR`, where Groovy reads a
  command chain or rejects the input: a quoted name or `new` after a dot
  of a cast type (`x as a."b"`, `x as T.new`), and type arguments on a
  dotted cast type followed in brackets or a condition by what Groovy
  compares or calls (`foo(x instanceof a.b<T> y, 1)`). Also an `ERROR`:
  a cast type's type arguments of more than 256 items, which the
  scanner's look-ahead reads as comparisons, 86 or more arguments
  separated by a comma and a space (`x as Map<T0, …, T85>`) or 129 by a
  comma alone; Groovy reads the type, as v0.6.0 did
  (`SPECIFICATION.md` §3.2.2). Read otherwise
  than Groovy: `x as a.b[0]` subscripts the cast, a space before `<`
  makes a comparison (`x as List <String>` is an `ERROR`), and after
  `return` or in brackets the type arguments of a name of three or more
  parts are read where Groovy compares (`return x as a.b.C<T> y`); see
  `docs/divergences-from-spec.md` §5.
- **BREAKING:** parentheses before `instanceof`, `!instanceof` or
  `!in`, or before `as` or `in` and what they take on the line, are
  that operator's operand, not a cast of the word after them, which
  changes trees for input that parsed without an `ERROR` at v0.6.0.
  `(x) instanceof Foo`, `return (x) instanceof Foo`,
  `(x) !instanceof Foo` and `(x) !in y` were the cast `(x)` of a name
  (`instanceof`, or `!` applied to the name) and another statement, and
  are the tests of `(x)` that Groovy reads. `(x) as Foo`,
  `def v = (x) as a.b.c` and `(x) in y` were the cast of the name `as`
  or `in` and another statement, and are a `cast_expression` and a
  `membership_expression` of the parentheses, as Groovy 6.0.0 reads
  them (GROOVY-10355); Groovy 3 to 5 read a call of the cast of a
  variable `as` or `in` where a statement starts. Block comments on the
  line count as spaces (`return (x) in /* c */ y`). `(Foo) in`,
  `(Foo) in.read()`, `(byte) in[i]` and `(x) in -1` still cast the
  name `in`. Now an `ERROR`: parentheses that hold only a generic or
  array type before these operators (`(List<String>) instanceof Foo`,
  `(String[]) as Foo`), where Groovy tests a class literal, and a
  command argument after the operator where no statement may follow
  (`class K { def f = (x) as Foo y }`). A new external token,
  `_relational_close`, is that `)` (`SPECIFICATION.md` §6.11).
- **BREAKING:** typed locals without an initializer are declarations
  (see Added), which changes trees for input that parsed without an
  `ERROR` at v0.6.0: `String s`, `int i;` and `foo.Bar q` were two
  statements, or a field access and a name, and
  `java.util.List<String> l` comparisons. Such a declaration ends at
  the line break before a line that starts a statement, as in Groovy:
  `String s` followed by `{ }`, `+ 1` or `(a) { }` on the next line was
  `String` and `s { }`, `s + 1` or a method `s`, and is the declaration
  and that statement. Followed by `in xs` or `as T` it is the
  declaration and the statements `in` and `xs`, where Groovy calls
  `in(xs)`; it was `String` and `s in xs`. `InputStream in` followed by
  `foo()` was `InputStream in foo()` and is the declaration and the
  call. A line that a name can continue (`.trim()`, `== 1`, `/ 2`) still
  reads `String` and the expression of `s`, where Groovy calls
  `String(s.trim())`. `final x` followed by `String s = 'a'` was the
  blank final `final x String` and `s = 'a'` and is two declarations, as
  in Groovy, whatever the value, a lambda included
  (`Runnable r = () -> { foo() }`). `{ String a = '''x` then
  `y''' -> a }` stays closure parameters, as at v0.6.0. A type and a name after another statement on its line
  declare: `move left By steps` is the call `move left` and the
  declaration `By steps`, where Groovy continues the call. Error
  recovery changes too: `String s =` at the end of the input is the
  declaration and an `ERROR` rather than a `MISSING` value, and
  `enum E<T> { A }` loses its `enum_declaration`.
- **BREAKING:** a `{ … }` that opens a statement, in a file, a block, a
  closure or a colon case, or after a label, is a `block`, as Groovy
  reads it (Apache's `statement` lists `block` first), which changes
  trees for input that parsed without an `ERROR` at v0.6.0: it was an
  `expression_statement` holding a `closure`. `{ a }` followed by
  `{ b }` on the next line was a closure called with a closure and is
  two blocks; `{ }` followed by `+ 1`, `- 1` or `++x` was a binary
  expression or `{ }++`, and is a block and a unary expression or an
  update; `{ a = b -> c }` was a closure with the parameter `a` and is a
  block that holds a lambda. Braces that an expression continues stay a
  closure (`{ foo() }()`, `{ foo() }.call()`, `{ foo() } as Runnable`,
  `{ }` followed by `.call()`), and so do braces with parameters
  (`{ a -> }`) and a `{` on the line after a call, its trailing closure.
  Read otherwise than Groovy: `foo { a }` followed by `{ b }`, or
  `foo { a } { b }`, is a call and a block, where Groovy passes `{ b }`
  as one more argument; `{ } foo` is a block and a statement, where
  Groovy calls the closure; `{ a = b -> { p -> p } }` stays the closure
  with the parameter `a`; and `as` or `in` on the next line casts or
  tests it. Now an `ERROR`:
  `{ }` followed by `()`, `(a, b)` or a bare `++` on the next line,
  which Groovy rejects, and `{ }` followed by nine or more comments and
  `.call()`, as the scanner reads at most eight comments after the `}`
  (`SPECIFICATION.md` §6.19). Two JDK test files with Java array
  initializers in fields recover as one `ERROR` for the whole file. A
  new external token, `_statement_block_end`, ends the block
  (`SPECIFICATION.md` §6.22).
- Twenty-seven external scanner tokens are added, so there are 38
  instead of 11: `_trait_keyword` and `_sealed_keyword` (#73,
  `SPECIFICATION.md` §6.13); `_var_keyword`, `_var_names_keyword`,
  `_yield_keyword`, `_in_keyword`, `_as_keyword` and `_contextual_key`,
  such a word as a named argument's key, aliased to `identifier`
  (§6.16); `_command_value_start` (#95), `_command_path_start` (#52)
  and `_command_statement_start`, empty starts of a parenthesis-free
  call (§6.14); `_array_value_start` and `_type_start`, which is never
  emitted (§6.15); `_lambda_start`, `_bare_lambda_start`, and
  `_body_block_end`, which is never emitted (§6.17);
  `_statement_paren` (§6.18); `_lambda_body_end`, and
  `_lambda_body_reread`, which no rule takes (§6.19);
  `_multi_assignment_comma`, the first `,` of `(a, b) = …` (§6.20);
  `_declaration_start`, and `_closure_parameters_start`, which is never
  emitted (§6.21); `_statement_block_end` (§6.22);
  `_relational_close`, and `_type_only_close`, which is never emitted
  (§6.11); `_cast_type_arguments_open` (§3.2.2);
  and `_unterminated_comment` (#75), an empty hidden extra (§6.6). The
  keyword and punctuation tokens are aliased, so the anonymous `trait`,
  `sealed`, `var`, `yield`, `in`, `as`, `(`, `)`, `,` and `<` nodes
  keep their kinds. The scanner now carries two bits of serialized
  state, one byte while either is set (§6.6, §6.19). Eight named node
  kinds are added (`array_creation_expression`, `array_initializer`,
  `compact_constructor_declaration`, `dimensions`, `dimensions_expr`,
  `lambda_expression`, `multi_assignment_expression`,
  `rest_declarator`) and one field, `dimensions`; none is removed.
  `SYMBOL_COUNT` goes from 321 to 408, `FIELD_COUNT` from 45 to 46 and
  `STATE_COUNT` from 4,025 to 4,658, so most symbol ids and the field
  ids after `dimensions` change. In `node-types.json`: every expression position adds
  `array_creation_expression`, `array_type`, `lambda_expression` and
  `multi_assignment_expression`; `command_chain` is added to the
  `value` of `variable_declarator`, `local_variable_declaration`,
  `multi_assignment_declaration`, `gstring_brace_interpolation` and
  `switch_expression`, to `assignment_expression.right`, to the
  children of `parenthesized_expression`, to the `condition` of
  `if_statement`, `while_statement` and `do_while_statement` and to
  `synchronized_statement.lock`; `command_chain.receiver` adds the six
  member-access kinds; `argument_list`, `list_literal`,
  `assignment_expression.right` and `multi_assignment_declaration` add
  `annotation`, and `multi_assignment_declaration` `rest_declarator`;
  `variable_declarator` gains `type` and `key`;
  `method_declaration` gains `default`; `class_body` adds
  `compact_constructor_declaration` and `multi_assignment_declaration`,
  and `enum_body` and `for_statement.init` the latter; the children of
  `source_file`, `block`, `closure`, `switch_case` and `switch_default`
  and `labeled_statement.statement` add `block`; and
  `object_creation_expression.type` drops `array_type`. (#52, #72, #73,
  #75, #94, #95, #101, #102)
- Parsing costs more than at v0.6.0, counted in user-space
  instructions. On files that parse without an `ERROR` in both
  versions, 148 real Groovy, Gradle and Jenkins files take 4.4% more
  and 241 Java files 3.0% more; with the files that have an `ERROR`,
  58 of 206 real files at v0.6.0 and 15 now, the real files take 4.9%
  fewer, as error recovery is costly. Lines of one kind of statement
  take 7% to 11% more (calls, assignments, dotted calls such as
  `log.info(y)`). A script method head with untyped parameters and a
  body (`def f(a, b) { }`) takes 10% to 13% more for two to sixteen
  parameters, about as much as a typed head (9% more with two), and
  1.8 times as much with twenty, past the scanner's look-ahead of
  sixteen names (`SPECIFICATION.md` §6.20); a typed local with an
  initializer (`String s = 'a'`) takes 42% fewer. The compiled parser
  is about 30% larger.
- A few edit sequences still reparse incrementally to a tree that
  differs from a parse from scratch. `SPECIFICATION.md` §6.17 to §6.22,
  `docs/divergences-from-spec.md` §20 for an edit of a `default`
  modifier after a field with modifiers only, and `SPECIFICATION.md`
  §6.14 for `def` typed and deleted before a dotted call of an
  interpolated string (`obj.Bar $/$a/$() { }`), give an example of each
  kind known but one, a closure whose two readings weigh the same
  (§6.22), and `bindings/rust/tests/incremental_reparse.rs` pins each
  example. Random edits find more, most of them in text with an `ERROR`
  both ways, which those sections count.
- `SPECIFICATION.md` targets the Groovy 6.x grammar (6.0.0 at the time
  of writing) instead of 4.x: earlier versions parse where their syntax
  is a subset, and where versions read the same text differently the
  latest release wins, as for nested array initializers after two or
  more `[]` (Groovy 5, GROOVY-8551) and `(x) as Foo` (Groovy 6.0.0,
  GROOVY-10355), with the exceptions noted where they are made.
  Coverage of Groovy 5 and 6 syntax is incomplete (`async`, `await`,
  `defer` and `val` are names); `docs/divergences-from-spec.md` lists
  the gaps.
- **BREAKING:** `def` before a type declares a typed local or field, as
  in Groovy, where `def` is a modifier: `def String s = 'a'`,
  `final def int a, b = 1`, `@Field def String[] names` and, in a type
  body, `private def Foo foo = mock(Foo)` are one
  `local_variable_declaration` (or `field_declaration`) with the `type`
  field and an anonymous `def`. At v0.6.0 the local was `def String`
  and the statement `s` or `s = 'a'`, with no `ERROR`; the field, a
  generic type (`def List<String> xs`) and a second name
  (`def String a, b`) were an `ERROR`, and so was `def int i = 0` as a
  `for` init. The scanner decides the type after `def` as it does a
  statement's (`SPECIFICATION.md` §6.21), so `def String` then `s = 1`
  on the next line stays two statements. A lowercase type
  (`def foo bar`), a modifier after `def` (`def final String x`) and a
  return type after `def` (`def String m() { }`) are read as before.
  Of 266 real files, one that had an `ERROR` has none.
- **BREAKING:** a line break after `return`, `break` or `continue` ends
  the statement, as Groovy takes the value or the label only on the
  keyword's line (Apache `RETURN expression?`, `BREAK identifier?`):
  `if (c) continue` then `process(f)` on the next line is a bare
  `continue_statement` and the call, where v0.6.0 read
  `continue process` and the parenthesized `(f)` with no `ERROR`;
  `return` then `foo()`, `obj.m 'x'`, `String s` or `{ }` on the next
  line is a bare `return_statement` and that statement, where v0.6.0
  returned the call, `obj.m`, `String` or the closure; and
  `if (c) break` then `foo = 1`, an `ERROR`, parses. A line comment
  after the keyword ends the statement inside it, and a block comment
  on the line keeps the label (`break /* c */ outer`). A new external
  token, `_jump_statement_end`, ends the statement
  (`SPECIFICATION.md` §6.23). `throw` and `assert` still take a value
  on the next line. No tree of 266 real files changes; 66 of 24,548
  Java files do, each at a `return` that ends its line.

### Fixed

- `trait` and `sealed` are names wherever they do not open a
  declaration, as in Groovy: `private String trait`,
  `private boolean sealed = true`, `static trait = 1`, `return trait`,
  `trait = x` and `sealed()` parse. Each was an `ERROR`, or opened a
  `trait_declaration` that swallowed the following members. Like
  `record`, both keywords now come from the external scanner
  (`SPECIFICATION.md` §6.13). `sealed` before another modifier
  (`sealed abstract class S { }`) is still an `ERROR`; other residuals
  are in `docs/divergences-from-spec.md` §12. (#73)
- A file with many unterminated `/*` is no longer read once per `/*`:
  the scanner, and the `record` header look-ahead, read to the end of
  input each time, so parse time grew with the square of the count.
  Input that parsed without an `ERROR` parses as before. On malformed
  input, error recovery near the first unterminated `/*` can differ and
  can be worse as well as better (`SPECIFICATION.md` §6.6). (#75)
- `var`, `yield`, `in` and `as` are names wherever their construct
  does not follow, as Apache's `identifier` rule lists them:
  `int var = 1`, `InputStream in = open()`, `int as = 1`, `in.close()`
  on the line after a statement, `return yield`, `{ var -> var }`,
  `for (String in : xs)` and `void var() { }` parse. Each was an
  `ERROR`, and `final var = (1)` in a method could take in the rest of
  the method. Like `record`, the keywords now come from the external
  scanner, which emits `var` before a declared name or a
  multi-assignment's names, `yield` and `in` before an expression, and
  `as` before a type (`SPECIFICATION.md` §6.16). `for (var x in xs)`
  and `for (var x : xs)` are loops; they were an `ERROR`. Still an
  `ERROR`: `for (var in xs)`, `for (String in in xs)`, `yield(1, 2)` and
  a method named `in` with a return type (`int in(int x) { x }`).
- Queries. `tags.scm`: every parenthesis-free call is a
  `@reference.call`, by its receiver's name or the last name of a dotted
  receiver (`sh 'make'`, `def out = sh script: 'make'`, `log.info 'x'`),
  and so is a call through `?.`, `??.`, `*.` or `.&` (`obj?.m(x)`); a
  call through `.@` and `array_creation_expression` stay untagged.
  `highlights.scm`: the method of a call through `?.`, `??.` or `*.`
  is `@function.call`, where it was `@property`, and the names and dots
  of an annotation's name are `@attribute`
  (`@groovy.transform.CompileStatic`), where they were `@variable` and
  `@operator`. `injections.scm`: SQL is injected into the first `"…"`
  or `"""…"""` argument of a call on `sql` or `Sql`, also without
  parentheses (`sql.execute "SELECT …", id`); with parentheses it was
  injected into later string arguments too, bind parameters and slashy
  strings included.
- A closure that opens with a typed declaration of two names
  (`def c = { String a, b }`, `foo { String a = 1, b }`) holds the
  declaration; it was closure parameters with a `MISSING` `->`.
- After `as`, `instanceof` and `!instanceof` a dotted name followed by
  array dimensions or type arguments is one type, an `array_type` or a
  `generic_type` over a `qualified_type`:
  `o instanceof JTextComponent.KeyBinding[]`, `x as a.b.C[]` and
  `x as java.util.List<String>` had a `MISSING` node (see Changed for
  the trees this alters). A `<` after a cast type's name opens type
  arguments only where they close, so `x as B<c` compares the cast
  with `c`, where it had a `MISSING` `>`; a new external token,
  `_cast_type_arguments_open`, is that `<` (`SPECIFICATION.md` §3.2.2).
  Runs of lines such as `l = x as util.List<T>` took time quadratic in
  their number, 4.4 billion instructions for 4,000 lines, and are
  linear, 0.17 billion.
- A lambda without parentheses in an enum's first constant's arguments
  parses (`enum E { A(v -> v) }`, `A(value -> value)`): those arguments
  are also a constructor's parameters, where `var` may start one, and the
  scanner turned down any name that began like `var` at its first
  letters. It now reads the whole name first. `A(var -> var)` is still an
  `ERROR` (`docs/divergences-from-spec.md` §22).
- The look-ahead for a slashy string's closing `/` stops after 64
  escaped slashes and takes the string to close there: it ran from every
  `/` where a slashy string may start, during error recovery every `/`,
  so a line of `\/` was read in time quadratic in its length, 17 s for
  100 KB, as at v0.6.0; it is linear now. A slashy string that closes
  reads as before however many escaped slashes it holds; past the bound
  an unterminated one is a slashy string with a `MISSING` `/` instead of
  another `ERROR` (`SPECIFICATION.md` §6.2).
- `tree-sitter.json` names `queries/groovy/injections.scm`, so
  `tree-sitter highlight` and other tools that read the file load the
  injections; v0.6.0 left them out. `locals.scm` stays unlisted, as
  `tree-sitter tags` rejects its `@local.definition.var` captures.
- A block comment between the key of a parenthesis-free call's named
  argument and its `:` is trivia, as Groovy's lexer hides it:
  `foo a /* c */ : 1` is the call `foo(a: 1)`, also when the comment
  spans lines; v0.6.0 read the statement `foo` and the label `a: 1`
  (`SPECIFICATION.md` §6.8).
- Queries. `injections.scm`: the body of a slashy or dollar-slashy
  string is injected as `regex`, each `string_fragment` as its own
  document, so the delimiters and `${…}` and `$name` interpolations
  stay out of the regex parser. The whole string was captured, which
  leaves its children out, so only a slashy string's opening `/` was
  injected and nothing of a dollar-slashy one. A body that an
  interpolation or a literal `$` splits is several documents
  (`docs/divergences-from-spec.md` §7). `highlights.scm`: a slashy
  string's closing `/` is `@string`, like its opening `/`; it was
  `@operator`, as it is the same anonymous `/` as division, which
  stays `@operator`.
- `assert x == 1, 'msg'` parses: a comma separates an assertion's
  message as a colon does, before or after a line break, as Apache's
  `assertStatement` takes `COLON | COMMA`. It was an `ERROR`, as at
  v0.6.0.
- A list literal takes spread elements (`[*a, *b]`, `[1, *a]`,
  `[*a, 1]`), a `spread_arguments` node each, as Apache's `list` takes
  `expressionList[true]`, and an argument list takes a spread map
  (`foo(*:m, x: 1)`, `foo(a: 1, *:m)`, `foo a: 1, *:m`), a
  `spread_map_entry`, as Apache's `namedArg` does. Each was an `ERROR`,
  as at v0.6.0. A spread first in a parenthesis-free call
  (`foo *a, 1`, `foo *:m`) is still an `ERROR`.

## [0.6.0] - 2026-11-12

### Added

- Enum declarations accept an `implements` clause
  (`enum Color implements Shaded, Comparable<Color> { … }`), as a
  `super_interfaces` child, as on classes. Previously the clause was
  an `ERROR`, and with a generic interface the enum's `name` became
  the type argument: `enum Level implements Supplier<String>` was
  named `String`. `node-types.json` adds `super_interfaces` to the
  children of `enum_declaration`. (#77, #81)
- `synchronized_statement` for Java's synchronized block,
  `synchronized (lock) { … }` (Apache `#synchronizedStmtAlt`), with
  `lock` and `body` fields. Previously the statement parsed as a
  `method_declaration` with a missing name: `synchronized` read as a
  modifier and `(lock)` as the parameters. (#76, #80)
- `tree-sitter.json` declares the tags query, so `tree-sitter tags`
  works from a checkout and `test/tags/` is run by `tree-sitter test`.
  (#100)

### Changed

- **BREAKING:** `final` on the variable of a `for` header is now a
  modifier instead of a type name, which changes trees for input that
  parsed without an `ERROR` at v0.5.0. `for (final x in xs)` and
  `for (final i = 0; ...)` no longer have `final` as their `type`; in
  the C-style form the name and value also move into a
  `variable_declarator`. A method declared with modifiers only outside
  a class body (`final f() { }`, `static f() { }`) now has `f` as its
  `name`; at v0.5.0 `f` was its `return_type` and `name` was a
  zero-width `quoted_identifier`. (#56)
- **BREAKING:** a parenthesis-free call whose only argument is a named
  argument (`archiveArtifacts artifacts: 'x'`, `foo a: 1`) is now a
  `command_chain` with a `named_argument`. At v0.5.0 it parsed without
  an `ERROR` as an expression statement followed by a labeled
  statement. A label after a block or groovydoc comment, on the line
  after a bare statement (`result`, then `/* note */ where: x`), now
  takes the same shape; v0.5.0 kept two statements there. After a
  comma, an argument is now a full expression. On the same line,
  `foo 1, 2 + 3` is one chain whose last argument is `2 + 3`; v0.5.0
  ended the chain at `2` and started the statement `+ 3`. The last
  argument also continues onto a next line that starts with `(`, `{`,
  `-`, `+`, `++` or `--`, where v0.5.0 started a new statement:
  `foo 1, 2` followed by `(a)` parses as the call `2(a)`, and followed
  by `++x` as `2++` and then `x`. A next line that starts with `[` is
  still a new statement (#63); `SPECIFICATION.md` §6.8 lists every
  case. (#57)
- **BREAKING:** a `[` that starts a new line no longer subscripts the
  expression before it. `x` followed by `[a]` on the next line was a
  `subscript_expression` at v0.5.0 and is now an expression statement
  followed by a `list_literal`. The rule also holds inside `(…)` and
  `[…]`, where Groovy ignores newlines, so a subscript or array type
  continued on a new line there (`foo(a` then `[0])`) parsed at v0.5.0
  and is now an `ERROR`; see `SPECIFICATION.md` §6.9. (#63)
- **BREAKING:** an untyped `final` parameter (`def f(final a)`) no
  longer reports `final` as its `type`; the `formal_parameter` has only
  a `name`. (#64)
- **BREAKING:** a typed local declaration with a comment between it
  and the statement before it is now a `local_variable_declaration`
  (see Fixed below), which changes trees for input that parsed without
  an `ERROR` at v0.5.0. `foo()`, then `// note`, then `String s = 'a'`
  on its own line was the expression statement `String` followed by
  the assignment `s = 'a'`. (#87)
- **BREAKING:** a method declaration with a body, outside a class body
  and with a comment between it and the statement before it, is now a
  `method_declaration` (see Fixed below), which changes trees for
  input that parsed without an `ERROR` at v0.5.0. `foo()`, then
  `/* note */`, then `def skip(task) { true }` was a variable `skip`
  followed by a cast of a closure; `String call() { 'x' }` in the same
  position was the expression `String` followed by a call with a
  closure. (#87)
- **BREAKING:** in a class body, a field with a modifier and no
  initializer is no longer split when a comment follows it and the
  next member is a method or starts with a modifier (see Fixed below),
  which changes trees for input that parsed without an `ERROR` at
  v0.5.0. `private int v // one` followed by `private String w` was a
  field named `int`, a field of type `v` named `private`, and a field
  `String w`; it is now the two fields. The same holds before a
  constructor or method with a modifier (`public A() { }`,
  `static m() { }`), a `static { }` block and a nested type, and
  before a method with a return type (`String m() { }`). (#87)
- **BREAKING:** comment-like text in a string body is now part of a
  `string_fragment` (see Fixed below), which changes trees for input
  that parsed without an `ERROR` at v0.5.0. `"${dir}/**/*.xml"` and
  `"/* c */ d"` held a `groovydoc_comment` or a `block_comment` in
  place of that text, and `//` at the start of a `"""…"""` body, or
  directly after an interpolation in one, was a `line_comment`. In a
  slashy string the comment could
  also take the closing `/`, so the string now ends earlier, at its
  first unescaped `/` as in Groovy: `/$a/**/b/` was one string that
  held a `groovydoc_comment`, and is now `/$a/ ** /b/`. Where the rest
  of the line is not an expression, as in `/$a/*c*//`, input that
  parsed at v0.5.0 is now an `ERROR`. (#87)
- **BREAKING:** in a dollar-slashy string, a run of slashes before `$`
  now ends at the closing `/$` (see Fixed below), which changes trees
  for input that parsed without an `ERROR` at v0.5.0. The string can
  end earlier: `$/a//$//$` was one string, and is now the string
  `$/a//$` followed by the line comment `//$`. Text that ends in two
  or more slashes has more `string_fragment` children: `$/a///$` was
  one fragment, `a//`, and is now `a` and one fragment for each slash.
  (#87)
- **BREAKING:** outside a class body, a method declaration with no
  body now gives way to statements (see Fixed below), which changes
  trees for input that parsed without an `ERROR` at v0.5.0. `x`
  followed by `foo()` on the next line, and `println foo()` on one
  line, were a `method_declaration` whose return type was `x` or
  `println`. Each is now an expression statement followed by a call.
  (#87)
- Six external scanner tokens are added: `_command_named_key` (#57),
  `_subscript_open` (#63), `_string_body` (#87), `_error_sentinel`
  (#57, #63), and `_cast_close` and `_not_in` (see Fixed below).
  `_command_named_key` is an empty token; the key after it is an
  ordinary `identifier`. One named node is added,
  `synchronized_statement` (#76), and one field, `lock`. Most symbol
  ids above 69 in the generated parser change, `SYMBOL_COUNT` goes
  from 308 to 321 and `FIELD_COUNT` from 44 to 45, so field ids after
  `lock` change too. In `node-types.json`: `command_chain`'s
  `argument` field lists every expression kind and `named_argument`;
  `trait_declaration` and `enum_declaration` gain a `super_interfaces`
  child; `closure_parameter` gains a `default` field, `annotation`
  children and `varargs_type` in its `type` field;
  `local_variable_declaration`, `import_declaration` and
  `package_declaration` gain `annotation` children; and the anonymous
  operator of `membership_expression` is of kind `!in`.
  (#57, #63, #65, #76, #77, #79, #87)
- **BREAKING:** `def` or `final` on a closure parameter after the
  first (`{ String k, def v -> }`, `{ a, final b -> }`) used to parse
  with no `ERROR`, the word read as the parameter's `type` (a
  `type_identifier` with the text `def` or `final`). It is now the
  anonymous keyword and the parameter has no `type`, as a `def` local
  or `for` variable has. (#79, #83)
- **BREAKING:** a `$` interpolation in a GString reads non-ASCII
  letters (see Fixed below), which changes trees for input that
  parsed without an `ERROR` at v0.5.0, in all four GString forms.
  Three shapes change: `"値は$名前です"` was literal text and now
  holds a `gstring_dollar_interpolation`; in `"$count件"` the
  interpolated name grows from `count` to `count件`; and in `"$a.名"`
  the path grows from `a` to `a.名`. (#78, #82)
- **BREAKING:** in a class body, a method with modifiers and a return
  type that follows a comment is a `method_declaration` (see Fixed
  below), which changes trees for input that parsed without an `ERROR`
  at v0.5.0. `void a() { }`, then `// note`, then
  `public void run() { }` gave a `field_declaration` named `void` and
  a `constructor_declaration` named `run`. (#84, #85)
- **BREAKING:** a member directly after the trailing comma of an
  enum's constants is a member (see Fixed below), which changes trees
  for input that parsed without an `ERROR` at v0.5.0. `A, B,` then
  `private final int weight` gave a third `enum_constant` named
  `private` and a field `final int weight`; it is now two constants
  and the field. (#88, #89)
- **BREAKING:** a cast no longer takes its operand from the next line
  where a statement could end, and on one line it wins wherever it
  parses (see Fixed below). Both change trees for input that parsed
  without an `ERROR` at v0.5.0. `x = (a)` followed by `foo()` on the
  next line was a `parenthesized_type_cast` whose value was the call;
  it is now a `parenthesized_expression` and a second statement.
  `x = (a)` followed by `-1` was a cast of `-1` and is now the
  subtraction `(a) - 1`. `x = (int) c`
  followed by `b = 1` on the next line was the parenthesized
  expression `(int)` and a declaration `c b = 1`; it is now a cast
  and an assignment. (#83)
- **BREAKING:** `def` and `var` on a method or constructor parameter,
  a `catch` parameter and a `try` resource are keywords instead of a
  type name, which changes trees for input that parsed without an
  `ERROR` at v0.5.0. `def f(def a)`, `catch (def e)` and
  `try (def r = open())` no longer have a `type` (a `type_identifier`
  with the text `def`); the node has only a `name`. An untyped `final`
  on a `catch` parameter or a resource (`catch (final e)`,
  `try (final r = open())`) was read as a type too and is now a
  modifier. Closure parameters changed the same way in #79. (#83)
- **BREAKING:** in a `"""…"""` string, a quote directly before a `$`
  is now a `string_fragment` of its own (see Fixed below), which
  changes trees for input that parsed without an `ERROR` at v0.5.0.
  `"""say "$name"""` was one `string_fragment`; it is now two, the
  text before the quote and the quote itself, followed by a
  `gstring_dollar_interpolation`. A quote before a literal `$`
  (`"""cost "$5" """`) splits the text the same way. (#100)

### Fixed

- `final` is now accepted on local variable declarations: `final String
  name = 'a'`, `final def n = 1`, the modifier-only form `final x = 1`
  (dynamically typed, as with `def`), and the blank form `final String
  name`. Previously a local starting with `final` was an `ERROR` node.
  (#56)
- Parenthesis-free calls now accept named arguments, as §5.5 specifies:
  `exclude group: 'a', module: 'b'`, `archiveArtifacts artifacts: 'x'`,
  GORM's `name blank: false, size: 1..100`. After the first argument,
  an argument may be any expression
  (`systemProperty 'k', System.getProperty('k')`, `assertEquals 'v', a + b`),
  and a named argument may start on the line after a comma. A same-line
  key is lexed by a new external scanner token, `_command_named_key`,
  which only matches when `:` follows on the same line, so `Type name = …`
  declarations and a bare statement followed by a Spock label on the next
  line parse as before, unless a block or groovydoc comment stands
  before the label (see Changed above). String keys (`foo 'a': 1`) are
  not accepted yet. (#57)
- A `[` that starts a new line now begins a list or map literal instead
  of subscripting the previous statement, so a Grails controller action
  ending in `[model: value]` parses. A `[` on the same line as the token
  before it, with or without spaces or tabs between them, parses as
  before. Implemented as an external scanner token, `_subscript_open`.
  (#63)
- `final` is now accepted on method and constructor parameters (`def f(final String a)`). (#64)
- Trait declarations accept an `implements` clause (`trait T implements A, B`). (#65)
- A typed local declaration after a comment is no longer split in two.
  `int y = 1 // note` followed by `String s = 'a'` parsed the second
  line as the expression `String` and the assignment `s = 'a'`: both
  readings are valid, nothing ranked them, and a comment changed which
  one won. The typed forms of `local_variable_declaration` now carry a
  dynamic precedence. This also keeps `final String s = 'a'` whole
  after a comment, and the blank `final String name` whole when a
  comment or an assignment follows it. (#87)
- A method declaration with a body, outside a class body, is no longer
  split when a comment precedes it. After a comment,
  `String call() { 'x' }` parsed as the expression `String` and the
  call `call() { 'x' }`, and `def skip(task) { true }` as a variable
  and a cast of a closure. A method with a body now carries a dynamic
  precedence. It is above the blank final local's, so that
  `final String f(a) { }` stays a method, and below the typed field's,
  so that `private final int v` followed by a constructor is still a
  field and a constructor. (#87)
- In a class body, a field with a modifier and no initializer is no
  longer split when a comment follows it. `private final int v`, then
  `/** doc */`, then `public A(int v) { }` parsed as a field named
  `int`, a field of type `v` named `public`, and a constructor; two
  commented fields in a row (`private int v // one`, then
  `private String w`) were split the same way. A keyword is an
  identifier wherever it has no action, so both readings held one
  typed field and nothing ranked them. A field with modifiers only now
  carries a negative dynamic precedence. (#87)
- A bare identifier followed by a call is no longer a method
  declaration with no body. `result` followed by `noExceptionThrown()`
  on the next line, and `println foo()` on one line, parsed as a
  method whose return type was the first word. A method with no body
  now carries a negative dynamic precedence, so it is chosen only
  where nothing else parses: in a class body, for abstract and
  interface methods. A script method with no body (`String foo()`),
  which Groovy rejects, now parses as two expression statements. (#87)
- The scanner no longer emits a comment inside a string body.
  `"//div[@id]"`, `" // c"` and `"${a}//b"` were an `ERROR`, because a
  line comment took the closing quote. `"${dir}/**/*.xml"`,
  `"/* c */ d"` and a `"""…"""` body that starts with a `//` line
  parsed with a comment node in place of the text. The bodies of `"…"`, `"""…"""`, `/…/` and
  `$/…/$` now reference a new external token, `_string_body`, which is
  never emitted; where it is valid the scanner emits nothing. A
  comment inside `${…}` is still a comment. (#87)
- A dollar-slashy string whose text ends in a slash now parses:
  `$/a//$` holds `a/`. It was an `ERROR`, because the text token took
  the slash that the closing `/$` needs. A slash directly before the
  closer is now a `string_fragment` of its own. (#87)
- A line escape, `\` at the end of a line, is now whitespace, as in
  Groovy's lexer; it was an `ERROR`. The line after it continues the
  current one, so `x \` followed by `[0]` is a subscript and `foo \`
  followed by `a: 1` is a named-argument call. A `\` before a lone
  `\r` is still an `ERROR` when more input follows. At the very end of
  the file a `\`, alone or with a `\r` after it, is skipped with no
  `ERROR`. (#87)
- A method with modifiers and a return type that follows a comment in a
  class body (`// note` then `public void run() { }`) is a
  `method_declaration` again. Since #61 it could parse, with no `ERROR`,
  as a modifier-only `field_declaration` named after the return type
  (`public void`) followed by a `constructor_declaration` named after the
  method. The modifier-only field now loses to any other complete
  reading; `static TTL = 900` still parses as a field. (#84, #85)
- An enum member directly after a trailing comma (`A, B,` then
  `private final int weight`), which Groovy accepts up to 4.x, is a
  member again. Since v0.4.0 it parsed, with no `ERROR`, as an extra
  `enum_constant` named after the first modifier (`private`), followed
  by the member without that modifier. The member has to start with a
  modifier, `def` or a declaration keyword. Groovy also takes a member
  that starts with a type or a name there (`A,` then `String x`, or
  `E(int w) { }`); those are still a parse error, see
  `docs/divergences-from-spec.md` §13. A constructor needs an access
  modifier (`A,` then `private E(int w) { }`); `static f() { }` in
  that position is a method. (#88, #89)
- Identifiers accept non-ASCII letters, as Groovy's lexer does (any
  Java letter): `void test名前を確認する() { }`, `def café = 1`.
  A name starts with a character of the Unicode general categories L,
  Nl, Sc or Pc and continues with those or Nd, Mn or Mc, which is the
  lexer's own rule (`€uro` is a name, `a·b` is not); before,
  the first non-ASCII letter was an `ERROR`, and in a method name it
  took the method with it. A `$` interpolation in a GString reads the
  same characters, except `$`, so `"値は$名前です"` now interpolates
  `名前です` where it was literal text with a `$` before; Changed
  above lists the trees this alters. The contextual `record`
  keyword accepts a non-ASCII record name, and a named argument in a
  parenthesis-free call takes a non-ASCII key (`foo 名前: 1`): the
  scanner token `_command_named_key` is now empty and the in-grammar
  lexer reads the key. No symbol is added. (#78, #82)
- Closure parameters take what a method parameter takes: `def`
  (`{ def b -> }`), `final`, annotations, varargs
  (`{ String... args -> }`) and a default (`{ a = 1 -> }`,
  `{ String prefix, int width = 8 -> }`). Each was an `ERROR` before.
  `closure_parameter` gains a `default` field, and its `type` field
  can hold a `varargs_type`. A closure body that opens with a
  declaration (`{ def a = 1 }`, `{ final def a = 1 }`) is still a
  `local_variable_declaration`. The parameter list and such a body
  share their first tokens, and the parser follows both until the `->`
  or the first token that only one accepts. A closure or block that
  opens with a declaration of two or more names therefore costs 15% to
  60% more to parse than at v0.5.0, and about twice for a long list
  (#74); a declaration of one name and an assignment cost the same as
  before. On input with an error, a closure can now recover as a
  parameter list with a missing `->` where it had an `ERROR` node:
  `{ x = 1, y }`. (#79, #83)
- An enum constant named `sealed` or `trait` is a constant again
  (`enum E { trait, B }`, `enum E { sealed }`). The first constant so
  named has been an `ERROR` since v0.5.0, where the two words became
  keywords at member start. `enum_constant` now takes either keyword
  as its name, as an `identifier`. A `sealed class` or `trait T { }`
  in the same position is still a declaration; see
  `docs/divergences-from-spec.md` §12. (#68, #89)
- A line comment ends at a lone `\r`, as it does for Groovy's lexer.
  It ended only at `\n`, so in a file with CR line endings
  `foo() // c` followed by `bar()` lost `bar()` to the comment, with
  no `ERROR`. A line comment between `record Name` and `(` had the
  same problem. In a CRLF file the `\r` is no longer part of the
  `line_comment` node, which is one byte shorter. (#100)
- A C-style cast follows Groovy's two rules. Where a line break can
  end a statement, its operand starts on the line of the `)`:
  `x = (a)` followed by `foo()` on the next line parsed, with no
  `ERROR`, as a cast of `foo()`, because newlines are whitespace to
  the grammar. A new external scanner token, `_cast_close`, is the `)`
  that has something after it on its line; a line escape continues
  the line and a line comment ends it. A cast that closes with a plain
  `)` loses to any other reading and is still a cast where there is
  none, as in `foo((String)` followed by `x)`. Inside `(…)` and `[…]`
  Groovy reads a cast on every later line; here `foo((a)` followed by
  `- b)` is a subtraction (#99). On one line the cast now wins
  whenever it parses (`(a) + b`, `(a) [0]`, `(a) (b)`), as in
  Apache's grammar. The two readings were unranked
  before, so the result depended on the surrounding code, and
  `a = (int) y` at the start of a closure lost the cast when the next
  line began with a literal. (#83)
- `final` before a type is accepted on a `catch` parameter and on a
  `try` resource (`catch (final Exception e)`,
  `try (final Foo r = open())`); both were an `ERROR`. A closure
  parameter takes `var` (`{ var x -> x }`), which was an `ERROR` too.
  Method and closure parameters now share one definition. (#83)
- Annotations are accepted on a local or script variable declaration,
  on an `import` and on a `package` declaration: `@Field def x = 1`,
  `@SuppressWarnings('unchecked') List<String> xs = []`,
  `@Grab('g:a:1')` before `import a.B`, and the Jenkins form
  `@Library('shared') _`, which declares the placeholder `_`. Each was
  an `ERROR`. After annotations alone only the name `_` is accepted.
  Inside a closure an annotated declaration recovered, after #79, as a
  parameter list with a missing `->`; it is now a
  `local_variable_declaration`. (#100)
- An interpolation directly after an embedded quote in a `"""…"""`
  string is an interpolation: `"""say "$name"""` and
  `"""a "${b}" c"""` parsed, with no `ERROR`, as plain text, because
  the text token took the `$` together with the quote before it. (#100)
- `!in` directly before a bracket parses: `a !in[1, 2]`, `a !in(b)`
  and `a !in{ … }` were an `ERROR`, because the operator token
  included the character after `!in`. `!in` is now an external scanner
  token, `_not_in`, which reads that character and leaves it. The
  anonymous `!in` node no longer includes the whitespace after it.
  (#100)
- Queries. `highlights.scm`: `synchronized` as a statement is
  `@keyword`, where every use of the word was `@keyword.modifier`;
  `->` and the `...` of a varargs type are `@operator`. `tags.scm`: a
  generic or qualified super type is a reference
  (`implements Supplier<String>`, `extends a.b.Base`); only a bare
  name was. `locals.scm`: a typed local (`String s = 'a'`), a blank
  final and a `try` resource are definitions, and an enum body, a
  record and a `try` statement are scopes. (#76, #77, #79)
- The Python source distribution builds. It held `src/parser.c` and
  `src/scanner.c` without `src/tree_sitter/*.h`, so installing from
  the sdist failed with `tree_sitter/parser.h: No such file or
  directory`, and it held no queries. A `MANIFEST.in` adds both.
  Wheels are built from a checkout, which has both. (#100)

## [0.5.0] - 2026-11-10

### Added

- Class, interface, trait, record, `@interface`, and enum bodies now
  accept instance initializer blocks — `class C { { init() } }`. The
  block is a bare `block` child of `class_body` or `enum_body`, next
  to the existing `static_initializer`; previously it closed the
  class early on a `MISSING "}"`. In an enum the block may follow
  `;`, another member, or an empty constant list, but not a constant
  directly (`A\n{ … }` stays an error; write `A;`). A block on the
  line after a field *with* an initialiser (`int x = 5\n{ … }`) is
  not an initializer but that expression's trailing closure, the
  reading Apache's grammar gives it too — end the field with `;` to
  get the initializer; see `docs/divergences-from-spec.md` §11.

  The block also follows a field with no initialiser on the same
  line, so `int x { … }` and a record's compact constructor
  (`public Name { … }`, unsupported) parse as a field plus a block
  with no `ERROR`, where v0.4.0 reported one. (#68)
- Nested type declarations are now class members: `static class
  Config { … }`, a member `interface`, `trait`, `record` or
  `@interface`, at any depth, in class, interface, trait, record,
  `@interface`, enum, and anonymous class bodies, joining the nested
  enum. Previously `_class_member` had no type-declaration
  alternatives, so a nested type was an `ERROR` node and broke the
  rest of the class body. They use the top-level node kinds
  (`class_declaration` and so on) as children of `class_body` /
  `enum_body`. This reserves `sealed` and `trait` at member start;
  see Changed below. `record` is not reserved: its keyword comes
  from the scanner only before a record header (see Fixed below).

  No node kind is added by either entry. (#60, #68)
- `record_declaration` takes the rest of Apache's `classDeclaration`
  shape: modifiers before `record` (`public record P(int x) { }`,
  `static record Q(int y) { }` as a member), type parameters
  (`record Pair<A, B>(A left, B right)`, in the `type_parameters`
  field), and an `implements` clause (a `super_interfaces` child, as
  on classes). Each was an `ERROR` or a silent `method_declaration`
  before. `node-types.json` adds `type_parameters` to the fields and
  `super_interfaces` to the children of `record_declaration`.
  (#60, #68)
- Enum constants accept annotations: `enum E { @Deprecated A, B }`
  was an `ERROR` that took the rest of the body with it. The
  annotations are `annotation` children of `enum_constant`, before
  its `name`; `node-types.json` adds them to its children. (#60, #68)

### Changed

- **BREAKING:** a `{` after the arguments of `new T(...)` is now
  always an anonymous class body (see Fixed below), which changes
  trees for input that parsed without an error at v0.4.0.
  `new Foo() { }`, and any body made only of members that are also
  valid statements, move from `method_invocation` / `closure` to
  `object_creation_expression` / `class_body`. Calls such as
  `new Foo().bar { }` and `foo() { }` keep their trailing closure.

  Groovy is narrower about the body than this grammar: it calls a
  closure on the new object when the `{` starts a later line (outside
  parentheses and brackets) or holds something that cannot be a class
  body. `new ArrayList()\n{ list.add(1) }` and `new T() { x -> x }`
  therefore move from a `method_invocation` with a `closure` to a
  `class_body` with an `ERROR` in it; write
  `new ArrayList().call { … }` to keep the call. See
  `docs/divergences-from-spec.md` §11. (#68)
- **BREAKING:** `sealed` and `trait` are keywords wherever a class
  member can start, as they already were at statement start; the
  nested type declarations above need them there. Input that parsed
  at v0.4.0 and is now an `ERROR`: a field named `sealed` or `trait`
  after a modifier (`private boolean sealed`,
  `static final String trait = "x"`), a member that opens with one
  of the words (`static trait = 1`, `sealed() { }`), and a first
  enum constant so named (`enum E { trait, B }`). `boolean sealed`
  and `String trait`, with no modifier, still parse; see
  `docs/divergences-from-spec.md` §12. (#68)
- **BREAKING:** a record header inside a class, interface, trait,
  enum or anonymous class body is now a `record_declaration`.
  `record Point(int x) { }` as a member parsed at v0.4.0, with no
  `ERROR`, as a `method_declaration` named `Point` with return type
  `record`; so did a method whose return type is spelled `record`
  (`record foo() { }`, `private record bar() { }`), which takes the
  same tree now. `record_declaration` joins the child lists of `class_body`
  and `enum_body` in `node-types.json`. In the other direction,
  `record` with a line break or a line comment before the name, then
  `Point(int x) { }` on the next line, was a `record_declaration` at
  v0.4.0 and now reads as a method declaration with return type
  `record`; Groovy rejects that input (`SPECIFICATION.md` §6.7).
  (#60, #68)
- **BREAKING:** `def f = (Foo) new T()` after another class member
  is now a `parenthesized_type_cast` around the creation (see Fixed
  below). At v0.4.0 that input parsed without an `ERROR` as a field
  `f = (Foo)` followed by a method `T()`; the tree changes for input
  that parsed. (#68)
- The `record` keyword is an external-scanner token
  (`alias($._record_keyword, 'record')`) instead of a grammar
  literal. The anonymous `record` node and the `"record" @keyword`
  capture are unchanged, but the generated parser's terminal symbol
  ids 55–150 shift down by one and `sym__record_keyword` is appended
  at 150; non-terminal ids, `SYMBOL_COUNT`, and field ids are the
  same as at v0.4.0. Other entries in this release that describe
  ids as unchanged refer to their own change only. (#60, #68)

### Fixed

- `record` is usable as a name. The word was a keyword wherever a
  record declaration could start, so `record.key()` and `record = 5`
  at statement start were `ERROR`s, and making records class members
  would have done the same to `private Record record`,
  `public void record() { }` and a first enum constant named `record`.
  The keyword now comes from the external scanner, which emits it
  only when a name follows on the same line and `(` after that, with
  comments allowed in between and a line break allowed before the
  `(` (`SPECIFICATION.md` §6.7); everywhere else the word is an
  identifier. One residual, an `ERROR` at v0.4.0 too: a command-style
  call whose argument is a call (`record Point(1, 2)`) reads as a
  record header. (#60, #68)

- Anonymous inner classes now parse. `new Runnable() { … }` had no
  production. At v0.3.0 the body was cut loose from the creation: in
  the reported field initialiser it spilled into the enclosing class
  behind an `ERROR`, in a script or method body it became a separate
  closure statement with no `ERROR` at all. Since v0.4.0 the
  trailing-closure call of #49 claimed it, yielding a
  `method_invocation` whose `closure` held the class members — a tree
  that looked like a valid call and carried an `ERROR` only when a
  member was not also a valid statement.
  `object_creation_expression` gains an optional `body` field holding
  a `class_body`, and every `{` that follows the arguments of
  `new T(...)` is read as that body. Together with the instance
  initializer above this covers double-brace initialisation
  (`new HashMap() {{ put(1, 2) }}`). Trees change for some input
  that already parsed without an error; see Changed above.

  Still unsupported, as before: enum constants with an anonymous
  class body (`A { … }`) and qualified creation
  (`outer.new Inner(...)`). Symbol and field ids are unchanged by
  this fix. (#68)

- A cast in front of `new` in a field initialiser that follows
  another member now parses as a cast. `def f = (Foo) new T()` after
  an earlier field or method was split, with no `ERROR`, into a field
  `f = (Foo)` and a method `T()` with return type `new`, whenever the
  arguments could pass for parameters (none, or bare names); the
  tree change is listed under Changed above. `new` is
  now a reserved word: it is the keyword everywhere except where a
  property or method name can stand (`foo.new`, `def new() { }`, and
  with them `def new = 1`). Input that used it as any other
  identifier (`def f(new) { }`, `import a.new.B`, `x as new`) is now
  an `ERROR`; none of that is valid Groovy. A statement that opens
  with `outer.new Inner()` now reads as a property access and a call,
  as it already did after `=`, instead of a method declaration.
  Symbol and field ids are unchanged. (#68)

## [0.4.0] - 2026-11-09

### Added

- `method_invocation` now accepts a trailing closure as the call's last
  argument, with or without a parenthesised argument list —
  `list.find { it > 5 }` and `list.inject(0) { acc, x -> acc + x }` both
  reduce to a single `method_invocation` node (`closure` field alongside
  the optional `arguments` field). Previously any call written this way
  outside of a bare statement-level command chain (`§5.5`) required
  parentheses around the closure or failed to parse. `arguments` is
  no longer a required field in `node-types.json`, so code that
  assumes every `method_invocation` has one must handle its absence.
  (#49)

### Changed

- **BREAKING:** raised the Rust crate's minimum supported Rust version
  (`rust-version`) from 1.85 to 1.90, the floor that `tree-sitter`
  0.27 and `tree-sitter-language` 0.1.8+ declare. Consumers on 1.85–1.89
  must stay on an earlier release of this crate. The `tree-sitter`
  dev dependency moves from 0.26 to 0.27; the `tree-sitter-language`
  requirement is unchanged at `0.1.7`. (#66)
- Bumped `tree-sitter-cli` from 0.26.13 to 0.27.2 and raised the
  declared range in `package.json` to `^0.27.2` so it matches the CLI
  the committed parser source is generated with. It is a dev
  dependency, so nothing changes for downstream consumers; regenerating
  under 0.27.2 leaves `src/parser.c` and `src/tree_sitter/*.h`
  byte-identical to the committed sources (still ABI 15). (#66)

### Fixed

- A nested `enum` inside a class body (`static enum EventType { ... }`,
  a common Groovy domain-model idiom for entity fields backed by a
  fixed set of values) now parses as a class member. Previously
  `_class_member` had no enum_declaration alternative at all, so `enum`
  silently misparsed as a field_declaration's type name and everything
  after it in the class body fell apart. (#49)
- `catch_formal_parameter`'s type is now optional — Groovy allows a
  bare `catch (e) { … }`, implicitly typed as Exception. Previously a
  type was mandatory. (#49)
- Constructor calls now accept the diamond operator (`new ArrayList<>()`,
  `new HashMap<>()`). Ordinary types and nested type arguments still
  require non-empty argument lists, so `List<> xs` and
  `new HashMap<String, List<>>()` remain errors. (#49)
- `field_declaration` now accepts a modifier-only field with no `def`
  and no type (`static MEDIUM_TTL = 900`), inferring the field's type as
  Object — a common static-constant idiom. Previously a field required
  `def` or an explicit type even when a modifier alone already
  distinguishes it from a bare expression statement. (#49)
- Typed fields retain their type when followed by a constructor rather
  than being split into a modifier-only field and a method declaration
  (`private final int value` followed by `C() {}`). (#49)
- `enum_declaration` now accepts modifiers (`public enum`, `static enum` —
  the latter for enums nested inside a class). Previously any modifier
  before `enum` was rejected. (#49)
- `for_in_statement` now also accepts the Java-compatible enhanced-for
  form Groovy allows side by side with its own `for (x in xs)`: `for
  (Type x : xs)` and `for (def x : xs)`. Only the `in` keyword form
  parsed before. (#49)
- `formal_parameter` now accepts annotations (`@PathVariable("id") Long
  id`, `@RequestBody Map body`). Spring/JPA-style annotated parameters
  are ubiquitous on framework-facing methods and previously misparsed
  the whole parameter list. (#49)
- Accept enum class members directly after the constants with no `;`
  between, as Groovy's own parser does (`classBody` in
  `GroovyParser.g4`, GROOVY-7773 / GROOVY-9306). Previously an enum
  such as

  ```groovy
  enum E {
      A(1), B, C(2)
      E(int v) { }
  }
  ```

  closed on a zero-width `MISSING "}"` after the last constant and the
  constructor was re-parsed as a top-level method invocation followed
  by a closure. The `;` and `,;` spellings keep parsing as before, a
  trailing `,` directly before a member stays an error as in Groovy,
  and an enum whose members carry a modifier or `def` no longer needs
  a leading `;` when it has no constants.

  Newlines are whitespace to this grammar, so where Groovy reads the
  line break the parser has to choose. After a bare first constant,
  `enum E { A\n E() { } }` is a constant followed by a constructor,
  not a method `E` returning type `A`. The same preference applies
  when there are no constants at all: `enum E { int f() { } }` reads
  as a constant `int` and a constructor `f`. Spell the member with a
  modifier (`private E() { }`), a `def`, or a leading `;` to make it a
  member — all three were already required at v0.3.0, which could not
  parse the member either way. Constants missing their comma
  (`enum E { A\n B }`) stay an `ERROR`, as in Groovy; at v0.3.0 they
  silently parsed as a field `A B`. Enum constants with an anonymous
  class body (`A { … }`) stay unsupported.

  Node kinds, symbol ids, and field ids are unchanged. (#55)

## [0.3.0] - 2026-08-30

### Changed

- **BREAKING:** the minimum supported Python is now 3.11. Wheels are
  tagged `cp311-abi3` (was `cp38-abi3`), the extension is compiled
  against the 3.11 limited API (`Py_LIMITED_API` 0x030B0000), and the
  `core` extra requires `tree-sitter~=0.26`.

  The previously declared 3.9 floor never worked. The committed parser
  is ABI 15, which needs the `tree-sitter` runtime at 0.25 or later,
  and every such release requires Python 3.10+. The newest runtime
  installable on 3.9 is 0.23.2, which rejects ABI 15 outright --
  `ValueError: Incompatible Language version 15. Must be between 13
  and 14` -- so `Parser` could never be constructed there, and
  `pip install dekobon-tree-sitter-groovy[core]` did not even resolve.

  Python 3.10 is a genuine removal rather than a correction: it works
  today with `tree-sitter` 0.26. It is dropped because it reaches end
  of life on 2026-10-31, which would make it a floor with two months
  of runway. (#42, #44)
- The Python build no longer requires the `wheel` package. `setup.py`
  takes `bdist_wheel` from `setuptools.command.bdist_wheel`, which
  setuptools has owned since 70.1; the `wheel` copy emitted a
  `FutureWarning` announcing its removal, and `wheel` is dropped from
  `build-system.requires`. (#42)
- Bumped `tree-sitter-cli` from 0.26.12 to 0.26.13 and the
  `tree-sitter` crate from 0.26.11 to 0.26.13, and raised the declared
  `tree-sitter-cli` range in `package.json` from `^0.26.9` to
  `^0.26.13` so it matches the CLI the committed parser source is
  generated with. Both are dev dependencies, so neither is installed
  for downstream consumers and no runtime version changes. The
  generated sources (`src/parser.c`, `src/tree_sitter/array.h`) are
  unchanged from the 0.26.12 output already committed. (#41)

### Fixed

- The vendored `src/tree_sitter/array.h` now compiles inside a C++
  translation unit. Its array macros cast their result through
  `decltype`, where previously the bare `void *` produced by its
  allocation helpers and swap temporary failed to convert to the
  array's element pointer type. C builds are unaffected — under C the
  cast expands to a no-op. This arrived as regenerated source in the
  `tree-sitter-cli` 0.26.12 bump. (#40)

## [0.2.2] - 2026-06-04

### Fixed

- Accept `;` as a terminator, separator, and empty statement / member
  in statement-list contexts (`source_file`, `block`, `closure`,
  `switch` `case`/`default` bodies) and member-list contexts (class /
  interface / trait / enum bodies). This covers the common
  abstract-method (`interface Foo { void bar(); }`), field
  (`int x = 1;`), and semicolon-terminated statement
  (`void f() { x(); }`) forms that previously produced `ERROR` nodes;
  at script top level the last also cascaded into a misparse of the
  whole method. (#20)
- Accept a `;` terminating a braceless single-statement body: keep the
  `else` attached after an `if` consequence ends in `;`
  (`if (x) a(); else b()`), allow a `do`-`while` body to end in `;`
  (`do a(); while (c)`), and accept a trailing `;` on a `switch`
  arrow-case body (`case 1 -> foo();`). (#20)
- Accept a typed or `def`/`var` variable declaration in the C-style
  `for` init clause (`for (int i = 0; …)`, `for (def i = 0; …)`), which
  previously errored. (#20)

## [0.2.1] - 2026-05-24

Maintenance release. No grammar, scanner, or query changes; this
ships the regenerated parser source from a newer `tree-sitter-cli`
plus dependency bumps. AST shape, query captures, and binding
public surface are all identical to `0.2.0`.

### Changed

- Bumped `tree-sitter-cli` from 0.25.10 to 0.26.9 (dev dependency).
  `src/parser.c` and `src/tree_sitter/array.h` have been regenerated
  with the new CLI; the committed parser source matches what CI
  expects.
- Bumped the Rust `tree-sitter` runtime from 0.26.8 to 0.26.9
  (patch).
- Bumped `node-addon-api` from 8.7.0 to 8.8.0 (minor).
- Pinned several GitHub Actions to current major versions
  (`actions/upload-artifact` v7, `actions/download-artifact` v8,
  `pypa/cibuildwheel` 3.4.1, and the actions-minor-and-patch
  group).
- CI: aligned the regenerate workflow with the upstream
  `tree-sitter-grammars/template`, added GitHub Actions to the
  CodeQL language matrix, and closed the release-pipeline known
  gaps surfaced after `v0.2.0`. Dependabot is now allowed to bump
  `tree-sitter-cli`.

## [0.2.0] - 2026-05-19

First version published through the automated release pipeline to
all three registries (`@dekobon/tree-sitter-groovy` on npm,
`dekobon-tree-sitter-groovy` on crates.io and PyPI). Grammar,
scanner, queries, and bindings are unchanged from `0.1.0`.

`0.1.0` was a manual bootstrap upload to crates.io, required
because crates.io does not yet support pending Trusted
Publishers — the crate had to exist before the publisher entry
could be registered. It has been yanked. npm and PyPI never saw
`0.1.0`.

### Changed

- **BREAKING: Python binding minimum is now 3.9** (previously 3.8).
  setuptools >= 82.0.1 — required by the build — dropped Python
  3.8 support, so the cibuildwheel matrix and `requires-python`
  both move to 3.9. abi3 wheels at the 3.9 floor still cover 3.9
  through 3.13+. No Python 3.8 user could have consumed this
  package via PyPI (the registry never had `0.1.0`), but Python
  3.8 users building from sdist need to either pin to a pre-`0.2.0`
  source release or upgrade their Python.
- **Release pipeline now uses Trusted Publishing on all three
  registries.** crates.io, npm, and PyPI all authenticate via
  short-lived OIDC tokens minted by GitHub Actions; no long-lived
  `NPM_TOKEN` or `CARGO_REGISTRY_TOKEN` secret on the repo.
- **`bindings/rust` packaging hardened**: the `[package].include`
  patterns in `Cargo.toml` are now anchored to the package root,
  so bare basenames like `LICENSE-MIT` no longer match
  same-named files under `node_modules/`. Published tarball
  contents are unchanged (34 files, 5.7 MiB).

## [0.1.0] - 2026-05-18

Initial release. Purpose-built Groovy 2.x--4.x parser with complete
operator coverage, six language bindings, and full editor query support.

### Added

#### Grammar

- **Complete operator coverage** -- every Groovy operator emits a
  distinct named node (no MISSING-child inspection or tree-shape
  heuristics required). Operators:
  - Arithmetic (`+` `-` `*` `/` `%` `**` `++` `--`)
  - Relational / equality (`<` `<=` `>` `>=` `==` `!=` `<=>`)
  - Identity (`===` `!==`)
  - Regex (`=~` find, `==~` match)
  - Logical (`&&` `||` `!` `==>` implication)
  - Bitwise (`&` `|` `^` `~` `<<` `>>` `>>>`)
  - Conditional (`? :` ternary, `?:` Elvis)
  - Assignment (`=` `+=` `-=` `*=` `/=` `%=` `**=` `<<=` `>>=`
    `>>>=` `&=` `^=` `|=` `?=`)
  - Range (`..` `..<` `<..` `<..<`)
  - Member access (`.` `?.` `??.` `*.` `.&` `.@` `::`)
  - Subscript (`[]` `?[]`)
  - Spread (`*args` `*:`)
  - Membership (`in` `!in` `instanceof` `!instanceof`)
  - Coercion (`as`)
  - Power (`**`, right-associative, Apache precedence)
- **Declarations** -- `class`, `trait`, `interface`, `enum`, `record`
  (Groovy 4), `sealed` / `non-sealed` / `permits`, `@interface`,
  method, constructor, field, `static` initializer, formal parameters
  (typed, default-valued, varargs), throws clauses, generics
  (`type_parameters`, `type_arguments`, bounded wildcards),
  inheritance (`extends`, `implements`), modifiers, annotations,
  package, import (static, wildcard, aliased).
- **Statements** -- `if` / `else`, `while`, `do-while`, `for`
  (C-style and for-in), `switch` (classic and Groovy 4 arrow form),
  `try` / `catch` / `finally` (multi-catch, try-with-resources),
  `return`, `break`, `continue`, `throw`, `assert`, `yield`,
  `labeled_statement`, `pipeline` (Jenkins).
- **Expressions** -- numeric literals (all bases, underscores, type
  suffixes), string literals (single-quoted, triple-single, double-
  quoted, triple-double, slashy regex, dollar-slashy), GString
  interpolation (`$name` and `${expr}` as parse-tree children),
  boolean / null literals, closures (typed and untyped parameters),
  list / map literals (trailing commas), object creation (`new`),
  parenthesized type cast, command chains (multi-argument),
  method invocation (named arguments, spread arguments), quoted
  identifiers, parenthesized expressions.
- **Types** -- `type_identifier`, `generic_type` (nested, bounded
  wildcards), `qualified_type`, `array_type`.
- **External scanner** (`src/scanner.c`) -- handles slashy-string vs.
  division disambiguation, GString interpolation boundaries, line /
  block / Groovydoc comment dispatch, and scaffolding for automatic
  semicolons and label colons.
- **Shebang** support (`#!` at file start).

#### Queries

- `highlights.scm` -- full syntax highlighting with Groovy 4
  contextual keywords (`var`, `record`, `sealed`, `non-sealed`,
  `permits`, `yield`).
- `locals.scm` -- scope, definition, and reference captures.
- `tags.scm` -- ctags-style definition / reference tagging.
- `indents.scm` -- indentation rules.
- `folds.scm` -- code folding regions.
- `injections.scm` -- slashy and dollar-slashy bodies inject as
  `regex`; Groovydoc injects as `javadoc`; SQL DSL pattern for
  `sql.execute """..."""`.

#### Bindings

- C, Go, Node.js, Python, Rust, and Swift bindings, all calling
  `tree_sitter_groovy()`.
- Node binding exposes `HIGHLIGHTS_QUERY` as a lazy property that
  reads `queries/groovy/highlights.scm` on first access, matching the
  Rust binding's `dekobon_tree_sitter_groovy::HIGHLIGHTS_QUERY`.
- Node binding no longer exports the deprecated `name` field
  (`require('@dekobon/tree-sitter-groovy').name` returns `undefined`).
  TypeScript declarations refreshed to match the current upstream
  `tree-sitter` CLI template.

#### Editor integration

- `ftdetect/groovy.lua` for Neovim filetype detection of extensions
  that Neovim's built-in detection misses (`*.gvy`, `*.gy`,
  `*.jenkinsfile`) plus the `Jenkinsfile.*` filename pattern common
  in repos with multiple pipelines.

#### Tests

- 63 corpus test files covering every operator family, statement
  type, and declaration form.
- 8 highlight assertion test files.
- 10 stress test files including real-world Jenkins pipeline and
  Gradle buildscript samples.
- Rust integration test (`parse_stress.rs`) asserting zero ERROR
  and zero MISSING nodes across all stress files.
- Dedicated regression test file (`regressions.txt`) pinning fixes
  for murtaza64/tree-sitter-groovy #16, #22, #36, #37, #39 and
  dekobon/big-code-analysis #246, #247.

#### Infrastructure

- CI workflow with multi-platform testing (Linux, Windows, macOS),
  ESLint, Clippy, cargo-deny, typos, taplo, markdownlint, and
  scanner-diff-gated fuzzing. Third-party actions are SHA-pinned.
  CI workflow runs with read-only `GITHUB_TOKEN` permissions.
- CodeQL analysis workflow for C, JavaScript, Python, Go, and Rust.
- Dependabot configuration for all six package ecosystems.
- `SPECIFICATION.md` -- authoritative grammar design document.
- `docs/divergences-from-spec.md` -- documented deviations from the
  specification with rationale.

### Known limitations

See [`docs/divergences-from-spec.md`](docs/divergences-from-spec.md)
for full details.

- Groovy 5 contextual keywords (`val`, `async`, `await`, `defer`)
  parse as plain identifiers (matches Groovy 2.x--4.x behavior).
- Typed local declarations require an initializer (`String x = ...`
  works; bare `String x` does not).
- `a\n+ b` joins as one expression rather than splitting into two
  statements.

[Unreleased]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/dekobon/tree-sitter-groovy/releases/tag/v0.1.0
