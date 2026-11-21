//! Cost test for the look-ahead of a call with a closure that a member
//! access follows (`foo { }.bar()`, `SPECIFICATION.md` §6.24).
//!
//! At every `name {` that opens a statement the scanner reads on from the
//! `{` for its `}` on the line and the access after it. On a line of nested
//! calls with closures (`a { a { a { …`) that read starts again at each `{`,
//! and without a bound each read would go to the end of the line, in time
//! quadratic in its length. The read stops at
//! `CLOSURE_CALL_LOOKAHEAD_CHARACTERS` (128) characters from its `{`, so the
//! bytes the parser asks for per byte of input stay the same at any length.
//! The test reads the input through a callback and compares that ratio at
//! two lengths of a line, with the closures closed, unclosed and followed by
//! accesses, and at two lengths of a file of short lines.
//!
//! It prints the ratios, so that a run with `--nocapture` shows them.

use tree_sitter::Parser;

/// Closures on the shorter line; the longer one has eight times as many.
const SHORT_RUN: usize = 250;

/// How many times longer the second line is.
const SCALE: usize = 8;

/// Bytes handed to the parser per call of the read callback. Small enough
/// that a read past a few closures needs another call.
const CHUNK_BYTES: usize = 64;

/// Most the bytes served per input byte may grow from the shorter line to the
/// longer. The bound keeps the two about equal (measured: 36.5 and 38.5 bytes
/// per byte for a line of closed closures, 43.3 and 47.4 for unclosed ones,
/// 19.8 and 20.3 for closures with accesses, 18.6 and 19.1 for unclosed
/// closures after accesses); without it every read went to
/// the end of the line, and the ratio grew with it, about eightfold here
/// (132.2 and 1,008.2 for the closed closures with the bound at 2^30).
const MAX_BYTES_GROWTH: f64 = 1.5;

/// Parses `source` and returns the bytes served to the parser per byte of
/// input. Lines nested deeper than the bound have errors, as the calls whose
/// accesses the read does not reach stay the `ERROR` they were before.
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

/// Asserts that the bytes served per input byte of `make(count)` do not grow
/// from `SHORT_RUN` to `SHORT_RUN * SCALE`.
fn assert_bounded(name: &str, make: impl Fn(usize) -> String) {
    let short = bytes_served_per_input_byte(&make(SHORT_RUN));
    let long = bytes_served_per_input_byte(&make(SHORT_RUN * SCALE));
    println!("{name}: {short:.1} and {long:.1} bytes served per input byte");
    assert!(
        long <= short * MAX_BYTES_GROWTH,
        "bytes served per input byte of {name} grew from {short:.1} to {long:.1}",
    );
}

#[test]
fn a_line_of_nested_closures_is_read_a_bounded_number_of_times() {
    assert_bounded("a line of closed closures", |n| {
        format!("{}{}\n", "a { ".repeat(n), "}".repeat(n))
    });
    assert_bounded("a line of unclosed closures", |n| {
        format!("{}\n", "a { ".repeat(n))
    });
    assert_bounded("a line of closures with accesses", |n| {
        format!("{}{}\n", "a { ".repeat(n), "}.b() ".repeat(n))
    });
    // The read goes into the brackets after each access, and each `a`
    // opens a statement of the closure before it.
    assert_bounded("a line of unclosed closures after accesses", |n| {
        format!("{}\n", "a { }.b { ".repeat(n))
    });
}

#[test]
fn lines_of_closures_are_read_a_bounded_number_of_times() {
    assert_bounded("lines of nested closures", |n| {
        "a { b { c { } } }\n".repeat(n)
    });
    assert_bounded("lines of calls with accesses", |n| {
        "foo { a }.bar()\n".repeat(n)
    });
    assert_bounded("lines of blocks", |n| "foo {\n}\n".repeat(n));
}
