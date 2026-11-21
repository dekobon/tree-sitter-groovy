//! Incremental-reparse test for the external scanner's state
//! (`SPECIFICATION.md` §6.6).
//!
//! The scanner remembers that a `/*` had no `*/` after it, and the
//! runtime stores that in the tree. An edit can make it false (append
//! `*/`) or move where it starts to hold (delete the first `/*`). Each
//! case here parses a text, applies one edit, reparses with the old
//! tree, and requires the result to equal a parse of the edited text
//! from scratch.
//!
//! A `(` that opens a line after an expression (§6.18) opens a statement
//! by what the parentheses hold, so edits there must make the statement
//! before it parse again, also after a reparse that reused it.
//!
//! A lambda's brace body ends at its `}` only where no expression goes on
//! after it (§6.19), so edits after the `}` must make the lambda parse
//! again.
//!
//! A typed declaration without an initializer (§6.21) is decided at its
//! type from the names, the list and the line after it, so edits there must
//! make the statement parse again.
//!
//! A `{` that opens a statement is a block where no expression goes on after
//! its `}` (§6.22), so edits after the `}` must make the statement parse
//! again.
//!
//! A `)` before a relational operator that Groovy applies to the
//! parentheses closes no cast (§6.11), which the scanner decides from the
//! word after it, so edits there must lex the `)` again.

use tree_sitter::{InputEdit, Node, Parser, Point, Tree};

/// Texts with zero, one or several unterminated block comments, in
/// the positions the scanner treats differently: after an expression,
/// at a statement start, inside a record or trait header, after
/// `sealed`, and after a comment that does close. Five hold
/// parenthesis-free calls (§6.14), as values and as statements with a
/// dotted receiver, whose start token depends on the receiver and on
/// what follows it on its line, so an edit there must lex it again. Five
/// after those hold a cast whose `)` an unterminated comment follows, where a
/// cast and a parenthesized expression are both possible at the `/*`: the
/// look-ahead for a lambda (§6.17) reads from the `(` through the comment,
/// for a name, a primitive and a dotted type alike, so inserting `*/` makes
/// the runtime lex the `(` again. The one before them holds lambdas, whose
/// start tokens depend on the `->` after the parentheses or the name, also
/// across a line break or a comment. The six after those open lines with
/// `(` after an expression (§6.18), which opens a statement or a call's
/// arguments by the line break before it, by whether a statement may start
/// there and by what the parentheses hold, so inserting or deleting a line
/// break, a `,` or an `=` there must lex the `(` again and parse the line
/// above anew.
///
/// The text after them holds lambdas whose brace bodies a call, a comment
/// and an operator follow (§6.19). The three after it hold multi-assignments
/// without `def` and script methods with untyped parameters, where the
/// first `,` after a name in parentheses is read on to the `)` and the token
/// after it (§6.20), so an edit there must lex the `,` again. The four after
/// it hold typed declarations without an initializer (§6.21), with a list, an
/// annotation, a generic type, closure parameters that share their start,
/// a line that goes on with the name, and a `return` before one after a
/// statement that forks (`(a) { }`), whose start token depends on what
/// follows the name on its line and the next, so an edit there must lex
/// the type again, and the `return` must not end before the token. The
/// one after it holds a typed declaration with an initializer, which the
/// token opens too, and a `(` two lines below it, which opens a statement or
/// a call's arguments (§6.18). The two after it hold blocks that open
/// statements, with a call, a comment and an operator after their `}`, on its
/// line and the next, in a file, a method, a label and a case (§6.22). The
/// last holds typed declarations after `def`, a local, one with a line break
/// after `def` and a field, whose start token after `def` depends on the
/// names after the type (§6.21), so an edit there must lex the type again.
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
    "def f = (a, b) -> a\nfoo(x -> x, (y)\n-> y)\ndef c = { a = (b) -> c }\ng = x /* c */ -> x\n",
    "x = (a)/* n\nc",
    "foo((String)/* why\narg)",
    "x = (int)/* n\nc",
    "x = (a.b.C)/* n\nc",
    "foo((a.B)/* why\narg)",
    "foo\n(1)\nx = y\n(a)\n",
    "bar(foo\n(1))\n[foo\n(1)]\n",
    "foo()\n(a, b) = l\n(c, d) = m\n",
    "while (c) go()\n(a, b) = l\n",
    "foo\n(a, b)\nfoo()\n()\n",
    "println x\n(1..3).each { }\n",
    "def v = a -> { y -> y }.x()\nfoo(b -> { } /* c */, (c) -> { z } + 1)\n",
    "def f(a, b) { }\n(a, b) = l\nx = (c, d) = m\n",
    "def f(a,\n  b) = l\nfinal g(a, /* c */ b) { }\n",
    "def f(a, b, c) throws E { (d, e) = f }\nfoo((a, b) = l)\n",
    "String s\nint i;\nfoo.Bar q\n",
    "String a, b = 1\n{ String c, d -> c }\nfinal String e, f\n",
    "@Field List<String> xs\nInputStream in\nopen()\n",
    "(a) { }\nreturn\nString s\nx = (a)\nString t\n.u()\n",
    "String s = \"a\"\nprintln x\n(a)\n",
    "{ a }.x()\n{ b } /* c */ + 1;\nlbl: { c }\n(1)\n",
    "def m() {\n  { a }\n  { b }\n  .c()\n}\nswitch (v) { case 1: { d } }\n",
    "def String s\ndef\nint a, b = 1\nclass C { def Foo f }\n",
];

/// Replacement texts for an edit. An empty one makes it a deletion.
const INSERTIONS: &[&str] = &[
    "", "*/", "/*", " */ ", "/** ", "/", "*", "\n", "x", ".", "->", "(", ")", ",", "=",
];

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

/// Applies every edit of each text, at every offset, that removes one of
/// `REMOVED_LENGTHS` bytes and inserts one of `insertions`, and requires
/// each reparse to equal the parse of the edited text from scratch.
fn assert_every_edit_reparses(texts: &[&str], insertions: &[&str]) {
    let mut parser = parser();
    let mut cases = 0_usize;
    for text in texts {
        for start in 0..=text.len() {
            for &removed in REMOVED_LENGTHS {
                let old_end = start + removed;
                if old_end > text.len() {
                    continue;
                }
                for insertion in insertions {
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

#[test]
fn reparse_after_edit_matches_parse_from_scratch() {
    assert_every_edit_reparses(TEXTS, INSERTIONS);
}

/// Texts with a `)` before a relational operator (§6.11), which closes a
/// parenthesized expression and no cast, and with names that start like
/// one (`(x) inside`) or that the operator's operand cannot start
/// (`(Foo) in.read()`), where it may close a cast, before a line a method
/// could start and a typed local, with comments before and after the
/// operator. The scanner reads the word after the `)`, and after `as` and
/// `in` the trivia and the first character after them, so an edit there
/// must lex the `)` again. The last text holds casts to generic types whose
/// type arguments close, across a line too, or do not (§3.2.2): the scanner
/// reads them to the `>` to decide the `<`, so an edit there must lex the
/// `<` again.
const RELATIONAL_CLOSE_TEXTS: &[&str] = &[
    "return (x) instanceof Foo\nbar() { }\n",
    "def v = (a.b) in y\n(Foo) in.read()\n",
    "(x) as Foo\n(x) !in y\nout[i] = (float) in[i]\n",
    "(x) !instanceof T\n(x) as\nFoo y = 1\n(x) inside\n",
    "return (x) in /* c */ y\n(x) /* d */ as T\n",
    "x as a.B<c\ny = x as Map<K,\n V>\nz = x as List<T> y\n",
];

/// Replacement texts for an edit of `RELATIONAL_CLOSE_TEXTS`.
const RELATIONAL_CLOSE_INSERTIONS: &[&str] = &["", "x", " ", "!", ".", "(", ")", "\n", "s", "n"];

#[test]
fn reparse_after_relational_close_edits_matches_parse_from_scratch() {
    assert_every_edit_reparses(RELATIONAL_CLOSE_TEXTS, RELATIONAL_CLOSE_INSERTIONS);
}

/// An edit: the offset, the number of bytes removed there, and the text
/// inserted in their place.
type Edit = (usize, usize, &'static str);

/// Texts with a `(` that opens a line after an expression (§6.18), each
/// with a sequence of edits: the first makes the reparse reuse the
/// statement before the `(`, the second turns `=` into `==` and with it
/// the parentheses from a multi-assignment into a call's arguments. The
/// scanner reads the parentheses as far where it can open no statement
/// as where it decides, so the second reparse lexes the `(` again.
const EDIT_SEQUENCES: &[(&str, &[Edit])] = &[
    ("foo\n(c, d) = [3, 4]\n", &[(13, 0, " "), (12, 0, "=")]),
    ("foo()\n(a, b) = l\n", &[(15, 0, " "), (14, 0, "=")]),
    ("println x\n(a, b) = l\n", &[(19, 0, " "), (18, 0, "=")]),
    (
        "while (c) go()\n(a, b) = l\n",
        &[(24, 0, " "), (23, 0, "=")],
    ),
];

#[test]
fn reparse_after_edit_sequence_matches_parse_from_scratch() {
    let mut parser = parser();
    for &(text, edits) in EDIT_SEQUENCES {
        let mut current = text.to_owned();
        let mut tree = parser.parse(&current, None).expect("parse original");
        for &(start, removed, insertion) in edits {
            let old_end = start + removed;
            let mut edited = String::with_capacity(current.len() + insertion.len());
            edited.push_str(&current[..start]);
            edited.push_str(insertion);
            edited.push_str(&current[old_end..]);
            let new_end = start + insertion.len();
            tree.edit(&InputEdit {
                start_byte: start,
                old_end_byte: old_end,
                new_end_byte: new_end,
                start_position: point_at(&current, start),
                old_end_position: point_at(&current, old_end),
                new_end_position: point_at(&edited, new_end),
            });
            tree = parser
                .parse(&edited, Some(&tree))
                .expect("incremental parse");
            let fresh = parser.parse(&edited, None).expect("fresh parse");
            assert_eq!(
                dumped(&tree),
                dumped(&fresh),
                "replacing {start}..{old_end} of {current:?} with {insertion:?}",
            );
            current = edited;
        }
    }
}

/// Texts with a lambda whose body is in braces (§6.19), each with a sequence
/// of edits after the `}`: inserting and removing a call, a subscript, an
/// operator, a closure argument or a line break, and removing a call after
/// a comment one character at a time. Each first edit makes the reparse
/// reuse the lambda's body as the expression it was or as the whole body,
/// and the next one must end the body there again or continue it. The last
/// three rename the parameter or change the next statement, so that the
/// reparse reuses the block of a lambda without parentheses whole, in a
/// state where only the body's end is valid, before a comment and what
/// starts the next statement.
const LAMBDA_BODY_EDIT_SEQUENCES: &[(&str, &[Edit])] = &[
    (
        "def v = a -> { y -> y }\nfoo()\n",
        &[(23, 0, ".call()"), (23, 7, "")],
    ),
    (
        "def v = a -> { y -> y }\nfoo()\n",
        &[(23, 0, "()"), (23, 2, "")],
    ),
    (
        "def v = a -> { y -> y }\nfoo()\n",
        &[(23, 0, "[0]"), (23, 3, "")],
    ),
    (
        "def v = a -> { y -> y }\nfoo()\n",
        &[(23, 0, " + 1"), (23, 4, "")],
    ),
    (
        "def v = a -> { y -> y }\nfoo()\n",
        &[(23, 0, " { z }"), (23, 6, "")],
    ),
    (
        "def v = a -> { foo() }\nbar()\n",
        &[
            (22, 0, " + 1"),
            (22, 4, ""),
            (22, 0, ".call()"),
            (22, 7, ""),
        ],
    ),
    (
        "def v = (a) -> { y -> y }\nbar()\n",
        &[(25, 0, ".call()"), (26, 0, "\n"), (26, 1, ""), (25, 7, "")],
    ),
    ("foo(a -> { y -> y })\n", &[(19, 0, "\n+ 1"), (19, 4, "")]),
    (
        "def v = a -> { y -> y } /* c */ .x()\n",
        &[(35, 1, ""), (34, 1, ""), (33, 1, ""), (32, 1, "")],
    ),
    (
        "def v = a -> { y -> y } // c\n.x()\n",
        &[(32, 1, ""), (31, 1, ""), (30, 1, ""), (29, 1, "")],
    ),
    (
        "def v = a -> { foo() }\n// c\n(1..3).each { }\n",
        &[(8, 1, "ab")],
    ),
    (
        "String v = a -> { foo() } // c\n(1..3).each { }\n",
        &[(35, 1, "4")],
    ),
    ("def v = a -> { foo() }\n/* c */ ++i\n", &[(8, 1, "ab")]),
];

/// Applies `edits` to `text` one after another, reparsing with the tree
/// before each, and returns the last text with its reparse and its parse
/// from scratch. Every reparse but the last must equal the parse from
/// scratch.
fn reparse_sequence(parser: &mut Parser, text: &str, edits: &[Edit]) -> (String, Tree, Tree) {
    let mut current = text.to_owned();
    let mut tree = parser.parse(&current, None).expect("parse original");
    let mut fresh = parser.parse(&current, None).expect("parse original");
    for (step, &(start, removed, insertion)) in edits.iter().enumerate() {
        if step > 0 {
            assert_eq!(
                dumped(&tree),
                dumped(&fresh),
                "before edit {step} of {text:?}: {current:?}",
            );
        }
        let old_end = start + removed;
        let mut edited = String::with_capacity(current.len() + insertion.len());
        edited.push_str(&current[..start]);
        edited.push_str(insertion);
        edited.push_str(&current[old_end..]);
        let new_end = start + insertion.len();
        tree.edit(&InputEdit {
            start_byte: start,
            old_end_byte: old_end,
            new_end_byte: new_end,
            start_position: point_at(&current, start),
            old_end_position: point_at(&current, old_end),
            new_end_position: point_at(&edited, new_end),
        });
        tree = parser
            .parse(&edited, Some(&tree))
            .expect("incremental parse");
        fresh = parser.parse(&edited, None).expect("fresh parse");
        current = edited;
    }
    (current, tree, fresh)
}

#[test]
fn reparse_after_lambda_body_edits_matches_parse_from_scratch() {
    let mut parser = parser();
    for &(text, edits) in LAMBDA_BODY_EDIT_SEQUENCES {
        let (edited, incremental, fresh) = reparse_sequence(&mut parser, text, edits);
        assert_eq!(
            dumped(&incremental),
            dumped(&fresh),
            "after the edits of {text:?}: {edited:?}",
        );
    }
}

/// Texts with a block that opens a statement (§6.22), in a file, after a
/// label, in a case and in a method, each with an operator, a call or a
/// subscript typed after its `}`, on its line or the next, and deleted
/// again: the block turns into a closure and back, so the reparse must
/// decide the `}` again. The first sequence, whose trees both have an
/// `ERROR`, reparses to a tree of its own where the scan in the state after
/// a reused block takes a statement to follow, as it did before the scan
/// handed such a `}` back as after a lambda's block (spec §6.22).
const STATEMENT_BLOCK_EDIT_SEQUENCES: &[(&str, &[Edit])] = &[
    (
        "switch (v) {\ncase 1: { a }\n(1)\n}\ndef w = 1\n",
        &[(9, 1, "."), (33, 0, "!in ")],
    ),
    ("{ y }\nfoo()\n", &[(5, 0, ".call()"), (5, 7, "")]),
    ("{ y }\nfoo()\n", &[(5, 0, "()"), (5, 2, "")]),
    ("{ y }\nfoo()\n", &[(5, 0, " + 1"), (5, 4, "")]),
    ("{ y }\nfoo()\n", &[(5, 0, "\n+ 1"), (5, 4, "")]),
    ("{ y }\nfoo()\n", &[(5, 0, "\n.x()"), (5, 5, "")]),
    ("lbl: { y }\nfoo()\n", &[(10, 0, ".call()"), (10, 7, "")]),
    (
        "switch (v) {\ncase 1: { y }\n}\n",
        &[(26, 0, " as T"), (26, 5, "")],
    ),
    (
        "def m() {\n  { y }\n  foo()\n}\n",
        &[(17, 0, "[0]"), (17, 3, "")],
    ),
];

#[test]
fn reparse_after_statement_block_edits_matches_parse_from_scratch() {
    let mut parser = parser();
    for &(text, edits) in STATEMENT_BLOCK_EDIT_SEQUENCES {
        let (edited, incremental, fresh) = reparse_sequence(&mut parser, text, edits);
        assert_eq!(
            dumped(&incremental),
            dumped(&fresh),
            "after the edits of {text:?}: {edited:?}",
        );
    }
}

/// Texts with a literal first argument of a statement's call (§6.14), each
/// with a sequence of edits: an operator typed after the literal one
/// character at a time and deleted again, the sign that opens the next line
/// turned into `*` and back, a `.` and a closure typed at the start of the
/// next line, and a name argument turned into a string and a number. The
/// scanner decides the token before the receiver from the literal and what
/// follows it, on its line or at the start of the next, so each edit there
/// must lex that token again.
const LITERAL_ARGUMENT_EDIT_SEQUENCES: &[(&str, &[Edit])] = &[
    (
        "println \"a\"\nfoo()\n",
        &[
            (11, 0, " "),
            (12, 0, "+"),
            (13, 0, " "),
            (14, 0, "b"),
            (14, 1, ""),
            (13, 1, ""),
            (12, 1, ""),
            (11, 1, ""),
        ],
    ),
    (
        "timeOutInMs 30\n",
        &[
            (14, 0, " "),
            (15, 0, "*"),
            (16, 0, " "),
            (17, 0, "6"),
            (18, 0, "0"),
        ],
    ),
    ("foo 1\n- x\n", &[(6, 1, "*"), (6, 1, "-")]),
    ("foo 'a'\nbar()\n", &[(8, 0, "."), (8, 1, "")]),
    ("sh 'ls'\nx\n", &[(8, 1, "{ }"), (8, 3, "x")]),
    ("foo x + 1\n", &[(4, 1, "'a'"), (4, 3, "1"), (4, 1, "x")]),
];

#[test]
fn reparse_after_literal_argument_edits_matches_parse_from_scratch() {
    let mut parser = parser();
    for &(text, edits) in LITERAL_ARGUMENT_EDIT_SEQUENCES {
        let (edited, incremental, fresh) = reparse_sequence(&mut parser, text, edits);
        assert_eq!(
            dumped(&incremental),
            dumped(&fresh),
            "after the edits of {text:?}: {edited:?}",
        );
    }
}

/// Texts with the names of a multi-assignment, or a script method's untyped
/// parameters, in parentheses (§6.20), each with a sequence of edits after
/// the first `,`: the method's body turned into `= l` and back, a name
/// added, a typed parameter, and the `=` turned into `==` and `{`. Each
/// edit changes what the look-ahead at the first `,` reads, so the reparse
/// must lex that `,` again. The last four put `= l` after the `)` of a
/// method head that has no body, after `final`, `final def` or an
/// annotation, which fork before the `(` (`foo()` then `final g` then
/// `(a, b)` on the next line): the parentheses the method took are its
/// parameters, and the reparse must read them again as a multi-assignment.
const MULTI_ASSIGNMENT_EDIT_SEQUENCES: &[(&str, &[Edit])] = &[
    (
        "def f(a, b) { }\nfoo()\n",
        &[
            (12, 3, "= l"),
            (12, 3, "{ }"),
            (10, 0, ", c"),
            (15, 3, "= l"),
        ],
    ),
    (
        "(a, b) = l\nfoo()\n",
        &[(7, 0, "="), (7, 2, "{"), (7, 1, "="), (4, 0, "int ")],
    ),
    (
        "def f(a,\n  b)\n= l\n",
        &[(14, 0, "{ }"), (14, 3, ""), (9, 0, "/* c */ ")],
    ),
    ("foo()\nfinal g\n(a, b)\n", &[(21, 0, "= l\n")]),
    ("foo()\nfinal def g\n(a, b)\n", &[(25, 0, "= l\n")]),
    ("@A final g\n(a, b)\n", &[(18, 0, "= l\n")]),
    (
        "list.each {\n  final g\n  (a, b)\n}\n",
        &[(31, 0, "  = l\n"), (31, 6, "")],
    ),
];

#[test]
fn reparse_after_multi_assignment_edits_matches_parse_from_scratch() {
    let mut parser = parser();
    for &(text, edits) in MULTI_ASSIGNMENT_EDIT_SEQUENCES {
        let (edited, incremental, fresh) = reparse_sequence(&mut parser, text, edits);
        assert_eq!(
            dumped(&incremental),
            dumped(&fresh),
            "after the edits of {text:?}: {edited:?}",
        );
    }
}

/// Texts with a typed declaration without an initializer (§6.21), or text
/// that becomes one, each with a sequence of edits: typing `String s` one
/// character at a time, adding and removing an initializer, a second name
/// and a commented initializer, changing the case of the type's first
/// letter, joining and splitting the type and the name, and the name and a
/// line that goes on with it, breaking and mending type arguments, ending
/// the name in a member access, after a parenthesized name that forks,
/// adding `->` to make closure parameters, and editing a `for` header and
/// the declaration after a `return`. Each edit changes what the look-ahead
/// at the type reads, so the reparse must lex the type again.
const DECLARATION_EDIT_SEQUENCES: &[(&str, &[Edit])] = &[
    (
        "foo()\n\nbar()\n",
        &[
            (6, 0, "S"),
            (7, 0, "t"),
            (8, 0, "r"),
            (9, 0, "i"),
            (10, 0, "n"),
            (11, 0, "g"),
            (12, 0, " "),
            (13, 0, "s"),
        ],
    ),
    (
        "String s\nfoo()\n",
        &[
            (8, 0, " = 1"),
            (8, 4, ""),
            (8, 0, ", b"),
            (8, 3, ""),
            (8, 0, " /* c */ = 1"),
            (8, 12, ""),
        ],
    ),
    (
        "string s\nfoo()\n",
        &[(0, 1, "S"), (0, 1, "s"), (0, 1, "S")],
    ),
    ("String\ns\nfoo()\n", &[(6, 1, " "), (6, 1, "\n")]),
    ("String s\n.trim()\n", &[(8, 1, ""), (8, 0, "\n")]),
    (
        "List<String> xs\nfoo()\n",
        &[(4, 1, ""), (4, 0, "<"), (4, 0, " "), (4, 1, "")],
    ),
    ("x = (a)\nString s\n", &[(16, 0, ".y"), (16, 2, "")]),
    ("{ String a, b }\n", &[(13, 0, " ->"), (13, 3, "")]),
    (
        "return\nString s\nfoo()\n",
        &[(15, 0, " = 1"), (15, 4, ""), (7, 1, "s"), (7, 1, "S")],
    ),
    (
        "for (int i; i < n; i++) { }\n",
        &[(10, 0, "\n"), (10, 1, ""), (10, 0, " = 0"), (10, 4, "")],
    ),
];

#[test]
fn reparse_after_declaration_edits_matches_parse_from_scratch() {
    let mut parser = parser();
    for &(text, edits) in DECLARATION_EDIT_SEQUENCES {
        let (edited, incremental, fresh) = reparse_sequence(&mut parser, text, edits);
        assert_eq!(
            dumped(&incremental),
            dumped(&fresh),
            "after the edits of {text:?}: {edited:?}",
        );
    }
}

/// Texts where the scanner decides a token from the text after it in one
/// parser state, while a reparse that reuses the node before it lexes the
/// same characters in another, each with the edits that left the old reading
/// before the scanner read as far in both states. The `<` after the type of
/// `as` or `instanceof` (§3.2.2): with the type arguments still open, a
/// reparse that reuses the type lexes the `<` after an expression, and the
/// `>` typed afterwards must still turn the comparison into type arguments.
/// The name `in` of a `for` header (§6.16): with no collection yet, a
/// reparse reuses the variable as a type (an edit on another line, or the
/// body's braces typed first), and the collection typed afterwards must
/// still make the loop. The `)` of a cast's type before
/// `()` (§6.17): the reading that survives without `->` is a call, and an
/// arrow typed or restored after it must still make the cast of a lambda.
/// The `(` after the block of an `if`, `else` or loop body (§6.17): a parse
/// from scratch lexes it where the braces may be a closure too, a reparse
/// that reuses the statement at a statement's start, and both must read a
/// lambda statement there, typed or edited.
const SECOND_STATE_EDIT_SEQUENCES: &[(&str, &[Edit])] = &[
    ("y = foo as List<String\n", &[(4, 3, "bar"), (22, 0, ">")]),
    ("p as p</**/", &[(0, 0, ""), (11, 0, "r>")]),
    (
        "if (foo instanceof List<String) { }\n",
        &[(6, 1, "x"), (30, 0, ">")],
    ),
    ("a = 1\nfor (x in ) doIt()\n", &[(4, 1, "2"), (16, 0, "ys")]),
    ("for(x in s)", &[(9, 0, ","), (12, 0, "8"), (9, 1, "")]),
    ("for (x in ) {", &[(13, 0, "\n    \n}"), (10, 0, "xs")]),
    ("r = (A) () x\n", &[(11, 1, "-> x")]),
    (
        "\ndef r = (Runnable) () -> { }\n",
        &[(23, 0, "x"), (23, 1, "")],
    ),
    ("if (c) { a() }\n(x) -> x\n", &[(23, 0, " "), (23, 1, "")]),
    (
        "while (c) { a() }\n\n",
        &[
            (18, 0, "("),
            (19, 0, "x"),
            (20, 0, ")"),
            (21, 0, " "),
            (22, 0, "-"),
            (23, 0, ">"),
            (24, 0, " "),
            (25, 0, "x"),
        ],
    ),
    (
        "if (c) { a() } else { b() }\n(x) -> x\n",
        &[(29, 0, "y, "), (29, 3, "")],
    ),
];

#[test]
fn reparse_after_edits_past_a_reused_node_matches_parse_from_scratch() {
    let mut parser = parser();
    for &(text, edits) in SECOND_STATE_EDIT_SEQUENCES {
        let (edited, incremental, fresh) = reparse_sequence(&mut parser, text, edits);
        assert_eq!(
            dumped(&incremental),
            dumped(&fresh),
            "after the edits of {text:?}: {edited:?}",
        );
    }
}

/// Texts with `in` or `as` on the line after a statement (§6.16), whose
/// token depends on what follows the word, each with edits after the word
/// that make it the keyword: `x = a` then `in y = 12` is the assignment
/// `x = ((a in y) = 12)`, and `as l= Cx` makes `l` the import's alias.
/// The reparse must not keep the statement before the word as it was.
const CONTEXTUAL_KEYWORD_EDIT_SEQUENCES: &[(&str, &[Edit])] = &[
    ("x = a\nin = 1\n", &[(8, 4, " y = 12")]),
    ("import a.B\nas = C\n", &[(14, 0, "l"), (18, 0, "x")]),
];

#[test]
fn reparse_after_contextual_keyword_edits_matches_parse_from_scratch() {
    let mut parser = parser();
    for &(text, edits) in CONTEXTUAL_KEYWORD_EDIT_SEQUENCES {
        let (edited, incremental, fresh) = reparse_sequence(&mut parser, text, edits);
        assert_eq!(
            dumped(&incremental),
            dumped(&fresh),
            "after the edits of {text:?}: {edited:?}",
        );
    }
}

/// A known limitation (`SPECIFICATION.md` §6.17): where a method head forks
/// before its `(` (after `final g`), the method's version reads the `(` of
/// its parameters on its own, with none of the look-ahead for a lambda that
/// the other version's `(` reads. When the method has typed parameters, so
/// that the other version ends at their names, the method's nodes are not
/// fragile and a reparse reuses them past an edit after the `)`: `-> a`
/// typed there is a lambda from scratch and an `ERROR` on reparse. This test
/// pins the difference. If it fails, the two parses agree again: remove the
/// limitation from §6.17 and move the text to
/// `MULTI_ASSIGNMENT_EDIT_SEQUENCES`.
#[test]
fn a_lambda_arrow_after_a_final_method_head_is_an_error_on_reparse() {
    const TEXT: &str = "foo()\nfinal g\n(int a, int b)\n";
    /// The end of the text, where `-> a` is typed.
    const INSERT_AT: usize = 29;

    let mut parser = parser();
    let (edited, incremental, fresh) = reparse(&mut parser, TEXT, INSERT_AT, INSERT_AT, "-> a\n");
    assert_eq!(edited, "foo()\nfinal g\n(int a, int b)\n-> a\n");
    assert!(!fresh.root_node().has_error());
    assert!(
        incremental.root_node().has_error(),
        "the reparse now matches the parse from scratch",
    );
}

/// A known limitation (`SPECIFICATION.md` §6.17): where a closure
/// parameter's default and a lambda's body share a parenthesized
/// expression, typing `{` before `T a = (b, c) -> (d) -> e }` reuses the
/// lambda `(d) -> e` of the old tree. The reparse reads a closure whose
/// body declares `T a = (b, c) -> (d) -> e`, where a parse from scratch
/// reads the parameter `T a` with the default `(b, c) -> (d)` and the body
/// `e`, as Groovy 3 does; neither has an `ERROR`. This test pins both
/// trees, so that it fails when either reading changes. If the reparse
/// matches the parse from scratch, remove the limitation from §6.17 and
/// divergences §26 and move the text to `LAMBDA_BODY_EDIT_SEQUENCES`.
#[test]
fn a_brace_typed_before_a_lambda_default_keeps_the_lambda_on_reparse() {
    const TEXT: &str = "def d =  T a = (b, c) -> (d) -> e }\n";
    /// The second space after `=`, where `{` replaces it.
    const BRACE_AT: usize = 8;
    const FRESH: &str = "(source_file (local_variable_declaration (variable_declarator \
        name: (identifier) value: (closure (closure_parameters (closure_parameter \
        type: (type_identifier) name: (identifier) default: (lambda_expression \
        parameters: (formal_parameters (formal_parameter name: (identifier)) \
        (formal_parameter name: (identifier))) body: (parenthesized_expression \
        (identifier))))) (expression_statement (identifier))))))";
    const REPARSED: &str = "(source_file (local_variable_declaration (variable_declarator \
        name: (identifier) value: (closure (local_variable_declaration \
        type: (type_identifier) name: (identifier) value: (lambda_expression \
        parameters: (formal_parameters (formal_parameter name: (identifier)) \
        (formal_parameter name: (identifier))) body: (lambda_expression \
        parameters: (formal_parameters (formal_parameter name: (identifier))) \
        body: (identifier))))))))";

    let mut parser = parser();
    let (edited, incremental, fresh) = reparse(&mut parser, TEXT, BRACE_AT, BRACE_AT + 1, "{");
    assert_eq!(edited, "def d = {T a = (b, c) -> (d) -> e }\n");
    assert_eq!(fresh.root_node().to_sexp(), FRESH);
    assert_eq!(
        incremental.root_node().to_sexp(),
        REPARSED,
        "the reparse changed; if it matches the parse from scratch, the limitation is gone",
    );
}

/// A known limitation (`docs/divergences-from-spec.md` §20): the members
/// before a `default` method fork, a field's name against a type, and the
/// runtime checks a reused node only while one stack version exists
/// (`SPECIFICATION.md` §6.18). Deleting the `lt` of the second `default`
/// reuses the field `static count`, though a name follows it now, and the
/// reparse reads `private String`, a field `y default` and `void a() { }`,
/// where a parse from scratch keeps `private String y` and
/// `default void a() { }`; neither has an `ERROR`, and the next edit of the
/// word heals it. This test pins the difference. If it fails, the two parses
/// agree again: remove the row from §20 and this test.
#[test]
fn editing_a_default_after_a_modifier_only_field_reparses_the_members_before() {
    const TEXT: &str = "class C {\n    private String y\n    default void a() { }\n    \
                        static count\n    default void b() { }\n}\n";
    /// The `lt` of the second `default`.
    const LT: std::ops::Range<usize> = 82..84;

    let mut parser = parser();
    let (edited, incremental, fresh) = reparse(&mut parser, TEXT, LT.start, LT.end, "");
    assert!(edited.contains("\n    defau void b() { }\n"), "{edited:?}");
    assert!(!fresh.root_node().has_error());
    assert!(!incremental.root_node().has_error());
    assert_ne!(
        dumped(&incremental),
        dumped(&fresh),
        "the reparse now matches the parse from scratch",
    );
}

/// A known limitation (`SPECIFICATION.md` §6.19): the scanner reads past a
/// `}` up to the token after it only where a comment follows on the `}`'s
/// line. A closure that the parse continued as an expression past a comment
/// on the next line is reused as that expression, so the reparse takes the
/// comment before it decides that the body ends, and the comment lands in
/// the lambda. This test pins the difference. If it fails, the two parses
/// agree again: remove the limitation from §6.19 and move the text to
/// `LAMBDA_BODY_EDIT_SEQUENCES`.
#[test]
fn a_comment_on_the_line_after_a_lambda_body_moves_into_it_on_reparse() {
    const TEXT: &str = "def v = a -> { y -> y }\n// c\n.x()\nbar()\n";
    /// The `.x()` after the comment, removed from its end.
    const EDITS: &[Edit] = &[(32, 1, ""), (31, 1, ""), (30, 1, ""), (29, 1, "")];

    let mut parser = parser();
    let (edited, incremental, fresh) = reparse_sequence(&mut parser, TEXT, EDITS);
    assert_eq!(edited, "def v = a -> { y -> y }\n// c\n\nbar()\n");
    assert!(!incremental.root_node().has_error());
    assert!(!fresh.root_node().has_error());

    let lambda_end = |tree: &Tree| {
        let declaration = tree.root_node().named_child(0);
        let declarator = declaration.and_then(|d| d.named_child(0));
        let value = declarator.and_then(|d| d.child_by_field_name("value"));
        value.map(|v| v.end_byte())
    };
    assert_eq!(lambda_end(&fresh), Some(23));
    assert_ne!(
        lambda_end(&incremental),
        Some(23),
        "the reparse now matches the parse from scratch",
    );
}

/// The same limitation after a labeled block that opens a statement
/// (`SPECIFICATION.md` §6.22), whose end the scanner decides as a lambda's:
/// the comment on the line after the `}` lands in the labeled statement. If
/// this fails, remove the limitation from §6.22 too.
#[test]
fn a_comment_on_the_line_after_a_labeled_block_moves_into_it_on_reparse() {
    const TEXT: &str = "lbl: { }\n// c\n.x()\nbar()\n";
    /// The `.x()` after the comment, removed from its end.
    const EDITS: &[Edit] = &[(17, 1, ""), (16, 1, ""), (15, 1, ""), (14, 1, "")];

    let mut parser = parser();
    let (edited, incremental, fresh) = reparse_sequence(&mut parser, TEXT, EDITS);
    assert_eq!(edited, "lbl: { }\n// c\n\nbar()\n");
    assert!(!incremental.root_node().has_error());
    assert!(!fresh.root_node().has_error());

    let labeled_end = |tree: &Tree| tree.root_node().named_child(0).map(|n| n.end_byte());
    assert_eq!(labeled_end(&fresh), Some(8));
    assert_ne!(
        labeled_end(&incremental),
        Some(8),
        "the reparse now matches the parse from scratch",
    );
}

/// A known limitation (`SPECIFICATION.md` §6.18): the runtime reuses a
/// node only while one stack version exists. A typed declaration after
/// `final` keeps two alive to its end, so the statement after it is reused
/// at the fork although the `(` on the line after it now makes the
/// parentheses its arguments. The reparse is free of errors but reads the
/// first line as `final String` and `s = "a"`. Without `final` the scanner
/// opens the declaration (§6.21), which then does not fork, and `TEXTS`
/// holds that text. This test pins the difference. If it fails, the two
/// parses agree again: remove the limitation from §6.18 and move the text
/// to `TEXTS`.
#[test]
fn a_comma_after_a_final_typed_declaration_reuses_the_split_statement() {
    const TEXT: &str = "final String s = \"a\"\nprintln x\n(a)\n";
    /// The offset before the `)`, where `,` is inserted.
    const INSERT_AT: usize = 33;

    let mut parser = parser();
    let (edited, incremental, fresh) = reparse(&mut parser, TEXT, INSERT_AT, INSERT_AT, ",");
    assert_eq!(edited, "final String s = \"a\"\nprintln x\n(a,)\n");
    assert!(!incremental.root_node().has_error());
    assert!(!fresh.root_node().has_error());

    // The typed declaration has a `type`; the modifier-only `final String`
    // the reparse reads has none.
    let typed = "(local_variable_declaration type:";
    let fresh = fresh.root_node().to_sexp();
    let incremental = incremental.root_node().to_sexp();
    assert!(fresh.contains(typed), "{fresh}");
    assert!(
        !incremental.contains(typed),
        "the reparse now matches the parse from scratch: {incremental}",
    );
}

/// A known limitation (`SPECIFICATION.md` §6.14): during error recovery the
/// runtime ignores an empty external token that leaves the scanner's state
/// as it was, and a later reparse reuses the leaf after it without calling
/// the scanner there. Typing `def` on the line before
/// `obj.Bar $/$a/$() { }` reads `def obj` and, by error recovery, a method
/// whose return type is `.Bar`; deleting `def` again reuses the `.` after
/// `obj`, so `_command_path_start` is not emitted before it. The reparse
/// reads a method that returns `obj.Bar` and is named by the string, where
/// a parse from scratch reads Groovy 3's call `obj.Bar($/$a/$() { })`;
/// neither has an `ERROR`. This test pins both trees, so that it fails when
/// either reading changes. If the reparse matches the parse from scratch,
/// remove the limitation from §6.14 and divergences §19 and move the text
/// to `EDIT_SEQUENCES`.
#[test]
fn deleting_a_def_before_a_dotted_call_of_a_string_reads_a_method_on_reparse() {
    const TEXT: &str = "\nobj.Bar $/$a/$() { }\n";
    /// `def` typed at the start of the text, then deleted.
    const EDITS: &[Edit] = &[(0, 0, "def"), (0, 3, "")];
    const FRESH: &str = "(source_file (command_chain receiver: (field_access \
        object: (identifier) field: (identifier)) argument: (method_invocation \
        function: (string_literal (gstring_dollar_interpolation value: (identifier))) \
        arguments: (argument_list) closure: (closure))))";
    const REPARSED: &str = "(source_file (method_declaration return_type: \
        (qualified_type (type_identifier) (type_identifier)) name: (quoted_identifier \
        (gstring_dollar_interpolation value: (identifier))) parameters: \
        (formal_parameters) body: (block)))";

    let mut parser = parser();
    let (edited, incremental, fresh) = reparse_sequence(&mut parser, TEXT, EDITS);
    assert_eq!(edited, TEXT);
    assert!(!fresh.root_node().has_error());
    assert!(!incremental.root_node().has_error());
    assert_eq!(fresh.root_node().to_sexp(), FRESH);
    assert_eq!(
        incremental.root_node().to_sexp(),
        REPARSED,
        "the reparse changed; if it matches the parse from scratch, the limitation is gone",
    );
}
