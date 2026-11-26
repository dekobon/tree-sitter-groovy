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
//!
//! A line break after `return`, `break` or `continue` ends the statement
//! (§6.23), so edits between the keyword and the next token must make the
//! statement parse again.
//!
//! A statement's call with a closure is the object of a member access that
//! follows it (§6.24) by what the scanner reads from the `{` to the first
//! argument after the accesses, so edits there must lex the start of the
//! statement again.

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
/// a line that goes on with the name, and a `return` on the line before one
/// after a statement that forks (`(a) { }`), whose start token depends on
/// what follows the name on its line and the next, so an edit there must lex
/// the type again. The
/// one after it holds a typed declaration with an initializer, which the
/// token opens too, and a `(` two lines below it, which opens a statement or
/// a call's arguments (§6.18). The two after it hold blocks that open
/// statements, with a call, a comment and an operator after their `}`, on its
/// line and the next, in a file, a method, a label and a case (§6.22). The
/// one after it holds typed declarations after `def`, a local, one with a
/// line break after `def` and a field, whose start token after `def` depends
/// on the names after the type (§6.21), so an edit there must lex the type
/// again. The last holds `return`, `break` and `continue` before a line
/// break, a comment and a name, where a line break after the keyword ends
/// the statement (§6.23), so an edit there must parse the statement again.
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
    "while (c) {\n  if (a) continue\n  f(x)\n  break l\n  break // c\n  y\n}\nreturn\nfoo()\n",
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

#[test]
fn reparse_after_slashy_comment_edits_matches_parse_from_scratch() {
    assert_every_edit_reparses(
        &[
            "x = a\n/* c */ /re/\n",
            "x = a\n/* one */ /* two */ /re/\n",
            "x = a /* c */ /re/ d\n",
            "x = a\n/* c */ b /c/ d\n",
            "foo(a\n/*c*/ /2)\nx = b/3 + c/4\n",
            "foo(a\n/*c*/ /2) /3 + b/4\n",
        ],
        &["", "\n", "/", "x"],
    );
}

/// Texts whose look-aheads tell a slashy string from a division by the token
/// before the `/` (§6.2): a lambda's default value (§6.17), the parentheses
/// after a closure call's access (§6.24), a `(` that opens a line (§6.18)
/// and a cast whose operand is on the next line (§6.11). An edit that turns
/// `++` into `+`, or `in` into a name, must lex the start of the look-ahead
/// again.
#[test]
fn reparse_after_slash_operand_edits_matches_parse_from_scratch() {
    assert_every_edit_reparses(
        &[
            "def f = (a = x++ / 2) -> a\n",
            "def g = (a = x in /x,y/) -> a\n",
            "foo { }.bar(/x/).baz()\n",
            "foo { x !in /}/ }.bar()\n",
            "foo\n(x in /a,b/)\n",
            "foo\n(x++ / 2, y)\n",
            "def s = foo((Foo)\n  (Bar) y++ / 2)\n",
        ],
        &["", "/", "+", "x", "\n", ")"],
    );
}

/// Closure parameters that the scanner reads to their `->` past an
/// annotation or `var` at the end of a line (§6.14, §6.21), a typed local
/// before a generic method or a call in parentheses on the next line
/// (§6.21), and `var` before parentheses in an enum constant's arguments
/// (§6.16): an edit in what the look-ahead read must lex its start again.
#[test]
fn reparse_after_parameter_line_edits_matches_parse_from_scratch() {
    assert_every_edit_reparses(
        &[
            "def c = { @A String a = b -> c }\n",
            "def c = {\n  String a = 1,\n  @A\n  String b -> a\n}\n",
            "def c = { String a = 1, var\n  b -> a }\n",
            "String s\n<T> T m(T t) { t }\n",
            "String s\n(Foo new Bar())\n",
            "enum E { A(var(1)) }\n",
        ],
        &["", "\n", "x", ",", "@", "-"],
    );
}

/// A `)` before `in` and a `[` closes no cast where the brackets hold
/// several elements (§6.11), which the scanner reads up to the first `,`,
/// so an edit up to there must lex the `)` again.
#[test]
fn reparse_after_membership_list_edits_matches_parse_from_scratch() {
    assert_every_edit_reparses(
        &[
            "r = (x) in [1, 2]\n",
            "r = (float) in[i]\n",
            "r = (x) in [f(a, b), c]\n",
        ],
        &["", ",", "x", "\n", "]", "["],
    );
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

/// Texts with `break`, `continue` or `return` that a `;` and then a comment
/// follow on the line (§6.23), each with the `;` removed: the reparse reuses
/// the statement, lexed without its token, which must end where a parse from
/// scratch ends it with the token, at the keyword before the space, and the
/// comment, which ends the statement as a line break does, must stay outside
/// it. The token used to follow the space, or the comment. The last three
/// hold a block comment that spaces follow before the line break, `/**/`,
/// and `/**/` that a line break comes to follow, which the statement held
/// in one of the two parses.
const JUMP_STATEMENT_EDIT_SEQUENCES: &[(&str, &[Edit])] = &[
    (
        "for (;;) {\n  if (a) break ;// c\n  (x) { }\n}\n",
        &[(26, 1, "")],
    ),
    (
        "for (;;) {\n  (x) { }\n  continue ;/* d */\n  foo()\n}\n",
        &[(32, 1, "")],
    ),
    (
        "def f() {\n  (x) { }\n  return ;// c\n  x\n}\n",
        &[(29, 1, "")],
    ),
    (
        "def f() {\n  (x) { }\n  return ;/* c */  \n  x\n}\n",
        &[(29, 1, "")],
    ),
    (
        "for (;;) {\n  (x) { }\n  break ;/**/\n  foo()\n}\n",
        &[(29, 1, "")],
    ),
    (
        "switch (x) {\n  case 1: (y) { }\n  return /**/  case 2: (y) { }\n}\n",
        &[(44, 0, "\n")],
    ),
];

#[test]
fn reparse_after_jump_statement_edits_matches_parse_from_scratch() {
    let mut parser = parser();
    for &(text, edits) in JUMP_STATEMENT_EDIT_SEQUENCES {
        let (edited, incremental, fresh) = reparse_sequence(&mut parser, text, edits);
        assert_eq!(
            dumped(&incremental),
            dumped(&fresh),
            "after the edits of {text:?}: {edited:?}",
        );
    }
}

/// A block comment after `break` or `return` that only spaces or tabs
/// follow before the line break ends the statement before it, as Groovy's
/// lexer reads it as the line break (§6.23); one that a label follows after
/// the spaces stays inside, before the label. The corpus cannot hold these:
/// `.editorconfig` trims trailing whitespace.
#[test]
fn a_block_comment_that_spaces_follow_ends_a_jump_statement_before_it() {
    let mut parser = parser();
    for (text, expected) in [
        (
            "while (a) {\n  break /* c */  \n  x\n}\n",
            "(source_file (while_statement condition: (identifier) body: (block \
             (break_statement) (block_comment) (expression_statement (identifier)))))",
        ),
        (
            "def f() {\n  return /**/\t\n  x\n}\n",
            "(source_file (method_declaration name: (identifier) parameters: \
             (formal_parameters) body: (block (return_statement) (groovydoc_comment) \
             (expression_statement (identifier)))))",
        ),
        (
            "l: while (a) {\n  break /* c */  l\n}\n",
            "(source_file (labeled_statement label: (identifier) statement: \
             (while_statement condition: (identifier) body: (block (break_statement \
             (block_comment) label: (identifier))))))",
        ),
    ] {
        let tree = parser.parse(text, None).expect("parse");
        assert_eq!(tree.root_node().to_sexp(), expected, "{text:?}");
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

/// Texts with `in` or `as` on the line after a statement and no operand
/// after the word, so that the word is a name (§6.16). From scratch the
/// scanner reads the word in the state after the expression before it, where
/// the operator is valid; a reparse that reuses that statement reads it in
/// the state after the statement, where it is not. Both must decide the
/// tokens before a statement's first name alike (§6.14, §6.21, §6.24):
/// `as { }.bar 1` after `x = 1` was an `ERROR` from scratch and a call after
/// an edit further on.
#[test]
fn reparse_after_an_operator_word_that_is_a_name_matches_parse_from_scratch() {
    assert_every_edit_reparses(
        &[
            "x = 1\nas { }.bar 1\n",
            "foo()\nas { y }.bar()\n",
            "x = 1\nas \"a\" + b\n",
            "x = 1\nas 1 + 2\n",
            "x = 1\nas (foo 1)\n",
            "x = 1\nas[] q\n",
            "x = 1\nas<T> q\n",
            "x = 1\nas.Bar q\n",
            "x = 1\nin.Bar q\n",
            "x = 1\nin<T> q\n",
            "x = 1\nas.Foo[].class\n",
        ],
        &["", "2", "y", " ", "\n", "."],
    );
}

/// A multi-assignment declaration while it is typed: without its `=` it is
/// an error production whose value is missing, and the statements after it
/// stay statements; with a token that starts no statement in place of the
/// `=`, recovery wraps the names in an `ERROR`. Typing the `=` and the
/// value, or deleting them, must give the tree of a parse from scratch.
///
/// The third text holds a word of ten characters before the `=`. Skipping
/// it costs what the missing value does, and a reparse kept the other
/// recovery until the error production got its weight
/// (`MISSING_VALUE_WEIGHT` in `grammar.js`).
#[test]
fn reparse_after_multi_assignment_declaration_edits_matches_parse_from_scratch() {
    assert_every_edit_reparses(
        &[
            "def (a, b)\nfoo()\n",
            "def (a, b) = l\nfoo()\n",
            "x = 1\ndef (a, b) abcdefghij = l\ndef s = 1\n",
            "def (a, b) : l\nfoo()\n",
            "def (a, b) =,\ndef c = 1\n",
            "def (a, b),\ndef c = 1\n",
            "def (a,\ndef c = 1\nfoo()\n",
            "def (k: a, b) =\nfoo()\n",
            "class C {\n  private (e, f)\n  void m() { }\n}\n",
        ],
        &["", "=", " ", "l", "\n", ",", ":", " instanceof "],
    );
}

/// A known limitation (`docs/divergences-from-spec.md` §29), through the
/// runtime. In `def (a, b) abcdefghij = f g`, skipping the word of ten
/// characters costs what the missing value of the error production does,
/// and both recoveries are alive until the statement after the line. A
/// reparse reuses that statement as one node in the first of them, and
/// while two stack versions are alive the runtime does not break a reused
/// node down when the token after it has changed. So ` instanceof ` typed
/// after `def s = 1` goes on with the value from scratch and is a statement
/// of its own on reparse. This test pins the difference. If it fails, the
/// two parses agree again: remove the limitation from §29 and move the text
/// to the test above.
#[test]
fn a_word_typed_after_the_statement_after_a_tied_multi_assignment_is_a_statement_on_reparse() {
    const TEXT: &str = "def (a, b) abcdefghij = f g\ndef s = 1\n";
    const INSERTION: &str = " instanceof ";

    let mut parser = parser();
    let (edited, incremental, fresh) =
        reparse(&mut parser, TEXT, TEXT.len(), TEXT.len(), INSERTION);
    assert_eq!(
        edited,
        "def (a, b) abcdefghij = f g\ndef s = 1\n instanceof "
    );
    let fresh = fresh.root_node().to_sexp();
    let incremental = incremental.root_node().to_sexp();
    assert!(fresh.contains("instanceof_expression"), "{fresh}");
    assert!(
        !incremental.contains("instanceof_expression"),
        "the reparse now matches the parse from scratch: {incremental}",
    );
}

/// A known limitation (`docs/divergences-from-spec.md` §29), through the
/// runtime. The names of a declaration are one node
/// (`_multi_assignment_list` in `grammar.js`), which a reparse reuses as a
/// whole, also when it holds an `ERROR` and the parser recovers from an
/// error typed before it. In `def(a,l(a,b)` the `(` after `l` is an `ERROR`
/// in a typed name, `l a`. With `,` typed after `def`, a parse from scratch
/// finds a cheaper recovery, one `ERROR` over the comma and the names up to
/// `l`; the reparse keeps the list and its typed name. As a left-recursive
/// rule the list had no such difference, as the runtime reuses none of its
/// nodes after an error in it, but it nested a node for each name
/// (`multi_assignment_comma_cost.rs`). This test pins the difference. If it
/// fails, the two parses agree again: remove the limitation from §29 and
/// move the text to
/// `reparse_after_multi_assignment_declaration_edits_matches_parse_from_scratch`.
#[test]
fn a_comma_typed_before_a_list_of_names_with_an_error_keeps_the_list_on_reparse() {
    const TEXT: &str = "def(a,l(a,b)";
    /// The end of `def`, where `,` is typed.
    const INSERT_AT: usize = 3;

    let mut parser = parser();
    let (edited, incremental, fresh) = reparse(&mut parser, TEXT, INSERT_AT, INSERT_AT, ",");
    assert_eq!(edited, "def,(a,l(a,b)");
    let fresh = fresh.root_node().to_sexp();
    let incremental = incremental.root_node().to_sexp();
    assert!(!fresh.contains("type_identifier"), "{fresh}");
    assert!(
        incremental.contains("type_identifier"),
        "the reparse now matches the parse from scratch: {incremental}",
    );
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

/// A known limitation (`SPECIFICATION.md` §6.2): the runtime reuses a node
/// only while one stack version exists, and breaks a reused node down for a
/// changed token after it only then. `(x) { }` keeps two alive past the
/// statement after it, so `return x` is reused whole at the fork, and the
/// `/` after it is lexed where a statement may start, which opens a slashy
/// string, where a parse from scratch reads the `/` after the operand as
/// division. `throw x` and `assert x` do the same. This test pins the
/// difference. If it fails, the two parses agree again: remove the
/// limitation from §6.2.
#[test]
fn a_slash_after_a_return_value_after_a_fork_opens_a_string_on_reparse() {
    const TEXT: &str = "def f() {\n  (x) { }\n  return x // c\n  x\n}\n";
    /// The offset between the slashes of `// c`, where `x` is inserted.
    const INSERT_AT: usize = 32;

    let mut parser = parser();
    let (edited, incremental, fresh) = reparse(&mut parser, TEXT, INSERT_AT, INSERT_AT, "x");
    assert_eq!(edited, "def f() {\n  (x) { }\n  return x /x/ c\n  x\n}\n");
    assert!(!incremental.root_node().has_error());
    assert!(!fresh.root_node().has_error());

    let divided = "(return_statement (binary_expression left: (binary_expression";
    let fresh = fresh.root_node().to_sexp();
    let incremental = incremental.root_node().to_sexp();
    assert!(fresh.contains(divided), "{fresh}");
    assert!(
        incremental
            .contains("(return_statement (identifier)) (expression_statement (string_literal"),
        "the reparse now matches the parse from scratch: {incremental}",
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

/// A known limitation (`SPECIFICATION.md` §6.18): the runtime reuses a node,
/// and breaks it down for a changed token after it, only while one stack
/// version exists. In a type body `final String z` before `static class C`
/// is also `final String` and a field `z` named `static` before `class C`,
/// which a weight ranks below. Typing `w` after a last `final String`,
/// which the `}` after it made a field named `String`, reuses that field at
/// a point where one version is left; the reduction after it splits the
/// two readings of the lines before, the one that holds the reused field
/// meets an `ERROR` at `w` and ends, and the other one stays. The fuzz seed
/// 4420640461419634763 found it on the corpus test of nested types after
/// typed fields in a record. This test pins the difference. If it fails,
/// the two parses agree again: remove the limitation from §6.18.
#[test]
fn a_name_typed_after_a_last_field_keeps_the_split_reading_before_it() {
    const TEXT: &str = "class A {\n  final String z\n  static class C { }\n  final String }";
    /// The offset after `final String ` on the last line, where `w` and a
    /// line break are typed.
    const INSERT_AT: usize = 63;

    let mut parser = parser();
    let (edited, incremental, fresh) = reparse(&mut parser, TEXT, INSERT_AT, INSERT_AT, "w\n");
    assert_eq!(
        edited,
        "class A {\n  final String z\n  static class C { }\n  final String w\n}"
    );
    assert!(!incremental.root_node().has_error());
    assert!(!fresh.root_node().has_error());

    // From scratch the first field is typed, `z` its name; the reparse
    // keeps the modifier-only field `final String` before it.
    let typed_first = "(class_body (field_declaration type: (type_identifier)";
    let fresh = fresh.root_node().to_sexp();
    let incremental = incremental.root_node().to_sexp();
    assert!(fresh.contains(typed_first), "{fresh}");
    assert!(
        !incremental.contains(typed_first),
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

/// Texts around a statement that calls a name with a closure and goes on
/// with a member access (§6.24), as an expression or as the receiver of a
/// parenthesis-free call. The scanner decides the token before the name
/// from the line up to the `}`, the trivia, accesses and brackets after it
/// and the first argument after them, so each edit there must lex that
/// token again.
const CLOSURE_CALL_TEXTS: &[&str] = &[
    "foo { }.bar()\n",
    "foo { a }\n.bar()\nx = 1\n",
    "if (c) foo { 'a' }.bar() else baz { }\n",
    "foo { b { } }.c { }.d()\nfoo { }\n",
    "foo { }\nbar { }.x\n",
    "l: foo { \"}\" }?.bar\n",
    "foo {\n}.bar()\n",
    "a { b { c { } }.d() }.e()\n",
    "foo { }.bar 1\nbaz()\n",
    "foo { }\n.bar x: 1\n",
    "foo { return /}/ }.bar(baz 1)\n",
    "foo { }.bar().baz 1\nx()\n",
    "a { }.b(1,\n2) { }.c\n",
];

#[test]
fn reparse_after_closure_call_edits_matches_parse_from_scratch() {
    assert_every_edit_reparses(
        CLOSURE_CALL_TEXTS,
        &[
            "", ".", "}", "{", "\n", " ", "'", "\"", "$", "/", "?", "*", ":", "x", "1", "//", "/*",
            "*/", "(", ")", ",", "..", "?.", "\\",
        ],
    );
}

/// `.bar()` typed after a closure one character at a time and deleted
/// again, then `?.b`; the closure split over two lines and joined, and a `$`
/// and a `/` typed into it and removed; an argument typed after an access
/// and deleted, which turns the call into a receiver and back; and a
/// comment between the closure and an access on the next line.
#[test]
fn reparse_after_closure_call_edit_sequences_matches_parse_from_scratch() {
    let mut parser = parser();
    let sequences: &[(&str, &[Edit])] = &[
        (
            "foo { }\nx = 1\n",
            &[
                (7, 0, "."),
                (8, 0, "b"),
                (9, 0, "("),
                (10, 0, ")"),
                (9, 2, ""),
                (8, 1, ""),
                (7, 1, ""),
                (7, 0, "?"),
                (8, 0, "."),
                (9, 0, "b"),
                (7, 3, ""),
            ],
        ),
        (
            "foo { a }.bar()\nbaz { }\n",
            &[
                (5, 0, "\n"),
                (5, 1, ""),
                (6, 0, "$"),
                (6, 1, ""),
                (6, 0, "/"),
                (6, 1, ""),
                (9, 1, ""),
                (9, 0, "."),
                (0, 0, "x = "),
                (0, 4, ""),
                (3, 1, ""),
                (3, 0, " "),
            ],
        ),
        (
            "foo { }.bar\nbaz()\n",
            &[
                (11, 0, " 1"),
                (11, 2, ""),
                (11, 0, " x"),
                (11, 2, ""),
                (11, 0, " {"),
            ],
        ),
        (
            "foo { }\n.bar()\n",
            &[
                (7, 1, ""),
                (7, 0, "\n"),
                (8, 0, "// c\n"),
                (8, 5, ""),
                (8, 1, ""),
                (8, 0, "."),
            ],
        ),
    ];
    for &(text, edits) in sequences {
        let (edited, incremental, fresh) = reparse_sequence(&mut parser, text, edits);
        assert_eq!(
            dumped(&incremental),
            dumped(&fresh),
            "after the edits of {text:?}: {edited:?}",
        );
    }
}

/// A known limitation (`SPECIFICATION.md` §6.18): the runtime reuses a node
/// only while one stack version exists. `(a) { }` keeps two alive past the
/// statement after it, the cast of the closure and the call of `(a)` with
/// it, so `foo { yield / 2 }.bar()` after it is reused whole at the fork.
/// Deleting the `b` of `(b) { }` on the next line leaves `()`, which opens
/// no statement and goes on with the call before it (§6.18). From scratch
/// `(a) { }` is the cast, as before the edit; the reparse keeps the call
/// reading of it, with no `ERROR`. `x++ / 2` in the closure does the same.
/// This test pins the difference. If it fails, the two parses agree again:
/// remove the limitation from §6.18.
#[test]
fn deleting_a_name_after_a_forked_call_flips_the_cast_before_it() {
    for text in [
        "(a) { }\nfoo { yield / 2 }.bar()\n(b) { }\n",
        "(x) { }\nfoo { x++ / 2 }.bar()\n(b) { }\n",
    ] {
        let at = text.rfind('b').expect("the name on the last line");
        let mut parser = parser();
        let (edited, incremental, fresh) = reparse(&mut parser, text, at, at + 1, "");
        assert!(edited.ends_with("\n() { }\n"), "{edited:?}");
        assert!(!incremental.root_node().has_error());
        assert!(!fresh.root_node().has_error());

        let first = |tree: &Tree| {
            let statement = tree.root_node().named_child(0);
            statement
                .and_then(|s| s.named_child(0))
                .map(|n| n.kind().to_owned())
        };
        assert_eq!(first(&fresh).as_deref(), Some("parenthesized_type_cast"));
        assert_eq!(
            first(&incremental).as_deref(),
            Some("method_invocation"),
            "the reparse now matches the parse from scratch: {edited:?}",
        );
    }
}

#[test]
fn reparse_after_excluded_method_name_edits_matches_parse_from_scratch() {
    const EXCLUDED_NAME_TEXTS: &[&str] = &[
        "@A this(x) { }\n@A int x = 1\nthis(1)\n",
        "private int(x) { }\nprivate int x\nprivate foo(x) { }\n",
        "class C {\n  @A <T> void (x) { }\n  private this(x) { }\n  C(x) { this(x) }\n  private trait(x) { }\n}\n",
        "@A\nthis (x) { }\nprivate <T> goto(x) { }\n@A var(x) { }\n",
        "x = 1\nthis(x)\nint(x)\nprivate sealed(x) { }\n",
    ];
    const EXCLUDED_NAME_INSERTIONS: &[&str] = &[
        "", " ", "\n", "(", "s", "@A ", "private ", "<T> ", "this", "int", ".",
    ];
    assert_every_edit_reparses(EXCLUDED_NAME_TEXTS, EXCLUDED_NAME_INSERTIONS);
}

/// Declarations after annotations alone (`@Field FOO = [a: 1]`, §5.9 in
/// `docs/divergences-from-spec.md`), whose first name takes a value, in a
/// script, a method, a closure and a type body, and `@A x` before a line
/// that goes on with a name, which stays a typed declaration. The reading
/// is decided at the `=` after the name, so an edit there, or one that
/// turns the name into a type or the annotation into a name, must parse
/// the statement again. `= 1` typed inside the first annotation's name is
/// left out: see
/// `a_value_typed_inside_an_annotation_name_recovers_differently_on_reparse`.
#[test]
fn reparse_after_annotated_untyped_declaration_edits_matches_parse_from_scratch() {
    assert_every_edit_reparses(
        &[
            "@Field FOO = [a: 1], BAR\n@A x\ny = 1\n",
            "def m() {\n  @Lazy y = 2\n}\n",
            "def c = { @A x = 1, y -> x }\ndef d = { @A x = 1, y }\n",
            "class S {\n  @Subject svc = new Svc()\n  @A int v\n  // c\n  public S() { }\n}\n",
            "@A _ = 1\n@B _ = 2, y = 3\n",
        ],
        &["", " ", "\n", "=", "@A ", "_", ",", "x", "->"],
    );
}

/// A known limitation, through the runtime's error recovery. Typing `= 1`
/// inside the name of the annotation in `@Field FOO = [a: 1], BAR`
/// (`@F= 1eld FOO …`) leaves an `ERROR` both ways, but not the same one: a
/// parse from scratch puts the annotation in an `ERROR` and reads `1` as a
/// statement, while the reparse keeps a declaration with the annotation,
/// an `ERROR` over `= 1`, and `eld` as the type of `FOO`. Neither tree is
/// valid Groovy. This test pins the difference. If it fails, the two parses
/// agree again: move the edit back into
/// `reparse_after_annotated_untyped_declaration_edits_matches_parse_from_scratch`.
#[test]
fn a_value_typed_inside_an_annotation_name_recovers_differently_on_reparse() {
    const TEXT: &str = "@Field FOO = [a: 1], BAR\n@A x\ny = 1\n";
    let mut parser = parser();
    for at in [2, 3] {
        let (edited, incremental, fresh) = reparse(&mut parser, TEXT, at, at + 1, "= 1");
        assert!(edited.starts_with("@F"), "{edited:?}");
        assert!(fresh.root_node().has_error());
        assert!(incremental.root_node().has_error());
        assert_ne!(
            dumped(&incremental),
            dumped(&fresh),
            "the reparse now matches the parse from scratch: {edited:?}",
        );
    }
}
