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

use std::time::{Duration, Instant};

use tree_sitter::Parser;

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
