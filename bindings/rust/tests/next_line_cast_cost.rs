//! Cost test for a cast whose operand is on a later line
//! (`SPECIFICATION.md` §6.11, `docs/divergences-from-spec.md` §15).
//!
//! A `)` that ends its line may close a cast whose operand is on the next
//! line, a reading ranked below every other. Where the next line opens with
//! `(`, as in a run of lines like `(x)`, that reading lost at each line but
//! stayed on the stack, and was reduced again at every line after it: a run
//! took time quadratic in its length (2.1 s for 4,000 lines of `(x)`). The
//! scanner now ends that reading at such a `)`, unless a `)` or `]` after the
//! next line's `(` closes brackets around both lines. It reads Groovy's
//! tokens for that bracket, so a bracket in a comment or a string counts for
//! none. In a run of such lines only a comment can hold one (a type holds no
//! string), so the second test has comments; the corpus tests in
//! `operators-coercion.txt` pin the strings, where a bracket counted in one
//! turned a valid cast down.
//!
//! The parser's work per line is what grew. The tests count the reductions in
//! the parser's log, which is deterministic where time is not, at two lengths
//! of a run.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tree_sitter::{LogType, Parser};

/// Lines in the shorter run; the longer one has eight times as many.
const SHORT_RUN: usize = 250;

/// How many times longer the second run is.
const SCALE: usize = 8;

/// Most the reductions per line may grow from the shorter run to the longer.
/// With no reading alive across lines a line takes the same reductions at
/// any length (measured: 4 per line of `(x)` at both lengths); the cast
/// reading reduced again at every line made them grow with the run, about
/// eightfold here (130 and 1,004 per line of `(x)` before).
const MAX_GROWTH: f64 = 1.5;

/// The prefix of the parser's log messages for a reduction.
const REDUCE_PREFIX: &str = "reduce sym:";

/// Parses `line` repeated `count` times and returns the reductions per line
/// that the parser's log reports.
fn reductions_per_line(line: &str, count: usize) -> f64 {
    let mut parser = Parser::new();
    parser
        .set_language(&dekobon_tree_sitter_groovy::LANGUAGE.into())
        .expect("load Groovy grammar");
    let reductions = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&reductions);
    parser.set_logger(Some(Box::new(move |log_type, message| {
        if log_type == LogType::Parse && message.starts_with(REDUCE_PREFIX) {
            seen.fetch_add(1, Ordering::Relaxed);
        }
    })));
    let tree = parser.parse(line.repeat(count), None).expect("parse");
    assert!(!tree.root_node().has_error(), "{line:?}");
    #[allow(clippy::cast_precision_loss, reason = "counts far below 2^52")]
    let ratio = reductions.load(Ordering::Relaxed) as f64 / count as f64;
    ratio
}

/// Asserts that the reductions per line of a run of `line` do not grow with
/// the length of the run.
fn assert_linear(line: &str) {
    let short = reductions_per_line(line, SHORT_RUN);
    let long = reductions_per_line(line, SHORT_RUN * SCALE);
    assert!(
        long <= short * MAX_GROWTH,
        "reductions per line of {line:?} grew from {short:.1} to {long:.1}",
    );
}

#[test]
fn runs_of_parenthesized_lines_take_a_bounded_number_of_reductions_per_line() {
    for line in [
        "(x)\n",
        "(a.b)\n",
        "(Foo)\n",
        "(a) = (b)\n",
        "(a, b) = (p)\n",
        "(x) // c\n",
    ] {
        assert_linear(line);
    }
}

/// A bracket in a comment on the next line closes no brackets around the
/// cast (measured: 4 reductions per line at both lengths; 129.5 and 1,004.5
/// where brackets were counted in comments).
#[test]
fn brackets_in_comments_keep_no_cast_alive() {
    for line in ["(x) /* ) */\n", "(x) /* ] */\n", "(x) // )\n"] {
        assert_linear(line);
    }
}
