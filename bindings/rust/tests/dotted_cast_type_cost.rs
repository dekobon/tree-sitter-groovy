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
//! The parser's work per line is what grew: each line added a stack version
//! that every later token was parsed on. The first test counts those
//! versions in the parser's log, which is deterministic where time is not.
//!
//! The scanner reads the type arguments after the name of a cast's type to
//! see whether they close (§3.2.2). Where they do not, as in a call whose
//! arguments are `x as B<c` each, the read must stop after a fixed number of
//! items, or it reads the rest of the line from each `<`. The second test
//! compares the bytes the parser asks for at two lengths of such a line.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tree_sitter::{LogType, Parser};

// -- Runs of dotted generic casts, by stack versions -----------------------

/// Lines in the shorter run; the longer one has eight times as many.
const SHORT_RUN: usize = 1_000;

/// How many times longer the second run is.
const SCALE: usize = 8;

/// Most stack versions the parser may hold at once while it parses a run.
/// A run that forks nowhere keeps one (measured: 1 at both lengths); the
/// reading that went on as comparisons kept one more per line (1,001 for
/// 1,000 lines and 8,001 for 8,000 before the first dot continued the type).
const MAX_VERSIONS: usize = 2;

/// The key of the version count in the parser's `process version` messages.
const VERSION_COUNT_KEY: &str = "version_count:";

/// Parses `line` repeated `count` times and returns the most stack versions
/// the parser's log reports at once.
fn most_stack_versions(line: &str, count: usize) -> usize {
    let mut parser = Parser::new();
    parser
        .set_language(&dekobon_tree_sitter_groovy::LANGUAGE.into())
        .expect("load Groovy grammar");
    let most = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&most);
    parser.set_logger(Some(Box::new(move |log_type, message| {
        if log_type != LogType::Parse || !message.starts_with("process version") {
            return;
        }
        let count = message
            .split_once(VERSION_COUNT_KEY)
            .and_then(|(_, rest)| rest.split(',').next())
            .and_then(|digits| digits.trim().parse::<usize>().ok())
            .expect("version count in a process message");
        seen.fetch_max(count, Ordering::Relaxed);
    })));
    let tree = parser.parse(line.repeat(count), None).expect("parse");
    assert!(!tree.root_node().has_error(), "{line:?}");
    most.load(Ordering::Relaxed)
}

/// Asserts that the parser keeps at most `MAX_VERSIONS` stack versions on
/// either length of a run of `line`.
fn assert_linear(line: &str) {
    for count in [SHORT_RUN, SHORT_RUN * SCALE] {
        let most = most_stack_versions(line, count);
        assert!(
            most <= MAX_VERSIONS,
            "{most} stack versions at once for {count} lines of {line:?}",
        );
    }
}

/// Lines whose runs are parsed: coercions and tests to dotted generic
/// types, with names of three parts and of two, after a name and a call.
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
