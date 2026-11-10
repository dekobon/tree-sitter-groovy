// Hand-written external scanner for tree-sitter-groovy.
//
// Handles §6.2 slashy-string-start, §6.6 line/block/groovydoc
// comments, and the §6.7 contextual `record` keyword. The first two
// start with `/`; combining their dispatch keeps the in-grammar lexer
// from partially consuming the leading `/`. The third is a look-ahead
// decision the grammar cannot express: `record` is a keyword only
// when a record header follows it.
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
    RECORD_KEYWORD,
};

static inline void advance(TSLexer *lexer) {
    lexer->advance(lexer, false);
}

static inline bool is_whitespace(int32_t c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r';
}

static inline bool is_inline_space(int32_t c) {
    return c == ' ' || c == '\t';
}

// Mirrors the grammar's `identifier` token, /[A-Za-z_$][A-Za-z0-9_$]*/.
static inline bool is_identifier_start(int32_t c) {
    return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')
        || c == '_' || c == '$';
}

static inline bool is_identifier_char(int32_t c) {
    return is_identifier_start(c) || (c >= '0' && c <= '9');
}

// SPECIFICATION.md §6.7 — `record` is a keyword only at the start of
// a record header: `record`, one or more spaces or tabs, a name,
// optional spaces or tabs, `(`, all on one line. Everywhere else the
// word is an identifier (`record.key()`, `private Record record`,
// `public void record()`), as Apache's `identifier` rule allows. The
// grammar cannot make that choice: with `word: $.identifier` the
// keyword wins wherever it has an action, and once
// record_declaration is a class member that is every member start.
//
// The scanner has already consumed `record` when this is called.
// `mark_end` fixes the token there; the look-ahead that follows is
// read but not kept. No newline is allowed before the name
// (`classDeclaration` in GroovyParser.g4 has no `nls` there), which
// keeps `private Record record` followed by a constructor on the next
// line a field and a constructor. None before `(` either: Apache
// allows it, but `record item` on one line and `(1 + 2)` on the next
// would otherwise read as a header. A command-style call
// `record Point(1, 2)` matches the pattern and misfires; §6.7 lists
// the residuals, all of them errors at v0.4.0 already.
static bool scan_record_header(TSLexer *lexer) {
    if (is_identifier_char(lexer->lookahead)) {
        return false;
    }
    lexer->mark_end(lexer);
    if (!is_inline_space(lexer->lookahead)) {
        return false;
    }
    while (is_inline_space(lexer->lookahead)) {
        advance(lexer);
    }
    if (!is_identifier_start(lexer->lookahead)) {
        return false;
    }
    // EOF-bounded: lookahead is 0 at end of input, which fails both
    // predicates below.
    while (is_identifier_char(lexer->lookahead)) {
        advance(lexer);
    }
    while (is_inline_space(lexer->lookahead)) {
        advance(lexer);
    }
    return lexer->lookahead == '(';
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
    // recovery. §6.7 `record` shares the whitespace skip and is
    // dispatched on `r` before the `/` branches.
    bool wants_comment = valid_symbols[LINE_COMMENT]
        || valid_symbols[BLOCK_COMMENT]
        || valid_symbols[GROOVYDOC_COMMENT];
    bool wants_slashy = valid_symbols[SLASHY_STRING_START];
    bool wants_record = valid_symbols[RECORD_KEYWORD];
    if (!wants_comment && !wants_slashy && !wants_record) {
        return false;
    }

    while (is_whitespace(lexer->lookahead)) {
        lexer->advance(lexer, true);
    }
    if (lexer->lookahead == 'r' && wants_record) {
        // An `r` can never open a comment or a slashy string, so
        // every path out of this branch returns. On `false` the
        // runtime rewinds to the token start and the in-grammar
        // lexer reads the word as an identifier.
        static const char keyword[] = "record";
        for (const char *c = keyword; *c != '\0'; c++) {
            if (lexer->lookahead != *c) {
                return false;
            }
            advance(lexer);
        }
        if (!scan_record_header(lexer)) {
            return false;
        }
        lexer->result_symbol = RECORD_KEYWORD;
        return true;
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
    // Slashy string opening `/` — we've consumed it; the body and
    // closing `/` are matched by the grammar's `slashy_string` rule
    // via regex tokens, so `$identifier` and `${expr}` segments can
    // be exposed as structured children. SPECIFICATION.md §5.4 /
    // §6.2 require a context-sensitive emit. Three pragmatic guards
    // keep `a / b` and similar division contexts from being
    // mis-tokenised as the opening of a slashy:
    //
    //   (1) Reject an empty body (the next char after the opening
    //       `/` is a closing `/`) — `//` is already a line comment
    //       and `/=` is already trapped above, so an empty slashy
    //       is excluded.
    //   (2) Reject a body that starts with whitespace — `a / b`
    //       has a space immediately after the operator `/`, but
    //       every legal slashy regex starts with a non-whitespace
    //       char.
    //   (3) Look ahead through the body to confirm there is a
    //       closing `/` on the same line. Without this, `a / b\nc`
    //       at end of file would emit a slashy opener and leave
    //       the body unterminated.
    if (!wants_slashy) {
        return false;
    }
    if (lexer->lookahead == '/' || is_whitespace(lexer->lookahead)) {
        return false;
    }
    // Look ahead (without consuming) to confirm a closing `/` exists
    // before a newline or EOF. `lexer->mark_end` records the position
    // where the slashy opener token ends; advancing past it during
    // look-ahead does not extend the token range as long as we do
    // not call `mark_end` again.
    lexer->mark_end(lexer);
    bool found_close = false;
    while (!lexer->eof(lexer)) {
        if (lexer->lookahead == '\\') {
            advance(lexer);
            if (!lexer->eof(lexer)) {
                advance(lexer);
            }
            continue;
        }
        if (lexer->lookahead == '/') {
            found_close = true;
            break;
        }
        if (lexer->lookahead == '\n' || lexer->lookahead == '\r') {
            break;
        }
        advance(lexer);
    }
    if (!found_close) {
        return false;
    }
    lexer->result_symbol = SLASHY_STRING_START;
    return true;
}
