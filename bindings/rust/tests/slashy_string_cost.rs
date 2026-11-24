//! Cost test for the look-ahead that decides a slashy string's start
//! (`SPECIFICATION.md` §6.2).
//!
//! At a `/` where a slashy string may start, which during error recovery is
//! every `/`, the scanner looks for the string's closing `/` on the line. An
//! escaped slash ends no look-ahead, so a line of `\/` was read from each of
//! its slashes to its end, in time quadratic in its length: 17 seconds for
//! 100,000 bytes. The look-ahead now stops at a fixed number of escaped
//! slashes. Time is not measured: the parser reads its input through a
//! callback here, and the test compares the bytes it asks for at two input
//! sizes.

use tree_sitter::Parser;

/// Escaped slashes in the smaller input; the larger one has eight times as
/// many.
const SMALL_ELEMENTS: usize = 500;

/// How many times larger the second input is.
const SCALE: usize = 8;

/// Bytes handed to the parser per call of the read callback. Small enough
/// that a look-ahead past a few elements needs another call.
const CHUNK_BYTES: usize = 64;

/// Most the bytes served per input byte may grow from the smaller input to
/// the larger. A bounded look-ahead keeps the two about equal (measured: 1.2
/// and 1.0 after `x = /`, 9.3 and 6.4 after `@`); one that reads to the end
/// of the line grows with it, eightfold here (255 and 2,005, 250 and 2,000
/// without the bound).
const MAX_GROWTH: f64 = 1.5;

/// Parses `source`, which has errors, and returns the bytes served to the
/// parser per byte of input.
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
fn a_slashy_string_of_escaped_slashes_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("x = /{}\n", "\\/".repeat(n)));
}

#[test]
fn escaped_slashes_in_error_recovery_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("@ {}\n", "/\\".repeat(n)));
}

// Where no division may follow, the look-ahead runs past line breaks to the
// closing `/` (a slashy string may span lines). It still stops at the first
// unescaped `/`, so each line's `/` below is read from the one before it
// only, and the escaped slashes stay bounded across lines.

#[test]
fn unclosed_slashy_strings_on_many_lines_are_read_a_bounded_number_of_times() {
    assert_linear(|n| "x = /a\n".repeat(n));
}

#[test]
fn escaped_slashes_across_lines_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("x = /{}\n", "\\/\n".repeat(n)));
}
