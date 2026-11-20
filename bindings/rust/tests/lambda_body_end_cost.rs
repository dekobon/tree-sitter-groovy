//! Cost test for the look-ahead that decides whether a lambda's brace body
//! ends at its `}` (`SPECIFICATION.md` §6.19).
//!
//! After such a `}` the scanner reads the comments that follow it and the
//! token after them. When that token continues the body, the comments are
//! read again, each from the one after it, so a run of comments before the
//! operator costs the square of its length: the look-ahead stops after a
//! small number of comments. Before a `}` that closes statements the
//! scanner reads as far again when a comment follows on its line, so many
//! lambdas with comments must cost no more per byte than a few, and it must
//! not read past the `}` of an interpolation, after which a `/*` in the
//! string is no comment and was read to the end of the input from every
//! such `}`. Time is not measured: the parser reads its input through a
//! callback here, and the test compares the bytes it asks for at two input
//! sizes, and for runs of comments with a fixed bound.

use std::fmt::Write as _;

use tree_sitter::Parser;

/// Elements in the smaller input; the larger one has eight times as many.
const SMALL_ELEMENTS: usize = 500;

/// How many times larger the second input is.
const SCALE: usize = 8;

/// Bytes handed to the parser per call of the read callback. Small
/// enough that a look-ahead past a few comments needs another call.
const CHUNK_BYTES: usize = 64;

/// Most the bytes served per input byte may grow from the smaller input
/// to the larger. A bounded look-ahead keeps the two about equal; one that
/// reads every comment of the run from every comment in it grows with the
/// run, almost eightfold here.
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

/// Most bytes served per input byte for runs of comments before an
/// operator, one just under the bound and one far over it. Each comment of
/// a run is read again from every one before it (measured: 2.8 for runs of
/// 7, and 1.1 for runs of 255, whose body ends at the ninth comment); the
/// bound of 256 comments used before served 261 per byte for runs of 255.
const MAX_BYTES_PER_BYTE_IN_A_COMMENT_RUN: f64 = 8.0;

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
fn comments_before_an_operator_after_a_lambda_body_are_read_a_bounded_number_of_times() {
    assert_linear(|n| {
        format!(
            "def v = (a) -> {{ y -> y }}{}.call()\n",
            " /* c */".repeat(n)
        )
    });
}

#[test]
fn lambda_bodies_with_comments_after_them_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("foo({}b)\n", "a -> { y -> y } /* c */ .x(), ".repeat(n)));
}

#[test]
fn interpolation_braces_before_a_comment_opener_are_read_a_bounded_number_of_times() {
    assert_linear(|n| {
        (0..n).fold(String::new(), |mut text, i| {
            let _ = writeln!(
                text,
                "task t{i} {{ doLast {{ sh \"rm -rf ${{buildDir}}/*\" }} }}"
            );
            text
        })
    });
    assert_linear(|n| "s = \"${x}/* \"\n".repeat(n));
}

#[test]
fn comment_runs_under_the_bound_before_an_operator_cost_a_fixed_amount() {
    for run in [7, 255] {
        let make = |n: usize| {
            format!(
                "def v = (a) -> {{ y -> y }}{} .call()\n",
                " /* c */".repeat(run)
            )
            .repeat(n)
        };
        let served = bytes_served_per_input_byte(&make(SMALL_ELEMENTS));
        assert!(
            served <= MAX_BYTES_PER_BYTE_IN_A_COMMENT_RUN,
            "runs of {run} comments: {served:.1} bytes served per input byte",
        );
    }
}
