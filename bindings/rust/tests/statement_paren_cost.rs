//! Cost test for the look-ahead that decides whether a `(` opening a line
//! after an expression opens the next statement (`SPECIFICATION.md` §6.18).
//!
//! At such a `(`, the scanner reads what the parentheses hold to see
//! whether they can open a statement. A closure inside them holds
//! statements, so a nest of `x` and `({ x` lines puts another such `(` on
//! every line, and each one reads the rest of the nest: the look-ahead must
//! stop after a fixed number of items, or the parse takes time quadratic
//! in the nesting. Time is not measured: the parser reads its input
//! through a callback here, and the test compares the bytes it asks for at
//! two input sizes.

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
/// (measured: 54 and 57 bytes per byte for the nest of closures); one that
/// reads to the end of the nest grows with it, almost eightfold here (204
/// and 1,565 without the bound).
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
fn nested_closures_in_parentheses_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("{}{}\n", "x\n({ x\n".repeat(n), "})".repeat(n)));
}

#[test]
fn unclosed_lists_on_many_lines_are_read_a_bounded_number_of_times() {
    assert_linear(|n| "x\n(a, b,\n".repeat(n));
}

#[test]
fn long_name_lists_are_read_a_bounded_number_of_times() {
    assert_linear(|n| {
        let names = (0..n).map(|i| format!("a{i}")).collect::<Vec<_>>();
        format!("x\n({})\n", names.join(", "))
    });
}
