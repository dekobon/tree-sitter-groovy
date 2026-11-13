// Hand-written external scanner for tree-sitter-groovy.
//
// Handles §6.2 slashy-string-start, §6.6 line/block/groovydoc
// comments, and the §6.7 contextual `record` keyword. The first two
// start with `/`; combining their dispatch keeps the in-grammar lexer
// from partially consuming the leading `/`. The third is a look-ahead
// decision the grammar cannot express: `record` is a keyword only
// when a record header follows it. Also handles §6.8
// COMMAND_NAMED_KEY, the key of a named argument in a
// parenthesis-free call, and §6.9 SUBSCRIPT_OPEN, a `[` on the same
// line as the token before it, and §6.11 CAST_CLOSE, the `)` of a
// cast whose operand starts on the same line. STRING_BODY (§6.10) is
// never emitted;
// it is valid only inside a string body, where the scanner emits
// nothing. ERROR_SENTINEL is never emitted either; the parser marks it
// valid only during error recovery, where COMMAND_NAMED_KEY,
// SUBSCRIPT_OPEN, CAST_CLOSE and the STRING_BODY check are suppressed.
//
// Stateless — `create` returns NULL and the runtime calls
// `destroy` / `serialize` / `deserialize` as no-ops. When a future
// branch needs persistent state, introduce a struct then.

#include "tree_sitter/parser.h"

#include <string.h>

enum TokenType {
    SLASHY_STRING_START,
    LINE_COMMENT,
    BLOCK_COMMENT,
    GROOVYDOC_COMMENT,
    RECORD_KEYWORD,
    COMMAND_NAMED_KEY,
    SUBSCRIPT_OPEN,
    CAST_CLOSE,
    STRING_BODY,
    ERROR_SENTINEL,
};

// Wider than the grammar's `identifier` token (SPECIFICATION.md
// §5.2): every non-ASCII code point counts as an identifier
// character, because the scanner has no Unicode tables. That is safe
// because the scanner never emits a name. It only looks ahead, for
// the name of a record header and for the key of a named argument,
// and the in-grammar lexer then reads the name with the real class.
// On valid input the two agree: outside strings and comments, a
// non-ASCII character can only be part of a name. On other input the
// name the lexer reads is shorter, and the parse fails there.
static inline bool is_identifier_start(int32_t c) {
    return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')
        || c == '_' || c == '$' || c > 0x7F;
}

static inline bool is_identifier_char(int32_t c) {
    return is_identifier_start(c) || (c >= '0' && c <= '9');
}

// SPECIFICATION.md §5.10 — a line escape, `\` followed by a line
// break, is whitespace to Groovy's lexer (GroovyLexer.g4
// `WS : ([ \t]+ | LineEscape+)`), so the line after it continues the
// current one. Skips spaces, tabs and line escapes. Returns false at a
// `\` that does not start a line escape: the `\` is consumed by then
// and no token starts with one, so the caller must return false to
// make the lexer rewind.
static bool skip_same_line_space(TSLexer *lexer, bool skip) {
    for (;;) {
        if (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
            lexer->advance(lexer, skip);
            continue;
        }
        if (lexer->lookahead != '\\') {
            return true;
        }
        lexer->advance(lexer, skip);
        if (lexer->lookahead == '\r') {
            lexer->advance(lexer, skip);
        }
        if (lexer->lookahead != '\n') {
            return false;
        }
        lexer->advance(lexer, skip);
    }
}

// SPECIFICATION.md §6.8 — decides that the key of a named argument
// in a parenthesis-free call (`exclude group: 'a'`) starts here: a
// name on the current line, followed by `:` and not `::`. The token is
// empty. `mark_end` fixes it at the start of the name, and the
// in-grammar lexer reads the name itself, so the key is an identifier
// by the grammar's own definition, non-ASCII letters included. A
// newline before the key means a new statement (a Spock `where:`
// label, say), so the caller does not skip one.
//
// The caller has skipped the same-line space and seen an identifier
// start.
static bool scan_command_named_key(TSLexer *lexer) {
    lexer->mark_end(lexer);
    // Keep the first few chars to reject `case:` / `default:`, which
    // are switch labels, not keys. Apache's `namedArgLabel` accepts both
    // keywords, so `foo default: 1` diverges on purpose: the one-liner
    // `switch (x) { case 1: foo default: bar }` in statements-switch.txt
    // depends on the rejection. A non-ASCII character is stored as 0,
    // which matches neither word; cut to a `char` it could.
    char word[8];
    unsigned len = 0;
    while (is_identifier_char(lexer->lookahead)) {
        if (len < sizeof(word)) {
            word[len] = lexer->lookahead <= 0x7F ? (char)lexer->lookahead : 0;
        }
        len++;
        lexer->advance(lexer, false);
    }
    if ((len == 4 && memcmp(word, "case", 4) == 0)
        || (len == 7 && memcmp(word, "default", 7) == 0)) {
        return false;
    }
    if (!skip_same_line_space(lexer, false)) {
        return false;
    }
    if (lexer->lookahead != ':') {
        return false;
    }
    lexer->advance(lexer, false);
    if (lexer->lookahead == ':') {
        return false;
    }
    lexer->result_symbol = COMMAND_NAMED_KEY;
    return true;
}

static inline void advance(TSLexer *lexer) {
    lexer->advance(lexer, false);
}

static inline bool is_whitespace(int32_t c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r';
}

static inline bool is_inline_space(int32_t c) {
    return c == ' ' || c == '\t';
}

static inline bool is_line_break(int32_t c) {
    return c == '\n' || c == '\r';
}

// Skips the trivia Groovy's lexer hides inside a record header:
// spaces, tabs, line escapes and block comments, plus line breaks and
// line comments when `newlines` is set. Returns false on a `/` that
// opens neither comment kind (a slashy string or division cannot be
// part of a header), on a `\` that is not a line escape, and on an
// unterminated block comment.
static bool skip_header_trivia(TSLexer *lexer, bool newlines) {
    for (;;) {
        for (;;) {
            if (!skip_same_line_space(lexer, false)) {
                return false;
            }
            if (!newlines || !is_line_break(lexer->lookahead)) {
                break;
            }
            advance(lexer);
        }
        if (lexer->lookahead != '/') {
            return true;
        }
        advance(lexer);
        if (lexer->lookahead == '*') {
            advance(lexer);
            // EOF-bounded: an unterminated comment ends the scan here.
            for (;;) {
                if (lexer->eof(lexer)) {
                    return false;
                }
                if (lexer->lookahead == '*') {
                    advance(lexer);
                    if (lexer->lookahead == '/') {
                        advance(lexer);
                        break;
                    }
                } else {
                    advance(lexer);
                }
            }
        } else if (newlines && lexer->lookahead == '/') {
            while (!lexer->eof(lexer) && !is_line_break(lexer->lookahead)) {
                advance(lexer);
            }
        } else {
            return false;
        }
    }
}

// SPECIFICATION.md §6.7 — `record` is a keyword only at the start of
// a record header: `record`, a name on the same line, then `(` with
// Groovy's lexer trivia allowed in between, or `<` straight after the
// name (type parameters; the grammar's `type_parameters` opener is
// `token.immediate` too). Everywhere else the word
// is an identifier (`record.key()`, `private Record record`,
// `public void record()`), as Apache's `identifier` rule allows. The
// grammar cannot make that choice: with `word: $.identifier` the
// keyword wins wherever it has an action, and once
// record_declaration is a class member that is every member start.
//
// The scanner has already consumed `record` when this is called.
// `mark_end` fixes the token there; the look-ahead that follows is
// read but not kept. Between `record` and the name only spaces, tabs
// and block comments may appear: `classDeclaration` in
// GroovyParser.g4 has no `nls` there, so `record` at the end of a line
// is never a keyword, which keeps `private Record record` followed by
// a constructor on the next line a field and a constructor. Before
// `(` the rule has `nls`, so line breaks and line comments are
// skipped there too. A command-style call `record Point(1, 2)`
// matches the pattern and misfires; §6.7 lists the residuals.
static bool scan_record_header(TSLexer *lexer) {
    if (is_identifier_char(lexer->lookahead)) {
        return false;
    }
    lexer->mark_end(lexer);
    if (!is_inline_space(lexer->lookahead) && lexer->lookahead != '/'
        && lexer->lookahead != '\\') {
        return false;
    }
    if (!skip_header_trivia(lexer, false)) {
        return false;
    }
    if (!is_identifier_start(lexer->lookahead)) {
        return false;
    }
    // EOF-bounded: lookahead is 0 at end of input, which fails the
    // predicate.
    while (is_identifier_char(lexer->lookahead)) {
        advance(lexer);
    }
    if (lexer->lookahead == '<') {
        return true;
    }
    if (!skip_header_trivia(lexer, true)) {
        return false;
    }
    return lexer->lookahead == '(';
}

// SPECIFICATION.md §6.11 — the `)` of a cast, already consumed when
// this is called. A cast needs its operand on the same line:
// `castParExpression castOperandExpression` in GroovyParser.g4 has no
// `nls` between the two, so `x = (a)` followed by `foo()` on the next
// line is two statements. Spaces, tabs and line escapes may come
// first. A line comment ends the line. A block comment counts as
// something on the line; it is not read, so that an unterminated one
// is not scanned to the end of the input here.
static bool cast_operand_follows(TSLexer *lexer) {
    lexer->mark_end(lexer);
    if (!skip_same_line_space(lexer, false)) {
        return true;
    }
    if (lexer->eof(lexer) || is_line_break(lexer->lookahead)) {
        return false;
    }
    if (lexer->lookahead == '/') {
        advance(lexer);
        return lexer->lookahead != '/';
    }
    return true;
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

    // SPECIFICATION.md §6.10 — STRING_BODY is valid only between the
    // quotes of a string whose body is lexed in pieces. Comments are
    // extras, so the parser offers them there as well, and this
    // function is called at every piece boundary. Without this check a
    // body that starts with `//` or `/*`, or has one after an
    // interpolation, lost that text to a comment token. The body
    // belongs to the in-grammar lexer.
    if (valid_symbols[STRING_BODY] && !valid_symbols[ERROR_SENTINEL]) {
        return false;
    }

    // Same-line tokens: SUBSCRIPT_OPEN (§5.10) and COMMAND_NAMED_KEY
    // (§5.5). Both skip only spaces, tabs and line escapes, the
    // same-line whitespace of Groovy's lexer; at a newline we fall
    // through to the branches below. Their first characters (`[` vs. an
    // identifier start) are disjoint, so one whitespace skip serves
    // both.
    bool wants_subscript = valid_symbols[SUBSCRIPT_OPEN];
    bool wants_named_key = valid_symbols[COMMAND_NAMED_KEY];
    if ((wants_subscript || wants_named_key)
        && !valid_symbols[ERROR_SENTINEL]) {
        if (!skip_same_line_space(lexer, true)) {
            return false;
        }
        // SPECIFICATION.md §5.10 — a `[` on the current line can continue
        // the expression (subscript or array type). After a newline the
        // in-grammar `[` starts a list or map literal instead.
        if (wants_subscript && lexer->lookahead == '[') {
            lexer->advance(lexer, false);
            lexer->result_symbol = SUBSCRIPT_OPEN;
            return true;
        }
        // An identifier is either the key or, once consumed, not something
        // the code below could start (a comment or slashy string), so stop
        // here. Falling through after the scan advanced past `b` in
        // `foo a, b` and a newline made the comment branch emit `b` as a
        // line_comment, since mark_end had already been called.
        // RECORD_KEYWORD can be valid here too, after a statement-start
        // identifier (`foo` then `record R(…)`). The key wins on the same
        // line: `foo record: 1` is a named argument, while a record header
        // there is not valid Groovy. A `record` on the next line is not
        // reached here, since a newline is not skipped.
        if (wants_named_key && is_identifier_start(lexer->lookahead)) {
            return scan_command_named_key(lexer);
        }
    }

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
    bool wants_cast_close = valid_symbols[CAST_CLOSE]
        && !valid_symbols[ERROR_SENTINEL];
    if (!wants_comment && !wants_slashy && !wants_record
        && !wants_cast_close) {
        return false;
    }

    // A line escape is whitespace too, so a comment, a slashy string or
    // `record` may follow one.
    while (is_whitespace(lexer->lookahead) || lexer->lookahead == '\\') {
        if (lexer->lookahead == '\\') {
            if (!skip_same_line_space(lexer, true)) {
                return false;
            }
        } else {
            lexer->advance(lexer, true);
        }
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
    if (lexer->lookahead == ')' && wants_cast_close) {
        // On `false` the runtime rewinds and the in-grammar lexer reads
        // a plain `)`, which closes a parenthesized expression but not
        // a cast.
        advance(lexer);
        if (!cast_operand_follows(lexer)) {
            return false;
        }
        lexer->result_symbol = CAST_CLOSE;
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
        // EOF guard. A lone `\r` ends the line too, as in Groovy's
        // lexer (`LineTerminator`), so the comment stops before it.
        while (!lexer->eof(lexer) && !is_line_break(lexer->lookahead)) {
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
        if (is_line_break(lexer->lookahead)) {
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
