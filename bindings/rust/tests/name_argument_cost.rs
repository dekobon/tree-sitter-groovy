//! Cost test for the look-ahead after a parenthesis-free call's name
//! argument (`SPECIFICATION.md` §6.14).
//!
//! A list of `name name,` elements with no `->` after it is no closure
//! parameter list, and on such malformed input error recovery restarts a
//! statement at every element or two. Each restart reads ahead from where
//! it starts, so the look-ahead must stop after a fixed number of items,
//! or the parse takes time quadratic in the list. Time is not measured:
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
/// (measured: 33 and 35 bytes per byte, 42 and 46 on one line); one that
/// reads to the end of the list grows with it, eightfold here (129 and
/// 1,022 before the bound).
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

/// `n` elements `bar aN`, joined by `separator`.
fn elements(n: usize, separator: &str) -> String {
    (0..n)
        .map(|i| format!("bar a{i}"))
        .collect::<Vec<_>>()
        .join(separator)
}

#[test]
fn lowercase_typed_lists_in_a_closure_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("{{ {}\n}}\n", elements(n, ",\n")));
}

#[test]
fn lowercase_typed_lists_on_one_line_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("{{ {} }}\n", elements(n, ", ")));
}

#[test]
fn lowercase_typed_lists_after_a_call_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("foo x, {}\n", elements(n, ",\n")));
}

/// A triple-quoted string is one item of the look-ahead and may span lines,
/// so one that does not close is read to the end of input by every
/// look-ahead that reaches it; the budget keeps those a fixed number.
#[test]
fn lists_before_an_unterminated_triple_quoted_string_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("{{ {},\n'''\n{}", elements(n, ",\n"), "x\n".repeat(n)));
}

/// Typed elements among lowercase-typed ones (`{ bar a, int b, … }` with no
/// `->`) are read within the budget as well (measured: 40 and 41 bytes per
/// byte; 259 and 2,045 without it). After a name argument, a run of typed
/// parameters (`foo bar(int a, int b, …`) ends the look-ahead at its first.
#[test]
fn typed_parameter_runs_are_read_a_bounded_number_of_times() {
    assert_linear(|n| {
        let elements = (0..n)
            .map(|i| format!("bar a{i}, int b{i}"))
            .collect::<Vec<_>>()
            .join(",\n");
        format!("{{ {elements},\n}}\n")
    });
    assert_linear(|n| {
        let params = (0..n)
            .map(|i| format!("int a{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("foo bar({params}\n")
    });
}
