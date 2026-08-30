from unittest import TestCase

import tree_sitter
import dekobon_tree_sitter_groovy


class TestLanguage(TestCase):
    def test_can_load_grammar(self):
        try:
            tree_sitter.Language(dekobon_tree_sitter_groovy.language())
        except Exception:
            self.fail("Error loading Groovy grammar")

    def test_can_parse_groovy(self):
        # Parses rather than only loading, because the GString
        # interpolation below routes through src/scanner.c. A wheel
        # that compiled parser.c without the external scanner still
        # imports cleanly and only fails once something is parsed.
        language = tree_sitter.Language(dekobon_tree_sitter_groovy.language())
        parser = tree_sitter.Parser(language)
        tree = parser.parse(b'def greet(name) { println "hi ${name}" }\n')
        self.assertFalse(tree.root_node.has_error, tree.root_node)
