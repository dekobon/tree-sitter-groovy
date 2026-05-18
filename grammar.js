/**
 * @file Apache Groovy grammar for tree-sitter
 * @author Elijah Zupancic <elijah@zupancic.name>
 * @license MIT
 *
 * The authoritative design document for this grammar is
 * SPECIFICATION.md at the repository root. Every rule below maps to
 * a section there. Read the spec before making non-trivial changes.
 *
 * The external scanner (src/scanner.c) handles four context-sensitive
 * lexing situations that pure grammar rules cannot disambiguate:
 *   - Slashy-string-start vs. division
 *   - Automatic-semicolon (newline-as-terminator)
 *   - Labelled-statement colon
 *   - Block-comment / Groovydoc single-token emit
 * See SPECIFICATION.md §6.
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

/* eslint-disable no-multi-spaces --
 * The aligned trailing comments make the Apache-mirror precedence
 * ladder readable at a glance.
 */
const PREC = {
  // Higher number = tighter binding in tree-sitter.
  // Mirrors apache/groovy:GroovyParser.g4 `expression` rule.
  // See SPECIFICATION.md §3.1.
  ASSIGN: 1,                  // = += -= *= /= %= **= <<= >>= >>>= &= ^= |= ?=
  CONDITIONAL: 2,             // ?: (Elvis) and ? : (ternary) — same level
  IMPLICATION: 3,             // ==>
  LOGICAL_OR: 4,              // ||
  LOGICAL_AND: 5,             // &&
  BITWISE_OR: 6,              // |
  BITWISE_XOR: 7,             // ^
  BITWISE_AND: 8,             // &
  EQUALITY: 9,                // == != <=> === !== =~ ==~
  RELATIONAL: 10,             // < <= > >= in !in instanceof !instanceof as
  SHIFT_OR_RANGE: 11,         // << >> >>> .. ..< <.. <..<
  ADDITIVE: 12,               // + -
  MULTIPLICATIVE: 13,         // * / %
  UNARY: 14,                  // prefix ++ -- + -          (Apache level 3 UNARY_ADD)
  POWER: 15,                  // **                         (Apache level 2)
  UNARY_NOT: 16,              // ! ~                        (Apache level 1, tighter than POWER)
  POSTFIX: 17,                // postfix ++ --, []
  ACCESS: 18,                 // . ?. ??. *. .& .@ ::
  PRIMARY: 19,                // literals, identifiers, ()
  // Type-context only — never competes with the expression ladder
  // above. Shares CONDITIONAL's numeric value by accident; the two
  // do not collide because `_type` and `_expression` reach the
  // resolver through disjoint parse stacks.
  ARRAY_TYPE: 2,
};
/* eslint-enable no-multi-spaces */

module.exports = grammar({
  name: 'groovy',

  // Placeholder for the real implementation. SPECIFICATION.md §3-§7
  // is the contract this rule set will fulfil.
  word: $ => $.identifier,

  extras: $ => [
    /\s/,
    $.line_comment,
    $.block_comment,
    $.groovydoc_comment,
  ],

  // Externals listed here must have a matching branch in
  // `src/scanner.c` AND be referenced by at least one rule below.
  // Tokens that the spec §6 reserves for the scanner but that no rule
  // consumes yet (automatic-semicolon, gstring-body/interpolation,
  // slashy-body/end, label-colon) are intentionally NOT listed —
  // listing them would mark them valid in tree-sitter's error-recovery
  // state and let dormant scanner branches mis-fire on unrelated input.
  externals: $ => [
    $._slashy_string_start,
    $.line_comment,
    $.block_comment,
    $.groovydoc_comment,
  ],

  conflicts: $ => [
    // See SPECIFICATION.md §7. Populated as rules accumulate.
    // (list_literal vs map_literal — tree-sitter resolves without
    //  an explicit conflict because `:` after the first element
    //  always disambiguates within one token of lookahead.)
    //
    // §5.15 — inside `( … )` an identifier could be either an
    // _expression (parenthesized_expression) or a _type
    // (parenthesized_type_cast). Tree-sitter explores both via
    // this conflict; the `)` plus the next token disambiguates.
    [$._expression, $._type],

    // §4 — `a.b.c` could be a field_access chain (expression
    // context) or a qualified_type (type context). Same family
    // as the _expression / _type conflict above but the implicit
    // qualified_type_repeat1 aux symbol forces a separate
    // declaration.
    [$._expression, $.qualified_type],
    [$._type, $.qualified_type],

    // §4 — `for ( identifier …` could continue as a for_statement
    // (identifier is part of the init expression) or as a
    // for_in_statement (identifier is the loop variable). The
    // token after the identifier (`;` / operator vs `in`)
    // disambiguates.
    [$.for_in_statement, $._expression],

    // §5.2 — `def identifier` could either be a
    // local_variable_declaration (variable_declarator with no
    // initializer) or the start of a method_declaration
    // (_property_name + parameters). The `(` after the name
    // decides; the conflict keeps both alternatives alive until
    // then.
    [$.variable_declarator, $._property_name],
  ],

  rules: {
    // Top-level. Real rule set lands incrementally per
    // SPECIFICATION.md §4 (statement and declaration coverage).
    source_file: $ => seq(
      optional($.shebang),
      repeat($._statement),
    ),

    // §4 — `#!/usr/bin/env groovy` line at file start. Single
    // token consumed by the in-grammar lexer.
    shebang: _ => token(seq('#!', /[^\n]*/)),

    // §4 — statement-level alternatives. Declarations and the rest
    // of control flow fill out this choice in subsequent iterations.
    // `block` is intentionally NOT here — a top-level `{ ... }`
    // continues to parse as an expression_statement-wrapped closure
    // (the historically intended form for Groovy scripts). Blocks
    // appear explicitly in the body slot of `if` / `while` / etc.
    _statement: $ => choice(
      $.expression_statement,
      $.if_statement,
      $.while_statement,
      $.do_while_statement,
      $.for_statement,
      $.for_in_statement,
      $.return_statement,
      $.break_statement,
      $.continue_statement,
      $.throw_statement,
      $.assert_statement,
      $.yield_statement,
      $.try_statement,
      $.class_declaration,
      $.trait_declaration,
      $.interface_declaration,
      $.annotation_type_declaration,
      $.enum_declaration,
      $.record_declaration,
      $.package_declaration,
      $.import_declaration,
      $.local_variable_declaration,
      $.multi_assignment_declaration,
      $.pipeline_statement,
      $.method_declaration,
      $.labeled_statement,
      $.command_chain,
    ),

    // §5.5 — command_chain is the parenthesis-free method-call
    // form (`println 'hello'`). v1 is intentionally conservative:
    // receiver is a bare identifier and there is exactly one
    // argument, which must be a non-identifier literal-shaped
    // expression (no risk of mis-parsing `foo bar` as a command
    // call on a bare identifier). Multi-argument, chained
    // (`foo bar baz`), and named-argument command chains land
    // later. Closes a subset of murtaza64 #5.
    command_chain: $ => prec(2, seq(
      field('receiver', $.identifier),
      field('argument', choice(
        $.string_literal,
        $.number_literal,
        $.boolean_literal,
        $.null_literal,
        $.closure,
      )),
    )),

    // §5.3 — `label: stmt`. Required only at statement-start
    // position; map_entries (which share `:`) are inside `[…]`
    // and don't collide here. Conflict declaration keeps both
    // labeled_statement and expression_statement alive after
    // `identifier`; the `:` decides.
    labeled_statement: $ => prec(1, seq(
      field('label', $.identifier),
      ':',
      field('statement', $._statement),
    )),

    // §4 — block of statements, used in control-flow bodies and
    // (eventually) method bodies. Shares `{ ... }` syntax with
    // closure (§5.8). A `prec(1, …)` keeps block preferred at use
    // sites where both `block` and a closure-bearing `_statement`
    // are valid alternatives (e.g. the body slot of `if` / `while`).
    block: $ => prec(1, seq(
      '{',
      repeat($._statement),
      '}',
    )),

    // §4 — if / else. `prec.right` resolves the dangling-else
    // ambiguity in favour of the closest `if`. Body accepts either
    // an explicit block (preferred) or a single statement.
    if_statement: $ => prec.right(seq(
      'if',
      '(',
      field('condition', $._expression),
      ')',
      field('consequence', choice($.block, $._statement)),
      optional(seq(
        'else',
        field('alternative', choice($.block, $._statement)),
      )),
    )),

    // §4 — while loop. do-while lands separately.
    while_statement: $ => seq(
      'while',
      '(',
      field('condition', $._expression),
      ')',
      field('body', choice($.block, $._statement)),
    ),

    // §4 — C-style for. init / condition / update slots all
    // optional; bare `for (;;)` is the infinite-loop idiom.
    // Typed variable declarations in init (`int i = 0`) land with
    // local_variable_declaration in a later iteration.
    for_statement: $ => seq(
      'for',
      '(',
      optional(field('init', $._expression)),
      ';',
      optional(field('condition', $._expression)),
      ';',
      optional(field('update', $._expression)),
      ')',
      field('body', choice($.block, $._statement)),
    ),

    // §4 — do-while. Body is required; condition follows the body.
    do_while_statement: $ => seq(
      'do',
      field('body', choice($.block, $._statement)),
      'while',
      '(',
      field('condition', $._expression),
      ')',
    ),

    // §4 — return / break / continue / throw / assert. Until
    // AUTOMATIC_SEMICOLON lands the parser will greedily consume
    // a trailing expression onto `return` / `throw` / `assert`;
    // explicit `;` or statement boundary is recommended in source.
    return_statement: $ => prec.right(seq(
      'return',
      optional($._expression),
    )),

    break_statement: $ => prec.right(seq(
      'break',
      optional(field('label', $.identifier)),
    )),

    continue_statement: $ => prec.right(seq(
      'continue',
      optional(field('label', $.identifier)),
    )),

    throw_statement: $ => seq('throw', $._expression),

    // §4 — `yield expr` in a Groovy 4 switch expression's case
    // body. `yield` is a contextual keyword (§5.2) so it remains
    // a usable identifier elsewhere — tree-sitter's word:
    // directive resolves the ambiguity.
    yield_statement: $ => prec.right(seq(
      'yield',
      $._expression,
    )),

    // §4 — power-assert: `assert e` or `assert e : msg`. The colon
    // separator may collide with map_entry in later iterations, but
    // assert is a statement-only construct so the context resolves
    // it.
    assert_statement: $ => prec.right(seq(
      'assert',
      $._expression,
      optional(seq(':', field('message', $._expression))),
    )),

    // §4 — try / catch / finally. At least one of catch/finally is
    // required per Java/Groovy semantics; the grammar relaxes that
    // to allow a bare `try { … }` and lets downstream tooling
    // surface the error if needed. Try-with-resources lands later
    // with local variable declarations.
    try_statement: $ => seq(
      'try',
      optional(field('resources', $.resource_specification)),
      field('body', $.block),
      repeat($.catch_clause),
      optional($.finally_clause),
    ),

    // §4 — `try (Foo f = expr; Bar b = expr) { … }`. Resources
    // are typed or `def`-style declarators with required `=`
    // initializer (per Java-style try-with-resources). Multiple
    // resources separated by `;`.
    resource_specification: $ => seq(
      '(',
      $.resource,
      repeat(seq(';', $.resource)),
      optional(';'),
      ')',
    ),

    resource: $ => seq(
      optional(field('type', $._type)),
      field('name', $.identifier),
      '=',
      field('value', $._expression),
    ),

    catch_clause: $ => seq(
      'catch',
      '(',
      field('parameter', $.catch_formal_parameter),
      ')',
      field('body', $.block),
    ),

    // §4 — `catch_formal_parameter` accepts either a single _type
    // or a multi_type for Java 7+ / Groovy multi-catch. Closes
    // murtaza64 #39.
    catch_formal_parameter: $ => seq(
      field('type', choice($._type, $.multi_type)),
      field('name', $.identifier),
    ),

    multi_type: $ => seq(
      $._type,
      repeat1(seq('|', $._type)),
    ),

    finally_clause: $ => seq(
      'finally',
      field('body', $.block),
    ),

    // §4 — class / trait / interface declarations. v1 supports the
    // shape `[keyword] Name { body }`. extends, implements, generics,
    // modifiers, and sealed / permits clauses land in later
    // iterations. trait closes the dekobon #247 trait row.
    // §4 — class declaration with optional Groovy 4 sealed modifier
    // and `permits` clause. The `non-sealed` form is a single
    // anonymous token (token('non-sealed')) so the dash doesn't
    // get lexed as subtraction.
    class_declaration: $ => seq(
      repeat($.annotation),
      repeat($._modifier),
      optional(choice('sealed', token('non-sealed'))),
      'class',
      field('name', $.identifier),
      optional($.superclass),
      optional($.super_interfaces),
      optional($.permits_clause),
      field('body', $.class_body),
    ),

    trait_declaration: $ => seq(
      repeat($.annotation),
      repeat($._modifier),
      optional(choice('sealed', token('non-sealed'))),
      'trait',
      field('name', $.identifier),
      optional($.extends_interfaces),
      optional($.permits_clause),
      field('body', $.class_body),
    ),

    interface_declaration: $ => seq(
      repeat($.annotation),
      repeat($._modifier),
      optional(choice('sealed', token('non-sealed'))),
      'interface',
      field('name', $.identifier),
      optional($.extends_interfaces),
      optional($.permits_clause),
      field('body', $.class_body),
    ),

    // §4 — class has a single super and zero-or-more interfaces.
    superclass: $ => seq('extends', $._type),

    super_interfaces: $ => seq(
      'implements',
      $._type,
      repeat(seq(',', $._type)),
    ),

    // §4 — trait / interface extend multiple interfaces (no
    // `implements` keyword in that position).
    extends_interfaces: $ => seq(
      'extends',
      $._type,
      repeat(seq(',', $._type)),
    ),

    // §4 — Java-style modifiers. Each one is a hard reserved
    // keyword (§5.2) so the `word: $.identifier` directive keeps
    // them from accidentally being treated as identifiers.
    _modifier: _ => choice(
      'public',
      'private',
      'protected',
      'static',
      'final',
      'abstract',
      'synchronized',
      'native',
      'transient',
      'volatile',
      'strictfp',
    ),

    permits_clause: $ => seq(
      'permits',
      $._type,
      repeat(seq(',', $._type)),
    ),

    // §4 — `@interface Foo { … }` declares an annotation type.
    // `@` followed by the `interface` keyword is unambiguous —
    // `interface` is a hard reserved keyword (§5.2) so the
    // alternative parse `annotation` (which needs identifier)
    // cannot fire here.
    annotation_type_declaration: $ => seq(
      repeat($.annotation),
      '@',
      'interface',
      field('name', $.identifier),
      field('body', $.class_body),
    ),

    // §4 — `{ method* }` for v1. Field declarations, static
    // initialisers, and inner classes land later.
    class_body: $ => seq(
      '{',
      repeat($._class_member),
      '}',
    ),

    _class_member: $ => choice(
      $.method_declaration,
    ),

    // §4 — `def name(params) body`. Typed return type (`String foo()
    // { … }`) lands once we can disambiguate from local variable
    // declaration. Modifiers and throws clause similarly defer.
    // prec(1) wins over local_variable_declaration when `(` follows
    // the name, so `def f() { … }` at script-top-level parses as a
    // method rather than a `def f` variable followed by `(…) { … }`.
    method_declaration: $ => prec.right(1, seq(
      repeat($.annotation),
      repeat($._modifier),
      choice('def', field('return_type', $._type)),
      field('name', $._property_name),
      field('parameters', $.formal_parameters),
      optional($.throws_clause),
      optional(field('body', $.block)),
    )),

    // §4 — `throws E1, E2, …` after a method signature.
    throws_clause: $ => prec.left(seq(
      'throws',
      $._type,
      repeat(seq(',', $._type)),
    )),

    formal_parameters: $ => seq(
      '(',
      optional(seq(
        $.formal_parameter,
        repeat(seq(',', $.formal_parameter)),
      )),
      ')',
    ),

    // §5.13 — formal parameter with optional type, optional
    // varargs marker (`Type... name`), and optional default value
    // (`name = expr`).
    formal_parameter: $ => seq(
      optional(field('type', choice($._type, $.varargs_type))),
      field('name', $.identifier),
      optional(seq('=', field('default', $._expression))),
    ),

    varargs_type: $ => seq($._type, '...'),

    // §4 — enum. Closes murtaza64 #36 first half. Constants list
    // (optionally trailing-comma) optionally followed by `;` and
    // further class members. Constants may carry constructor
    // arguments (`RED(0xff0000)`); v1 supports that via the
    // optional argument_list.
    enum_declaration: $ => seq(
      repeat($.annotation),
      'enum',
      field('name', $.identifier),
      field('body', $.enum_body),
    ),

    enum_body: $ => seq(
      '{',
      optional(seq(
        $.enum_constant,
        repeat(seq(',', $.enum_constant)),
        optional(','),
      )),
      optional(seq(';', repeat($._class_member))),
      '}',
    ),

    enum_constant: $ => seq(
      field('name', $.identifier),
      optional(field('arguments', $.argument_list)),
    ),

    // §4 — record (Groovy 4+). `record Name(Type a, Type b) [{ body }]`.
    // `record` is a contextual keyword — tree-sitter's word: directive
    // keeps `record` usable as a plain identifier elsewhere.
    record_declaration: $ => prec.right(seq(
      repeat($.annotation),
      'record',
      field('name', $.identifier),
      field('components', $.record_components),
      optional(field('body', $.class_body)),
    )),

    record_components: $ => seq(
      '(',
      optional(seq(
        $.record_component,
        repeat(seq(',', $.record_component)),
      )),
      ')',
    ),

    record_component: $ => seq(
      field('type', $._type),
      field('name', $.identifier),
    ),

    // §4 — package and import. Trailing `;` is optional in Groovy;
    // we accept both forms by not requiring it.
    package_declaration: $ => seq(
      'package',
      field('name', $.qualified_name),
    ),

    // §4 — `import [static] qualified.Name [.*] [as Alias]`.
    // `.*` is a single anonymous token so it lexes distinctly from
    // a continuation of qualified_name (`.identifier`). Wildcard
    // imports only appear in this position, so the lexer-wide
    // longest-match is safe.
    import_declaration: $ => seq(
      'import',
      optional('static'),
      field('name', $.qualified_name),
      optional(token('.*')),
      optional(seq('as', field('alias', $.identifier))),
    ),

    qualified_name: $ => seq(
      $.identifier,
      repeat(seq('.', $.identifier)),
    ),

    // §4 / §5.12 — `def x = …`, `var x = …`, and `Type x = …`.
    // The typed form requires every declarator to carry an
    // `= value` initializer; bare `Type x` without initializer
    // would conflict with method_invocation `Type(x)` and is
    // disallowed in v1. Multiple declarators per statement
    // (`def a = 1, b = 2`) are supported in all three forms.
    local_variable_declaration: $ => prec.right(choice(
      seq(
        choice('def', 'var'),
        $.variable_declarator,
        repeat(seq(',', $.variable_declarator)),
      ),
      seq(
        field('type', $._type),
        $._initialized_declarator,
        repeat(seq(',', $._initialized_declarator)),
      ),
    )),

    _initialized_declarator: $ => seq(
      field('name', $.identifier),
      '=',
      field('value', $._expression),
    ),

    variable_declarator: $ => seq(
      field('name', $.identifier),
      optional(seq('=', field('value', $._expression))),
    ),

    // §4 — `def (a, b) = expr`. Closes murtaza64 #22. v1 requires
    // the leading `def`; the no-`def` form `(a, b) = expr` lands
    // later (it shares a prefix with parenthesized_expression and
    // needs a conflict declaration).
    multi_assignment_declaration: $ => seq(
      'def',
      '(',
      $.variable_declarator,
      repeat(seq(',', $.variable_declarator)),
      ')',
      '=',
      field('value', $._expression),
    ),

    // §4 / §10 row #37 — Jenkins-style `pipeline { … }` block.
    // Closes murtaza64 #37 (pipeline as a regular statement, not
    // a file-trailer-only construct). The body is a closure, since
    // that's how Jenkins's DSL is shaped — `pipeline` is followed
    // by a closure literal with nested stages, steps, etc.
    pipeline_statement: $ => seq(
      'pipeline',
      field('body', $.closure),
    ),

    // §5.9 — annotation usage. Lands on its own here; class /
    // method / record / enum declarations gain an optional
    // leading `repeat($.annotation)` so a `@Foo @Bar class X { }`
    // attaches the annotations to the class node.
    annotation: $ => seq(
      '@',
      field('name', $.qualified_name),
      optional(field('arguments', $.argument_list)),
    ),

    // §4 — `for (x in xs)`. Untyped variable for v1. Typed
    // (`for (Type x in xs)`) lands later. The `in` keyword here
    // is a structural delimiter, not the membership_expression
    // operator — context disambiguates because the for-header is
    // not an expression position.
    for_in_statement: $ => seq(
      'for',
      '(',
      field('variable', $.identifier),
      'in',
      field('value', $._expression),
      ')',
      field('body', choice($.block, $._statement)),
    ),

    expression_statement: $ => $._expression,

    // §3.2 PRIMARY — minimal alternative set for now. Every primary
    // expression form from §3.2 / §5 will land in this choice over
    // subsequent iterations.
    _expression: $ => choice(
      $.identifier,
      $.number_literal,
      $.string_literal,
      $.boolean_literal,
      $.null_literal,
      $.parenthesized_expression,
      $.unary_expression,
      $.list_literal,
      $.map_literal,
      $.closure,
      $.binary_expression,
      $.range_expression,
      $.identity_expression,
      $.spaceship_expression,
      $.regex_find_expression,
      $.regex_match_expression,
      $.ternary_expression,
      $.elvis_expression,
      $.logical_implication_expression,
      $.assignment_expression,
      $.power_expression,
      $.subscript_expression,
      $.safe_subscript_expression,
      $.method_invocation,
      $.update_expression,
      $.unary_update_expression,
      $.field_access,
      $.safe_navigation_expression,
      $.safe_chain_dot_expression,
      $.spread_dot_expression,
      $.method_pointer_expression,
      $.direct_field_access_expression,
      $.method_reference_expression,
      $.object_creation_expression,
      $.parenthesized_type_cast,
      $.cast_expression,
      $.membership_expression,
      $.instanceof_expression,
      $.switch_expression,
    ),

    // §4 — Groovy 4 switch-expression. Single rule for both classic
    // `case … :` and arrow `case … -> …` branches; arrow form
    // closes murtaza64 #36 (second half). Used as a statement via
    // expression_statement, or as an expression directly per the
    // §8.2 #36 regression.
    switch_expression: $ => seq(
      'switch',
      '(',
      field('value', $._expression),
      ')',
      field('body', $.switch_block),
    ),

    switch_block: $ => seq(
      '{',
      repeat(choice(
        $.switch_case,
        $.switch_arrow_case,
        $.switch_default,
      )),
      '}',
    ),

    switch_case: $ => seq(
      'case',
      field('value', $._expression),
      ':',
      repeat($._statement),
    ),

    switch_arrow_case: $ => seq(
      'case',
      field('value', $._expression),
      '->',
      field('body', choice($.block, $._statement)),
    ),

    switch_default: $ => seq(
      'default',
      choice(
        seq(':', repeat($._statement)),
        seq('->', field('body', choice($.block, $._statement))),
      ),
    ),

    // §3.2 — `x as Type`. RELATIONAL level (10), left-associative.
    // RHS is a _type, NOT an _expression — that is the distinction
    // from `x === foo` and other binary ops at the same level.
    cast_expression: $ => prec.left(PREC.RELATIONAL, seq(
      field('value', $._expression),
      'as',
      field('type', $._type),
    )),

    // §3.2 — `x in xs` and `x !in xs`. RELATIONAL level. Both
    // operators share one node kind; the `operator` field
    // distinguishes them.
    membership_expression: $ => {
      const ops = ['in', '!in'];
      return choice(...ops.map(op => prec.left(PREC.RELATIONAL, seq(
        field('element', $._expression),
        field('operator', op),
        field('collection', $._expression),
      ))));
    },

    // §3.2 / §3.2.2 — `x instanceof T` and `x !instanceof T`. RHS
    // is `_type`, NOT `_expression`. The Apache trailing-context
    // rule for `!instanceof` / `!in` (must be followed by
    // whitespace or open-bracket) is enforced in v1 only by the
    // word boundary on the surrounding tokens; a proper scanner
    // branch lands later if tests show a mis-tokenisation.
    instanceof_expression: $ => {
      const ops = ['instanceof', '!instanceof'];
      return choice(...ops.map(op => prec.left(PREC.RELATIONAL, seq(
        field('value', $._expression),
        field('operator', op),
        field('type', $._type),
      ))));
    },

    // §3.2 / §5.15 — `new Foo(args)`. We only handle the plain
    // constructor-call form here; array creation `new Foo[10]`,
    // generic types, and array initialisers land later. PREC.PRIMARY
    // so it composes with the access tier from the left
    // (`new Foo().bar`).
    object_creation_expression: $ => prec.right(PREC.PRIMARY, seq(
      'new',
      field('type', $._type),
      field('arguments', $.argument_list),
    )),

    // §5.15 — C-style cast `(Foo) x`. Distinct from
    // `parenthesized_expression` followed by something (resolved
    // by tree-sitter's GLR; declared as a conflict if needed).
    // PREC.UNARY right-associative so `(Foo) -x` parses as
    // `(Foo)(-x)` and the type binds to the rightward expression.
    parenthesized_type_cast: $ => prec.right(PREC.UNARY, seq(
      '(',
      field('type', $._type),
      ')',
      field('value', $._expression),
    )),

    // §4 — `_type`. Supports unqualified type identifiers,
    // qualified types (`java.util.List`), and array types
    // (`int[]`, `String[][]`). Generics (`List<String>`) are
    // tracked in `docs/divergences-from-spec.md` §5.
    _type: $ => choice(
      alias($.identifier, $.type_identifier),
      $.array_type,
      $.qualified_type,
    ),

    qualified_type: $ => prec.left(seq(
      alias($.identifier, $.type_identifier),
      repeat1(seq('.', alias($.identifier, $.type_identifier))),
    )),

    array_type: $ => prec(PREC.ARRAY_TYPE, seq($._type, '[', ']')),

    // §3.1 ACCESS tier / §3.2 — seven distinct node kinds for the
    // dot family. All at PREC.ACCESS left-associative so chains like
    // `a.b.c` parse `(a.b).c`. Right operand is the property /
    // field / method name (an identifier; method_reference also
    // accepts the `new` keyword per §5.14). Each operator emits a
    // unique node kind so downstream tooling (`get_op_type()`) can
    // discriminate without tree-shape heuristics.

    field_access: $ => prec.left(PREC.ACCESS, seq(
      field('object', $._expression),
      '.',
      field('field', $._property_name),
    )),

    safe_navigation_expression: $ => prec.left(PREC.ACCESS, seq(
      field('object', $._expression),
      '?.',
      field('property', $._property_name),
    )),

    // Groovy 4+ — see SPECIFICATION.md §3.2. Unlike `?.`, which only
    // short-circuits when the immediate receiver is null, `??.`
    // propagates null through the whole chain.
    safe_chain_dot_expression: $ => prec.left(PREC.ACCESS, seq(
      field('object', $._expression),
      '??.',
      field('property', $._property_name),
    )),

    spread_dot_expression: $ => prec.left(PREC.ACCESS, seq(
      field('object', $._expression),
      '*.',
      field('property', $._property_name),
    )),

    method_pointer_expression: $ => prec.left(PREC.ACCESS, seq(
      field('object', $._expression),
      '.&',
      field('method', $._property_name),
    )),

    direct_field_access_expression: $ => prec.left(PREC.ACCESS, seq(
      field('object', $._expression),
      '.@',
      field('field', $._property_name),
    )),

    // §5.2 — property / method names after a dot operator (or as a
    // method declaration name) can be either an identifier or a
    // *quoted identifier*: a string literal that escapes spaces,
    // keywords, or special chars. `map."key with spaces"` and
    // `def "abstract"() { … }` both rely on this.
    _property_name: $ => choice(
      $.identifier,
      alias($.string_literal, $.quoted_identifier),
    ),

    // §5.14 — method reference. Groovy 3+. The RHS is either an
    // identifier (`String::length`) or the `new` keyword
    // (`Foo::new`).
    method_reference_expression: $ => prec.left(PREC.ACCESS, seq(
      field('target', $._expression),
      '::',
      field('name', choice($.identifier, 'new')),
    )),

    // §3.2 — postfix `++` / `--`. Distinct node kind from the
    // prefix form (`unary_update_expression`) per the spec table.
    // PREC.POSTFIX, left-associative.
    update_expression: $ => prec.left(PREC.POSTFIX, seq(
      field('operand', $._expression),
      field('operator', choice('++', '--')),
    )),

    // §3.2 — prefix `++` / `--`. Apache groups these with UNARY_ADD
    // (level 3); we use PREC.UNARY to match.
    unary_update_expression: $ => prec.right(PREC.UNARY, seq(
      field('operator', choice('++', '--')),
      field('operand', $._expression),
    )),

    // §3.2 POSTFIX / §3.2.1 — `f(args)` and the subscript-yielding-
    // callable composition `arr[i](args)`. The callee slot is any
    // expression so subscript / future field_access / future
    // safe_navigation chains compose naturally. PREC.POSTFIX
    // matches subscript so chains like `obj[i](x)[j]` work.
    method_invocation: $ => prec.left(PREC.POSTFIX, seq(
      field('function', $._expression),
      field('arguments', $.argument_list),
    )),

    // §3.2 / §5.11 — argument list. Positional args are `_expression`;
    // named args (Groovy's `k: v` in call position) are `map_entry`
    // aliased to `named_argument` so downstream tools can tell them
    // apart from positional values. Spread (`*expr`) lands later.
    argument_list: $ => seq(
      '(',
      optional(seq(
        $._argument,
        repeat(seq(',', $._argument)),
        optional(','),
      )),
      ')',
    ),

    _argument: $ => choice(
      $._expression,
      alias($.map_entry, $.named_argument),
      $.spread_arguments,
    ),

    // §5.11 — `*expr` only inside argument list / list literal.
    // Distinct from a binary `*` because the LHS is absent.
    spread_arguments: $ => seq(
      '*',
      field('value', $._expression),
    ),

    // §3.1 POSTFIX / §3.2 — subscript and safe-subscript. The
    // distinction `arr?[k]` (safe) vs `arr[k]` (regular) is the
    // explicit ask of dekobon #247 for the `?[]` row. Both at
    // PREC.POSTFIX, left-associative — `a[b][c]` parses as
    // `(a[b])[c]`.
    subscript_expression: $ => prec.left(PREC.POSTFIX, seq(
      field('object', $._expression),
      '[',
      field('index', $._expression),
      ']',
    )),

    // `?[` is one token. `a ? [b] : c` (with space) still parses
    // as ternary plus list because tree-sitter's lexer never
    // crosses a whitespace boundary to extend a multi-char token.
    safe_subscript_expression: $ => prec.left(PREC.POSTFIX, seq(
      field('object', $._expression),
      '?[',
      field('index', $._expression),
      ']',
    )),

    // §3.1 level 2 / §3.2 — `**`. Distinct node kind so metric tools
    // count power separately from multiplicative ops. Right-assoc
    // per Apache: `1 ** 2 ** 3` parses as `1 ** (2 ** 3)`. POWER
    // binds tighter than UNARY (+ -) but looser than UNARY_NOT
    // (! ~) — see PREC table comments.
    power_expression: $ => prec.right(PREC.POWER, seq(
      field('left', $._expression),
      '**',
      field('right', $._expression),
    )),

    // §3.1 level 13.5 / §3.2 — `==>`. Right-associative per Apache
    // (`<assoc=right>`), distinct node kind so downstream tools can
    // discriminate from `=>` (lambda arrow, not yet wired).
    logical_implication_expression: $ => prec.right(PREC.IMPLICATION, seq(
      field('left', $._expression),
      '==>',
      field('right', $._expression),
    )),

    // §3.1 level 15 / §3.2 — single `assignment_expression` node
    // covers every augmented form (`+=`, `-=`, `*=`, `/=`, `%=`,
    // `**=`, `<<=`, `>>=`, `>>>=`, `&=`, `^=`, `|=`) plus plain `=`
    // and Elvis-assign `?=`. The operator field distinguishes them.
    // Right-associative per Apache. ASSIGN is the loosest precedence
    // tier and binds looser than CONDITIONAL above it.
    assignment_expression: $ => {
      const ops = [
        '=',
        '+=', '-=', '*=', '/=', '%=', '**=',
        '<<=', '>>=', '>>>=',
        '&=', '^=', '|=',
        '?=',
      ];
      return choice(...ops.map(op => prec.right(PREC.ASSIGN, seq(
        field('left', $._expression),
        field('operator', op),
        field('right', $._expression),
      ))));
    },

    // §3.1 level 14 / §3.2 — ternary and Elvis share the CONDITIONAL
    // precedence level and are both right-associative. Apache groups
    // them in one `conditionalExprAlt` so `a ? b ?: c : d` parses as
    // `a ? (b ?: c) : d` because the middle slot recursively re-enters
    // the same precedence level. We replicate this with two distinct
    // rules at PREC.CONDITIONAL; right-associativity + recursion via
    // `$._expression` give the same result.
    //
    // elvis_expression is the headline ask of
    // dekobon/big-code-analysis #246: downstream tools need to
    // discriminate elvis from ternary by node kind alone.
    ternary_expression: $ => prec.right(PREC.CONDITIONAL, seq(
      field('condition', $._expression),
      '?',
      field('consequence', $._expression),
      ':',
      field('alternative', $._expression),
    )),

    elvis_expression: $ => prec.right(PREC.CONDITIONAL, seq(
      field('value', $._expression),
      '?:',
      field('default', $._expression),
    )),

    // §3.2 — distinct-node ops that share precedence with existing
    // binary_expression tiers but emit unique node kinds per the
    // operator-coverage matrix. The integration target (`get_op_type()`
    // in dekobon/big-code-analysis) needs to discriminate these by
    // node kind alone, not by tree-shape heuristics.

    // §3.2 range_expression — inclusive `..`, exclusive `..<` (right),
    // exclusive `<..` (left), exclusive `<..<` (both). SHIFT_OR_RANGE
    // precedence level, left-associative.
    range_expression: $ => {
      const ranges = ['..', '..<', '<..', '<..<'];
      return choice(...ranges.map(op => prec.left(PREC.SHIFT_OR_RANGE, seq(
        field('start', $._expression),
        field('operator', op),
        field('end', $._expression),
      ))));
    },

    // §3.2 identity_expression — `===` (identity) and `!==`
    // (non-identity), EQUALITY level. Per Apache, these live in
    // the equality alternative.
    identity_expression: $ => {
      const ops = ['===', '!=='];
      return choice(...ops.map(op => prec.left(PREC.EQUALITY, seq(
        field('left', $._expression),
        field('operator', op),
        field('right', $._expression),
      ))));
    },

    // §3.2 spaceship_expression — `<=>`, EQUALITY level. Three-way
    // comparison; no `operator` field since there is only one form.
    spaceship_expression: $ => prec.left(PREC.EQUALITY, seq(
      field('left', $._expression),
      '<=>',
      field('right', $._expression),
    )),

    // §3.2 regex_find_expression — `=~`, EQUALITY level. Matches a
    // pattern at any position. Fields are `subject` / `pattern` so
    // downstream tools can tell the regex-bearing operand from the
    // input expression.
    regex_find_expression: $ => prec.left(PREC.EQUALITY, seq(
      field('subject', $._expression),
      '=~',
      field('pattern', $._expression),
    )),

    // §3.2 regex_match_expression — `==~`, EQUALITY level. Anchored
    // whole-string match. Same field shape as regex_find_expression.
    regex_match_expression: $ => prec.left(PREC.EQUALITY, seq(
      field('subject', $._expression),
      '==~',
      field('pattern', $._expression),
    )),

    // §3.2 — single binary_expression node with an `operator` field
    // for every Java-shaped binary operator (per §3.2 table row 1).
    // Apache groups arithmetic operators across two precedence
    // levels (MUL: * / %, ADD: + -); we keep them on one rule with
    // distinct precedence numbers so the *node kind* stays uniform
    // while the precedence ladder matches Apache exactly.
    //
    // Currently covers ADDITIVE and MULTIPLICATIVE. Shift/range,
    // relational, equality, bitwise, and logical levels join this
    // rule in later iterations — each is a single line addition.
    binary_expression: $ => {
      /* eslint-disable no-multi-spaces --
       * Aligned columns make the precedence ladder readable.
       */
      const ops = [
        // ADDITIVE — §3.1 level 5
        ['+',   PREC.ADDITIVE],
        ['-',   PREC.ADDITIVE],
        // MULTIPLICATIVE — §3.1 level 4
        ['*',   PREC.MULTIPLICATIVE],
        ['/',   PREC.MULTIPLICATIVE],
        ['%',   PREC.MULTIPLICATIVE],
        // SHIFT — §3.1 level 6. Ranges (`..`, `..<`, etc.) share
        // this precedence level but use distinct node kinds, so
        // they live in their own rule (lands later).
        ['<<',  PREC.SHIFT_OR_RANGE],
        ['>>',  PREC.SHIFT_OR_RANGE],
        ['>>>', PREC.SHIFT_OR_RANGE],
        // RELATIONAL — §3.1 level 7. `in`, `!in`, `instanceof`,
        // `!instanceof`, and `as` share this level but emit
        // distinct node kinds (`membership_expression`,
        // `instanceof_expression`, `cast_expression`), so they
        // land in their own rules later.
        ['<',   PREC.RELATIONAL],
        ['<=',  PREC.RELATIONAL],
        ['>',   PREC.RELATIONAL],
        ['>=',  PREC.RELATIONAL],
        // EQUALITY — §3.1 level 8. `===`, `!==`, `<=>`, `=~`,
        // `==~` share this level but emit distinct node kinds.
        ['==',  PREC.EQUALITY],
        ['!=',  PREC.EQUALITY],
        // BITWISE — §3.1 levels 9, 10, 11 (each tighter than the
        // next: & < ^ < | by precedence).
        ['&',   PREC.BITWISE_AND],
        ['^',   PREC.BITWISE_XOR],
        ['|',   PREC.BITWISE_OR],
        // LOGICAL — §3.1 levels 12, 13 (&& tighter than ||).
        ['&&',  PREC.LOGICAL_AND],
        ['||',  PREC.LOGICAL_OR],
      ];
      /* eslint-enable no-multi-spaces */
      return choice(...ops.map(([op, p]) => prec.left(p, seq(
        field('left', $._expression),
        field('operator', op),
        field('right', $._expression),
      ))));
    },

    // §5.8 — closure expression. `{ -> }` is the explicit no-parameter
    // form; `{ a, b -> body }` has typed-less identifier parameters;
    // `{ body }` has no `->` and an implicit `it` parameter (we do
    // NOT emit a synthetic `it` per §5.8). Closure parameter types,
    // defaults, and varargs land later — for v1 of this rule we
    // accept untyped identifier parameters only, which is what the
    // overwhelming majority of Groovy / Jenkins / Spock closures
    // use in practice.
    closure: $ => seq(
      '{',
      optional($.closure_parameters),
      repeat($._statement),
      '}',
    ),

    closure_parameters: $ => seq(
      optional(seq(
        $.closure_parameter,
        repeat(seq(',', $.closure_parameter)),
      )),
      '->',
    ),

    closure_parameter: $ => field('name', $.identifier),

    // §3.2 PRIMARY / §5.13 — list literal with trailing-comma support.
    // Empty list is `[]`; the empty map literal is the distinct `[:]`
    // form (see map_literal). Tree-sitter splits the parse between
    // list_literal and map_literal until a `:` or `]` disambiguates;
    // the conflict declaration above keeps both alternatives alive.
    list_literal: $ => seq(
      '[',
      optional(seq(
        $._expression,
        repeat(seq(',', $._expression)),
        optional(','),
      )),
      ']',
    ),

    // §3.2 PRIMARY / §5.3 — map literal. `[:]` is the empty map;
    // anything else is one-or-more `key: value` entries. The
    // distinction between map_entry and a labelled statement is
    // contextual — labels are statement-start-only (see §5.3 and
    // the LABEL_COLON scanner branch), so inside `[ … ]` a `:` after
    // an identifier is always a map key separator.
    map_literal: $ => {
      const entry = choice($.map_entry, $.spread_map_entry);
      return seq(
        '[',
        choice(
          ':',
          seq(
            entry,
            repeat(seq(',', entry)),
            optional(','),
          ),
        ),
        ']',
      );
    },

    map_entry: $ => seq(
      field('key', $._expression),
      ':',
      field('value', $._expression),
    ),

    // §5.11 — `*: expr` only inside a map literal. `*:` is a single
    // anonymous token (longest-match keeps it intact); a bare `*`
    // followed by a `:` colon outside this context would have to be
    // syntactically impossible by the surrounding rules.
    spread_map_entry: $ => seq(
      '*:',
      field('value', $._expression),
    ),

    // §3.2 PRIMARY — `( expr )`. Distinct from `parenthesized_type_cast`
    // (§5.15), which lands once we have `_type`. The parser disambiguates
    // by what's inside the parens.
    parenthesized_expression: $ => seq('(', $._expression, ')'),

    // §3.2 — single `unary_expression` node kind but two precedence
    // alternatives, matching Apache §3.1:
    //   UNARY_NOT (`!` `~`)  — level 16, tighter than POWER (15),
    //                          so `~2 ** 3` parses as `(~2) ** 3`.
    //   UNARY     (`+` `-`)  — level 14, looser than POWER (15),
    //                          so `-2 ** 3` parses as `-(2 ** 3)`.
    // Both alternatives produce the same node kind, so downstream
    // tools see one consistent shape and discriminate by operator.
    unary_expression: $ => choice(
      prec.right(PREC.UNARY_NOT, seq(
        field('operator', choice('!', '~')),
        field('operand', $._expression),
      )),
      prec.right(PREC.UNARY, seq(
        field('operator', choice('+', '-')),
        field('operand', $._expression),
      )),
    ),

    // §3.2 PRIMARY — boolean and null are keywords in Groovy. The
    // `word: $.identifier` directive above lets tree-sitter resolve
    // the keyword-vs-identifier ambiguity automatically: a literal
    // 'true' / 'false' / 'null' that matches the identifier regex
    // is reclassified as the keyword token when one of these rules
    // is the only legal next reduction.
    boolean_literal: _ => choice('true', 'false'),

    null_literal: _ => 'null',

    // §5.7 — string flavours. Single and triple-single never
    // interpolate. Double-quoted and triple-double *do* interpolate
    // GString segments in Groovy proper, but for v1 we tokenise
    // them as flat strings; the interpolation scanner branch
    // (§6.3) lands as a follow-up. Slashy and dollar-slashy land
    // alongside that scanner work.
    string_literal: $ => choice(
      $._single_quoted_string,
      $._triple_single_quoted_string,
      $._double_quoted_string,
      $._triple_double_quoted_string,
      $._dollar_slashy_string,
      $._slashy_string_start,
    ),

    _single_quoted_string: _ => token(seq(
      '\'',
      repeat(choice(
        /[^'\\\n\r]/,
        /\\[\s\S]/,
      )),
      '\'',
    )),

    _triple_single_quoted_string: _ => token(seq(
      '\'\'\'',
      repeat(choice(
        /[^'\\]/,
        /\\[\s\S]/,
        /'[^']/,
        /''[^']/,
      )),
      '\'\'\'',
    )),

    _double_quoted_string: _ => token(seq(
      '"',
      repeat(choice(
        /[^"\\\n\r]/,
        /\\[\s\S]/,
      )),
      '"',
    )),

    _triple_double_quoted_string: _ => token(seq(
      '"""',
      repeat(choice(
        /[^"\\]/,
        /\\[\s\S]/,
        /"[^"]/,
        /""[^"]/,
      )),
      '"""',
    )),

    // §5.7 — `$/...$/` dollar-slashy strings. Spec calls this a
    // simple grammar rule (no scanner needed) because `$` is not a
    // binary operator, so there is no division-vs-slashy
    // disambiguation issue. Treated as a flat token in v1;
    // interpolation structure lands with the GString scanner.
    // Body escape rules:
    //   $$  → literal $
    //   $/  → literal /
    //   $X  → bare X (interpolation in real Groovy; flat here)
    //   /X  with X != $ → literal /
    // The closing `/$` is matched by longest-match.
    _dollar_slashy_string: _ => token(seq(
      '$/',
      repeat(choice(
        /[^/$]/,
        /\/[^$]/,
        /\$\$/,
        /\$\//,
        /\$[^/$]/,
      )),
      '/$',
    )),

    // §5.1 — hex (0x…), binary (0b…), decimal int / float / scientific,
    // each with optional type suffix from { G g L l I i D d F f }.
    // Underscores between digits are accepted permissively (the spec
    // forbids leading/trailing `_`, but that is a semantic check,
    // not a grammar one — see §1.2).
    number_literal: _ => token(choice(
      /0[xX][0-9a-fA-F_]+[GgLlIi]?/,
      /0[bB][01_]+[GgLlIi]?/,
      /[0-9][0-9_]*\.[0-9][0-9_]*([eE][+-]?[0-9][0-9_]*)?[GgFfDd]?/,
      /[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*[GgFfDd]?/,
      /[0-9][0-9_]*[GgLlIiDdFf]?/,
    )),

    identifier: _ => /[A-Za-z_$][A-Za-z0-9_$]*/,
  },
});
