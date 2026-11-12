# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
  closure. Trees change inside a class body too: a field with a
  modifier and no initializer, then a comment, then a method
  (`private final int v`, `// note`, `String m() { }`) is now that
  field and that method. It was a field named `int`, a field named
  `String` of type `v`, and a constructor `m`. (#87)
- **BREAKING:** outside a class body, a method declaration with no
  body now gives way to statements (see Fixed below), which changes
  trees for input that parsed without an `ERROR` at v0.5.0. `x`
  followed by `foo()` on the next line, and `println foo()` on one
  line, were a `method_declaration` whose return type was `x` or
  `println`. Each is now an expression statement followed by a call.
  (#87)
- Three external scanner tokens are added: `_command_named_key` (#57),
  `_subscript_open` (#63) and `_error_sentinel` (both), so most symbol
  ids above 69 in the generated parser change and `SYMBOL_COUNT` goes
  from 308 to 313. Node names and field ids are unchanged. In
  `node-types.json`, `command_chain`'s `argument` field lists every
  expression kind and `named_argument`, and `trait_declaration` gains a
  `super_interfaces` child. (#57, #63, #65)

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
  precedence, and the typed field's rises from 1 to 2 so that
  `private final int v` followed by a constructor is still a field and
  a constructor. The two weights also fix a class body in which a
  comment stands between a field with a modifier and no initializer
  and the method after it. (#87)
- A bare identifier followed by a call is no longer a method
  declaration with no body. `result` followed by `noExceptionThrown()`
  on the next line, and `println foo()` on one line, parsed as a
  method whose return type was the first word. A method with no body
  now carries a negative dynamic precedence, so it is chosen only
  where nothing else parses: in a class body, for abstract and
  interface methods. A script method with no body (`String foo()`),
  which Groovy rejects, now parses as two expression statements. (#87)
- A line escape, `\` at the end of a line, is now whitespace, as in
  Groovy's lexer; it was an `ERROR`. The line after it continues the
  current one, so `x \` followed by `[0]` is a subscript and `foo \`
  followed by `a: 1` is a named-argument call. A `\` before a lone
  `\r` is still an `ERROR`. A `\` that ends the file is skipped with
  no `ERROR`. (#87)

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

[Unreleased]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/dekobon/tree-sitter-groovy/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/dekobon/tree-sitter-groovy/releases/tag/v0.1.0
