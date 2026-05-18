from unittest import TestCase

import tree_sitter
import dekobon_tree_sitter_groovy


class TestLanguage(TestCase):
    def test_can_load_grammar(self):
        try:
            tree_sitter.Language(dekobon_tree_sitter_groovy.language())
        except Exception:
            self.fail("Error loading Groovy grammar")
