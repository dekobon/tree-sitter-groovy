//! Incremental-reparse test for the external scanner's state
//! (`SPECIFICATION.md` §6.6).
//!
//! The scanner remembers that a `/*` had no `*/` after it, and the
//! runtime stores that in the tree. An edit can make it false (append
//! `*/`) or move where it starts to hold (delete the first `/*`). Each
//! case here parses a text, applies one edit, reparses with the old
//! tree, and requires the result to equal a parse of the edited text
//! from scratch.

use tree_sitter::{InputEdit, Node, Parser, Point, Tree};

/// Texts with zero, one or several unterminated block comments, in
/// the positions the scanner treats differently: after an expression,
/// at a statement start, inside a record or trait header, after
/// `sealed`, and after a comment that does close. The last five hold
/// parenthesis-free calls (§6.14), as values and as statements with a
/// dotted receiver, whose start token depends on the receiver and on
/// what follows it on its line, so an edit there must lex it again.
const TEXTS: &[&str] = &[
    "",
    "a = 1\nb = 2\n",
    "foo a /* x\nfoo b /* y\nfoo c /* z\n",
    "/* a */ x\n/* b\ny /* c\nz\n",
    "record A /* x\nrecord B(int n) { }\n/** doc\nrecord C /* y\n",
    "trait A /* x\ntrait B { }\nclass C {\n  private String trait\n  trait D /* y\n}\n",
    "sealed /* x\nsealed class S { }\nclass C {\n  private boolean sealed\n  sealed /* y\n}\n",
    "class C {\n  /* one */\n  int a /* two\n  int b /* three\n}\n",
    "def s = \"/* in a string\"\nfoo /* x\ndef t = /a*/\n",
    "x /* a\r\ny /* b\r\n",
    "def x = foo 1\ny = bar 'a', 2\nz.w = baz a: 1\n",
    "def x = foo\n1\nx = foo /* c */ true\n",
    "obj.m 1\nx.y\n'a'\nrecord.add a: 1, 2\n",
    "def r = a?.b 'c'\nx.y a.b 'c'\nfoo\nobj.m /* c */ 1\n",
    "java.util.List 'f'() { }\nobj.m 'a'(b)\n",
];

/// Replacement texts for an edit. An empty one makes it a deletion.
const INSERTIONS: &[&str] = &["", "*/", "/*", " */ ", "/** ", "/", "*", "\n", "x", "."];

/// Byte lengths an edit removes.
const REMOVED_LENGTHS: &[usize] = &[0, 1, 2, 3];

fn parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&dekobon_tree_sitter_groovy::LANGUAGE.into())
        .expect("load Groovy grammar");
    parser
}

/// The row and column of `byte` in `text`, as tree-sitter counts them.
fn point_at(text: &str, byte: usize) -> Point {
    let before = &text[..byte];
    let row = before.matches('\n').count();
    let column = before.rfind('\n').map_or(byte, |nl| byte - nl - 1);
    Point { row, column }
}

/// Every node with its kind and byte range, anonymous ones included.
fn dump(node: Node<'_>, out: &mut Vec<String>) {
    out.push(format!(
        "{} {}..{}{}",
        node.kind(),
        node.start_byte(),
        node.end_byte(),
        if node.is_missing() { " missing" } else { "" },
    ));
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        dump(child, out);
    }
}

fn dumped(tree: &Tree) -> Vec<String> {
    let mut out = Vec::new();
    dump(tree.root_node(), &mut out);
    out
}

/// Parses `text`, replaces `start..old_end` with `insertion`, and
/// returns the edited text, its reparse with the old tree, and its
/// parse from scratch.
fn reparse(
    parser: &mut Parser,
    text: &str,
    start: usize,
    old_end: usize,
    insertion: &str,
) -> (String, Tree, Tree) {
    let mut edited = String::with_capacity(text.len() + insertion.len());
    edited.push_str(&text[..start]);
    edited.push_str(insertion);
    edited.push_str(&text[old_end..]);
    let new_end = start + insertion.len();

    let mut old_tree = parser.parse(text, None).expect("parse original");
    old_tree.edit(&InputEdit {
        start_byte: start,
        old_end_byte: old_end,
        new_end_byte: new_end,
        start_position: point_at(text, start),
        old_end_position: point_at(text, old_end),
        new_end_position: point_at(&edited, new_end),
    });
    let incremental = parser
        .parse(&edited, Some(&old_tree))
        .expect("incremental parse");
    let fresh = parser.parse(&edited, None).expect("fresh parse");
    (edited, incremental, fresh)
}

#[test]
fn reparse_after_edit_matches_parse_from_scratch() {
    let mut parser = parser();
    let mut cases = 0_usize;
    for text in TEXTS {
        for start in 0..=text.len() {
            for &removed in REMOVED_LENGTHS {
                let old_end = start + removed;
                if old_end > text.len() {
                    continue;
                }
                for insertion in INSERTIONS {
                    if removed == 0 && insertion.is_empty() {
                        continue;
                    }
                    let (_, incremental, fresh) =
                        reparse(&mut parser, text, start, old_end, insertion);
                    assert_eq!(
                        dumped(&incremental),
                        dumped(&fresh),
                        "replacing {start}..{old_end} of {text:?} with {insertion:?}",
                    );
                    cases += 1;
                }
            }
        }
    }
    assert!(cases > 0, "no edit was tried");
}

/// A known limitation (`SPECIFICATION.md` §6.6): the marker of an
/// unterminated comment is empty, and where a cast and a
/// parenthesized expression are both still possible at the first
/// unterminated `/*`, the node before it is reused after the comment
/// is closed. The reparse is free of errors but is not the parse from
/// scratch. This test pins the difference. If it fails, the two
/// parses agree again: remove the limitation from §6.6 and from
/// `docs/divergences-from-spec.md`, and move the text to `TEXTS`.
#[test]
fn closing_a_comment_after_a_cast_reuses_the_parenthesized_expression() {
    const TEXT: &str = "x = (a)/* n\nc";
    /// The offset after the line break, where `*/` is inserted.
    const INSERT_AT: usize = 12;

    let mut parser = parser();
    let (edited, incremental, fresh) = reparse(&mut parser, TEXT, INSERT_AT, INSERT_AT, "*/");
    assert_eq!(edited, "x = (a)/* n\n*/c");
    assert!(!incremental.root_node().has_error());
    assert!(!fresh.root_node().has_error());

    let fresh = fresh.root_node().to_sexp();
    let incremental = incremental.root_node().to_sexp();
    assert!(fresh.contains("parenthesized_type_cast"), "{fresh}");
    assert!(
        incremental.contains("parenthesized_expression"),
        "the reparse now matches the parse from scratch: {incremental}",
    );
}
