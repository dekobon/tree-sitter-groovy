//! Cost ceilings for three shapes of input that this grammar rejects and
//! parses in time above linear in its length (`docs/divergences-from-spec.md`
//! §29). At v0.6.0 each was linear. None is fixed: these tests pin today's
//! cost so that it does not grow further.
//!
//! - Nested parenthesis-free calls with a spread argument and `=`,
//!   `foo 1, *a = foo 1, *a = … 1`, which Groovy 3 reads as
//!   `foo(1, *(a = foo(…)))`. A spread argument is an `ERROR` in a command
//!   chain, and each nesting level keeps stack versions alive: the parser's
//!   reductions grow with the square of the nesting and its stack versions
//!   linearly (measured: 1,408, 5,283 and 20,533 reductions and 63, 113 and
//!   213 versions at 25, 50 and 100 levels; v0.6.0 409 and 4 at 100).
//! - Runs of unfinished annotations, `@C(, @C(, …`. The parser's counts are
//!   linear (measured: 1,244, 2,494 and 4,994 reductions at 250, 500 and
//!   1,000 annotations, at most 6 versions); the time is not, because the
//!   runtime rebuilds the growing `ERROR` node, which its log does not show.
//!   The ceiling below pins the counts only.
//! - Lines of `case 1:` outside a `switch`: a version that nests the lines
//!   as labels closes the whole nest at each line (measured: 5,151, 20,301
//!   and 80,601 reductions at 100, 200 and 400 lines; v0.6.0 401 at 400).
//!
//! The counts come from the parser's log, which is deterministic where time
//! is not.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tree_sitter::{LogType, Parser};

/// Nesting levels of the spread-argument calls.
const SPREAD_LEVELS: usize = 50;

/// Most reductions for `SPREAD_LEVELS` levels: 5,283 measured, about 1.5
/// times that.
const MAX_SPREAD_REDUCTIONS: usize = 8_000;

/// Most stack versions at once for `SPREAD_LEVELS` levels: 113 measured.
const MAX_SPREAD_VERSIONS: usize = 170;

/// Unfinished annotations in a run.
const ANNOTATIONS: usize = 500;

/// Most reductions for `ANNOTATIONS` annotations: 2,494 measured.
const MAX_ANNOTATION_REDUCTIONS: usize = 4_000;

/// Most stack versions at once for `ANNOTATIONS` annotations: 6 measured.
const MAX_ANNOTATION_VERSIONS: usize = 10;

/// Lines of `case 1:`.
const CASE_LINES: usize = 200;

/// Most reductions for `CASE_LINES` lines: 20,301 measured.
const MAX_CASE_REDUCTIONS: usize = 30_000;

/// The key of the version count in the parser's `process version` messages.
const VERSION_COUNT_KEY: &str = "version_count:";

/// What the parser's log says about one parse.
struct Cost {
    reductions: usize,
    most_versions: usize,
}

/// Parses `source`, which must hold an `ERROR`, and counts the reductions
/// and the most stack versions at once in the parser's log.
fn cost(source: &str) -> Cost {
    let mut parser = Parser::new();
    parser
        .set_language(&dekobon_tree_sitter_groovy::LANGUAGE.into())
        .expect("load Groovy grammar");
    let reductions = Arc::new(AtomicUsize::new(0));
    let most_versions = Arc::new(AtomicUsize::new(0));
    let (reduced, seen) = (Arc::clone(&reductions), Arc::clone(&most_versions));
    parser.set_logger(Some(Box::new(move |log_type, message| {
        if log_type != LogType::Parse {
            return;
        }
        if message.starts_with("reduce") {
            reduced.fetch_add(1, Ordering::Relaxed);
        } else if message.starts_with("process version") {
            let count = message
                .split_once(VERSION_COUNT_KEY)
                .and_then(|(_, rest)| rest.split(',').next())
                .and_then(|digits| digits.trim().parse::<usize>().ok())
                .expect("version count in a process message");
            seen.fetch_max(count, Ordering::Relaxed);
        }
    })));
    let tree = parser.parse(source, None).expect("parse");
    assert!(
        tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    let cost = Cost {
        reductions: reductions.load(Ordering::Relaxed),
        most_versions: most_versions.load(Ordering::Relaxed),
    };
    assert!(
        cost.reductions > 0 && cost.most_versions > 0,
        "no reductions or version counts in the parser's log",
    );
    cost
}

#[test]
fn nested_spread_argument_calls_stay_under_the_measured_cost() {
    let source = format!("{}1\n", "foo 1, *a = ".repeat(SPREAD_LEVELS));
    let cost = cost(&source);
    assert!(
        cost.reductions <= MAX_SPREAD_REDUCTIONS,
        "{} reductions for {SPREAD_LEVELS} levels",
        cost.reductions,
    );
    assert!(
        cost.most_versions <= MAX_SPREAD_VERSIONS,
        "{} stack versions for {SPREAD_LEVELS} levels",
        cost.most_versions,
    );
}

#[test]
fn runs_of_unfinished_annotations_stay_under_the_measured_cost() {
    let cost = cost(&"@C(, ".repeat(ANNOTATIONS));
    assert!(
        cost.reductions <= MAX_ANNOTATION_REDUCTIONS,
        "{} reductions for {ANNOTATIONS} annotations",
        cost.reductions,
    );
    assert!(
        cost.most_versions <= MAX_ANNOTATION_VERSIONS,
        "{} stack versions for {ANNOTATIONS} annotations",
        cost.most_versions,
    );
}

#[test]
fn case_lines_outside_a_switch_stay_under_the_measured_cost() {
    let cost = cost(&"case 1:\n".repeat(CASE_LINES));
    assert!(
        cost.reductions <= MAX_CASE_REDUCTIONS,
        "{} reductions for {CASE_LINES} lines",
        cost.reductions,
    );
}
