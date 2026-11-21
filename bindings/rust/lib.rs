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

    /// The `*` of a rest binding is the `rest` field of its declarator
    /// (`SPECIFICATION.md` §4, multi-assignment). The field is on an
    /// anonymous node, which no output of `tree-sitter parse` labels, so the
    /// corpus tests cannot pin it.
    #[test]
    fn a_rest_binding_has_its_star_in_the_rest_field() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("Error loading Groovy parser");
        for (source, rest_index) in [
            ("def (h, List<Integer> *t) = l\n", 1),
            ("final (*init, last) = l\n", 0),
        ] {
            let tree = parser.parse(source, None).expect("parse");
            let declaration = tree.root_node().named_child(0).expect("a statement");
            assert_eq!(
                declaration.kind(),
                "multi_assignment_declaration",
                "{source:?}"
            );
            let mut cursor = declaration.walk();
            let declarators: Vec<_> = declaration
                .named_children(&mut cursor)
                .filter(|child| child.kind() == "variable_declarator")
                .collect();
            assert_eq!(declarators.len(), 2, "{source:?}");
            for (index, declarator) in declarators.iter().enumerate() {
                let rest = declarator.child_by_field_name("rest");
                if index == rest_index {
                    let star = rest.expect("a rest field");
                    assert_eq!(star.kind(), "*", "{source:?}");
                    assert_eq!(
                        declarator
                            .child_by_field_name("name")
                            .map(|n| n.start_byte()),
                        Some(star.end_byte()),
                        "{source:?}",
                    );
                } else {
                    assert_eq!(rest, None, "{source:?}");
                }
            }
        }
    }
}
