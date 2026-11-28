//! Cost tests for annotations among modifiers (`SPECIFICATION.md` §6.26).
//!
//! After a modifier in a type body, the `(` of an annotation's arguments
//! has one reading: the `(` of a multi-assignment after modifiers alone is
//! a token of its own (`_multi_assignment_open`), so the parser does not
//! fork over the arguments. Where a statement starts, `final` and the
//! annotations after it may open a declaration or a local, so the
//! arguments of those annotations do fork, a few steps per argument, as
//! the conflict at `final` keeps both readings alive. And the modifiers of a declaration are no node of their
//! own (`_modifiers` is inlined): a reparse reuses them as the leaves they
//! were at v0.6.0, so after an edit every stack version reuses the same
//! nodes and the versions merge where they did. Time is not measured: the
//! tests read the parser's log.

use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tree_sitter::{InputEdit, LogType, Parser, Point, Tree};

/// Elements in the smaller input; the larger one has eight times as many.
const SMALL_ELEMENTS: usize = 400;

/// How many times larger the second input is.
const SCALE: usize = 8;

/// Most the steps with more than one stack version that annotation
/// arguments after a modifier take, whatever their number. Measured: 3,
/// at 400 and at 3,200 arguments, as with the annotation before the
/// modifier. A fork over the arguments took 8 steps per argument.
const ANNOTATION_ARGUMENT_FORKS: usize = 3;

/// Nodes a reparse reuses per member at least: its modifiers and its type.
const REUSED_NODES_PER_MEMBER: usize = 2;

/// Characters of an input that a failed assertion's message shows.
const SOURCE_PREVIEW_CHARS: usize = 120;

fn parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&dekobon_tree_sitter_groovy::LANGUAGE.into())
        .expect("load Groovy grammar");
    parser
}

/// What the parser's log shows of a parse: the steps with more than one
/// stack version, the tokens lexed and the nodes reused.
#[derive(Debug, Default)]
struct Work {
    forked_steps: usize,
    lexed: usize,
    reused: usize,
}

/// Parses `source`, from scratch or after `old`, and counts its work.
fn work(parser: &mut Parser, source: &str, old: Option<&Tree>) -> (Tree, Work) {
    /// The prefix of the parser's log line for each step of a stack version.
    const STEP: &str = "process version:";
    /// The field of that line that counts the stack versions.
    const VERSIONS: &str = "version_count:";

    let forked = Arc::new(AtomicUsize::new(0));
    let lexed = Arc::new(AtomicUsize::new(0));
    let reused = Arc::new(AtomicUsize::new(0));
    // Steps whose version count the log gave: none means the log's format
    // changed, and every count of forked steps would read as zero.
    let logged = Arc::new(AtomicUsize::new(0));
    let (f, l, r, g) = (
        Arc::clone(&forked),
        Arc::clone(&lexed),
        Arc::clone(&reused),
        Arc::clone(&logged),
    );
    parser.set_logger(Some(Box::new(move |log_type, message| {
        match log_type {
            LogType::Lex => {}
            LogType::Parse => {
                if message.starts_with("lexed_lookahead") {
                    l.fetch_add(1, Ordering::Relaxed);
                } else if message.starts_with("reuse_node") {
                    r.fetch_add(1, Ordering::Relaxed);
                } else if message.starts_with(STEP) {
                    let Some(versions) = message
                        .split(VERSIONS)
                        .nth(1)
                        .and_then(|rest| rest.split(',').next())
                        .and_then(|count| count.trim().parse::<usize>().ok())
                    else {
                        return;
                    };
                    g.fetch_add(1, Ordering::Relaxed);
                    if versions > 1 {
                        f.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }
    })));
    let tree = parser.parse(source, old).expect("parse");
    parser.set_logger(None);
    assert!(logged.load(Ordering::Relaxed) > 0, "the log gave no version counts");
    assert!(!tree.root_node().has_error(), "{source:.SOURCE_PREVIEW_CHARS$}");
    let work = Work {
        forked_steps: forked.load(Ordering::Relaxed),
        lexed: lexed.load(Ordering::Relaxed),
        reused: reused.load(Ordering::Relaxed),
    };
    (tree, work)
}

fn forked_steps(source: &str) -> usize {
    work(&mut parser(), source, None).1.forked_steps
}

fn arguments(count: usize) -> String {
    (0..count)
        .map(|i| format!("a{i} = {i}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn annotation_arguments_after_a_modifier_fork_a_bounded_number_of_steps() {
    for count in [SMALL_ELEMENTS, SMALL_ELEMENTS * SCALE] {
        let after = forked_steps(&format!("class C {{ private @Foo({}) int x }}\n", arguments(count)));
        assert!(
            after <= ANNOTATION_ARGUMENT_FORKS,
            "{after} forked steps over {count} arguments after a modifier",
        );
        let mut pairs = String::from("class C { ");
        for i in 0..count {
            write!(pairs, "public @A{i}(v = {i}) static ").expect("write to a String");
        }
        pairs.push_str("int x }\n");
        let pairs = forked_steps(&pairs);
        assert!(
            pairs <= ANNOTATION_ARGUMENT_FORKS,
            "{pairs} forked steps over {count} modifier and annotation pairs",
        );
    }
}

/// After `def`, an annotation's arguments are read once, wherever the
/// declaration is: the forked steps over them do not grow with their
/// number. Measured: 0 for a local and a field, 3 for a method, 5 for a
/// parameter, at 400 and at 3,200 arguments.
#[test]
fn annotation_arguments_after_def_fork_a_bounded_number_of_steps() {
    let forms = |a: &str| {
        [
            format!("def @Foo({a}) x = 1\n"),
            format!("class C {{ def @Foo({a}) x }}\n"),
            format!("def @Foo({a}) m() {{ }}\n"),
            format!("def f(def @Foo({a}) x) {{ }}\n"),
        ]
    };
    let small = forms(&arguments(SMALL_ELEMENTS));
    let large = forms(&arguments(SMALL_ELEMENTS * SCALE));
    for (small, large) in small.iter().zip(&large) {
        let (few, many) = (forked_steps(small), forked_steps(large));
        assert!(
            many <= few && many <= 2 * ANNOTATION_ARGUMENT_FORKS,
            "{few} forked steps over {SMALL_ELEMENTS} arguments after def, {many} over eight times as many: {small:.40}",
        );
    }
}

fn point_at(text: &str, byte: usize) -> Point {
    let before = &text[..byte];
    let row = before.matches('\n').count();
    let column = byte - before.rfind('\n').map_or(0, |newline| newline + 1);
    Point { row, column }
}

/// A class of `count` members, fields and methods in turn, each method
/// with `modifiers` before its return type.
fn mixed_class(count: usize, modifiers: &str) -> String {
    let mut source = String::from("class C {\n");
    for i in 0..count {
        if i % 2 == 0 {
            writeln!(source, "  private int f{i}").expect("write to a String");
        } else {
            writeln!(source, "  {modifiers} String m{i}() {{ }}").expect("write to a String");
        }
    }
    source.push_str("}\n");
    source
}

#[test]
fn a_reparse_in_a_class_of_fields_and_methods_reuses_their_modifiers() {
    for modifiers in ["public", "public @A", "public @A static"] {
        for count in [SMALL_ELEMENTS, SMALL_ELEMENTS * SCALE] {
            let source = mixed_class(count, modifiers);
            // `x` typed into the name of the field in the middle.
            let at = source
                .find(&format!("f{}\n", (count / 2) & !1))
                .expect("a field in the middle")
                + 1;
            let mut edited = source.clone();
            edited.insert(at, 'x');
            let mut parser = parser();
            let (mut tree, _) = work(&mut parser, &source, None);
            tree.edit(&InputEdit {
                start_byte: at,
                old_end_byte: at,
                new_end_byte: at + 1,
                start_position: point_at(&source, at),
                old_end_position: point_at(&source, at),
                new_end_position: point_at(&edited, at + 1),
            });
            let (_, reparse) = work(&mut parser, &edited, Some(&tree));
            let (_, fresh) = work(&mut parser, &edited, None);
            // Each member's modifiers and type, at least, are reused; with
            // the modifiers a node of their own, a method's were not.
            assert!(
                reparse.reused >= REUSED_NODES_PER_MEMBER * count,
                "{modifiers}: {} nodes reused of {count} members",
                reparse.reused,
            );
            assert!(
                reparse.forked_steps <= fresh.forked_steps,
                "{modifiers}: {} forked steps on the reparse, {} from scratch",
                reparse.forked_steps,
                fresh.forked_steps,
            );
            assert!(reparse.lexed <= fresh.lexed);
        }
    }
}
