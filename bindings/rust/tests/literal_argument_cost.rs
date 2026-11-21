//! Cost test for a literal first argument of a statement's call
//! (`SPECIFICATION.md` §6.14).
//!
//! Before a statement's one-name receiver the scanner reads a literal first
//! argument whole, and the trivia and the token after it, to decide whether
//! the argument goes on past the literal (`println "a" + b`). A string is
//! read with the code of its `${…}`, which may hold a closure whose
//! statements start the same way, so the look-ahead at each statement of a
//! nest like `foo "${ { foo "${ { …` reads the strings nested in its own.
//! Read whole, a nest that does not close would be read to the end of the
//! input from each of its statements, in time quadratic in its length, and
//! one that closes to the close of each string. `skip_string` follows at
//! most `STRING_NESTING_DEPTH` (8) strings, one inside another, so each
//! statement reads a bounded part of the nest.
//!
//! The test reads the input through a callback and compares the bytes the
//! parser asks for at two lengths of a nest, which a look-ahead that read to
//! the end of the nest from every statement would make grow with it.

/// Levels in the shorter nest; the longer one has eight times as many.
const SHORT_NEST: usize = 250;

/// How many times longer the second nest is.
const SCALE: usize = 8;

/// Bytes handed to the parser per call of the read callback. Small enough
/// that the look-ahead past a few levels needs another call.
const CHUNK_BYTES: usize = 64;

/// Most the bytes served per input byte may grow from the shorter nest to
/// the longer. The bounded look-ahead keeps the two about equal (measured:
/// 12.6 and 12.8 bytes per byte for the nest that does not close, 9.4 and
/// 9.4 for the one that closes); one that read to the end of the nest grows
/// with it, about eightfold here (126.6 and 1,001.6, and 128.9 and 1,003.9,
/// with the bound at 2^30).
const MAX_BYTES_GROWTH: f64 = 1.5;

/// Parses `source` and returns the bytes served to the parser per byte of
/// input, and whether the tree has an `ERROR` or a `MISSING` node.
fn bytes_served_per_input_byte(source: &str) -> (f64, bool) {
    let bytes = source.as_bytes();
    let mut parser = tree_sitter::Parser::new();
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
    (ratio, tree.root_node().has_error())
}

/// Asserts that the bytes served per input byte of a nest of `levels(n)`
/// levels do not grow with `n`, and that the nest parses with an `ERROR`
/// exactly when `has_error` says it does.
fn assert_linear(levels: impl Fn(usize) -> String, has_error: bool) {
    let short_source = levels(SHORT_NEST);
    let long_source = levels(SHORT_NEST * SCALE);
    let (short, short_error) = bytes_served_per_input_byte(&short_source);
    let (long, long_error) = bytes_served_per_input_byte(&long_source);
    assert_eq!(short_error, has_error, "{:?}", &short_source[..40]);
    assert_eq!(long_error, has_error, "{:?}", &long_source[..40]);
    assert!(
        long <= short * MAX_BYTES_GROWTH,
        "bytes served per input byte of {:?} grew from {short:.1} to {long:.1}",
        &short_source[..40],
    );
}

#[test]
fn a_nest_of_strings_that_does_not_close_is_read_a_bounded_number_of_times() {
    assert_linear(|n| "foo \"${ {\n".repeat(n), true);
    assert_linear(|n| "foo \"${ { foo 1 + 2\n".repeat(n), true);
}

#[test]
fn a_nest_of_strings_that_closes_is_read_a_bounded_number_of_times() {
    assert_linear(
        |n| format!("{}x{}\n", "foo \"${ { ".repeat(n), " }}\"".repeat(n)),
        false,
    );
}
