//! This crate provides Apache Groovy language support for the [tree-sitter][] parsing library.
//!
//! Typically, you will use the [LANGUAGE][LANGUAGE] constant to add this language to a
//! tree-sitter [Parser][], and then use the parser to parse some code:
//!
//! ```
//! let code = r#"
//! def greet(name) {
//!     def msg = name ?: 'World'
//!     println "Hello, ${msg}!"
//! }
//! greet 'Groovy'
//! "#;
//! let mut parser = tree_sitter::Parser::new();
//! let language = dekobon_tree_sitter_groovy::LANGUAGE;
//! parser
//!     .set_language(&language.into())
//!     .expect("Error loading Groovy parser");
//! let tree = parser.parse(code, None).unwrap();
//! assert!(!tree.root_node().has_error());
//! ```
//!
//! [LANGUAGE]: crate::LANGUAGE
//! [Parser]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Parser.html
//! [tree-sitter]: https://tree-sitter.github.io/

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_groovy() -> *const ();
}

/// The tree-sitter [`LanguageFn`] for this grammar.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_groovy) };

/// The content of the [`node-types.json`][] file for this grammar.
///
/// [`node-types.json`]: https://tree-sitter.github.io/tree-sitter/using-parsers#static-node-types
pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

/// The content of the [`highlights.scm`][] query for this grammar.
///
/// [`highlights.scm`]: https://tree-sitter.github.io/tree-sitter/syntax-highlighting#highlights
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/groovy/highlights.scm");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("Error loading Groovy parser");
    }

    /// Returns the text of each node of `source` that `capture` of the
    /// locals query takes, in the order of the query's matches.
    fn locals_captures(source: &str, capture: &str) -> Vec<String> {
        use tree_sitter::StreamingIterator;

        let language = super::LANGUAGE.into();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&language)
            .expect("Error loading Groovy parser");
        let tree = parser.parse(source, None).expect("parse");
        assert!(!tree.root_node().has_error(), "{source:?}");
        let query =
            tree_sitter::Query::new(&language, include_str!("../../queries/groovy/locals.scm"))
                .expect("locals query");
        let index = query
            .capture_index_for_name(capture)
            .expect("a capture of the locals query");
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut texts = Vec::new();
        let mut captures = cursor.captures(&query, tree.root_node(), source.as_bytes());
        while let Some((found, capture_index)) = captures.next() {
            let capture = found.captures()[*capture_index];
            if capture.index == index {
                texts.push(source[capture.node.byte_range()].to_owned());
            }
        }
        texts
    }

    /// The rest binding of a multi-assignment, a `rest_declarator`, defines
    /// its name as the other names do (`queries/groovy/locals.scm`). The
    /// CLI's `tree-sitter test` runs no locals query.
    #[test]
    fn a_rest_binding_defines_its_name() {
        for (source, names) in [
            ("def (a, *b) = l\n", ["a", "b"]),
            ("final (*init, last) = l\n", ["init", "last"]),
            ("def (h, List<Integer> *t) = l\n", ["h", "t"]),
        ] {
            assert_eq!(
                locals_captures(source, "local.definition.var"),
                names,
                "{source:?}"
            );
        }
    }
}
