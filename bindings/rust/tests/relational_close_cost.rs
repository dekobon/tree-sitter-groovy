//! Cost test for the look-ahead that decides what a `)` closes
//! (`SPECIFICATION.md` §6.11).
//!
//! At every `)` that may close a cast's type, the scanner reads on along
//! the line to tell a cast's operand from a relational operator that Groovy
//! applies to the parentheses (`(x) instanceof Foo`, `(x) in y`). After
//! `as` and `in` it reads the spaces and block comments on the line and the
//! first character of what follows, and no further: a line of `(x) in y && …` puts another such
//! `)` before each element, and a look-ahead that read to the end of the
//! line would read the rest of it from each one, which takes time
//! quadratic in the length of the line. Time is not measured: the parser
//! reads its input through a callback here, and the test compares the bytes
//! it asks for at two input sizes.

use tree_sitter::Parser;

/// Elements in the smaller input; the larger one has eight times as many.
const SMALL_ELEMENTS: usize = 500;

/// How many times larger the second input is.
const SCALE: usize = 8;

/// Bytes handed to the parser per call of the read callback. Small
/// enough that a look-ahead past a few elements needs another call.
const CHUNK_BYTES: usize = 64;

/// Most the bytes served per input byte may grow from the smaller input
/// to the larger. A bounded look-ahead keeps the two about equal
/// (measured: 2.1 and 2.1 bytes per byte for a line of `in` tests); one
/// that reads to the end of the line grows with it, eightfold here (251
/// and 2,001 without the bound).
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
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    #[allow(clippy::cast_precision_loss, reason = "sizes far below 2^52")]
    let ratio = served as f64 / bytes.len() as f64;
    ratio
}

/// Asserts that the bytes served per input byte do not grow with the
/// number of elements `make` puts in its input.
fn assert_linear(make: fn(usize) -> String) {
    let small = bytes_served_per_input_byte(&make(SMALL_ELEMENTS));
    let large = bytes_served_per_input_byte(&make(SMALL_ELEMENTS * SCALE));
    assert!(
        large <= small * MAX_GROWTH,
        "bytes served per input byte grew from {small:.1} to {large:.1}",
    );
}

#[test]
fn a_line_of_in_tests_after_parentheses_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("{}true\n", "(x) in y && ".repeat(n)));
}

#[test]
fn a_line_of_instanceof_tests_after_parentheses_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("{}true\n", "(x) instanceof T || (y) !in z || ".repeat(n)));
}

#[test]
fn a_line_of_coercions_of_parentheses_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("{}1\n", "(x) as T + (y) as  a.B + ".repeat(n)));
}

#[test]
fn a_line_of_tests_with_comments_around_the_operator_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("{}true\n", "(x) /* a */ in /* b */ y && ".repeat(n)));
}
