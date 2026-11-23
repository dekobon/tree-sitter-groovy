//! Cost test for dollar-slashy strings in the scanner's look-aheads
//! (`SPECIFICATION.md` §5.7, §6.6).
//!
//! `$` starts a name to the scanner, so a look-ahead that reads the `$` of
//! `$/…/$` must stop there or read the string as one item. One that reads
//! on takes the body of `$/*.txt/$` for a block comment, and no scan ever
//! reaches that `/*` to remember that it does not close (§6.6): each such
//! string was read to the next `*/` or to the end of input. Time is not
//! measured: the parser reads its input through a callback here, and the
//! test compares the bytes it asks for at two input sizes.

use tree_sitter::Parser;

/// Elements in the smaller input; the larger one has eight times as many.
const SMALL_ELEMENTS: usize = 500;

/// How many times larger the second input is.
const SCALE: usize = 8;

/// Bytes handed to the parser per call of the read callback. Small
/// enough that a look-ahead past a few elements needs another call.
const CHUNK_BYTES: usize = 64;

/// Most the bytes served per input byte may grow from the smaller input
/// to the larger. A look-ahead that stops at the string keeps the two
/// equal (measured: 1.5 and 1.5 bytes per byte for `println $/*.txt/$`);
/// one that reads to the end of input once per string grows with the
/// input, eightfold here (251.5 and 2,001.5 before the fix, and twice that
/// for `x = ($/*.txt/$)`, which two look-aheads read).
const MAX_GROWTH: f64 = 1.5;

/// Parses `source` and returns the bytes served to the parser per byte of
/// input.
fn bytes_served_per_input_byte(source: &str) -> f64 {
    let bytes = source.as_bytes();
    let mut parser = Parser::new();
    parser
        .set_language(&dekobon_tree_sitter_groovy::LANGUAGE.into())
        .expect("load Groovy grammar");

    let mut served = 0_usize;
    let tree = parser
        .parse_with_options(
            &mut |offset, _| {
                let end = bytes.len().min(offset + CHUNK_BYTES);
                let chunk = &bytes[offset.min(end)..end];
                served += chunk.len();
                chunk
            },
            None,
            None,
        )
        .expect("parse");
    assert_eq!(tree.root_node().end_byte(), bytes.len());
    #[allow(clippy::cast_precision_loss, reason = "sizes far below 2^52")]
    let ratio = served as f64 / bytes.len() as f64;
    ratio
}

/// Asserts that the bytes served per input byte do not grow with the
/// number of copies of `unit` in the input.
fn assert_linear_in_copies(unit: &str) {
    let small = bytes_served_per_input_byte(&unit.repeat(SMALL_ELEMENTS));
    let large = bytes_served_per_input_byte(&unit.repeat(SMALL_ELEMENTS * SCALE));
    assert!(
        large <= small * MAX_GROWTH,
        "{unit:?}: bytes served per input byte grew from {small:.1} to {large:.1}",
    );
}

/// Asserts the same for the input `make` builds from a number of elements.
fn assert_linear(make: fn(usize) -> String) {
    let small = bytes_served_per_input_byte(&make(SMALL_ELEMENTS));
    let large = bytes_served_per_input_byte(&make(SMALL_ELEMENTS * SCALE));
    assert!(
        large <= small * MAX_GROWTH,
        "bytes served per input byte grew from {small:.1} to {large:.1}",
    );
}

/// The first argument of a call and the value of a declaration, an
/// assignment, an argument and a map entry: the look-aheads of §6.14 and
/// §6.17, which decide at the `$`.
#[test]
fn glob_strings_where_a_call_or_a_lambda_may_start_are_read_once() {
    for unit in [
        "println $/*.txt/$\n",
        "a.b $/*.txt/$\n",
        "def x = $/*.txt/$\n",
        "x = a $/*.txt/$\n",
        "x = \"${foo $/*.txt/$}\"\n",
        "foo($/*.txt/$)\n",
        "foo(1, $/*.txt/$)\n",
        "x = [a: $/*.txt/$]\n",
        "if ($/*.txt/$) { }\n",
    ] {
        assert_linear_in_copies(unit);
    }
}

/// Inside parentheses: the look-aheads for a lambda's parameters (§6.17)
/// and for a statement's parentheses (§6.18), with the string as an
/// element, a default value and an annotation's argument.
#[test]
fn glob_strings_in_parentheses_are_read_once() {
    for unit in [
        "x = ($/*.txt/$)\n",
        "x = (a = $/*.txt/$)\n",
        "x = (a = $/src/*.txt/$)\n",
        "x = (@A($/*.txt/$) a) -> a\n",
        "return ($/*.txt/$)\n",
        "x = 1\n($/*.txt/$)\n",
        "x = 1\n(foo $/*.txt/$)\n",
        "{\n($/*.txt/$)\n}\n",
        "{\n(1 + $/src/*.txt/$)\n}\n",
    ] {
        assert_linear_in_copies(unit);
    }
}

/// After a name argument and in a declaration: the look-aheads for closure
/// parameters (§6.14) and for a typed declaration (§6.21).
#[test]
fn glob_strings_in_lists_and_declarations_are_read_once() {
    for unit in [
        "foo a, $/*.txt/$\n",
        "foo a, $/src/*.txt/$\n",
        "{ a = $/*.txt/$ -> a }\n",
        "{ String s = $/*.txt/$ }\n",
        "{ String s = $/src/*.txt/$ }\n",
        "String $/*.txt/$\n",
        "foo.Bar $/*.txt/$\n",
    ] {
        assert_linear_in_copies(unit);
    }
}

/// Where a keyword's construct or a type's arguments are looked for: the
/// headers of §6.7 and §6.13, the names of §6.16 and the type arguments of
/// §3.2.2 and §6.14.
#[test]
fn glob_strings_after_contextual_words_and_types_are_read_once() {
    for unit in [
        "record $/*.txt/$\n",
        "trait $/*.txt/$\n",
        "var ($/*.txt/$)\n",
        "y = x as a.B<$/*.txt/$\n",
        "foo bar(Map<$/*.txt/$)\n",
    ] {
        assert_linear_in_copies(unit);
    }
}

/// A dollar-slashy string that does not close is read by each look-ahead
/// that reaches it, within the items of its budget and the characters of
/// `DOLLAR_SLASHY_LOOKAHEAD_CHARACTERS` (`src/scanner.c`).
#[test]
fn an_unterminated_dollar_slashy_default_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("x = {}$/*\n{}", "(a = ".repeat(n), "x\n".repeat(n)));
}

/// A string's opener and closer differ, so in a nest of interpolations that
/// each open another string none closes, and without the bound in
/// characters each `(` would read to the end of input (measured: 89.3 and
/// 97.7 bytes per byte; 250.9 and 2,000.9 without the bound).
#[test]
fn nested_unterminated_dollar_slashy_strings_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("x = {}\n", "$/ ${ (a = ".repeat(n)));
    assert_linear(|n| format!("x = {}\n", "$/ ${ foo a, ".repeat(n)));
}
