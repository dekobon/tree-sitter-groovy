//! Cost test for dotted types after `as`, `instanceof` and `!instanceof`
//! (`SPECIFICATION.md` §3.1, `docs/divergences-from-spec.md` §5).
//!
//! A dotted type there also read as field accesses on a cast of its first
//! name, `(x as java).util.List`, and with type arguments as comparisons,
//! `(x as java).util.List < T > …`, whose `>` takes the next line as its
//! operand. Kept as a second parse stack, that reading went on through every
//! following line of the same shape, so a run of such lines took time
//! quadratic in its length. The first dot now continues the type.
//!
//! The progress callbacks, which count parse operations, and the parser's
//! log grew linearly with the run while the time did not, so this test
//! measures time: the fastest of a few parses of each run, per line, at two
//! run lengths. The bound is generous, to stay clear of timing noise.
//!
//! The scanner reads the type arguments after the name of a cast's type to
//! see whether they close (§3.2.2). Where they do not, as in a call whose
//! arguments are `x as B<c` each, the read must stop after a fixed number of
//! items, or it reads the rest of the line from each `<`. The second test
//! compares the bytes the parser asks for at two lengths of such a line.

use std::time::{Duration, Instant};

use tree_sitter::Parser;

// -- Runs of dotted generic casts, timed --------------------------------

/// Lines in the shorter run; the longer one has eight times as many.
const SHORT_RUN: usize = 1_000;

/// How many times longer the second run is.
const SCALE: usize = 8;

/// Parses of each run; the fastest counts.
const ATTEMPTS: usize = 3;

/// Most the time per line may grow from the shorter run to the longer.
/// Linear work keeps the two about equal (measured in a debug build: 0.9
/// to 1.0 for each line below); the reading that went on as comparisons
/// made it grow 2.6 times for `l = x as java.util.List<T>`, and 4.0 times
/// for `l = x as util.List<T>` before the first dot continued the type.
const MAX_GROWTH: f64 = 1.6;

/// The fastest of `ATTEMPTS` parses of `line` repeated `count` times.
fn fastest_parse(parser: &mut Parser, line: &str, count: usize) -> Duration {
    let source = line.repeat(count);
    (0..ATTEMPTS)
        .map(|_| {
            let start = Instant::now();
            let tree = parser.parse(&source, None).expect("parse");
            let elapsed = start.elapsed();
            assert!(!tree.root_node().has_error(), "{line:?}");
            elapsed
        })
        .min()
        .expect("at least one attempt")
}

/// Asserts that the time per line does not grow with the length of a run
/// of `line`.
fn assert_linear(line: &str) {
    let mut parser = Parser::new();
    parser
        .set_language(&dekobon_tree_sitter_groovy::LANGUAGE.into())
        .expect("load Groovy grammar");
    let short = fastest_parse(&mut parser, line, SHORT_RUN).as_secs_f64();
    let long = fastest_parse(&mut parser, line, SHORT_RUN * SCALE).as_secs_f64();
    #[allow(clippy::cast_precision_loss, reason = "a small constant")]
    let growth = long / (short * SCALE as f64);
    assert!(
        growth <= MAX_GROWTH,
        "time per line grew {growth:.2} times for {line:?} ({short:.4} s, then {long:.4} s)",
    );
}

/// Lines whose runs are parsed: coercions and tests to dotted generic
/// types, with names of three parts and of two, after a name and a call.
/// One test parses them one after another, so that no other test of this
/// file runs beside the timing.
const LINES: &[&str] = &[
    "l = x as java.util.List<T>\n",
    "l = x as util.List<T>\n",
    "v = m.get('k') as java.util.List<String>\n",
    "ok = v instanceof java.util.List<T>\n",
];

#[test]
fn runs_of_dotted_generic_cast_types_are_parsed_in_linear_time() {
    for line in LINES {
        assert_linear(line);
    }
}

// -- Type arguments that do not close, by bytes read -----------------------

/// Elements in the shorter line; the longer one has eight times as many.
const SHORT_LINE: usize = 500;

/// Bytes handed to the parser per call of the read callback. Small enough
/// that a look-ahead past a few elements needs another call.
const CHUNK_BYTES: usize = 64;

/// Most the bytes served per input byte may grow from the shorter line to
/// the longer. A bounded look-ahead keeps the two about equal (measured: 31.0
/// and 31.9 bytes per byte); one that reads to the end of the line grows with
/// it, eightfold here (251 and 2,001 without the bound).
const MAX_READ_GROWTH: f64 = 1.5;

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
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    #[allow(clippy::cast_precision_loss, reason = "sizes far below 2^52")]
    let ratio = served as f64 / bytes.len() as f64;
    ratio
}

#[test]
fn a_line_of_unclosed_cast_type_arguments_is_read_a_bounded_number_of_times() {
    let line = |n: usize| format!("f({}1)\n", "x as B<c, ".repeat(n));
    let short = bytes_served_per_input_byte(&line(SHORT_LINE));
    let long = bytes_served_per_input_byte(&line(SHORT_LINE * SCALE));
    assert!(
        long <= short * MAX_READ_GROWTH,
        "bytes served per input byte grew from {short:.1} to {long:.1}",
    );
}
