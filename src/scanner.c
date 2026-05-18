// Hand-written external scanner for tree-sitter-groovy.
//
// Handles §6.2 slashy-string-start and §6.6 line/block/groovydoc
// comments. Both start with `/`; combining the dispatch keeps the
// in-grammar lexer from partially consuming the leading `/`. See
// `docs/divergences-from-spec.md` for §6 tokens not yet wired.
//
// Stateless — `create` returns NULL and the runtime calls
// `destroy` / `serialize` / `deserialize` as no-ops. When a future
// branch needs persistent state, introduce a struct then.

#include "tree_sitter/parser.h"

enum TokenType {
    SLASHY_STRING_START,
    LINE_COMMENT,
    BLOCK_COMMENT,
    GROOVYDOC_COMMENT,
};

static inline void advance(TSLexer *lexer) {
    lexer->advance(lexer, false);
}

void *tree_sitter_groovy_external_scanner_create(void) {
    return NULL;
}

void tree_sitter_groovy_external_scanner_destroy(void *payload) {
    (void)payload;
}

unsigned tree_sitter_groovy_external_scanner_serialize(
    void *payload,
    char *buffer
) {
    (void)payload;
    (void)buffer;
    return 0;
}

void tree_sitter_groovy_external_scanner_deserialize(
    void *payload,
    const char *buffer,
    unsigned length
) {
    (void)payload;
    (void)buffer;
    (void)length;
}

bool tree_sitter_groovy_external_scanner_scan(
    void *payload,
    TSLexer *lexer,
    const bool *valid_symbols
) {
    (void)payload;

    // SPECIFICATION.md §6.6 — line / block / groovydoc comments
    // and §6.2 slashy strings. Both start with `/`; combining their
    // dispatch keeps the in-grammar lexer from partially consuming
    // the leading `/`. Leading whitespace is skipped via
    // advance(true) so the scanner gets a chance to fire after the
    // in-grammar `\s` SKIP rule — without this, tree-sitter's lexer
    // eats `\n`, does NOT re-invoke the external scanner, then
    // fails on the following `/` and consumes it through error
    // recovery.
    bool wants_comment = valid_symbols[LINE_COMMENT]
        || valid_symbols[BLOCK_COMMENT]
        || valid_symbols[GROOVYDOC_COMMENT];
    bool wants_slashy = valid_symbols[SLASHY_STRING_START];
    if (!wants_comment && !wants_slashy) {
        return false;
    }

    while (lexer->lookahead == ' '
           || lexer->lookahead == '\t'
           || lexer->lookahead == '\n'
           || lexer->lookahead == '\r') {
        lexer->advance(lexer, true);
    }
    if (lexer->lookahead != '/') {
        return false;
    }

    advance(lexer);
    // Dispatch on the char after `/`:
    //   `/`  → line comment
    //   `*`  → block / groovydoc comment
    //   `=`  → augmented assign, in-grammar handles it
    //   else → slashy string (when valid_symbols allow)
    int32_t next = lexer->lookahead;
    if (next == '/') {
        if (!valid_symbols[LINE_COMMENT]) {
            return false;
        }
        advance(lexer);
        // EOF-bounded loop: lexer->eof() guarantees termination on
        // unterminated input. Do not tighten without preserving the
        // EOF guard.
        while (!lexer->eof(lexer) && lexer->lookahead != '\n') {
            advance(lexer);
        }
        lexer->result_symbol = LINE_COMMENT;
        return true;
    }
    if (next == '*') {
        if (!wants_comment) {
            return false;
        }
        advance(lexer);
        bool is_groovydoc = false;
        if (lexer->lookahead == '*') {
            advance(lexer);
            is_groovydoc = true;
            if (lexer->lookahead == '/') {
                advance(lexer);
                if (!valid_symbols[GROOVYDOC_COMMENT]) {
                    return false;
                }
                lexer->result_symbol = GROOVYDOC_COMMENT;
                return true;
            }
        }
        // EOF-bounded loop: a `/* …` without a closer terminates
        // here via lexer->eof(), not by going past end-of-input.
        while (!lexer->eof(lexer)) {
            if (lexer->lookahead == '*') {
                advance(lexer);
                if (lexer->lookahead == '/') {
                    advance(lexer);
                    if (is_groovydoc) {
                        if (!valid_symbols[GROOVYDOC_COMMENT]) {
                            return false;
                        }
                        lexer->result_symbol = GROOVYDOC_COMMENT;
                    } else {
                        if (!valid_symbols[BLOCK_COMMENT]) {
                            return false;
                        }
                        lexer->result_symbol = BLOCK_COMMENT;
                    }
                    return true;
                }
            } else {
                advance(lexer);
            }
        }
        return false;
    }
    if (next == '=') {
        // `/=` is the augmented-assign operator. The in-grammar
        // lexer owns it. Returning false here causes tree-sitter
        // to rewind the cursor to before the leading `/`, so the
        // in-grammar lexer can match `/=` cleanly.
        return false;
    }
    // Slashy string body — we've consumed the opening `/`,
    // continue from the body.
    if (!wants_slashy) {
        return false;
    }
    // EOF-bounded loop: unterminated `/…` terminates here via
    // lexer->eof().
    while (!lexer->eof(lexer)) {
        if (lexer->lookahead == '\\') {
            advance(lexer);
            if (!lexer->eof(lexer)) {
                advance(lexer);
            }
            continue;
        }
        if (lexer->lookahead == '/') {
            advance(lexer);
            lexer->result_symbol = SLASHY_STRING_START;
            return true;
        }
        advance(lexer);
    }
    return false;
}
