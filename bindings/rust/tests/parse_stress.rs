//! Stress-corpus integration test per `SPECIFICATION.md` §8.3 / §8.4.
//!
//! Walks `test/stress/` and asserts the parser produces zero `ERROR`
//! and zero `MISSING` nodes for every file. The `MISSING`-node check
//! is the direct anti-regression for `dekobon/big-code-analysis#246`,
//! which surfaced because the prior grammar inserted a synthetic
//! missing operand into elvis-chain parses.

use std::path::PathBuf;

use tree_sitter::{Node, Parser};

/// Returns the absolute path to `<repo-root>/test/stress`.
fn stress_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("test").join("stress")
}

/// Recursively collects every `*.groovy` file under `dir`.
fn collect_groovy_files(dir: &PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(collect_groovy_files(&path));
            } else if path.extension().and_then(|s| s.to_str()) == Some("groovy") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Walks `node` and the entire subtree, returning the first node it
/// finds whose kind is `ERROR` or whose `is_missing()` flag is set.
fn find_first_problem(node: Node<'_>) -> Option<Node<'_>> {
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_first_problem(child) {
            return Some(found);
        }
    }
    None
}

#[test]
fn parses_stress_corpus_with_no_errors() {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_groovy::LANGUAGE.into())
        .expect("load Groovy grammar");

    let files = collect_groovy_files(&stress_dir());
    assert!(
        !files.is_empty(),
        "expected at least one stress file under {:?}",
        stress_dir()
    );

    let mut failures = Vec::new();
    for file in &files {
        let source = std::fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("read {file:?}: {e}"));
        let tree = parser
            .parse(&source, None)
            .unwrap_or_else(|| panic!("parser returned None for {file:?}"));
        if let Some(problem) = find_first_problem(tree.root_node()) {
            let kind = if problem.is_missing() {
                format!("MISSING {}", problem.kind())
            } else {
                "ERROR".to_string()
            };
            failures.push(format!(
                "{}:{}:{}: {}",
                file.display(),
                problem.start_position().row + 1,
                problem.start_position().column + 1,
                kind,
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "stress corpus has {} parse failure(s):\n{}",
        failures.len(),
        failures.join("\n"),
    );
}
