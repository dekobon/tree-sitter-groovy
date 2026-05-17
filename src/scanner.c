// Hand-written external scanner for tree-sitter-groovy.
//
// Handles the four context-sensitive lexing situations from
// SPECIFICATION.md §6 that pure grammar rules cannot disambiguate:
//
//   1. Slashy-string-start vs. division (§6.2)
//   2. GString interpolation boundaries (§6.3)
//   3. Automatic-semicolon (newline-as-terminator, §6.4)
//   4. Labelled-statement colon (§6.5)
//   5. Block-comment / Groovydoc single-token emit (§6.6)
//
// This file is hand-written and survives `tree-sitter generate`
// unchanged. Symbols are tree_sitter_groovy_external_scanner_*.

#include "tree_sitter/parser.h"
#include <string.h>

enum TokenType {
    AUTOMATIC_SEMICOLON,
    SLASHY_STRING_START,
    SLASHY_STRING_BODY,
    SLASHY_STRING_END,
    DOLLAR_SLASHY_STRING,
    GSTRING_BODY,
    GSTRING_INTERPOLATION_START,
    LABEL_COLON,
    BLOCK_COMMENT,
    GROOVYDOC_COMMENT,
};

typedef struct {
    // Scanner state lives here across calls. Per SPECIFICATION.md §6,
    // we track:
    //   - the kind of the previous non-trivia token (to decide
    //     slashy-vs-division)
    //   - the current paren / bracket / brace depth (to know when
    //     newlines are statement-terminators vs. whitespace)
    //   - a stack of GString contexts (for interpolation brace
    //     balancing)
    //
    // Concrete fields will be added as the rules are wired up.
    uint8_t placeholder;
} Scanner;

void *tree_sitter_groovy_external_scanner_create(void) {
    Scanner *scanner = (Scanner *)calloc(1, sizeof(Scanner));
    return scanner;
}

void tree_sitter_groovy_external_scanner_destroy(void *payload) {
    free(payload);
}

unsigned tree_sitter_groovy_external_scanner_serialize(
    void *payload,
    char *buffer
) {
    Scanner *scanner = (Scanner *)payload;
    if (buffer != NULL) {
        memcpy(buffer, scanner, sizeof(Scanner));
    }
    return sizeof(Scanner);
}

void tree_sitter_groovy_external_scanner_deserialize(
    void *payload,
    const char *buffer,
    unsigned length
) {
    Scanner *scanner = (Scanner *)payload;
    if (length == sizeof(Scanner)) {
        memcpy(scanner, buffer, sizeof(Scanner));
    } else {
        memset(scanner, 0, sizeof(Scanner));
    }
}

bool tree_sitter_groovy_external_scanner_scan(
    void *payload,
    TSLexer *lexer,
    const bool *valid_symbols
) {
    (void)payload;
    (void)lexer;
    (void)valid_symbols;
    // TODO(spec §6.2-§6.6): implement the five tokens. Stub returns
    // false so the grammar falls back to in-grammar rules.
    return false;
}
