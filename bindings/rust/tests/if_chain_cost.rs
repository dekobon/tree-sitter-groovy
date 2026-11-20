//! Cost test for `else if` chains whose blocks start with the assignment of
//! a lambda (`SPECIFICATION.md` §5.16, `docs/divergences-from-spec.md` §26).
//!
//! The braces of `if (x) { handler = (e) -> log(e) }` also read as a closure
//! with the parameter `handler = (e)`, and a weight ranks the block above
//! it. While the two readings of a consequence were alternatives inline in
//! `if_statement`, they were ranked only where that statement was reduced,
//! which in an `else if` chain is at the chain's end: every link kept both
//! readings on the stack. A chain that ends the input then took time far
//! above linear in its length, and inside a method the runtime's limit on
//! stack versions dropped the block of a link for the closure, from twelve
//! links on. The consequence is now a rule of its own, so the readings merge
//! at its `}`.
//!
//! The first test counts the stack versions in the parser's log, which is
//! deterministic where time is not. The second requires every consequence of
//! a chain in a method to be a block.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tree_sitter::{LogType, Node, Parser};

/// One link of the chain: an `if` whose block starts with the assignment of
/// a lambda, up to the `else` that the next link or the last block follows.
const LINK: &str = "if (x) {\n  handler = (e) -> log(e)\n} else ";

/// The block that ends a chain.
const LAST_BLOCK: &str = "{\n  foo()\n}\n";

/// Links in the shorter chain; the longer one has eight times as many.
const SHORT_CHAIN: usize = 50;

/// How many times longer the second chain is.
const SCALE: usize = 8;

/// Most stack versions the parser may hold at once while it parses a chain.
/// The two readings of one consequence make two until its `}` (measured: 2
/// at both lengths); kept to the chain's end they were 79 for 50 links.
const MAX_VERSIONS: usize = 3;

/// Links of the chain in a method. The block of a link was dropped for the
/// closure from twelve links on; this is well past it.
const METHOD_CHAIN: usize = 40;

/// The key of the version count in the parser's `process version` messages.
const VERSION_COUNT_KEY: &str = "version_count:";

fn parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&dekobon_tree_sitter_groovy::LANGUAGE.into())
        .expect("load Groovy grammar");
    parser
}

/// Parses a chain of `links` links that ends the input and returns the most
/// stack versions the parser's log reports at once.
fn most_stack_versions(links: usize) -> usize {
    let mut parser = parser();
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
    let source = format!("{}{LAST_BLOCK}", LINK.repeat(links));
    let tree = parser.parse(&source, None).expect("parse");
    assert!(!tree.root_node().has_error(), "{links} links");
    let most = most.load(Ordering::Relaxed);
    assert!(most > 0, "no version count in the parser's log");
    most
}

#[test]
fn an_else_if_chain_of_lambda_assignments_keeps_few_stack_versions() {
    for links in [SHORT_CHAIN, SHORT_CHAIN * SCALE] {
        let most = most_stack_versions(links);
        assert!(
            most <= MAX_VERSIONS,
            "{most} stack versions at once for a chain of {links} links",
        );
    }
}

/// Counts the nodes of `kind` under `node`, itself included.
fn count_kind(node: Node<'_>, kind: &str) -> usize {
    let mut cursor = node.walk();
    let below: usize = node
        .children(&mut cursor)
        .map(|child| count_kind(child, kind))
        .sum();
    below + usize::from(node.kind() == kind)
}

#[test]
fn every_consequence_of_an_else_if_chain_in_a_method_is_a_block() {
    let source = format!("def m() {{\n{}{LAST_BLOCK}}}\n", LINK.repeat(METHOD_CHAIN));
    let mut parser = parser();
    let tree = parser.parse(&source, None).expect("parse");
    let root = tree.root_node();
    assert!(!root.has_error(), "{}", root.to_sexp());
    assert_eq!(count_kind(root, "if_statement"), METHOD_CHAIN);
    assert_eq!(count_kind(root, "lambda_expression"), METHOD_CHAIN);
    assert_eq!(count_kind(root, "closure"), 0, "{}", root.to_sexp());
}
