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
//! The parser's work per line is what grew. The first tests count the
//! reductions in the parser's log, which is deterministic where time is not,
//! at two lengths of a run. The scan for those brackets reads on from every
//! such `(` for at most `ENCLOSING_BRACKET_LOOKAHEAD_CHARACTERS` (256); the
//! last test reads the input through a callback and compares the bytes the
//! parser asks for at the two lengths, which a scan that read to the end of
//! the run from every line would make grow with it.

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

/// Bytes handed to the parser per call of the read callback. Small enough
/// that the scan past a few lines needs another call.
const CHUNK_BYTES: usize = 64;

/// Most the bytes served per input byte may grow from the shorter run to the
/// longer. The bounded scan keeps the two about equal (measured: 67.5 and
/// 78.4 bytes per byte for runs of `(x)`, 30.0 and 31.8 for `(a) = (b)`); one
/// that reads to the end of the run from every line grows with it, almost
/// eightfold here (125.8 and 1,000.8 for `(x)` with the bound at 2^30).
const MAX_BYTES_GROWTH: f64 = 1.5;

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

/// Parses `line` repeated `count` times and returns the bytes served to the
/// parser per byte of input.
fn bytes_served_per_input_byte(line: &str, count: usize) -> f64 {
    let source = line.repeat(count);
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
    assert!(!tree.root_node().has_error(), "{line:?}");
    #[allow(clippy::cast_precision_loss, reason = "sizes far below 2^52")]
    let ratio = served as f64 / bytes.len() as f64;
    ratio
}

#[test]
fn the_scan_for_brackets_around_both_lines_reads_a_bounded_number_of_bytes() {
    for line in ["(x)\n", "(a) = (b)\n", "(x) // c\n"] {
        let short = bytes_served_per_input_byte(line, SHORT_RUN);
        let long = bytes_served_per_input_byte(line, SHORT_RUN * SCALE);
        assert!(
            long <= short * MAX_BYTES_GROWTH,
            "bytes served per input byte of {line:?} grew from {short:.1} to {long:.1}",
        );
    }
}
