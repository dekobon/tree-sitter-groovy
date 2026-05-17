package tree_sitter_groovy_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_groovy "github.com/dekobon/tree-sitter-groovy/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_groovy.Language())
	if language == nil {
		t.Errorf("Error loading Groovy grammar")
	}
}
