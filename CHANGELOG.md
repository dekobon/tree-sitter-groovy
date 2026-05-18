# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Swift bindings re-export the public C header via
  `publicHeadersPath: "bindings/c"` instead of carrying a duplicate.
- `pyproject.toml` adopts the PEP 639 SPDX `license = "Apache-2.0 OR MIT"`
  expression; build requirement bumped to `setuptools>=77`.
- `line_comment` is now an external-scanner token rather than an
  in-grammar regex. The in-grammar form partially consumed the
  leading `/` of `//`, `/*`, and `/**` before failing, which
  prevented the external scanner from seeing the `/`. Routing all
  three comment kinds through one scanner dispatcher fixes this.

### Fixed

- Quoted-name property/method highlights — `obj."some prop"` and
  `def "foo bar"() { … }` now receive `@property` / `@function`
  highlights.
- `x as java.util.Map` and `x instanceof java.lang.Number` now
  extend the qualified type through every `.identifier` segment
  instead of splitting as `(field_access (cast x as java.util)
  Map)`. The dedicated `_cast_type` / `_dotted_type` rule shifts
  each dot continuation at a precedence (`PREC.ACCESS + 1`)
  tighter than `field_access`. Closes the qualified-type half of
  divergence doc §5. (SPECIFICATION.md §3.2.2)
- `!instanceof` and `!in` now require the trailing-context
  predicate from SPECIFICATION.md §3.2.2 (Apache `NOT_INSTANCEOF`
  / `NOT_IN` lexer rules). Each is matched as a single token
  whose pattern includes a trailing whitespace (or `[ ( {` for
  `!in`), so `x !instanceofX` no longer mis-tokenises as
  `!instanceof` + identifier `X`. Regression tests pinned in
  `test/corpus/operators-instanceof.txt` and
  `test/corpus/operators-membership.txt`.

### Added

- `generic_type` covering `List<String>`, nested
  (`Map<String, List<Integer>>`), qualified bases
  (`java.util.List<String>`), and bounded wildcards
  (`? extends T`, `? super T`, bare `?`) in every position that
  accepts a `_type` (variable declaration, formal parameter,
  closure parameter, return type, `new`, parenthesized cast,
  `as` cast). Class / interface / trait declarations and method
  declarations also accept `type_parameters` /
  `method_type_parameters` (`class Box<T extends Comparable>`,
  `<T> T identity(T x)`, multi-bound
  `<T extends Number & Comparable>`).
  `test/corpus/declarations-generics.txt` pins the AST shape and
  `test/stress/generics.groovy` provides a zero-ERROR stress sample.
  Closes the generics gap in `docs/divergences-from-spec.md` §5.
  (SPECIFICATION.md §4 / §5.14)
- `closure_parameter` now accepts an optional `type` field, so
  `{ String s -> s }` and `{ List<String> xs -> xs }` parse with
  full type structure instead of as identifier-only parameters.
  Corpus tests added in `test/corpus/expressions-closure.txt`.
  (SPECIFICATION.md §5.8)
- `field_declaration` inside class / interface / trait bodies.
  Supports `def x = …`, `Type x`, `Type x = …`,
  `Type x = …, y = …`, generic-typed fields, and annotations /
  modifiers prefixes. Methods continue to win via higher
  precedence when `(` follows the name. Corpus tests in
  `test/corpus/declarations-fields.txt`; tags / highlights /
  locals queries updated to mark field-position declarators as
  `@property`, `@definition.field`, and `@local.definition.field`
  respectively. (SPECIFICATION.md §4)
- `constructor_declaration` as a distinct node kind for
  class-body constructors (`Foo()`, `Foo(String name)`,
  `private Foo() throws IOException { … }`). Tagged as
  `@definition.constructor`, highlighted as `@constructor`, and
  scoped as `@local.scope`. Corpus tests in
  `test/corpus/declarations-constructors.txt`.
  (SPECIFICATION.md §4)
- `static_initializer` for `static { … }` class-body blocks.
  Recognised as `@local.scope` and emitted with a `body` field
  pointing at the contained block. Corpus coverage in
  `test/corpus/declarations-constructors.txt`.
  (SPECIFICATION.md §4)
- `command_chain` now accepts multiple comma-separated arguments
  (`events 'passed', 'failed', 'skipped'`) — the dominant Gradle /
  Jenkins DSL invocation shape. (SPECIFICATION.md §5.5)
- Stress samples `jenkins_pipeline.groovy` and
  `gradle_buildscript.groovy` covering realistic DSL shapes
  (nested `pipeline { stages { stage { steps { ... } } } }`,
  `dependencies { implementation '...' }`,
  `tasks.named('test') { ... }`).
- Slashy regex `/.../` and dollar-slashy `$/.../$` now expose
  GString interpolation as parse-tree children:
  `gstring_dollar_interpolation` and `gstring_brace_interpolation`
  appear alongside `string_fragment` text segments. The external
  scanner's slashy emission now yields the opening `/` only (with
  a same-line lookahead confirming a closing `/` exists); the
  grammar-level `_slashy_string` and `_dollar_slashy_string` rules
  compose the body via the shared `gstringPart` helper. Closes
  the slashy half of §5.7 and divergence doc #7. Corpus tests in
  `test/corpus/expressions-strings.txt`. (SPECIFICATION.md §5.7)
- `injections.scm` now routes triple-double-quoted strings passed
  to a `sql.execute """..."""` (or `Sql.execute`) call as
  `@injection.language sql`, demonstrating the named-call DSL
  pattern documented in SPECIFICATION.md §9.2. Highlight
  assertions in `test/highlight/strings.groovy` pin GString
  interpolation `${...}` / `}` to `@punctuation.special` and the
  contained identifier reference to `@variable`.
- `gstring_dollar_interpolation` and `gstring_brace_interpolation`
  for double-quoted (`"..."`) and triple-double-quoted
  (`"""..."""`) strings. Each interpolation segment is exposed as
  a parse-tree child with a `value` field, and literal-text
  segments parse as `string_fragment`. The bare `$` fallback
  (e.g. the trailing `$` in `"^a.c$"`) is preserved as a single-
  char `string_fragment`. Highlight queries route `${` / `}` to
  `@punctuation.special` and the contained identifier / expression
  to `@variable` / inherited captures. Closes divergence doc §2.
  (SPECIFICATION.md §5.7, §6.3)
- Tighter slashy-string scanner: SLASHY_STRING_START rejects an
  immediate `/`, whitespace, or newline after the opening `/`,
  and rejects bodies that span a newline before any non-whitespace
  content. `a / b` and `a /= b` keep parsing as binary / augmented
  division; `def x = /pat/`, `[1, /pat/, 2]`, and `return /home/`
  keep parsing as slashy regex. Corpus tests pinned in
  `test/corpus/operators-regex.txt`. Closes divergence doc §7.
  (SPECIFICATION.md §5.4, §6.2)
- `queries/groovy/tags.scm` — `ctags`-style definition / reference
  query covering class / interface / trait / annotation-type / enum
  / record / method declarations, enum constants, method-invocation
  references, `new` expressions, and inheritance clauses.
  (SPECIFICATION.md §2.1)
- Highlight assertions for `non-sealed`, `permits`, and `yield`.

- External-scanner support for `line_comment`, `block_comment`, and
  `groovydoc_comment` as single tokens. Empty `/**/`, `/***/`, and
  `/* */` all parse correctly, closing
  `murtaza64/tree-sitter-groovy#16`. Corpus tests pinned in
  `test/corpus/comments.txt`. (SPECIFICATION.md §6.6)
- `expression_statement` and `_expression` infrastructure as the
  scaffolding all later expression rules will hang off.
- `number_literal` covering decimal, hex (`0x…`), binary (`0b…`),
  scientific (`1e10`, `1.5e-7`), underscore separators, and the
  `G g L l I i D d F f` type suffixes. Highlighted as `@number`.
  (SPECIFICATION.md §5.1)
- `test/corpus/expressions-numbers.txt` (16 cases) and
  `test/highlight/numbers.groovy` (5 assertions).
- `string_literal` covering plain single-quoted (`'…'`) and
  triple-single-quoted (`'''…'''`) flavours. Handles `\X` escapes,
  unicode bodies, and (for triple-single) embedded `'` and `''`
  sequences plus unescaped newlines. Highlighted as `@string`.
  Double-quoted, slashy, and dollar-slashy variants land with the
  GString scanner in a later iteration. (SPECIFICATION.md §5.7)
- `test/corpus/expressions-strings.txt` (12 cases) and
  `test/highlight/strings.groovy` (4 assertions).
- `boolean_literal` (`true`/`false`) and `null_literal` (`null`).
  Tree-sitter's `word: $.identifier` directive resolves the
  keyword-vs-identifier ambiguity: `trueish` stays an identifier,
  bare `true` parses as `boolean_literal`. Highlighted as
  `@boolean` and `@constant.builtin`. (SPECIFICATION.md §3.2)
- `test/corpus/expressions-literals.txt` (5 cases) and
  `test/highlight/literals.groovy` (3 assertions).
- Dormant scanner branches for the remaining §6 tokens
  (`DOLLAR_SLASHY_STRING`, `LABEL_COLON`, `SLASHY_STRING_BODY`,
  `SLASHY_STRING_END`). Each branch has real emission code and
  is gated on `valid_symbols`; they fire only when a future
  grammar refactor consumes the token. With these additions,
  every §6 token type has an explicit scanner branch (not a stub
  return), satisfying SPECIFICATION.md §1.1 goal 1 in full.
- Slashy and dollar-slashy regex injection rules in
  `queries/groovy/injections.scm` via `#match?` on the
  string-literal prefix (`/` and `$/` respectively). Editors
  that consume the query will highlight slashy / dollar-slashy
  bodies as regex.
- Internal sweep of partial (`[~]`) entries in
  `docs/IMPLEMENTATION_PROGRESS.md`: every previously-partial
  item is now either fully done or has a documented divergence
  pointer. No user-visible behaviour change.
- Dormant scanner branches for `GSTRING_INTERPOLATION_START`
  and `GSTRING_BODY`. The interpolation-start branch detects
  `$` followed by either `{` (full-expression form) or an
  identifier-start char (short form). The body branch consumes
  literal text up to `"`, `$`, or `\\`. Both are gated on
  `valid_symbols`, so they fire only when a grammar rule
  consumes the token; double-quoted strings remain flat for v1
  because no such rule has landed. Internal-only; no
  user-visible behaviour change. Divergences §2 updated.
- Scanner-state groundwork for AUTOMATIC_SEMICOLON: `Scanner`
  struct now carries `paren_depth`, `bracket_depth`,
  `brace_depth` fields and a working emission branch in
  `tree_sitter_groovy_external_scanner_scan` that fires when the
  parser puts `_automatic_semicolon` in `valid_symbols` and
  lookahead is `\n`. The branch is currently dormant — no
  grammar rule consumes the token — but the scaffolding is in
  place. Internal-only; no user-visible behaviour change.
  Divergences §3 updated with the deferred wire-up plan.
- `command_chain.receiver` now captured as `@function.call` so
  parenthesis-free calls highlight consistently with
  `method_invocation`. Stress corpus
  (`test/stress/closures_and_lists.groovy`) extended with four
  command_chain shapes that exercise both string and closure
  arguments.
- `command_chain` (parenthesis-free method-call) at the statement
  level. v1 is intentionally conservative: receiver is a bare
  identifier and the single argument must be a literal-shaped
  expression (string / number / boolean / null / closure). List
  and map literals are deliberately excluded so `arr[0]` keeps
  parsing as `subscript_expression` rather than as a command call.
  Multi-argument and chained-continuation forms
  (`foo bar baz`) wait for a more involved precedence design.
  Closes a subset of murtaza64 #5. 7 corpus cases in
  `expressions-command-chain.txt` including regression tests
  that subscript and adjacent-identifier statements still parse
  as before. (SPECIFICATION.md §5.5, §10 row #5)
- Inheritance clauses on declarations:
  - `superclass` — `class Sub extends Super` (single super only).
  - `super_interfaces` — `class Foo implements A, B, C`.
  - `extends_interfaces` — `trait T extends Other` and
    `interface I extends A, B` (multiple supers).
  Each is an optional clause inserted between the declaration's
  name and its body (`permits_clause` comes after). 8 corpus
  cases in `declarations-inheritance.txt` including
  sealed-with-extends-and-permits and public-abstract with full
  inheritance. Highlights: `extends` and `implements` captured
  as `@keyword`. (SPECIFICATION.md §4)
- Java-style modifier keywords on declarations: `public`,
  `private`, `protected`, `static`, `final`, `abstract`,
  `synchronized`, `native`, `transient`, `volatile`, `strictfp`.
  Accepted before `class` / `trait` / `interface` keywords and
  before `def` / typed-return method declarations. Each is
  captured as `@keyword.modifier`. 7 corpus cases in
  `declarations-modifiers.txt` including `public final class`,
  `public static void main(String[] args)`, and a method that
  combines annotations + modifiers + typed return.
  (SPECIFICATION.md §4)
- `@type.builtin` highlight rule for primitive type names
  (`boolean`, `byte`, `char`, `double`, `float`, `int`, `long`,
  `short`, `void`). Fires wherever the type appears as a
  `type_identifier` — i.e. method return types, formal
  parameter types, cast targets, array element types, and
  variable declaration types.
- Slashy regex literals `/…/` via the external scanner. The
  scanner branch dispatches on the character after `/`:
  `/` → line comment, `*` → block / groovydoc comment, `=` →
  augmented assign (handled by in-grammar lexer), anything else
  → slashy string body. Body honours `\X` escapes and accepts
  newlines (multi-line slashy is allowed). Single whole-string
  token in v1; the START / BODY / END split lands when GString
  interpolation goes in. 6 corpus cases appended to
  `expressions-strings.txt` including regex match (`s =~ /pat/`),
  slashy in list and call positions, and the
  `a / b` binary-division regression. (SPECIFICATION.md §6.2)
- `$/.../$` dollar-slashy strings added as a pure-grammar
  `string_literal` alternative. Per SPECIFICATION.md §5.7 no
  scanner is needed because `$` is not a binary operator. Body
  rules: `$$` → literal `$`, `$/` → literal `/`, embedded `/`
  (not followed by `$`) is fine. Multi-line is allowed.
  Interpolation structure will be carved out later when the
  GString scanner branch lands. 6 corpus cases appended to
  `expressions-strings.txt`. (SPECIFICATION.md §5.7)
- `qualified_type` (`java.util.List`) added to `_type` for most
  positions: `new`, parenthesised type cast, formal parameters,
  array element. The `cast_expression` (`x as Type`) and
  `instanceof_expression` (`x instanceof Type`) slots have a
  context-sensitive precedence quirk where the trailing
  `.Identifier` is preferred as a `field_access` continuation
  over extending the qualified_type — the workaround is to
  parenthesise: `x as (java.util.Map)`. Documented in
  `docs/divergences-from-spec.md` §5. 3 corpus cases in
  `declarations-qualified-types.txt`. (SPECIFICATION.md §4)
- `array_type` (`int[]`, `String[][]`) added to `_type`. Works
  in local variable declarations, formal parameters, method
  return types, and casts. Self-recursive for multi-dimensional
  arrays. `prec(2)` keeps the closing `]` of an array type from
  conflicting with the trailing `]` of subscript or list_literal
  in adjacent positions. 5 corpus cases in
  `declarations-array-types.txt`. (SPECIFICATION.md §4)
- `@label` highlight captures on `labeled_statement.label`,
  `break_statement.label`, and `continue_statement.label`.
- `docs/divergences-from-spec.md` populated with 6 entries
  documenting where `grammar.js` differs from `SPECIFICATION.md`:
  (1) the conflicts list, (2) deferred string interpolation,
  (3) deferred `AUTOMATIC_SEMICOLON`, (4) `non-sealed` as a
  single `token()`, (5) `_type` restricted to a single type
  identifier, and (6) typed local variable declarations
  requiring an initializer. Internal docs only — no
  user-visible behaviour change.
- Typed-return method declarations:
  `String greet() { … }`, `int add(int a, int b) { … }`,
  `void doIt() throws Exception { … }`, abstract typed-return
  methods inside interfaces. The existing
  `[$._expression, $._type]` conflict declaration already
  covered the disambiguation; the rule change is purely adding
  a `choice('def', field('return_type', $._type))` at the head
  of `method_declaration`. 4 corpus cases appended to
  `declarations-method-signatures.txt`. (SPECIFICATION.md §4)
- Typed local variable declarations:
  `String name = 'Groovy'`, `int count = 42`, `long total = a + b`,
  `int x = 1, y = 2`. The typed form requires every declarator
  to carry an `= value` initializer — `Type x` without an
  initializer would conflict with `method_invocation Type(x)` and
  is intentionally disallowed for v1. `def x` and `var x` without
  initializer remain supported via the existing pathway. 4 corpus
  cases appended to `declarations-variable.txt`. Typed-return
  method declarations follow the same disambiguation pattern and
  land next. (SPECIFICATION.md §4, §5.12)
- `test/corpus/operators-precedence-tiebreaks.txt` pinning the
  five Apache-spec tiebreak cases from §8.1 plus two extra
  unary-vs-power cases: range/shift left-assoc, ternary-with-
  nested-Elvis, Elvis right-assoc chain, implication right-
  assoc, power right-assoc, `-2 ** 3` → `-(2 ** 3)`, and
  `~2 ** 3` → `(~2) ** 3`. (SPECIFICATION.md §8.1)
- Stress corpus (`test/stress/`) with 6 synthetic Groovy files
  covering arithmetic / ranges, class + methods, closures + lists,
  control flow, imports + package, and operators. Authoring
  notes in `test/stress/SOURCES.md`.
- `bindings/rust/tests/parse_stress.rs` integration test (wired
  via `[[test]]` in `Cargo.toml`) that walks `test/stress/` and
  asserts zero ERROR and zero MISSING nodes for every file —
  the direct anti-regression for the
  `dekobon/big-code-analysis#246` failure mode where the prior
  grammar inserted synthetic missing operands.
- `labeled_statement` for `name: stmt`. Carries `label` and
  `statement` fields. `prec(1)` plus context disambiguation keeps
  labels parsing correctly at statement-start position without
  needing the LABEL_COLON external-scanner branch — map_entries
  (which share `:`) live inside `[…]` and `case … :` lives inside
  `switch_block`, so neither competes with a top-level label.
  Labels integrate with the existing `label` field on
  `break_statement` / `continue_statement`. 4 corpus cases in
  `statements-labeled.txt` including a labeled-nested-loop shape
  with labeled break. (SPECIFICATION.md §4, §5.3)
- Three new highlight test files:
  - `keywords.groovy` (18 assertions): every modifier / control-flow
    keyword via the `<-` syntax for column-0 assertions.
  - `types.groovy` (6 assertions): name positions of `class`, `trait`,
    `interface`, `enum`, `record`, and `@interface`.
  - `functions.groovy` (7 assertions): declaration site, two call
    sites, dotted-call, method-reference RHS, method-pointer RHS.
- Reorganised `queries/groovy/highlights.scm` into priority layers
  with the precedence rule documented at the top: last-match-wins
  per node, so specific captures (declared names, call sites,
  type uses) intentionally come AFTER the broader fallbacks
  (`(identifier) @variable`, `@property`, etc.).
- Try-with-resources via new `resource_specification` and
  `resource` rules. `try_statement` accepts an optional
  `resources` field between `try` and the body. Multiple
  resources are separated by `;` with an optional trailing `;`.
  Resources carry an optional type, required name, and required
  `= value` initializer. 3 corpus cases appended to
  `statements-try.txt`.
- `yield_statement` for Groovy 4 switch-expression case bodies.
  `yield` is a contextual keyword — usable as an identifier
  elsewhere via the `word:` directive. 3 corpus cases in
  `statements-yield.txt`. Highlights: `yield` as
  `@keyword.control`. (SPECIFICATION.md §4)
- Quoted identifiers in property / method-name positions per §5.2:
  `map."key with spaces"`, `def "test name"() { … }`, and the
  Spock-style `def "abstract"() { … }` shape that lets methods
  bear keyword-like names. Introduced a hidden `_property_name`
  rule (choice of `identifier` and `string_literal` aliased as
  `quoted_identifier`), threaded through `field_access`,
  `safe_navigation_expression`, `safe_chain_dot_expression`,
  `spread_dot_expression`, `method_pointer_expression`,
  `direct_field_access_expression`, and `method_declaration.name`.
  Required adding `[$.variable_declarator, $._property_name]` to
  the conflict list so `def identifier` keeps both
  local-variable and method-declaration alternatives alive
  until the next token (`(` vs `=`/`,`/end). 5 corpus cases in
  `expressions-quoted-identifier.txt`. (SPECIFICATION.md §5.2, §7)
- Groovy 4 sealed-type support on `class_declaration`,
  `trait_declaration`, and `interface_declaration`:
  - Optional `sealed` or `non-sealed` modifier before the keyword.
    `non-sealed` is a single anonymous `token('non-sealed')` so
    the dash never lexes as subtraction.
  - `permits_clause` (`permits A, B, …`) after the name.
- `annotation_type_declaration` for `@interface Name { … }`. Body
  reuses `class_body`. 3 corpus cases in
  `declarations-annotation-type.txt`. 5 corpus cases for sealed
  in `declarations-sealed.txt`.
- `method_declaration.body` is now optional, supporting abstract
  methods (`def foo()` with no body) used in interfaces and
  annotation types. `prec.right(1, …)` keeps the body greedily
  attached when a `{ … }` block follows.
- Highlights: `sealed`, `non-sealed`, `permits` as `@keyword`;
  `annotation_type_declaration.name` as `@type.definition`.
  (SPECIFICATION.md §4)
- Method-signature enrichment:
  - `throws_clause` (`throws E1, E2, …`) optional after the
    `formal_parameters` of a `method_declaration`.
  - `formal_parameter` carries an optional `default` field for
    default-value parameters (`name = expr`).
  - `varargs_type` for `Type... name` syntax — distinct from
    `_type` so downstream tools see the trailing `...` as a
    discriminator.
  7 corpus cases in `declarations-method-signatures.txt`.
  Highlights: `throws` as `@keyword.control`.
  (SPECIFICATION.md §4, §5.13)
- Filled out the four remaining query files
  (`queries/groovy/{folds,indents,injections,locals}.scm`):
  - `folds.scm`: fold ranges for class / enum / switch / block /
    closure bodies plus large literal collections and doc
    comments.
  - `indents.scm`: indent-begin on container nodes, indent-end on
    closing delimiters, branch alignment for switch cases.
  - `injections.scm`: `groovydoc_comment` injects as `javadoc`.
    Slashy / dollar-slashy regex injection waits for the string
    scanner branch.
  - `locals.scm`: scope, definition, and reference captures per
    §9.3 covering method / closure / for / for-in / catch scopes
    and the corresponding declaration sites.
  (SPECIFICATION.md §9)
- `test/corpus/regressions.txt` aggregating one pinned anchor per
  closed source-grammar issue: murtaza64 #16, #22, #36, #37, #39
  plus dekobon #246 (Elvis), #247 trait, #247 exclusive range.
  Future grammar edits that break any of these surface as a
  single localised test failure. (SPECIFICATION.md §8.2)
- `method_declaration` is now also a `_statement` (was only a
  `_class_member` before). Top-level script methods like
  `def toS(c) { … }` parse as `method_declaration` rather than
  `local_variable_declaration + parens-cast`. `prec(1)` on
  `method_declaration` wins over `local_variable_declaration`
  when `(` follows the name.
- `pipeline_statement` — Jenkins-style `pipeline { … }` as a
  regular statement. Closes `murtaza64/tree-sitter-groovy#37`
  (pipeline no longer constrained to end-of-file). Body is a
  `closure`. 4 corpus cases in `statements-pipeline.txt` including
  the pinned #37 regression shape (pipeline followed by a `def`).
  Highlights: `pipeline` as `@keyword`. (SPECIFICATION.md §4,
  §10 row #37)
- `shebang` — `#!…\n` at file start, parsed as the first child of
  `source_file` (before any statements). 3 corpus cases in
  `declarations-shebang.txt`. Highlights: `shebang` as
  `@keyword.directive`. (SPECIFICATION.md §4)
- `local_variable_declaration` (`def x = …` and `var x = …`) and
  `multi_assignment_declaration` (`def (a, b) = expr`). The latter
  closes `murtaza64/tree-sitter-groovy#22`. `variable_declarator`
  carries optional `value` field; multiple declarators on one
  statement (`def a = 1, b = 2`) are supported. Typed-prefix
  declarations (`String x = …`) and the no-`def` multi-assignment
  form land later. 8 corpus cases in `declarations-variable.txt`
  including the pinned #22 anchor. Highlights: `var` as `@keyword`;
  declared names as `@variable`. (SPECIFICATION.md §4, §5.12,
  §10 row #22)
- `package_declaration`, `import_declaration`, `qualified_name`,
  and `annotation`. `import` carries `static` modifier, optional
  `.*` wildcard suffix (a single anonymous token so it lexes
  distinctly from a continuation of qualified_name), and optional
  `as Alias`. Annotations now prefix class / trait / interface /
  enum / record / method declarations via a leading
  `repeat($.annotation)`. 14 corpus cases across
  `declarations-package-import.txt` (8) and
  `declarations-annotation.txt` (6). Highlights:
  `package` / `import` / `static` as `@keyword`; annotation `@`
  and qualified name as `@attribute`. (SPECIFICATION.md §4, §5.9)
- `enum_declaration` plus `enum_body` and `enum_constant`.
  Constants may carry constructor args; an optional `;` separates
  the constants list from further class members (methods). Closes
  the first half of `murtaza64/tree-sitter-groovy#36`. 5 corpus
  cases in `declarations-enum.txt`. (SPECIFICATION.md §4, §8.2,
  §10 row #36)
- `record_declaration` plus `record_components` and
  `record_component` (Groovy 4+). `record Name(Type a, Type b)
  [{ body }]`. Optional body is wrapped in `prec.right` so the
  parser greedily attaches the `{ … }` block to the record
  declaration. 4 corpus cases in `declarations-record.txt`.
  Highlights: `enum`, `record` as `@keyword`; enum constants
  as `@constant`; record components as `@variable.parameter`.
  (SPECIFICATION.md §4)
- Minimal class-family declarations:
  - `class_declaration`     `class Name { body }`
  - `trait_declaration`     `trait Name { body }`   (closes dekobon #247 trait row)
  - `interface_declaration` `interface Name { body }`
  - `class_body`            `{ class_member* }`
  - `method_declaration`    `def name(params) body`
  - `formal_parameters`     `( formal_parameter, … )`
  - `formal_parameter`      `[type] name`
  extends / implements / generics / annotations / modifiers /
  sealed / permits / @interface land in subsequent iterations.
  Type-prefixed methods (`String foo() { … }`) wait for
  `local_variable_declaration` so the `Type name` vs
  `Type name = …` ambiguity is resolved together. 7 corpus cases
  in `declarations-class.txt`, including the §8.2 trait-anchor
  shape. Highlights: `class`/`trait`/`interface`/`def` as
  `@keyword`; declaration names as `@type.definition` /
  `@function`; parameter names as `@variable.parameter`.
  (SPECIFICATION.md §4, §8.2, §10 row dekobon #247 trait)
- `switch_expression` plus `switch_block`, `switch_case`,
  `switch_arrow_case`, and `switch_default`. Both Groovy 4 arrow
  form (`case … -> body`) and the classic form (`case … : stmts`)
  are supported in the same `switch_block`. `switch_arrow_case`
  closes murtaza64 #36 second half. switch_expression sits in
  the `_expression` choice so it also acts as a statement via
  `expression_statement`. 8 corpus cases in `statements-switch.txt`
  including the field-access-case shape from #36. Highlights:
  `switch`, `case`, `default` as `@keyword.control`.
  (SPECIFICATION.md §4, §8.2, §10 row #36)
- `try_statement` plus the supporting `catch_clause`,
  `finally_clause`, `catch_formal_parameter`, and `multi_type`
  rules. `catch_formal_parameter` takes a `type` field that
  accepts either a single `_type` or a `multi_type` for Java 7
  / Groovy multi-catch (`catch (A | B e)`) — closes
  `murtaza64/tree-sitter-groovy#39`. Try-with-resources lands with
  local variable declarations. 6 corpus cases in
  `statements-try.txt` including the pinned #39 regression shape
  and a three-type multi-catch. Highlights: `try`, `catch`,
  `finally` captured as `@keyword.control`. (SPECIFICATION.md §4,
  §8.2, §10 row #39)
- Jump and check statements:
  - `return_statement` (with optional return value)
  - `break_statement` (with optional `label`)
  - `continue_statement` (with optional `label`)
  - `throw_statement` (expression required)
  - `assert_statement` (`assert expr [: message]`)
  - `do_while_statement` (`do body while (cond)`)
  Until AUTOMATIC_SEMICOLON lands, `return foo` and `return\nfoo`
  both consume the trailing expression — explicit `;` is the
  workaround. 14 corpus cases in `statements-jump.txt`. Highlights:
  `return`, `break`, `continue`, `throw`, `assert`, `do` captured
  as `@keyword.control`. (SPECIFICATION.md §4)
- `for_statement` (C-style `for (init; cond; update) body`) and
  `for_in_statement` (`for (x in xs) body`). Both accept either an
  explicit block or a single statement as body. The
  `[$.for_in_statement, $._expression]` conflict declaration keeps
  both alternatives alive after `for ( <identifier>` until `;` or
  `in` disambiguates. C-style init / condition / update slots are
  all optional, so `for (;;)` parses as the infinite-loop idiom.
  Typed for-in (`for (Type x in xs)`) lands with local variable
  declarations. 8 corpus cases in `statements-for.txt` including
  the `for (i in 0..<n)` exclusive-range shape from dekobon #247.
  Highlights: `for` captured as `@keyword.control`.
  (SPECIFICATION.md §4)
- `block` (statement container, `{ ... }`), `if_statement`, and
  `while_statement`. `block` is `prec(1, …)` so it wins over
  closure at body slots where both `{ … }` shapes would otherwise
  be valid; top-level `{ … }` keeps parsing as the closure
  expression form that Groovy scripts use. `if` / `else` carries
  `condition`, `consequence`, optional `alternative` fields; the
  dangling-else ambiguity resolves to the closest `if` via
  `prec.right`. `while` carries `condition` / `body`. The body
  slot of both accepts either an explicit block or a single
  statement, matching Java/Groovy convention. 10 corpus cases in
  `statements-if-while.txt`. Highlights query: `if`, `else`,
  `while` captured as `@keyword.control`.
  (SPECIFICATION.md §4)
- Double-quoted `"..."` and triple-double `"""..."""` strings, as
  additional alternatives under `string_literal`. v1 treats them
  as flat tokens — `$identifier` and `${expr}` parse as part of
  the string body and emit no interpolation structure. The
  GString interpolation scanner branch (§6.3) lands as a follow-up
  and will split these into segment children. 10 new corpus cases
  extend `expressions-strings.txt`. (SPECIFICATION.md §5.7)
- `spread_arguments` (`*expr`) added to `argument_list`. `f(*xs)`
  expands an iterable into positional arguments per §5.11.
  Distinct node kind so downstream tooling discriminates from
  multiplication. Carries a `value` field. Closes the `*args` row
  of dekobon #247.
- `spread_map_entry` (`*: expr`) added to `map_literal`.
  `[*: other, k: v]` merges `other`'s entries with the explicit
  ones per §5.11. `*:` is a single anonymous token (longest-match);
  carries a `value` field. Closes the `*:` row of dekobon #247.
- 6 corpus cases in `operators-spread.txt`. Highlights query gains
  `*:` as `@operator`. (SPECIFICATION.md §3.2, §5.11)
- Three RELATIONAL-tier distinct-node operators, all at
  `PREC.RELATIONAL` left-associative:
  - `cast_expression` (`x as T`)       value / type    (RHS is `_type`)
  - `membership_expression` (`x in xs`, `x !in xs`)  element / operator / collection
  - `instanceof_expression` (`x instanceof T`, `x !instanceof T`)  value / operator / type (RHS is `_type`)
  14 corpus cases across `operators-membership.txt`,
  `operators-instanceof.txt`, and `operators-as-cast.txt`.
  Keywords `as`, `in`, `!in`, `instanceof`, `!instanceof`, and
  `new` highlighted as `@keyword.operator`. Closes the `!in`,
  `!instanceof`, and `as` rows of dekobon #247. (SPECIFICATION.md
  §3.1, §3.2, §3.2.2)
- `object_creation_expression` for `new Foo(args)` at
  `PREC.PRIMARY` right-associative. Fields `type` / `arguments`.
  Array creation `new Foo[10]` and array initialisers land later.
  Highlighted via `@type` on the type-identifier and `@keyword`
  on `new`. 6 corpus cases in `expressions-object-creation.txt`.
  (SPECIFICATION.md §3.2)
- `parenthesized_type_cast` for the C-style `(Type) x` cast at
  `PREC.UNARY` right-associative. Fields `type` / `value`. The
  ambiguity with `parenthesized_expression` is resolved through
  a `[$._expression, $._type]` conflict declaration — both
  alternatives stay alive until the token after `)` (or the
  shape of what's inside `(…)`) disambiguates. 6 corpus cases in
  `operators-coercion.txt`. (SPECIFICATION.md §3.2, §5.15, §7)
- `_type` (hidden) with `type_identifier` (aliased identifier)
  so type-position identifiers are tagged distinctly from value-
  position ones. Highlighted as `@type`. Generic types, array
  types, and qualified names land later.
- Full dot-family ACCESS tier per §3.1, all at `PREC.ACCESS`
  left-associative, all distinct node kinds:
  - `field_access` (`.`)                   object / field
  - `safe_navigation_expression` (`?.`)    object / property
  - `safe_chain_dot_expression` (`??.`)    object / property  (Groovy 4+)
  - `spread_dot_expression` (`*.`)         object / property
  - `method_pointer_expression` (`.&`)     object / method
  - `direct_field_access_expression` (`.@`) object / field
  - `method_reference_expression` (`::`)   target / name (`new` allowed)
  25 corpus cases across `operators-access.txt`,
  `operators-safe-navigation.txt`, `operators-safe-chain-dot.txt`,
  `operators-spread-dot.txt`, `operators-method-pointer.txt`,
  `operators-direct-field.txt`, `operators-method-reference.txt`.
  Highlights query: property / field names highlight as
  `@property`; method-pointer / method-reference RHS as
  `@function`; `new` (used by `::new` and later by
  `object_creation_expression`) as `@keyword`; all access tokens
  added to the operator alternation. Closes the `.&`, `.@`, `?.`,
  `*.`, and `::` rows of dekobon #247. (SPECIFICATION.md §3.1
  ACCESS, §3.2, §5.14)
- `update_expression` for postfix `++` / `--` at `PREC.POSTFIX`,
  left-associative. Fields `operand` / `operator`.
- `unary_update_expression` for prefix `++` / `--` at `PREC.UNARY`,
  right-associative. Fields `operator` / `operand`. Distinct
  node kind from `update_expression` per §3.2. 7 corpus cases in
  `expressions-update.txt`. Highlights query gains `++` / `--`
  tokens. (SPECIFICATION.md §3.2)
- `method_invocation` for `f(args)` at `PREC.POSTFIX`,
  left-associative. The `function` field accepts any
  `_expression`, so subscript chains compose naturally per §3.2.1
  — `stepImplementations[Platform](ctx)` parses as
  `method_invocation` with `function` being a `subscript_expression`.
  Anonymous `(`/`)` brackets bracket an `argument_list` field.
  `argument_list` carries positional arguments (`_expression`)
  and named arguments (`map_entry` aliased as `named_argument`
  so downstream tools can discriminate by node kind). Trailing
  commas accepted. Highlights query gains a `@function.call`
  capture for bare-identifier callees. 12 corpus cases in
  `expressions-method-invocation.txt`. Spread args (`*expr`) and
  trailing-closure sugar land later. (SPECIFICATION.md §3.2,
  §3.2.1, §5.11)
- `subscript_expression` `[]` and `safe_subscript_expression` `?[]`
  at `PREC.POSTFIX`, left-associative. Fields `object` / `index`.
  The safe-subscript variant uses `?[` as one anonymous token,
  but `?` followed by whitespace then `[` still parses as ternary
  plus list — the lexer never crosses whitespace to extend a
  multi-char operator. 10 corpus cases in `operators-subscript.txt`
  including the ternary-with-list disambiguation. Closes the
  `?[]` row of dekobon #247. (SPECIFICATION.md §3.1, §3.2)
- `power_expression` for `**` at `PREC.POWER`, right-associative.
  Distinct node kind so metric tools count power separately from
  multiplicative ops. Tighter than `+ -` (UNARY) but looser than
  `! ~` (UNARY_NOT) — `unary_expression` now splits internally
  into two precedence alternatives at `PREC.UNARY` and
  `PREC.UNARY_NOT` so that `-2 ** 3` parses as `-(2 ** 3)` while
  `~2 ** 3` parses as `(~2) ** 3` per Apache. 8 corpus cases in
  `operators-power.txt`. (SPECIFICATION.md §3.1 levels 1, 2, 3)
- `logical_implication_expression` for `==>` at `PREC.IMPLICATION`,
  right-associative per Apache (`a ==> b ==> c` → `a ==> (b ==> c)`).
  Distinct node kind so downstream tools don't have to discriminate
  between `==>`, `===`, `==~`, and `==` by tree-shape.
  (SPECIFICATION.md §3.1, §3.2)
- `assignment_expression` covering plain `=` plus 13 augmented
  forms (`+=`, `-=`, `*=`, `/=`, `%=`, `**=`, `<<=`, `>>=`, `>>>=`,
  `&=`, `^=`, `|=`, `?=`) at `PREC.ASSIGN`, right-associative.
  Single node kind with `left` / `operator` / `right` fields; the
  operator field distinguishes the 14 variants. Elvis-assign `?=`
  satisfies the issue-247 row. 17 corpus cases in
  `operators-assignment.txt`. (SPECIFICATION.md §3.1, §3.2)
- `ternary_expression` (`? :`) and `elvis_expression` (`?:`) at
  `PREC.CONDITIONAL`, right-associative. Per Apache (§3.1) both
  share the same precedence level — `a ? b ?: c : d` parses as
  `a ? (b ?: c) : d` and `a ?: b ?: c` parses as `a ?: (b ?: c)`.
  `elvis_expression` carries `value` / `default` fields;
  `ternary_expression` carries `condition` / `consequence` /
  `alternative`. The elvis kind being distinct is the headline ask
  of `dekobon/big-code-analysis#246`. 10 corpus cases across
  `operators-elvis.txt` and `operators-ternary.txt`, including the
  `#246` chain anchor and the Apache precedence tiebreak.
  (SPECIFICATION.md §3.1, §3.2, §10 row #246)
- Distinct-node operators per SPECIFICATION.md §3.2:
  - `range_expression` — `..`, `..<`, `<..`, `<..<` at
    `PREC.SHIFT_OR_RANGE`. Fields `start` / `operator` / `end`.
  - `identity_expression` — `===`, `!==` at `PREC.EQUALITY`.
    Distinct from `binary_expression` for `==` / `!=` even though
    they share a precedence level. Fields `left` / `operator` / `right`.
  - `spaceship_expression` — `<=>` at `PREC.EQUALITY`. Fields
    `left` / `right` (no operator field — there is only one form).
  - `regex_find_expression` — `=~` at `PREC.EQUALITY`. Fields
    `subject` / `pattern`.
  - `regex_match_expression` — `==~` at `PREC.EQUALITY`. Fields
    `subject` / `pattern`. The longest-match lexer picks `==~`
    over `==` automatically.
  19 corpus cases across `operators-range.txt`,
  `operators-identity.txt`, `operators-spaceship.txt`, and
  `operators-regex.txt`. Highlights query updated with all new
  operator tokens.
- `binary_expression` covering the full Java-shaped operator set
  on one node kind, with `left` / `operator` / `right` fields:
  arithmetic (`+ - * / %`), shift (`<< >> >>>`), relational
  (`< <= > >=`), equality (`== !=`), bitwise (`& ^ |`), logical
  (`&& ||`). Each tier sits at its Apache-mirrored precedence
  level so `a + b * c` → `a + (b * c)`, `a + b << c` → `(a + b) << c`,
  `a == b && c == d` → `(a == b) && (c == d)`, etc. All
  left-associative. 36 corpus cases total (13 arithmetic in
  `expressions-arithmetic.txt`, 23 mixed-tier in
  `expressions-binary.txt`) pin every operator individually plus
  inter-tier precedence tiebreaks. Highlight query now lists all
  binary operator tokens for `@operator` capture.
  (SPECIFICATION.md §3.1, §3.2)
- `closure` for `{ -> }`, `{ a, b -> body }`, and implicit-`it`
  `{ body }`. Parameters are untyped identifiers for v1; typed,
  default-valued, and varargs parameters land later. We do NOT emit
  a synthetic `it` for the parameterless form — downstream tools
  that care can match a `closure` with no `closure_parameters`
  child. 8 corpus cases in `test/corpus/expressions-closure.txt`.
  (SPECIFICATION.md §5.8)
- `list_literal`, `map_literal`, and `map_entry` with trailing-comma
  support. Empty list is `[]`; empty map is the distinct `[:]` form.
  Map entries carry `key` and `value` fields. tree-sitter resolves
  the list-vs-map ambiguity with one token of lookahead (`:` versus
  `,` or `]` after the first element), so no explicit conflict
  declaration is needed. 13 corpus cases in
  `test/corpus/expressions-list-map.txt`. (SPECIFICATION.md §3.2,
  §5.3, §5.13)
- `parenthesized_expression` for `( expr )` (distinct from the
  later `parenthesized_type_cast`, §5.15).
- `unary_expression` covering prefix `+`, `-`, `!`, `~` at
  `PREC.UNARY`. The Apache spec splits these across two levels
  (UNARY_NOT for `! ~`, UNARY_ADD for `+ -`); we keep them in one
  rule until the binary tiers force the split. Operator
  highlighted as `@operator`. 8 corpus cases in
  `test/corpus/expressions-unary.txt`; 2 additional highlight
  assertions in `test/highlight/literals.groovy`.
  (SPECIFICATION.md §3.2)
- `docs/RALPH_LOOP_PROMPT.md` and `docs/IMPLEMENTATION_PROGRESS.md`
  to drive an iterative implementation loop against SPECIFICATION.md.

- Initial repository skeleton: `grammar.js` stub, multi-language
  bindings (C, Go, Node, Python, Rust, Swift), queries directory,
  test corpus directory, packaging metadata (`package.json`,
  `Cargo.toml`, `pyproject.toml`, `tree-sitter.json`, `go.mod`,
  `Package.swift`, `binding.gyp`, `setup.py`, `Makefile`), CI workflow,
  and agentic skills under `.claude/skills/`.
- `SPECIFICATION.md` — authoritative grammar design document covering
  full Groovy operator coverage and statement-level constructs, with
  per-issue resolution map for `dekobon/big-code-analysis#246`,
  `#247`, and all open `murtaza64/tree-sitter-groovy` issues.
