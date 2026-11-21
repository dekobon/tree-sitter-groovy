//! Cost ceilings for shapes of input that this grammar rejects and parses in
//! time above linear in its length (`docs/divergences-from-spec.md` §29), and
//! linear bounds for two inputs that were among them. At v0.6.0 each was
//! linear. The rejected shapes left are not fixed: these tests pin today's
//! cost so that it does not grow further.
//!
//! - Nested parenthesis-free calls with an argument that is an `ERROR`
//!   after a comma and `=`, `foo 1, %a = foo 1, %a = … 1`. Each nesting
//!   level keeps stack versions alive: the parser's reductions grow with the
//!   square of the nesting and its stack versions linearly (measured: 1,408,
//!   5,283 and 20,533 reductions and 63, 113 and 213 versions at 25, 50 and
//!   100 levels; v0.6.0 409 and 4 at 100).
//! - The same nest with a spread argument, `foo 1, *a = foo 1, *a = … 1`,
//!   which Groovy reads as `foo(1, *(a = foo(…)))`. It cost as much while a
//!   command chain took no spread argument; now that it takes one, the nest
//!   parses without an `ERROR` in one stack version and reductions linear in
//!   its depth (measured: 201 and 1,601 at 50 and 400 levels).
//! - Runs of unfinished annotations, `@C(, @C(, …`. The parser's counts are
//!   linear (measured: 1,244, 2,494 and 4,994 reductions at 250, 500 and
//!   1,000 annotations, at most 6 versions); the time is not, because the
//!   runtime rebuilds the growing `ERROR` node, which its log does not show.
//!   The ceiling below pins the counts only.
//! - Lines of `case 1:` outside a `switch`, which Groovy rejects. While
//!   `case` was a name, a version nested the lines as labels and closed the
//!   whole nest at each line (measured: 5,151, 20,301 and 80,601 reductions
//!   at 100, 200 and 400 lines; v0.6.0 401 at 400). `case` is reserved now,
//!   and the reductions are linear in the lines (measured: 797 and 6,397 at
//!   200 and 1,600 lines, at most 2 versions).
//!
//! The counts come from the parser's log, which is deterministic where time
//! is not.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tree_sitter::{LogType, Parser};

/// Nesting levels of the calls with an `ERROR` argument.
const NEST_LEVELS: usize = 50;

/// Most reductions for `NEST_LEVELS` levels: 5,283 measured, about 1.5
/// times that.
const MAX_NEST_REDUCTIONS: usize = 8_000;

/// Most stack versions at once for `NEST_LEVELS` levels: 113 measured.
const MAX_NEST_VERSIONS: usize = 170;

/// Nesting levels of the shallower spread-argument nest; the deeper one has
/// `SCALE` times as many.
const SPREAD_LEVELS: usize = 50;

/// How many times longer the second input of a linear bound is.
const SCALE: usize = 8;

/// Most reductions per nesting level of the spread-argument nest: 4
/// measured at both depths (201 and 1,601), where a command chain without
/// spread arguments made 5,283 at 50 levels.
const MAX_SPREAD_REDUCTIONS_PER_LEVEL: usize = 6;

/// Most stack versions at once for the spread-argument nest: 1 measured.
const MAX_SPREAD_VERSIONS: usize = 2;

/// Unfinished annotations in a run.
const ANNOTATIONS: usize = 500;

/// Most reductions for `ANNOTATIONS` annotations: 2,494 measured.
const MAX_ANNOTATION_REDUCTIONS: usize = 4_000;

/// Most stack versions at once for `ANNOTATIONS` annotations: 6 measured.
const MAX_ANNOTATION_VERSIONS: usize = 10;

/// Lines of `case 1:` in the shorter run; the longer one has `SCALE` times
/// as many.
const CASE_LINES: usize = 200;

/// Most reductions per line of `case 1:`: 4 measured at both lengths (797
/// and 6,397), where a `case` that was a name made 20,301 at 200 lines.
const MAX_CASE_REDUCTIONS_PER_LINE: usize = 6;

/// Most stack versions at once for the lines of `case 1:`: 2 measured.
const MAX_CASE_VERSIONS: usize = 3;

/// The key of the version count in the parser's `process version` messages.
const VERSION_COUNT_KEY: &str = "version_count:";

/// What the parser's log says about one parse.
struct Cost {
    reductions: usize,
    most_versions: usize,
}

/// Parses `source`, which must hold an `ERROR` exactly when `rejected`, and
/// counts the reductions and the most stack versions at once in the parser's
/// log.
fn cost(source: &str, rejected: bool) -> Cost {
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
    assert_eq!(
        tree.root_node().has_error(),
        rejected,
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
fn nested_calls_with_an_error_argument_stay_under_the_measured_cost() {
    let source = format!("{}1\n", "foo 1, %a = ".repeat(NEST_LEVELS));
    let cost = cost(&source, true);
    assert!(
        cost.reductions <= MAX_NEST_REDUCTIONS,
        "{} reductions for {NEST_LEVELS} levels",
        cost.reductions,
    );
    assert!(
        cost.most_versions <= MAX_NEST_VERSIONS,
        "{} stack versions for {NEST_LEVELS} levels",
        cost.most_versions,
    );
}

#[test]
fn nested_spread_argument_calls_parse_in_linear_cost() {
    for levels in [SPREAD_LEVELS, SPREAD_LEVELS * SCALE] {
        let source = format!("{}1\n", "foo 1, *a = ".repeat(levels));
        let cost = cost(&source, false);
        assert!(
            cost.reductions <= MAX_SPREAD_REDUCTIONS_PER_LEVEL * levels,
            "{} reductions for {levels} levels",
            cost.reductions,
        );
        assert!(
            cost.most_versions <= MAX_SPREAD_VERSIONS,
            "{} stack versions for {levels} levels",
            cost.most_versions,
        );
    }
}

#[test]
fn runs_of_unfinished_annotations_stay_under_the_measured_cost() {
    let cost = cost(&"@C(, ".repeat(ANNOTATIONS), true);
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
fn case_lines_outside_a_switch_parse_in_linear_cost() {
    for lines in [CASE_LINES, CASE_LINES * SCALE] {
        let cost = cost(&"case 1:\n".repeat(lines), true);
        assert!(
            cost.reductions <= MAX_CASE_REDUCTIONS_PER_LINE * lines,
            "{} reductions for {lines} lines",
            cost.reductions,
        );
        assert!(
            cost.most_versions <= MAX_CASE_VERSIONS,
            "{} stack versions for {lines} lines",
            cost.most_versions,
        );
    }
}
