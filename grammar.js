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

/* eslint-disable no-multi-spaces, no-unused-vars --
 * The PREC table is referenced from SPECIFICATION.md and will be used
 * once expression rules land. The aligned trailing comments make the
 * Apache-mirror precedence ladder readable at a glance.
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
  UNARY: 14,                  // prefix ++ -- + - ! ~
  POWER: 15,                  // **
  POSTFIX: 16,                // postfix ++ --, []
  ACCESS: 17,                 // . ?. ??. *. .& .@ ::
  PRIMARY: 18,                // literals, identifiers, ()
};
/* eslint-enable no-multi-spaces, no-unused-vars */

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

  externals: $ => [
    $._automatic_semicolon,
    $._slashy_string_start,
    $._slashy_string_body,
    $._slashy_string_end,
    $._dollar_slashy_string,
    $._gstring_body,
    $._gstring_interpolation_start,
    $._label_colon,
    $.block_comment,
    $.groovydoc_comment,
  ],

  conflicts: $ => [
    // See SPECIFICATION.md §7. Will be populated as rules are added.
  ],

  rules: {
    // Top-level placeholder. Real rule set is forthcoming per
    // SPECIFICATION.md §4 (statement and declaration coverage).
    source_file: $ => repeat($._statement),

    // TODO(spec §4): replace this identifier-only stub with the real
    // statement / declaration choice once those rules land. Using
    // $.identifier (and not $.line_comment) avoids shadow-consumption
    // by `extras`, which would otherwise leave _statement unmatchable.
    _statement: $ => $.identifier,

    line_comment: _ => token(seq('//', /[^\n]*/)),

    identifier: _ => /[A-Za-z_$][A-Za-z0-9_$]*/,
  },
});
