//! Cost tests for the look-ahead at the first comma of `(a, …`
//! (`SPECIFICATION.md` §6.20).
//!
//! At the first `,` after a name in parentheses where a multi-assignment
//! may start, the scanner reads the names after it to see whether the
//! parentheses go on as a method's parameters, which ends the reading of
//! `def f(a, b) { }` as `def f` and a multi-assignment there. Where no
//! method takes the parentheses (`(a, b) { }`, `(a, b c)`), error recovery
//! re-enters the state after the first name at each later name, and each
//! comma there starts a look-ahead, so the look-ahead must stop after a
//! fixed number of names, or the parse takes time quadratic in the list's
//! length. Time is not measured: the parser reads its input through a
//! callback here, and the tests compare the bytes it asks for at two input
//! sizes. One test counts the parse steps that have more than one stack
//! version, which the look-ahead keeps from growing with a method head's
//! untyped parameters.
//!
//! The last test is about the names of a declaration, `def (a, b) = l`,
//! and the shape of their rule rather than a look-ahead: it reads the
//! parser's log for the rule's reductions.

use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tree_sitter::{LogType, Node, Parser, Tree};

/// Elements in the smaller input; the larger one has eight times as many.
const SMALL_ELEMENTS: usize = 500;

/// How many times larger the second input is.
const SCALE: usize = 8;

/// Bytes handed to the parser per call of the read callback. Small
/// enough that a look-ahead past a few elements needs another call.
const CHUNK_BYTES: usize = 64;

/// Most the bytes served per input byte may grow from the smaller input
/// to the larger. A bounded look-ahead keeps the two about equal
/// (measured: 2.6 and 2.1 bytes per byte for one long multi-assignment, for
/// one unclosed list, for a broken list in a block and for lists that a
/// brace or a second name ends, 3.7 and 2.4 for one long parameter list,
/// 1.9 and 2.0 for many heads, 2.1 and 2.1 for nested lists, and 38 and 39
/// for unclosed lists on many lines, as many as before the look-ahead); one
/// with no bound grew with the lists that end in a brace or a second name,
/// from 259 to 2,065 bytes per byte.
const MAX_GROWTH: f64 = 1.5;

/// Names in a list longer than the look-ahead's bound of 16 names after the
/// first.
const PAST_THE_BOUND: usize = 20;

fn parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&dekobon_tree_sitter_groovy::LANGUAGE.into())
        .expect("load Groovy grammar");
    parser
}

/// Parses `source` and returns the bytes served to the parser per byte of
/// input.
fn bytes_served_per_input_byte(source: &str) -> f64 {
    let bytes = source.as_bytes();
    let mut parser = parser();
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

/// `count` names, `a0` to `a{count - 1}`, separated by `, `.
fn names(count: usize) -> String {
    (0..count)
        .map(|i| format!("a{i}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn a_long_multi_assignment_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("({}) = l\n", names(n)));
}

#[test]
fn a_long_untyped_parameter_list_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("def f({}) {{ }}\n", names(n)));
}

#[test]
fn many_untyped_method_heads_are_read_a_bounded_number_of_times() {
    assert_linear(|n| {
        let mut source = String::new();
        for i in 0..n {
            writeln!(source, "def f{i}(a, b, c) {{ }}").expect("write to a String");
        }
        source
    });
}

#[test]
fn nested_name_lists_are_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("x = {}b{}\n", "(a, ".repeat(n), ")".repeat(n)));
}

#[test]
fn unclosed_name_lists_on_many_lines_are_read_a_bounded_number_of_times() {
    assert_linear(|n| "def f\n(a, b,\n".repeat(n));
}

#[test]
fn an_unclosed_name_list_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("({}\n", names(n)));
}

#[test]
fn a_broken_list_in_a_block_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("def m() {{ ({}, 1) }}\n", names(n)));
}

#[test]
fn a_list_that_a_brace_follows_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("({}) {{ }}\n", names(n)));
}

#[test]
fn a_list_that_a_second_name_ends_is_read_a_bounded_number_of_times() {
    assert_linear(|n| format!("def m() {{ ({} x) }}\n", names(n)));
}

/// Parse steps of `source` that have more than one stack version.
fn steps_with_more_than_one_version(source: &str) -> usize {
    /// The prefix of the parser's log line for each step of a stack version.
    const STEP: &str = "process version:";
    /// The field of that line that counts the stack versions.
    const VERSIONS: &str = "version_count:";

    let steps = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&steps);
    // Steps whose version count the log gave: none means the log's format
    // changed, and every count of forked steps would read as zero.
    let logged = Arc::new(AtomicUsize::new(0));
    let read = Arc::clone(&logged);
    let mut parser = parser();
    parser.set_logger(Some(Box::new(move |log_type, message| {
        if log_type != LogType::Parse || !message.starts_with(STEP) {
            return;
        }
        let Some(versions) = message
            .split(VERSIONS)
            .nth(1)
            .and_then(|rest| rest.split(',').next())
            .and_then(|count| count.trim().parse::<usize>().ok())
        else {
            return;
        };
        read.fetch_add(1, Ordering::Relaxed);
        if versions > 1 {
            counted.fetch_add(1, Ordering::Relaxed);
        }
    })));
    let tree = parser.parse(source, None).expect("parse");
    assert!(!tree.root_node().has_error(), "{source:?}");
    parser.set_logger(None);
    assert!(
        logged.load(Ordering::Relaxed) > 0,
        "no `{STEP} … {VERSIONS}` line in the parser's log for {source:?}",
    );
    steps.load(Ordering::Relaxed)
}

/// The reading of a script method's head as `def f` and a multi-assignment
/// ends at the first `,`, so a head forks for as many steps with eight
/// untyped parameters as with two, whatever the names' characters and the
/// whitespace between them, and so does a head whose parameters after the
/// first have a type, a default value or an annotation. Without the
/// look-ahead the reading lived to the `{`, or to the first token a list of
/// names does not take, and forked two steps more per parameter.
#[test]
fn a_method_head_forks_no_longer_with_more_untyped_parameters() {
    let two = steps_with_more_than_one_version("def f(a, b) { }\n");
    let heads = [
        "def f(a, b, c, d, e, g, h, k) { }\n",
        "def f(a, $b, _c, \u{e9}) { }\n",
        "def f(a,\u{c}b,\u{b}c, d)\u{c}{ }\n",
        "def f(a, b /* c */, // d\n  c)\n{ }\n",
        "def f(a, int b) { }\n",
        "def f(a, b = 1) { }\n",
        "def f(a, @A b) { }\n",
        "def f(a, java.util.List<T> b) { }\n",
        "def f(a, String[] b) { }\n",
    ];
    for head in heads {
        assert_eq!(
            steps_with_more_than_one_version(head),
            two,
            "{head:?} forked longer than a head with two parameters",
        );
    }
    let two = steps_with_more_than_one_version("final f(a, b) throws E { }\n");
    let eight = steps_with_more_than_one_version("final f(a, b, c, d, e, g, h, k) throws E { }\n");
    assert_eq!(
        eight, two,
        "a final head with a throws clause forked longer"
    );
}

/// The first named child of `node` of kind `kind`.
fn child_of_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == kind)
}

fn parse(source: &str) -> Tree {
    let tree = parser().parse(source, None).expect("parse");
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp(),
    );
    tree
}

#[test]
fn a_multi_assignment_longer_than_the_bound_is_one() {
    let tree = parse(&format!("({}) = l\n", names(PAST_THE_BOUND)));
    let statement =
        child_of_kind(tree.root_node(), "expression_statement").expect("an expression statement");
    let assignment =
        child_of_kind(statement, "multi_assignment_expression").expect("a multi-assignment");
    let mut cursor = assignment.walk();
    assert_eq!(
        assignment
            .children_by_field_name("name", &mut cursor)
            .count(),
        PAST_THE_BOUND,
    );
}

#[test]
fn a_method_head_longer_than_the_bound_is_one() {
    let tree = parse(&format!("def f({}) {{ }}\n", names(PAST_THE_BOUND)));
    let method =
        child_of_kind(tree.root_node(), "method_declaration").expect("a method declaration");
    let parameters = method
        .child_by_field_name("parameters")
        .expect("its parameters");
    assert_eq!(parameters.named_child_count(), PAST_THE_BOUND);
}

/// Names in the declaration of the test below.
const DECLARED_NAMES: usize = 200;

/// The names of a declaration, `def (a0, a1, …) = l`, are one rule over a
/// `repeat` (`_multi_assignment_list` in `grammar.js`), and the runtime
/// balances the nodes of a `repeat`. As a left-recursive rule, which the
/// list was for a few commits, the names nested a node each, and
/// `Node::child`, `Node::next_sibling` and `Tree::changed_ranges` took time
/// quadratic in the names (2 s to walk the siblings of 8,000 names,
/// a hundred times what the `repeat` takes). Time is not measured:
/// the parser's log shows the shape, one reduction of the rule and those
/// of its `repeat`.
#[test]
fn the_names_of_a_declaration_are_one_rule_over_a_repeat() {
    /// The log line of a reduction of the rule; the `,` ends its name.
    const LIST: &str = "reduce sym:_multi_assignment_list,";
    /// The log line of a reduction of the rule's `repeat`.
    const REPEAT: &str = "reduce sym:_multi_assignment_list_repeat1,";

    let lists = Arc::new(AtomicUsize::new(0));
    let repeats = Arc::new(AtomicUsize::new(0));
    let (list_seen, repeat_seen) = (Arc::clone(&lists), Arc::clone(&repeats));
    let mut parser = parser();
    parser.set_logger(Some(Box::new(move |log_type, message| {
        if log_type != LogType::Parse {
            return;
        }
        if message.starts_with(LIST) {
            list_seen.fetch_add(1, Ordering::Relaxed);
        } else if message.starts_with(REPEAT) {
            repeat_seen.fetch_add(1, Ordering::Relaxed);
        }
    })));
    let source = format!("def ({}) = l\n", names(DECLARED_NAMES));
    let tree = parser.parse(&source, None).expect("parse");
    parser.set_logger(None);
    assert!(!tree.root_node().has_error(), "{source:?}");
    assert!(
        repeats.load(Ordering::Relaxed) >= DECLARED_NAMES - 1,
        "{} `{REPEAT}` lines in the parser's log for {DECLARED_NAMES} names",
        repeats.load(Ordering::Relaxed),
    );
    assert_eq!(
        lists.load(Ordering::Relaxed),
        1,
        "`{LIST}` lines in the parser's log: the rule nests a node for each name again",
    );
}
