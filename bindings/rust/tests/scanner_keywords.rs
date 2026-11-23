//! Consistency test for the scanner's list of grammar keywords
//! (`SPECIFICATION.md` §6.14).
//!
//! The parser takes only a name after `_command_statement_start`, and
//! the in-grammar lexer reads any keyword there as one, so the scanner
//! turns down every keyword of the grammar before a statement's first
//! name (`is_grammar_keyword` in `src/scanner.c`): `return new Foo()`
//! must stay a `return`. The scanner spells the keywords out by hand.
//! This test fails when they and the grammar's drift apart.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// Reads `<repo-root>/<path>`.
fn read_repo_file(path: &str) -> String {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&file)
        .unwrap_or_else(|error| panic!("read {}: {error}", file.display()))
}

/// Whether `text` is spelled as a name, which the scanner reads as one
/// word. `non-sealed` is not: the scanner stops at the `-`.
fn is_word(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// The string literals of the generated grammar that are spelled as
/// names. A keyword is lexed as such wherever it has a parse action.
fn grammar_words(json: &str) -> BTreeSet<String> {
    const STRING_RULE: &str = "\"type\": \"STRING\",";
    const VALUE_KEY: &str = "\"value\": \"";
    let mut words = BTreeSet::new();
    let mut rest = json;
    while let Some(at) = rest.find(STRING_RULE) {
        rest = &rest[at + STRING_RULE.len()..];
        let start = rest
            .find(VALUE_KEY)
            .expect("a value after each STRING rule")
            + VALUE_KEY.len();
        let value = &rest[start..];
        let end = value.find('"').expect("the value's closing quote");
        if is_word(&value[..end]) {
            words.insert(value[..end].to_owned());
        }
    }
    words
}

/// The string literals in the body of the scanner's `is_grammar_keyword`.
fn scanner_words(source: &str) -> BTreeSet<String> {
    const SIGNATURE: &str = "static bool is_grammar_keyword(";
    let start = source
        .find(SIGNATURE)
        .expect("is_grammar_keyword in the scanner");
    let function = &source[start..];
    let end = function
        .find("\n}\n")
        .expect("the end of is_grammar_keyword");
    function[..end]
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

#[test]
fn scanner_turns_down_every_grammar_keyword() {
    let grammar = grammar_words(&read_repo_file("src/grammar.json"));
    let scanner = scanner_words(&read_repo_file("src/scanner.c"));
    assert!(!grammar.is_empty(), "no keywords found in src/grammar.json");
    let missing: Vec<_> = grammar.difference(&scanner).collect();
    let extra: Vec<_> = scanner.difference(&grammar).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "keywords of src/grammar.json missing from is_grammar_keyword in \
         src/scanner.c: {missing:?}; words there that are no keyword: {extra:?}",
    );
}
