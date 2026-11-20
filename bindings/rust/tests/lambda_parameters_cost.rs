//! Cost test for the look-ahead that decides a lambda's start
//! (`SPECIFICATION.md` §6.17).
//!
//! At every `(` where an expression may start, the scanner reads what the
//! parentheses hold to see whether they are a lambda's parameters followed
//! by `->`. A parameter's default value may hold another `(` where an
//! expression starts, so `(a = (a = (a = …` is read again from each `(`:
//! the look-ahead must stop after a fixed number of items, or the parse
//! takes time quadratic in the nesting. Time is not measured: the parser
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
/// (measured: 70 and 74 bytes per byte for nested default values); one
/// that reads to the closing parenthesis grows with the nesting, eightfold
/// here (256 and 2,007 without the bound).
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
fn nested_default_values_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("x = {}1{}\n", "(a = ".repeat(n), ")".repeat(n)));
}

#[test]
fn nested_lambdas_with_default_values_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("x = {}1{}\n", "(a = ".repeat(n), ") -> 1".repeat(n)));
}

/// Type arguments in a default value are read to their `>`, and a `<` that
/// turns out to be an operator (`X<b, c = …`) is read on as the default's.
#[test]
fn nested_defaults_with_type_arguments_are_read_a_bounded_number_of_times() {
    assert_linear(|n| {
        format!(
            "x = {}1{}\n",
            "(a = new M<A, B>(".repeat(n),
            ")) -> a".repeat(n)
        )
    });
    assert_linear(|n| {
        format!(
            "x = {}1{}\n",
            "(a = X<b, c = ".repeat(n),
            ") -> a".repeat(n)
        )
    });
}

/// The `${…}` of a double-quoted string is read as code, with the strings
/// in it read whole, so a nest of strings in interpolations is read again
/// from each `(` before one of its quotes. The look-ahead follows a bounded
/// number of strings into one another (`STRING_NESTING_DEPTH` in the
/// scanner), or a nest is read to its end from each of its levels, closed
/// or not.
#[test]
fn nested_interpolated_strings_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("x = {}1\n", "(a = \"${".repeat(n)));
    assert_linear(|n| format!("x = {}1{}\n", "(a = \"${".repeat(n), "}\") -> a".repeat(n)));
}

#[test]
fn nested_parentheses_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("x = {}a{}\n", "(".repeat(n), ")".repeat(n)));
}

#[test]
fn long_name_lists_are_read_a_bounded_number_of_times() {
    assert_linear(|n| {
        let names = (0..n).map(|i| format!("a{i}")).collect::<Vec<_>>();
        format!("x = ({})\n", names.join(", "))
    });
}

/// A triple-quoted string is one item of the look-ahead and may span lines,
/// so one that does not close is read to the end of input by every
/// look-ahead that reaches it; the budget keeps those a fixed number.
#[test]
fn an_unterminated_triple_quoted_default_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("x = {}'''\n{}", "(a = ".repeat(n), "x\n".repeat(n)));
}

#[test]
fn lambdas_on_one_line_are_read_a_bounded_number_of_times() {
    assert_linear(|n| {
        let lambdas = (0..n)
            .map(|i| format!("(a{i}) -> a{i}, b{i} -> b{i}"))
            .collect::<Vec<_>>();
        format!("foo({})\n", lambdas.join(", "))
    });
}
