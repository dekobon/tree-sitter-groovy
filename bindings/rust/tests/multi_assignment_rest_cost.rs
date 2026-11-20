//! Cost test for the look-ahead that decides whether `(a, …` goes on as a
//! multi-assignment (`SPECIFICATION.md` §6.20).
//!
//! At the `,` after the first name in parentheses where a multi-assignment
//! may start, the scanner reads the other names up to the `)` and the token
//! after it. After `def f` that reading runs beside a method's parameter
//! list, so `def f(a0, a1, …)` is read to its end. The look-ahead runs at
//! the first `,` of a list only, stops at the first `(` and after a fixed
//! number of items: read again at each `,`, a long list would take time
//! quadratic in its length. Time is not measured: the parser reads its
//! input through a callback here, and the test compares the bytes it asks
//! for at two input sizes.

use std::fmt::Write as _;

use tree_sitter::{Node, Parser, Tree};

/// Elements in the smaller input; the larger one has eight times as many.
const SMALL_ELEMENTS: usize = 500;

/// How many times larger the second input is.
const SCALE: usize = 8;

/// Bytes handed to the parser per call of the read callback. Small
/// enough that a look-ahead past a few elements needs another call.
const CHUNK_BYTES: usize = 64;

/// Most the bytes served per input byte may grow from the smaller input
/// to the larger. A bounded look-ahead keeps the two about equal
/// (measured: 3.0 and 2.2 bytes per byte for one long multi-assignment, 4.2
/// and 2.4 for one long parameter list, 1.9 and 2.0 for many heads, 2.3 and
/// 2.3 for nested lists, and 38 and 39 for unclosed lists on many lines, as
/// many as before the look-ahead); one that read the rest of the list again
/// at each `,`, with no budget, grew with it: 259 and 2,065 for the long
/// multi-assignment.
const MAX_GROWTH: f64 = 1.5;

/// Names in a list longer than the look-ahead's budget of 256 items, one
/// for each name after the first.
const PAST_THE_BUDGET: usize = 300;

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
fn a_multi_assignment_longer_than_the_budget_is_one() {
    let tree = parse(&format!("({}) = l\n", names(PAST_THE_BUDGET)));
    let statement =
        child_of_kind(tree.root_node(), "expression_statement").expect("an expression statement");
    let assignment =
        child_of_kind(statement, "multi_assignment_expression").expect("a multi-assignment");
    let mut cursor = assignment.walk();
    assert_eq!(
        assignment
            .children_by_field_name("name", &mut cursor)
            .count(),
        PAST_THE_BUDGET,
    );
}

#[test]
fn a_method_head_longer_than_the_budget_is_one() {
    let tree = parse(&format!("def f({}) {{ }}\n", names(PAST_THE_BUDGET)));
    let method =
        child_of_kind(tree.root_node(), "method_declaration").expect("a method declaration");
    let parameters = method
        .child_by_field_name("parameters")
        .expect("its parameters");
    assert_eq!(parameters.named_child_count(), PAST_THE_BUDGET);
}
