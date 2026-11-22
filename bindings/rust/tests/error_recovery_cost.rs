//! Cost ceilings for shapes of input that this grammar rejects and parses in
//! time above linear in its length (`docs/divergences-from-spec.md` §29), and
//! linear bounds for inputs that were among them. At v0.6.0 each was
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
//!   at 100, 200 and 400 lines; v0.6.0 401 at 400). Reserving `case` moved
//!   that cost to lines of `case A:`, whose `A:` recovery read as a label,
//!   and to lines of `x = case` (20,302 reductions at 200 lines and
//!   1,282,402 at 1,600 for both). A `case` label where a statement starts
//!   is an error production now (`_stray_case`), which recovery completes
//!   with a missing `switch`, so each line recovers on its own: the lines,
//!   with a name or a literal, alone or under a header that is no `switch`,
//!   take about 1 reduction and 11 `process version` messages a line (one
//!   for each step of a stack version) and at most 4 versions. Recovery
//!   that inserts a missing token logs none of the reductions it makes, so
//!   the steps are bounded as well.
//! - The reserved `case` where an expression goes on, which that production
//!   does not reach: `foo(case, ` repeated, whose counts are linear
//!   (measured: 297, 597 and 1,497 reductions at 100, 200 and 500, at most 5
//!   versions) and whose time is not, as for the annotations, and `x ? y :`
//!   followed by a line of `case`, repeated, whose reductions grow with the
//!   square of the lines (measured: 443, 8,393 and 92,243 at 100, 200 and
//!   500 lines, at most 4 versions). While `case` was a name both parsed in
//!   linear time.
//! - A multi-assignment declaration with a run of `, b) =` after its `=`,
//!   `def (a, b) =, b) =, b) = … 1`. While the names were a rule of their
//!   own, which the parser reduced before the `=`, recovery could not go
//!   back into the list at the `,`, as it did at v0.6.0. It skipped tokens,
//!   which left each `b … =` an assignment nested in the one before, and
//!   every error reduced the whole nest (measured: 307, 1,232 and 4,957
//!   reductions at 25, 50 and 100 repeats, at most 4 versions; v0.6.0 80,
//!   155 and 305, at most 3). With the `=` and the value in the rule of the
//!   names' parentheses the list stays on the stack, and the run takes
//!   about 2 reductions a repeat in at most 3 versions (measured: 106 and
//!   806 reductions at 50 and 400 repeats). A list of keyed pairs,
//!   `def (k: a, j: b) =`, took no plain name, so `, b` did not continue it
//!   and its run cost what it had (1,232 reductions at 50 repeats); one
//!   list takes names and pairs alike now, wider than Groovy, and that run
//!   is linear as well. Where no list of names takes it, after `x =` or
//!   alone, the run is quadratic, here and at v0.6.0; after the names of a
//!   multi-assignment without `def`, `(a, b) =`, it is linear.
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

/// Case lines in the shorter run; the longer one has `SCALE` times as many.
const CASE_LINES: usize = 200;

/// The lines that are repeated: a case label with a literal, which no label
/// starts with, with a name, which recovery read as a label nested in the
/// line before, that name before a comment and before an empty statement,
/// and an assignment of the reserved word.
const CASE_LINE_SHAPES: [&str; 5] = [
    "case 1:\n",
    "case A:\n",
    "case A: // c\n",
    "case A: ;\n",
    "x = case\n",
];

/// What the lines stand in: nothing, and the braces of a mistyped `switch`
/// header, in a file and in a method.
const CASE_LINE_FRAMES: [(&str, &str); 3] = [
    ("", ""),
    ("swit (x) {\n", "}\n"),
    ("def m() {\nswit (x) {\n", "}\n}\n"),
];

/// Most reductions per line: at most 1.01 measured (1,616 for 1,600 lines
/// of `case A: ;` in a method), where the lines of `case A:` made 20,302 at
/// 200 lines while recovery read `A:` as a label.
const MAX_CASE_REDUCTIONS_PER_LINE: usize = 2;

/// Most steps of a stack version per line: 11 measured for the lines with a
/// comment, 9 for the others, at both lengths.
const MAX_CASE_STEPS_PER_LINE: usize = 14;

/// Most stack versions at once for the lines: 4 measured. It was 2 for the
/// lines of `case 1:` before the error production; the versions that
/// recovery keeps for the missing `switch` raise it.
const MAX_CASE_VERSIONS: usize = 6;

/// Calls in the run of `foo(case, `.
const STRAY_CASE_CALLS: usize = 500;

/// Most reductions for `STRAY_CASE_CALLS` calls: 1,497 measured.
const MAX_STRAY_CASE_CALL_REDUCTIONS: usize = 2_500;

/// Most stack versions at once for `STRAY_CASE_CALLS` calls: 5 measured.
const MAX_STRAY_CASE_CALL_VERSIONS: usize = 8;

/// Repeats of `x ? y :` and a line of `case`.
const TERNARY_CASE_LINES: usize = 200;

/// Most reductions for `TERNARY_CASE_LINES` repeats: 8,393 measured.
const MAX_TERNARY_CASE_REDUCTIONS: usize = 12_000;

/// Most stack versions at once for `TERNARY_CASE_LINES` repeats: 4 measured.
const MAX_TERNARY_CASE_VERSIONS: usize = 6;

/// Repeats of `, b) =` after the `=` of a multi-assignment declaration in
/// the shorter run; the longer one has `SCALE` times as many.
const MULTI_ASSIGNMENT_RUN: usize = 50;

/// The declarations the run follows: the keywords that open one, typed
/// names, a rest binding, keyed pairs, an annotation, the member of a type
/// body and a `for` init.
const MULTI_ASSIGNMENT_RUN_FRAMES: [(&str, &str); 9] = [
    ("def (a, b) =", " 1\n"),
    ("final (a, b) =", " 1\n"),
    ("var (a, b) =", " 1\n"),
    ("def (int a, String b) =", " 1\n"),
    ("def (a, *b) =", " 1\n"),
    ("def (k: a, j: b) =", " 1\n"),
    ("@A def (a, b) =", " 1\n"),
    ("class C { def (a, b) =", " 1 }\n"),
    ("for (def (a, b) =", " 1; ;) { }\n"),
];

/// Most reductions per repeat of the run: at most 2.2 measured (109 for 50
/// repeats in a `for` init, 809 for 400), where the names as a rule of
/// their own made 1,232 at 50 repeats.
const MAX_MULTI_ASSIGNMENT_RUN_REDUCTIONS_PER_REPEAT: usize = 4;

/// Most stack versions at once for the run: 3 measured.
const MAX_MULTI_ASSIGNMENT_RUN_VERSIONS: usize = 5;

/// The key of the version count in the parser's `process version` messages.
const VERSION_COUNT_KEY: &str = "version_count:";

/// What the parser's log says about one parse.
struct Cost {
    reductions: usize,
    most_versions: usize,
    steps: usize,
}

/// Parses `source`, which must hold an `ERROR` exactly when `rejected`, and
/// counts the reductions, the steps of stack versions (its `process version`
/// messages) and the most stack versions at once in the parser's log.
fn cost(source: &str, rejected: bool) -> Cost {
    let mut parser = Parser::new();
    parser
        .set_language(&dekobon_tree_sitter_groovy::LANGUAGE.into())
        .expect("load Groovy grammar");
    let reductions = Arc::new(AtomicUsize::new(0));
    let most_versions = Arc::new(AtomicUsize::new(0));
    let steps = Arc::new(AtomicUsize::new(0));
    let (reduced, seen, stepped) = (
        Arc::clone(&reductions),
        Arc::clone(&most_versions),
        Arc::clone(&steps),
    );
    parser.set_logger(Some(Box::new(move |log_type, message| {
        if log_type != LogType::Parse {
            return;
        }
        if message.starts_with("reduce") {
            reduced.fetch_add(1, Ordering::Relaxed);
        } else if message.starts_with("process version") {
            stepped.fetch_add(1, Ordering::Relaxed);
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
        steps: steps.load(Ordering::Relaxed),
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
    for (before, after) in CASE_LINE_FRAMES {
        for line in CASE_LINE_SHAPES {
            for lines in [CASE_LINES, CASE_LINES * SCALE] {
                let source = format!("{before}{}{after}", line.repeat(lines));
                let cost = cost(&source, true);
                assert!(
                    cost.reductions <= MAX_CASE_REDUCTIONS_PER_LINE * lines,
                    "{} reductions for {lines} lines of {line:?} after {before:?}",
                    cost.reductions,
                );
                assert!(
                    cost.steps <= MAX_CASE_STEPS_PER_LINE * lines,
                    "{} version steps for {lines} lines of {line:?} after {before:?}",
                    cost.steps,
                );
                assert!(
                    cost.most_versions <= MAX_CASE_VERSIONS,
                    "{} stack versions for {lines} lines of {line:?} after {before:?}",
                    cost.most_versions,
                );
            }
        }
    }
}

#[test]
fn a_reserved_case_in_expressions_stays_under_the_measured_cost() {
    let calls = cost(&"foo(case, ".repeat(STRAY_CASE_CALLS), true);
    assert!(
        calls.reductions <= MAX_STRAY_CASE_CALL_REDUCTIONS,
        "{} reductions for {STRAY_CASE_CALLS} calls",
        calls.reductions,
    );
    assert!(
        calls.most_versions <= MAX_STRAY_CASE_CALL_VERSIONS,
        "{} stack versions for {STRAY_CASE_CALLS} calls",
        calls.most_versions,
    );
    let ternaries = cost(&"x ? y :\ncase\n".repeat(TERNARY_CASE_LINES), true);
    assert!(
        ternaries.reductions <= MAX_TERNARY_CASE_REDUCTIONS,
        "{} reductions for {TERNARY_CASE_LINES} repeats",
        ternaries.reductions,
    );
    assert!(
        ternaries.most_versions <= MAX_TERNARY_CASE_VERSIONS,
        "{} stack versions for {TERNARY_CASE_LINES} repeats",
        ternaries.most_versions,
    );
}

#[test]
fn a_run_after_a_multi_assignment_declaration_parses_in_linear_cost() {
    for (before, after) in MULTI_ASSIGNMENT_RUN_FRAMES {
        for repeats in [MULTI_ASSIGNMENT_RUN, MULTI_ASSIGNMENT_RUN * SCALE] {
            let source = format!("{before}{}{after}", ", b) =".repeat(repeats));
            let cost = cost(&source, true);
            assert!(
                cost.reductions <= MAX_MULTI_ASSIGNMENT_RUN_REDUCTIONS_PER_REPEAT * repeats,
                "{} reductions for {repeats} repeats after {before:?}",
                cost.reductions,
            );
            assert!(
                cost.most_versions <= MAX_MULTI_ASSIGNMENT_RUN_VERSIONS,
                "{} stack versions for {repeats} repeats after {before:?}",
                cost.most_versions,
            );
        }
    }
}
