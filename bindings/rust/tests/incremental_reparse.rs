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
/// last holds a typed declaration with an initializer, which the token opens
/// too, and a `(` two lines below it, which opens a statement or a call's
/// arguments (§6.18).
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
