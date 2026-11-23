//! Consistency test for an assumption the scanner makes about the parse
//! table (`SPECIFICATION.md` §6.17, §6.18).
//!
//! Where no expression ends, the scanner reads the `(` of a line as far as
//! `STATEMENT_PAREN` does after an expression only in its `LAMBDA_START`
//! branch: a statement may start wherever `STATEMENT_PAREN` is valid, and so
//! may a lambda. This test fails when the generated parser has an external
//! scanner state with the first token and not the second, where that `(`
//! would no longer be read.

use std::path::PathBuf;

/// The declaration of the table of valid external tokens in `parser.c`.
const STATES_TABLE: &str = "static const bool ts_external_scanner_states[";

/// Reads `<repo-root>/<path>`.
fn read_repo_file(path: &str) -> String {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&file)
        .unwrap_or_else(|error| panic!("read {}: {error}", file.display()))
}

/// The entries of the table, each its index and the text between its `{`
/// and `}`.
fn external_scanner_states(parser: &str) -> Vec<(&str, &str)> {
    let start = parser
        .find(STATES_TABLE)
        .expect("the external scanner states in src/parser.c");
    let table = &parser[start..];
    let end = table.find("\n};\n").expect("the end of the table");
    table[..end]
        .split("\n  [")
        .skip(1)
        .map(|entry| {
            let (index, body) = entry
                .split_once("] = {")
                .expect("an entry's index and opening brace");
            (
                index,
                body.split_once("\n  }").map_or(body, |(inner, _)| inner),
            )
        })
        .collect()
}

/// Whether `entry` marks the external token `name` valid.
fn is_valid(entry: &str, name: &str) -> bool {
    entry.contains(&format!("[ts_external_token__{name}] = true"))
}

#[test]
fn every_state_with_a_statement_paren_takes_a_lambda_start() {
    let parser = read_repo_file("src/parser.c");
    let states = external_scanner_states(&parser);
    let with_paren: Vec<_> = states
        .iter()
        .filter(|(_, entry)| is_valid(entry, "statement_paren"))
        .collect();
    assert!(!with_paren.is_empty(), "no state with _statement_paren");
    let missing: Vec<_> = with_paren
        .iter()
        .filter(|(_, entry)| !is_valid(entry, "lambda_start"))
        .map(|(index, _)| index)
        .collect();
    assert!(
        missing.is_empty(),
        "external scanner states with _statement_paren and no _lambda_start: {missing:?}",
    );
}
