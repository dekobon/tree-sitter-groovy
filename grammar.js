/**
 * @file Apache Groovy grammar for tree-sitter
 * @author Elijah Zupancic <elijah@zupancic.name>
 * @license MIT
 *
 * The authoritative design document for this grammar is
 * SPECIFICATION.md at the repository root. Every rule below maps to
 * a section there. Read the spec before making non-trivial changes.
 *
 * The external scanner (src/scanner.c) handles three context-sensitive
 * lexing situations that pure grammar rules cannot disambiguate:
 *   - Slashy-string-start vs. division
 *   - Line / block / Groovydoc comment single-token emit
 *   - The contextual `record` keyword, emitted only before a record
 *     header so the word stays an identifier everywhere else
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

/**
 * GString body segment choice shared between `"..."` and `"""..."""`.
 * The text rule differs (single-quoted bans newlines, triple-quoted
 * allows them) but every other alternative is identical.
 *
 * `_string_body` is never emitted. Listing it here makes it valid
 * wherever a body segment may start, which is how the external scanner
 * learns that it is inside a string (SPECIFICATION.md §6.10).
 *
 * @param {GrammarSymbols<string>} $ Grammar reference symbols.
 * @param {RuleOrLiteral} textRule The flavour-specific literal-text rule.
 * @returns {ChoiceRule} The shared body-segment choice.
 */
function gstringPart($, textRule) {
  return choice(
    alias(textRule, $.string_fragment),
    $.gstring_dollar_interpolation,
    $.gstring_brace_interpolation,
    alias($._gstring_literal_dollar, $.string_fragment),
    $._string_body,
  );
}

/**
 * The members of a type body, listed once. `enum_body` repeats the
 * list for the member directly after a trailing comma, where a plain
 * constructor is not allowed (see `enum_declaration`), and the two
 * lists must not drift apart.
 *
 * @param {GrammarSymbols<string>} $ Grammar reference symbols.
 * @param {RuleOrLiteral | null} constructor The constructor rule for
 *   this position, or null where a constructor cannot appear.
 * @returns {RuleOrLiteral[]} The member rules, in a fixed order.
 */
function classMembers($, constructor) {
  return [
    $.method_declaration,
    $.field_declaration,
    ...(constructor ? [constructor] : []),
    $.static_initializer,
    $.enum_declaration,
    $.class_declaration,
    $.interface_declaration,
    $.trait_declaration,
    $.record_declaration,
    $.annotation_type_declaration,
  ];
}

/**
 * A constructor. The enum position directly after a trailing comma
 * requires at least one modifier (see `enum_declaration`); everywhere
 * else modifiers are optional.
 *
 * @param {GrammarSymbols<string>} $ Grammar reference symbols.
 * @param {RuleOrLiteral} modifiers The modifier list for this position.
 * @returns {PrecRightRule} The constructor rule body.
 */
function constructorDeclaration($, modifiers) {
  return prec.right(3, seq(
    repeat($.annotation),
    modifiers,
    field('name', $.identifier),
    field('parameters', $.formal_parameters),
    optional($.throws_clause),
    field('body', $.block),
  ));
}

module.exports = grammar({
  name: 'groovy',

  // Placeholder for the real implementation. SPECIFICATION.md §3-§7
  // is the contract this rule set will fulfil.
  word: $ => $.identifier,

  // §5.2 — `new` is a hard keyword. Keyword extraction alone still
  // lexes it as `identifier` wherever the keyword has no action,
  // which let `def f = (Foo) new T()` after another member split
  // into a field `= (Foo)` and a method `T()` returning `new`. The
  // empty `property` set keeps `foo.new` (Apache `namePart`) valid;
  // `_property_name` also names methods, so `new` is not reserved
  // where a method name may follow (`def new`, `int new`).
  reserved: {
    global: _ => ['new'],
    property: _ => [],
  },

  extras: $ => [
    /\s/,
    // §5.10 — a line escape, `\` at the end of a line. Groovy's lexer
    // skips it as it skips spaces and tabs.
    /\\\r?\n/,
    $.line_comment,
    $.block_comment,
    $.groovydoc_comment,
  ],

  // Externals listed here must have a matching branch in
  // `src/scanner.c` AND be referenced by at least one rule below
  // (`_command_named_key` and `_subscript_open` included). There are
  // two exceptions. No rule references `_error_sentinel`: it exists so
  // the scanner can detect error recovery, when tree-sitter marks
  // every external valid. It is kept last here and in the scanner's
  // `TokenType` enum, and the two orders must match. The scanner never
  // emits `_string_body`: a string body references it so that the
  // scanner can tell it is inside one.
  // Tokens the spec §6 proposes as external scanner branches but
  // that this grammar implements in-grammar (GString interpolation
  // via regex tokens, slashy body via in-grammar `_slashy_text`,
  // label colon via grammar precedence, and statement boundaries
  // via `repeat(choice($._statement, ';'))` — newline is `extras`
  // whitespace and `;` is an anonymous list element, see §5.10)
  // are intentionally NOT listed: listing an unwired external token
  // would let dormant scanner branches mis-fire in error-recovery
  // states.
  externals: $ => [
    $._slashy_string_start,
    $.line_comment,
    $.block_comment,
    $.groovydoc_comment,
    $._record_keyword,
    // §5.5 — `name:` key of a named argument in a parenthesis-free call.
    // Scanner-side so the newline before a Spock label (`result` then
    // `where:`) is not swallowed, and so `Type name =` stays a declaration.
    $._command_named_key,
    // §5.10 — `[` on the same line as the token before it (spaces, tabs and
    // line escapes allowed). A `[` after a newline isn't one and can only
    // start a list or map literal: a statement, not a subscript of the line
    // above.
    $._subscript_open,
    // §6.10 — never emitted: valid only between the quotes of a string
    // whose body is lexed in pieces. Comments are extras, so the parser
    // offers them there too; the scanner must emit nothing, or `//` and
    // `/*` at a piece boundary become comments.
    $._string_body,
    // Never referenced by a rule: valid only during error recovery, when
    // the scanner must not emit context-sensitive tokens.
    $._error_sentinel,
  ],

  conflicts: $ => [
    // `final def x` / `final x = 1` in a block vs. a modified member at top level.
    [$._modifier, $.local_variable_declaration],
    // See SPECIFICATION.md §7. Populated as rules accumulate.
    // (list_literal vs map_literal — tree-sitter resolves without
    //  an explicit conflict because `:` after the first element
    //  always disambiguates within one token of lookahead.)

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

    // §3.2.2 — `_cast_type` overlaps `_type` on identifier / array /
    // generic alternatives; only the dotted path diverges. The
    // companion three-way conflict pulls in `_dotted_type` so the
    // dotted shape can win the trailing `.identifier` from the
    // surrounding `field_access` reduction.
    [$._cast_type, $._type],
    [$._cast_type, $._dotted_type, $.qualified_type],

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

    // §4 — `static NAME` inside a class body: after one or more
    // modifiers and one identifier, that identifier could be the
    // modifier-only field_declaration's variable_declarator name
    // (`static MEDIUM_TTL = 900`) or the start of a typed
    // field_declaration/method_declaration's `_type` (`static Foo
    // bar`). The next token (`=`/`,`/`;` vs a second identifier)
    // disambiguates.
    [$.variable_declarator, $._type],

    // §4 — `enum E { E(v, w) { } }`: after `E(` an identifier could be
    // a constant's argument (`_expression`) or a constructor's untyped
    // parameter (`formal_parameter`); the `{` after `)` decides. The
    // conflict is global by necessity — it is between the two
    // reductions, not their containers — and was checked inert
    // outside an enum body: declaring it on the v0.3.0 grammar makes
    // tree-sitter report it unnecessary. Regenerate without it to
    // confirm that still holds.
    [$.formal_parameter, $._expression],
    // `{ final x -> }` vs. a closure body opening with a modified
    // declaration; the `->` decides.
    [$._modifier, $.closure_parameter],
    // `{ final def x` is also a final local (#56), so all three stay
    // alive there.
    [$._modifier, $.local_variable_declaration, $.closure_parameter],
    // `{ a = 1 -> }` (a default) vs. `{ a = 1 }` (an assignment).
    [$._expression, $.closure_parameter],
    [$._initialized_declarator, $.closure_parameter],
    [$.variable_declarator, $.closure_parameter],
  ],

  rules: {
    // Top-level. Real rule set lands incrementally per
    // SPECIFICATION.md §4 (statement and declaration coverage).
    source_file: $ => seq(
      optional($.shebang),
      repeat(choice($._statement, ';')),
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
      $.synchronized_statement,
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
    // form (`println 'hello'`, `events 'a', 'b', 'c'`). v1 is
    // intentionally conservative: receiver is a bare identifier
    // and the first argument is restricted to literal-shaped
    // expressions, closures and named arguments (no risk of
    // mis-parsing `foo bar` as a command call on a bare identifier).
    // After a comma an argument is a named argument or any
    // expression: Apache's `argumentListElement` without spread
    // arguments (`foo 'a', *args` is an ERROR). Chained
    // `foo bar baz` lands later. Closes a subset of murtaza64 #5.
    command_chain: $ => prec(2, seq(
      field('receiver', $.identifier),
      field('argument', choice($._command_argument, alias($._command_named_argument, $.named_argument))),
      repeat(seq(',', field('argument', choice(
        alias($._command_named_argument, $.named_argument),
        alias($._command_continued_named_argument, $.named_argument),
        $._expression,
      )))),
    )),

    // §5.5 item 3 — named arguments in a parenthesis-free call:
    // `exclude group: 'a', module: 'b'`, `archiveArtifacts artifacts: 'x'`,
    // GORM's `name blank: false, size: 1..100`. The key is an identifier
    // lexed by the external scanner only when `:` follows on the same
    // line, so `Type name = …` and a bare statement followed by a Spock
    // label on the next line are unaffected.
    _command_named_argument: $ => seq(
      field('key', alias($._command_named_key, $.identifier)),
      ':',
      field('value', $._expression),
    ),

    // After a comma a newline cannot end the statement, so a key on a
    // continuation line (`sh script: 'make',` then `returnStdout: true`)
    // needs no scanner token. Same-line keys normally come from the
    // scanner; `case` and `default`, which the scanner rejects, reach
    // this rule on the same line too (`foo 'a', default: 1`).
    _command_continued_named_argument: $ => seq(
      field('key', $.identifier),
      ':',
      field('value', $._expression),
    ),

    _command_argument: $ => choice(
      $.string_literal,
      $.number_literal,
      $.boolean_literal,
      $.null_literal,
      $.closure,
    ),

    // §5.3 — `label: stmt`. Required only at statement-start
    // position; map_entries (which share `:`) are inside `[…]`
    // and don't collide here. `prec(1, ...)` keeps both
    // labeled_statement and expression_statement alive after
    // `identifier`; the `:` decides. A scanner-token form
    // (`_label_colon`) was attempted but interacted with
    // named-argument command chains (`archiveArtifacts artifacts: 'x'`),
    // which now lex their key through `_command_named_key` instead;
    // see divergence #5.
    labeled_statement: $ => prec(1, seq(
      field('label', $.identifier),
      ':',
      field('statement', $._statement),
    )),

    // §4 — block of statements: control-flow, method and constructor
    // bodies, static / instance initializers. Shares `{ ... }` syntax
    // with closure (§5.8). A `prec(1, …)` keeps block preferred at use
    // sites where both `block` and a closure-bearing `_statement`
    // are valid alternatives (e.g. the body slot of `if` / `while`).
    block: $ => prec(1, seq(
      '{',
      repeat(choice($._statement, ';')),
      '}',
    )),

    // §4 — if / else. `prec.right` resolves the dangling-else
    // ambiguity in favour of the closest `if`. Body accepts either
    // an explicit block (preferred) or a single statement. The
    // `optional(';')` after the consequence keeps the else attached
    // when a braceless consequence is terminated by a semicolon
    // (`if (x) a(); else b()`); without it the `;` is claimed by the
    // enclosing statement list (§5.10) and the `else` is severed. It
    // sits outside the else clause so the no-else form (`if (x) a();
    // b()`) does not require an `else` after eating the `;`.
    if_statement: $ => prec.right(seq(
      'if',
      '(',
      field('condition', $._expression),
      ')',
      field('consequence', choice($.block, $._statement)),
      optional(';'),
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
    // optional; bare `for (;;)` is the infinite-loop idiom. The init
    // slot accepts a single expression (`i = 0`) or a variable
    // declaration (`int i = 0`, `def i = 0`, incl. multiple
    // declarators). Apache `GroovyParser.g4` `forInit:
    // localVariableDeclaration | expressionList` also allows a
    // comma-separated expression list (`i = 0, j = 1`); that form is
    // not yet supported.
    for_statement: $ => seq(
      'for',
      '(',
      optional(field('init', choice($._expression, $.local_variable_declaration))),
      ';',
      optional(field('condition', $._expression)),
      ';',
      optional(field('update', $._expression)),
      ')',
      field('body', choice($.block, $._statement)),
    ),

    // §4 — do-while. Body is required; condition follows the body.
    // `optional(';')` lets a braceless body be terminated before
    // `while` (`do a(); while (c)`, the Java/Groovy `do S while (e);`
    // form); without it the `;` errors since `while` is required next.
    do_while_statement: $ => seq(
      'do',
      field('body', choice($.block, $._statement)),
      optional(';'),
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

    // §4 — `synchronized (lock) { … }` (Apache `#synchronizedStmtAlt`).
    // The body must be a block.
    synchronized_statement: $ => seq(
      'synchronized',
      '(',
      field('lock', $._expression),
      ')',
      field('body', $.block),
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
    // murtaza64 #39. The type itself is optional — Groovy allows a
    // bare `catch (e) { … }`, implicitly typed as Exception.
    catch_formal_parameter: $ => seq(
      optional(field('type', choice($._type, $.multi_type))),
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
      optional(field('type_parameters', $.type_parameters)),
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
      optional(field('type_parameters', $.type_parameters)),
      optional($.extends_interfaces),
      optional($.super_interfaces),
      optional($.permits_clause),
      field('body', $.class_body),
    ),

    interface_declaration: $ => seq(
      repeat($.annotation),
      repeat($._modifier),
      optional(choice('sealed', token('non-sealed'))),
      'interface',
      field('name', $.identifier),
      optional(field('type_parameters', $.type_parameters)),
      optional($.extends_interfaces),
      optional($.permits_clause),
      field('body', $.class_body),
    ),

    // §4 / §5.14 — generic type parameter list on a class /
    // interface / trait / method declaration. The `<` is
    // `token.immediate` so the parser only treats it as a type
    // parameter opener when it follows the declaration name with
    // no intervening whitespace.
    type_parameters: $ => seq(
      token.immediate('<'),
      $.type_parameter,
      repeat(seq(',', $.type_parameter)),
      '>',
    ),

    type_parameter: $ => seq(
      field('name', alias($.identifier, $.type_identifier)),
      optional(seq(
        'extends',
        $._type,
        repeat(seq('&', $._type)),
      )),
    ),

    // §4 — class has a single super and zero-or-more interfaces.
    superclass: $ => seq('extends', $._type),

    super_interfaces: $ => seq(
      'implements',
      $._type,
      repeat(seq(',', $._type)),
    ),

    // §4 — trait / interface list their super-interfaces with `extends`.
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

    // §4 — member list shared by class / interface / trait / record /
    // `@interface` bodies and the anonymous inner class body of
    // `object_creation_expression`.
    // The body accepts `;` as an empty member / terminator the same
    // way statement lists do (§5.10): a field or abstract method is
    // commonly written `int x = 1;` / `void f();`, and a stray `;` is
    // a no-op member. `;` is anonymous, so it never appears in the AST.
    //
    // A bare `block` is an instance initializer. It sits in the member
    // repeat — here and in the four identical repeats of `enum_body` —
    // rather than in `_class_member`, because `enum_body` also places
    // a lone `_class_member` directly after its constants. A block in
    // that slot is an LR conflict on `A() {` (constructor, or constant
    // plus initializer) and, once that is resolved, silently accepts
    // `A { }` (a constant with an anonymous class body, unsupported)
    // as constant-plus-initializer.
    class_body: $ => seq(
      '{',
      repeat(choice($._class_member, $.block, ';')),
      '}',
    ),

    // Nested type declarations are members, as in Apache's
    // `memberDeclaration`. Each kind must be listed: a declaration
    // keyword with no action at member start lexes as an identifier,
    // so `class Inner { … }` would silently parse as a field `Inner`
    // of type `class` followed by an instance initializer block, and
    // a nested `record R(int x) { }` as a method `R` returning
    // `record`. The record keyword comes from the scanner (§6.7), so
    // listing it here does not reserve `record` as a member name.
    //
    // The price: `trait` and `sealed` are keywords wherever a member
    // can start, as they already are where a statement can. A member
    // cannot open with one of them as a plain identifier, with or
    // without modifiers (`static trait = 1`, `sealed() { }`), nor use
    // one as the name after `modifier(s) Type`, because `private Foo`
    // is already a complete field (`private boolean sealed` is an
    // ERROR; `boolean sealed` is not). See divergence 12. No
    // `conflicts` entry is needed.
    _class_member: $ => choice(...classMembers($, $.constructor_declaration)),

    // §4 — constructors share the method_declaration shape but have
    // no return type and the name matches the enclosing class. We
    // don't enforce the name match (downstream tooling can), but we
    // emit a distinct node kind so IDE features (rename, jump) can
    // tell constructors apart from methods. `prec(3, …)` wins over
    // both field and method when the identifier is followed by `(`
    // and no return type precedes it.
    constructor_declaration: $ => constructorDeclaration($, repeat($._modifier)),

    _modified_constructor_declaration: $ => constructorDeclaration($, repeat1($._modifier)),

    // §4 — `static { … }` class-body initializer block. Runs once
    // when the class is loaded. Distinct node kind so downstream
    // tools can index it.
    static_initializer: $ => seq(
      'static',
      field('body', $.block),
    ),

    // §4 — class / interface / trait field. Java-style shape: optional
    // annotations, optional modifiers, `def` or a `_type`, then one or
    // more declarators (each with optional initializer). Method
    // declarations share the same prefix; `prec(2, …)` on
    // `method_declaration` and the trailing `(` discriminate.
    //
    // A modifier alone (no `def`, no type) also introduces a field —
    // `static MEDIUM_TTL = 900` infers Object, a common constant-field
    // idiom. Gated behind `repeat1($._modifier)` (not `repeat`) so this
    // alternative can't match a bare, modifier-less identifier, which
    // would collide with a plain expression_statement.
    field_declaration: $ => choice(
      // Prefer the typed field when both parses succeed, rather than
      // splitting `private final int v\n C() {}` after `int`. The
      // weight is above a method body's: the split reading holds a
      // method (`v C() {}`), which would otherwise tie.
      prec.dynamic(4, seq(
        repeat($.annotation),
        repeat($._modifier),
        choice('def', field('type', $._type)),
        $.variable_declarator,
        repeat(seq(',', $.variable_declarator)),
      )),
      // The negative weight settles the split that the typed weight
      // cannot. `private final int v` followed by `public A() { }` is
      // also `private final int`, a field `v public` and a constructor
      // `A() { }`: a keyword is an identifier wherever it has no
      // action. Both readings hold one typed field, so they tied, and
      // a comment between the two members made the split win.
      prec.dynamic(-1, seq(
        repeat($.annotation),
        repeat1($._modifier),
        $.variable_declarator,
        repeat(seq(',', $.variable_declarator)),
      )),
    ),

    // §4 — `def name(params) body`. Typed return type (`String foo()
    // { … }`) lands once we can disambiguate from local variable
    // declaration. Modifiers and throws clause similarly defer.
    // prec(1) wins over local_variable_declaration when `(` follows
    // the name, so `def f() { … }` at script-top-level parses as a
    // method rather than a `def f` variable followed by `(…) { … }`.
    // Modifiers alone (no `def`, no return type) also introduce a
    // method outside a class body: `final f() { }`. In a class body
    // that shape stays a constructor_declaration.
    //
    // A method with a body carries a dynamic precedence because its
    // tokens also parse as statements: `String f() { }` as the
    // expression `String` followed by the call `f() { }`, and
    // `def f(a) { }` as a variable `f` followed by the cast
    // `(a) { }`. Without a weight, a comment before the method made
    // the split win. The weight is above the blank final local's
    // (`final Type name`, see local_variable_declaration):
    // `final String f(a) { }` is also that local followed by the cast
    // `(a) { }`.
    //
    // A method without a body carries a negative one. Its tokens are
    // also `x` followed by the call `foo()`, and outside a class body
    // only that reading is valid Groovy: a script method needs a
    // body. In a class body nothing competes, so abstract and
    // interface methods are unaffected.
    method_declaration: $ => {
      const signature = seq(
        repeat($.annotation),
        choice(
          seq(
            repeat($._modifier),
            optional(field('type_parameters', $.method_type_parameters)),
            choice('def', field('return_type', $._type)),
          ),
          repeat1($._modifier),
        ),
        field('name', $._property_name),
        field('parameters', $.formal_parameters),
        optional($.throws_clause),
      );
      return prec.right(2, choice(
        prec.dynamic(-1, signature),
        prec.dynamic(3, seq(signature, field('body', $.block))),
      ));
    },

    // §4 / §5.14 — method-level generic type parameters
    // (`<T> T identity(T x) { x }`). Distinct from `type_parameters`
    // (used on class/trait/interface) because here the `<` cannot be
    // `token.immediate` — it appears at the start of the declaration
    // with optional whitespace from preceding modifiers / annotations.
    method_type_parameters: $ => seq(
      '<',
      $.type_parameter,
      repeat(seq(',', $.type_parameter)),
      '>',
    ),

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

    // §5.13 — formal parameter with optional annotations (Spring's
    // `@PathVariable`, `@RequestBody`, JPA's `@Param`, … — ubiquitous
    // on framework-facing methods), optional type, optional varargs
    // marker (`Type... name`), and optional default value
    // (`name = expr`).
    // `final` is the only modifier accepted here.
    formal_parameter: $ => seq(
      repeat($.annotation),
      optional('final'),
      optional(field('type', choice($._type, $.varargs_type))),
      field('name', $.identifier),
      optional(seq('=', field('default', $._expression))),
    ),

    varargs_type: $ => seq($._type, '...'),

    // §4 — enum. Closes murtaza64 #36 first half. Constants may
    // carry constructor arguments (`RED(0xff0000)`) via the optional
    // argument_list.
    //
    // The body follows GroovyParser.g4 `classBody` for `t == 2`:
    // after the constants either a trailing `,` closes the list, or
    // members follow after `;`, `,;`, or nothing at all — Groovy's
    // own parser treats the newline as the separator (GROOVY-7773,
    // GROOVY-9306), and newlines are extras here, so the member
    // simply follows the last constant. With no constants the body
    // is an ordinary member list.
    //
    // Groovy 2.x also accepts a member directly after a trailing `,`
    // (`A, B,\n private int x`); Groovy 3+ rejects it. Without this
    // alternative the modifier had no action after the `,`, lexed as
    // an identifier, and became a constant named `private` with no
    // ERROR. The first member here is any `_class_member`, but a
    // constructor needs at least one modifier. `B(x) { … }` after a
    // `,`, with or without annotations, is a constant with an
    // anonymous class body in every Groovy (unsupported, so an
    // ERROR), not a constructor. With a modifier it cannot be a
    // constant, and with no constructor form here `private B(x) { }`
    // parsed, with no ERROR, as a method named `B`: the modifier-only
    // form of `method_declaration` took it. `prec(1)` on
    // `enum_constant` keeps a bare identifier after the `,` a
    // constant, so `A,\n String x`, which every Groovy rejects, stays
    // an ERROR.
    //
    // An instance initializer (`block`) is accepted wherever the
    // member list repeats — after `;`, after a first member, or with
    // no constants — but not directly after a constant. Newlines are
    // extras, so `A\n{ … }` (in the .g4 a constant, then an
    // initializer) cannot be told from `A { … }` (a constant with an
    // anonymous class body, unsupported); both stay errors.
    //
    // The four member repeats are spelled out, and must stay
    // identical to the one in `class_body`, so that the generator
    // folds all five into one aux symbol (`aux_sym_class_body_repeat1`).
    // A differing copy, or a hidden rule that is not inlined, adds a
    // symbol and renumbers every id after it — the alias kinds
    // `named_argument`, `quoted_identifier` and `type_identifier`
    // included — which a patch release must not do.
    enum_declaration: $ => seq(
      repeat($.annotation),
      repeat($._modifier),
      'enum',
      field('name', $.identifier),
      optional($.super_interfaces),
      field('body', $.enum_body),
    ),

    enum_body: $ => seq(
      '{',
      choice(
        seq(
          $.enum_constant,
          repeat(seq(',', $.enum_constant)),
          optional(choice(
            ',',
            seq(optional(','), ';', repeat(choice($._class_member, $.block, ';'))),
            seq($._class_member, repeat(choice($._class_member, $.block, ';'))),
            seq(
              ',',
              choice(...classMembers(
                $,
                alias($._modified_constructor_declaration, $.constructor_declaration),
              )),
              repeat(choice($._class_member, $.block, ';')),
            ),
          )),
        ),
        repeat(choice($._class_member, $.block, ';')),
      ),
      '}',
    ),

    // §4 — `prec(1)` makes a bare identifier at the start of the body
    // a constant rather than the type of the first member: `A\n E() { }`
    // is a constant followed by a constructor, not a method `E`
    // returning `A`. The mirror case with no constants, `int f() { }`
    // as the first thing in the body, therefore reads as constant `int`
    // plus constructor `f` — a newline is whitespace here and nothing
    // else tells the two apart; a modifier, `def`, or a leading `;`
    // makes it a member. Static precedence rather than `prec.dynamic`
    // with an `[enum_constant, _type]` conflict: both yield the same
    // tree on every valid shape, but the GLR form silently accepts
    // `A\n B` (constants missing their comma, which Groovy rejects) as
    // a field `A B`, where this form reports the ERROR. Annotations
    // share the prefix with an annotated member; the same precedence
    // decides after them, so `@Foo A\n int x` is a constant and a
    // field.
    enum_constant: $ => prec(1, seq(
      repeat($.annotation),
      field('name', $.identifier),
      optional(field('arguments', $.argument_list)),
    )),

    // §4 — record (Groovy 4+), after Apache's `classDeclaration`:
    // `[modifiers] record Name[<T>](Type a, Type b) [implements I] [{ body }]`.
    // Records cannot `extends`, and the body is optional here where
    // Groovy requires it.
    // §5.2 / §6.7 — `record` is a contextual keyword. The keyword
    // comes from the external scanner, which emits it only when a
    // name follows on the same line and `(` or `<` after that; in
    // every other position the word lexes as `identifier`. A grammar literal
    // cannot do this: keyword extraction yields the keyword wherever
    // it has an action, which made `record.key()` at statement start,
    // and `private Record record` once record_declaration became a
    // class member, errors. The alias keeps the anonymous `record`
    // node that the queries capture.
    //
    // The dynamic precedence settles a GLR tie the scanner creates.
    // After `private Record`, the `[variable_declarator, _type]`
    // conflict forks on the next word: `record` is the field's name
    // in one branch and a type in the other. When a record header
    // follows on the next line, the first branch lexes its `record`
    // as the keyword and the second as that field's name, and both
    // survive (`private Record` / `record record` / constructor
    // `Inner(int x) { }` is a valid member list). Without a weight,
    // which tree wins depends on tree shape, and a comment between
    // the two lines flipped it. Favouring the record declaration is
    // right whenever both parse.
    record_declaration: $ => prec.dynamic(1, prec.right(seq(
      repeat($.annotation),
      repeat($._modifier),
      alias($._record_keyword, 'record'),
      field('name', $.identifier),
      optional(field('type_parameters', $.type_parameters)),
      field('components', $.record_components),
      optional($.super_interfaces),
      optional(field('body', $.class_body)),
    ))),

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
    // `final` is the only modifier accepted here.
    // `final x = 1` with no type or `def` is the modifier-only form;
    // the variable is dynamically typed, as with `def`. `final` also
    // allows the typed form without an initializer (`final String
    // name`, a blank final), with a single declarator.
    //
    // The typed forms carry a dynamic precedence because their tokens
    // also parse another way: `String s = 'a'` as the expression
    // `String` followed by `s = 'a'`, and `final String s = 'a'` as
    // the modifier-only `final String` followed by `s = 'a'`. Without
    // a weight, which tree wins depends on tree shape, and a comment
    // before the declaration flipped it. The blank form ranks above
    // the initialized one, or `final String name` followed by
    // `name = 'a'` would tie with `final String` followed by
    // `name name = 'a'`.
    //
    // Newlines do not end a declaration, so these read wrongly with
    // no ERROR node:
    // - `final x` followed by a line that starts with an identifier
    //   reads `x` as the type and that identifier as the name. `y = 2`
    //   becomes `final x y = 2`, the same resolution modifier-only
    //   fields get (`static x` then `y = 1`), and `String s = 'a'`
    //   becomes `final x String` followed by `s = 'a'`.
    // - `final String name` followed by a line reading `foo() { }` is
    //   `final String` followed by the method `name foo() { }`. A
    //   method with a body outranks the blank form, so that
    //   `final String f(a) { }` stays a method (see method_declaration).
    // - A bare `x` followed by a line reading `y = 1` is the typed
    //   declaration `x y = 1`.
    local_variable_declaration: $ => prec.right(choice(
      seq(
        optional('final'),
        choice('def', 'var'),
        $.variable_declarator,
        repeat(seq(',', $.variable_declarator)),
      ),
      prec.dynamic(1, seq(
        optional('final'),
        field('type', $._type),
        $._initialized_declarator,
        repeat(seq(',', $._initialized_declarator)),
      )),
      seq(
        'final',
        $.variable_declarator,
        repeat(seq(',', $.variable_declarator)),
      ),
      prec.dynamic(2, seq(
        'final',
        field('type', $._type),
        field('name', $.identifier),
      )),
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

    // §4 — `for (x in xs)`, plus the Java-compatible enhanced-for
    // form Groovy accepts side by side: `for (Type x : xs)` / `for
    // (def x : xs)`. The `in` / `:` keyword here is a structural
    // delimiter, not the membership_expression operator or a
    // labelled_statement colon — context disambiguates because the
    // for-header is not an expression position.
    for_in_statement: $ => seq(
      'for',
      '(',
      optional('final'),
      optional(choice('def', field('type', $._type))),
      field('variable', $.identifier),
      choice('in', ':'),
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
      repeat(choice($._statement, ';')),
    ),

    // The arrow body may be terminated by `;` (`case 1 -> foo();`),
    // matching the Java/Groovy `case L -> expr;` arrow form. The `;`
    // is anonymous and absorbed here rather than by the switch block,
    // which has no statement-list of its own.
    switch_arrow_case: $ => seq(
      'case',
      field('value', $._expression),
      '->',
      field('body', choice($.block, $._statement)),
      optional(';'),
    ),

    switch_default: $ => seq(
      'default',
      choice(
        seq(':', repeat(choice($._statement, ';'))),
        seq('->', field('body', choice($.block, $._statement)), optional(';')),
      ),
    ),

    // §3.2 — `x as Type`. RELATIONAL level (10), left-associative.
    // RHS is `_cast_type`, a `_type`-shaped sibling that swaps in
    // `_dotted_type` for `qualified_type` so the trailing
    // `.identifier` of `x as java.util.Map` extends the type
    // through every dot segment (instead of being claimed by
    // `field_access` at higher precedence than `cast_expression`).
    cast_expression: $ => prec.left(PREC.RELATIONAL, seq(
      field('value', $._expression),
      'as',
      field('type', $._cast_type),
    )),

    // §3.2.2 — RHS type of `as`, `instanceof`, and `!instanceof`.
    // Same alternatives as `_type` except `qualified_type` is
    // replaced by `_dotted_type`, whose tail shifts each dot at a
    // precedence tighter than `field_access`.
    _cast_type: $ => choice(
      alias($.identifier, $.type_identifier),
      $.array_type,
      alias($._dotted_type, $.qualified_type),
      $.generic_type,
    ),

    _dotted_type: $ => seq(
      alias($.identifier, $.type_identifier),
      $._dotted_type_tail,
    ),

    _dotted_type_tail: $ => prec.right(PREC.ACCESS + 1, seq(
      '.',
      alias($.identifier, $.type_identifier),
      optional($._dotted_type_tail),
    )),

    // §3.2 — `x in xs` and `x !in xs`. RELATIONAL level. Both
    // operators share one node kind; the `operator` field
    // distinguishes them. `!in` is wrapped in `token(prec(1, …))`
    // so it lexes as a single token despite the embedded space
    // requirement implied by §3.2.2's trailing-context rule —
    // Apache's `NOT_IN` predicate is approximated by requiring at
    // least one trailing whitespace, `[`, `(`, or `{` after `!in`.
    // §3.2 / §3.2.2 — `x in xs` and `x !in xs`. `!in` is a single
    // token whose match requires a trailing whitespace / bracket
    // character (Apache `NOT_IN` predicate) so `a !inXs` does not
    // mis-tokenise as `!in` followed by `Xs`.
    membership_expression: $ => {
      const ops = [
        'in',
        token(prec(1, seq('!in', /[ \t\r\n\[({]/))),
      ];
      return choice(...ops.map(op => prec.left(PREC.RELATIONAL, seq(
        field('element', $._expression),
        field('operator', op),
        field('collection', $._expression),
      ))));
    },

    // §3.2 / §3.2.2 — `x instanceof T` and `x !instanceof T`.
    // `!instanceof` is a single token whose match requires a
    // trailing whitespace (Apache `NOT_INSTANCEOF` predicate) so
    // `a !instanceofX` does not mis-tokenise as `!instanceof`
    // followed by identifier `X`.
    instanceof_expression: $ => {
      const ops = [
        'instanceof',
        token(prec(1, seq('!instanceof', /[ \t\r\n]/))),
      ];
      return choice(...ops.map(op => prec.left(PREC.RELATIONAL, seq(
        field('value', $._expression),
        field('operator', op),
        field('type', $._cast_type),
      ))));
    },

    // §3.2 — `new Foo(args)` and the anonymous inner class
    // `new Foo(args) { members }`. Array creation `new Foo[10]` and
    // array initialisers land later. PREC.PRIMARY only labels the
    // §3.1 tier. The number decides nothing: the rule's one conflict
    // (below) is against itself, where it cancels out, and
    // `new Foo().bar` composes without any precedence.
    //
    // The optional body takes every `{` that follows the arguments.
    // Right associativity does that statically — the shift into
    // `class_body` beats reducing the bodiless form — so no conflict
    // is declared and the closure reading never forks.
    //
    // Apache is narrower. Its `creator` has no `nls` before the body,
    // so `new T()\n{ … }` is a closure called on the new object
    // (observed on Groovy 4.0.33). By the same grammar a same-line
    // `{` whose content cannot be class members should be one too
    // (`new T() { x -> x }`; not run through the compiler). Here both
    // are read as a body, usually with an ERROR in it (divergence 11).
    // Parsing them needs a declared conflict on this rule with
    // `prec.dynamic` on the body, and that form was not taken: a body
    // holding one member `class_body` cannot parse, or one still
    // being typed, would then fall back silently to the closure tree
    // that #68 reported.
    object_creation_expression: $ => prec.right(PREC.PRIMARY, seq(
      'new',
      field('type', choice($._type, alias($._diamond_type, $.generic_type))),
      field('arguments', $.argument_list),
      optional(field('body', $.class_body)),
    )),

    // §3.2 — Apache's createdName permits diamond, but ordinary types
    // (including nested type arguments) do not. Keep the public shape.
    _diamond_type: $ => prec.right(PREC.ARRAY_TYPE, seq(
      field('base', choice(
        alias($.identifier, $.type_identifier),
        $.qualified_type,
      )),
      field('arguments', alias($._diamond_type_arguments, $.type_arguments)),
    )),

    _diamond_type_arguments: _ => seq(token.immediate('<'), '>'),

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
    // qualified types (`java.util.List`), array types
    // (`int[]`, `String[][]`), and generic types (`List<String>`,
    // `Map<String, Integer>`). `generic_type` always carries explicit
    // angle brackets, so it never competes with comparison operators
    // in expression context (which only reach `_type` through a
    // separate parse stack).
    _type: $ => choice(
      alias($.identifier, $.type_identifier),
      $.array_type,
      $.qualified_type,
      $.generic_type,
    ),

    // Near-twin of `_dotted_type` below; the two cannot be merged
    // without breaking either field-access (which needs
    // `qualified_type` to NOT outrank `field_access`) or
    // cast / instanceof RHS (which needs `_dotted_type` to outrank
    // `field_access`). Edits here should be mirrored in
    // `_dotted_type` / `_dotted_type_tail`.
    qualified_type: $ => prec.left(seq(
      alias($.identifier, $.type_identifier),
      repeat1(seq('.', alias($.identifier, $.type_identifier))),
    )),

    array_type: $ => prec(PREC.ARRAY_TYPE, seq($._type, alias($._subscript_open, '['), ']')),

    // §4 / §5.14 — Java-style generic type arguments. Allows nested
    // generics (`Map<String, List<Integer>>`) and bounded wildcards
    // (`? extends Foo`, `? super Bar`). The `<` is `token.immediate`
    // so it only matches when the angle bracket directly follows the
    // base name with no intervening whitespace — `a < b` still parses
    // as a binary expression because of the space before `<`.
    generic_type: $ => prec.right(PREC.ARRAY_TYPE, seq(
      field('base', choice(
        alias($.identifier, $.type_identifier),
        $.qualified_type,
      )),
      field('arguments', $.type_arguments),
    )),

    type_arguments: $ => seq(
      token.immediate('<'),
      $._type_argument,
      repeat(seq(',', $._type_argument)),
      '>',
    ),

    _type_argument: $ => choice(
      $._type,
      $.wildcard,
    ),

    wildcard: $ => seq(
      '?',
      optional(seq(
        choice('extends', 'super'),
        $._type,
      )),
    ),

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
      reserved('property', $.identifier),
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
    //
    // Trailing closure (§5.5's command_chain covers the bare-
    // identifier statement form; this covers the general expression
    // form used constantly on property-access chains: `list.find {
    // it > 5 }`, `list.inject(0) { acc, x -> acc + x }`). Groovy
    // treats a closure literal as the call's last argument whether
    // or not parens are present, so `argument_list` becomes optional
    // once a trailing closure is there to anchor the call.
    //
    // The args-and-closure alternative sits one precedence level
    // above plain POSTFIX so `f(x) { … }` reduces to a single
    // method_invocation node (arguments AND closure fields on the
    // same node) instead of GLR instead reaching the same span by
    // nesting a closure-only method_invocation around a plain call.
    method_invocation: $ => choice(
      prec.left(PREC.POSTFIX + 1, seq(
        field('function', $._expression),
        field('arguments', $.argument_list),
        field('closure', $.closure),
      )),
      prec.left(PREC.POSTFIX, seq(
        field('function', $._expression),
        field('arguments', $.argument_list),
      )),
      prec.left(PREC.POSTFIX, seq(
        field('function', $._expression),
        field('closure', $.closure),
      )),
    ),

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
    // `[` must be on the object's line (see `_subscript_open`): a Grails
    // controller action that ends in `[model: value]` on its own line returns
    // a map; it isn't a subscript of the statement before it.
    subscript_expression: $ => prec.left(PREC.POSTFIX, seq(
      field('object', $._expression),
      alias($._subscript_open, '['),
      field('index', $._expression),
      ']',
    )),

    // `?[` is one token. `a ? [b] : c` (with space) still parses
    // as ternary plus list because tree-sitter's lexer never
    // crosses a whitespace boundary to extend a multi-char token.
    // Unlike `[` (see `_subscript_open`), `?[` may still follow a
    // newline: no statement starts with `?[`, so there is nothing
    // to split off.
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
    // form; `{ a, b -> body }` has identifier parameters;
    // `{ String a, int b -> body }` has typed parameters; `{ body }`
    // has no `->` and an implicit `it` parameter (we do NOT emit a
    // synthetic `it` per §5.8). A parameter takes what a method
    // parameter does (Apache's closure uses `formalParameterList`):
    // annotations, `final`, `def` or a type (varargs included), and a
    // default. The body may open with the same words, so the `->`
    // decides between the two readings.
    closure: $ => seq(
      '{',
      optional($.closure_parameters),
      repeat(choice($._statement, ';')),
      '}',
    ),

    closure_parameters: $ => seq(
      optional(seq(
        $.closure_parameter,
        repeat(seq(',', $.closure_parameter)),
      )),
      '->',
    ),

    closure_parameter: $ => seq(
      repeat($.annotation),
      optional('final'),
      optional(choice('def', field('type', choice($._type, $.varargs_type)))),
      field('name', $.identifier),
      optional(seq('=', field('default', $._expression))),
    ),

    // §3.2 PRIMARY / §5.13 — list literal with trailing-comma support.
    // Empty list is `[]`; the empty map literal is the distinct `[:]`
    // form (see map_literal). Tree-sitter splits the parse between
    // list_literal and map_literal until a `:` or `]` disambiguates;
    // the conflict declaration above keeps both alternatives alive.
    // Either `[` token opens a literal, so a same-line `[` resolves as
    // it did with one token: `if (a) { } [1].each { }` is still a block
    // followed by a list, not a subscripted closure.
    list_literal: $ => seq(
      choice('[', alias($._subscript_open, '[')),
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
        choice('[', alias($._subscript_open, '[')),
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
    // interpolate. Double-quoted and triple-double interpolate
    // `$identifier(.identifier)*` and `${expression}` as structured
    // children, so consumers can walk into the GString's expression
    // segments. Slashy bodies and dollar-slashy bodies remain flat
    // tokens (their internal `$` is rare in practice and the
    // scanner cannot interrupt a slashy mid-body without per-flavor
    // state).
    string_literal: $ => choice(
      $._single_quoted_string,
      $._triple_single_quoted_string,
      $._double_quoted_string,
      $._triple_double_quoted_string,
      $._dollar_slashy_string,
      $._slashy_string,
    ),

    // §5.7 — slashy regex `/.../`. The opening `/` is emitted by
    // the external scanner (after the context guard from §5.4 / §6.2
    // and a lookahead that confirms a closing `/` exists on the same
    // line). The body is parsed as `gstringPart` so `$identifier` and
    // `${expr}` segments expose the same structured children as
    // double-quoted GStrings. The closing `/` is `token.immediate`
    // so it cannot skip whitespace between the body and the closer.
    _slashy_string: $ => seq(
      $._slashy_string_start,
      repeat(gstringPart($, $._slashy_text)),
      token.immediate('/'),
    ),

    _slashy_text: _ => token.immediate(repeat1(choice(
      /[^\/\\$\n\r]/,
      /\\[\s\S]/,
    ))),

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

    // §5.7 / §6.3 — double-quoted GString. The opening `"` is
    // tokenised separately from the body so that `$identifier` and
    // `${expr}` segments parse as proper child nodes. The text
    // fragment stops before any `$`, so `$identifier` and `${expr}`
    // get a chance to match; literal `$` (followed by `"` or a
    // digit etc.) is recovered as `$` plus the fallback shape.
    _double_quoted_string: $ => seq(
      '"',
      repeat(gstringPart($, $._gstring_text)),
      token.immediate('"'),
    ),

    _gstring_text: _ => token.immediate(repeat1(choice(
      /[^"\\$\n\r]/,
      /\\[\s\S]/,
    ))),

    // Bare `$` inside a GString body that does not start a valid
    // interpolation (`$"`, `$5`, `$$`, `$ `) is matched as a one-
    // char literal fragment. This rule must have lower precedence
    // than `_gstring_dollar_path` so `$name` still wins.
    _gstring_literal_dollar: _ => token.immediate(prec(-1, /\$/)),

    // §5.7 — triple-double GString. Like `_double_quoted_string`
    // but newlines and embedded `"` / `""` are part of the body.
    _triple_double_quoted_string: $ => seq(
      '"""',
      repeat(gstringPart($, $._triple_gstring_text)),
      token.immediate('"""'),
    ),

    _triple_gstring_text: _ => token.immediate(repeat1(choice(
      /[^"\\$]/,
      /\\[\s\S]/,
      /"[^"]/,
      /""[^"]/,
    ))),

    // §5.7 — `$identifier(.identifier)*` interpolation. The whole
    // `$identifier...` shape is matched as a single immediate token
    // so the `$` only consumes when followed by an identifier start;
    // `_gstring_literal_dollar` handles the lone-`$` fallback above.
    gstring_dollar_interpolation: $ => seq(
      field('value', alias(
        $._gstring_dollar_path,
        $.identifier,
      )),
    ),

    _gstring_dollar_path: _ => token.immediate(
      /\$[\p{XID_Start}_][\p{XID_Continue}]*(\.[\p{XID_Start}_][\p{XID_Continue}]*)*/,
    ),

    // §5.7 / §6.3 — `${expression}` interpolation.
    gstring_brace_interpolation: $ => seq(
      token.immediate('${'),
      field('value', $._expression),
      '}',
    ),

    // §5.7 — `$/...$/` dollar-slashy strings. The body is parsed
    // through `gstringPart` so `$identifier` and `${expr}` segments
    // expose structured children. `_dollar_slashy_text` consumes
    // every literal-text alternative: `/X` where X is not `$`,
    // `$$` (literal `$`), `$/` (literal `/`), and any other
    // non-`/` non-`$` char. The closer `/$` is `token.immediate`
    // so it always wins over text when `/` is followed by `$`.
    //
    // A run of slashes is text only up to an ordinary character
    // (`a//b`). The text token must not take a `/` that the closer
    // needs, and a regex cannot look ahead, so the slashes of a run
    // that ends at `$` are lexed one at a time: for `//$` the single
    // `/` matches first, then `/$` is the longer match and closes
    // the string. Each such slash is its own `string_fragment`.
    _dollar_slashy_string: $ => seq(
      '$/',
      repeat(choice(
        gstringPart($, $._dollar_slashy_text),
        alias(token.immediate('/'), $.string_fragment),
      )),
      token.immediate('/$'),
    ),

    _dollar_slashy_text: _ => token.immediate(repeat1(choice(
      /[^/$]/,
      /\/+[^/$]/,
      /\$\$/,
      /\$\//,
    ))),

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

    // Groovy's lexer takes any Java letter (`isJavaIdentifierStart` /
    // `isJavaIdentifierPart`), not only ASCII; `XID_Start` and
    // `XID_Continue` are the Unicode classes closest to those.
    identifier: _ => /[\p{XID_Start}_$][\p{XID_Continue}$]*/,
  },
});
