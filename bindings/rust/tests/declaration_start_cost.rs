//! Cost test for the look-ahead that decides whether a statement is a typed
//! declaration without an initializer (`SPECIFICATION.md` §6.21).
//!
//! At a statement that starts with a type and a name, the scanner reads the
//! list of names after it, and after the `=` of an initializer the value up
//! to the `->` that would make the list closure parameters
//! (`{ String a, b = 1 -> }`). In a nest of closures that each open with
//! such a list or such a declaration, and on lines of a broken one, every
//! declaration reads the rest of the nest: the look-ahead must stop after a
//! fixed number of items, or the parse takes time quadratic in the nesting. Time is not measured:
//! the parser reads its input through a callback here, and the test
//! compares the bytes it asks for at two input sizes.

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
/// (measured: 41 and 43 bytes per byte for the nest of lists, 45 and 46
/// unclosed, 26 and 26 for the broken lines, 60 and 64 for the nest of
/// declarations); one that reads to the end of the nest grows with it,
/// eightfold here (253 and 2,003, 251 and 2,001, 251 and 2,001, and 253
/// and 2,003 without the bound).
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
fn nested_lists_with_defaults_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("{}{}\n", "T a, b = { ".repeat(n), "}".repeat(n)));
}

#[test]
fn unclosed_nested_lists_with_defaults_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("{}\n", "{ T a, b = ".repeat(n)));
}

/// The lines open closures: the look-ahead reads an initializer to the
/// `->` only where closure parameters may start, so outside a closure the
/// lines cost 2 bytes per byte with the bound or without it.
#[test]
fn broken_lists_on_many_lines_are_read_a_bounded_number_of_times() {
    assert_linear(|n| "x\n{ T a, b = (1,\n".repeat(n));
}

#[test]
fn nested_closure_valued_declarations_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("{}{}\n", "Closure c = { ".repeat(n), "}".repeat(n)));
}

/// A block comment between the type and the name is read by the look-ahead
/// as trivia, once per declaration.
#[test]
fn comments_between_type_and_name_are_read_a_bounded_number_of_times() {
    assert_linear(|n| "T /* c */ a\n".repeat(n));
    assert_linear(|n| format!("{}{}\n", "T /* c */ a, b = { ".repeat(n), "}".repeat(n)));
}

/// A comment that spans lines between the type and the name is read once
/// by the look-ahead, however long it is.
#[test]
fn a_long_comment_between_type_and_name_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("T /*{}*/ a\n", " c\n".repeat(n)));
}
