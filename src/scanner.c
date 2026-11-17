// Hand-written external scanner for tree-sitter-groovy.
//
// Handles §6.2 slashy-string-start, §6.6 line/block/groovydoc
// comments, and the contextual `record` (§6.7), `trait` and `sealed`
// (§6.13) keywords. The first two start with `/`; combining their
// dispatch keeps the in-grammar lexer from partially consuming the
// leading `/`. The keywords are look-ahead decisions the grammar
// cannot express: each word is a keyword only when the declaration it
// opens follows it. Also handles §6.8
// COMMAND_NAMED_KEY, an empty token before the key of a named
// argument in a parenthesis-free call; §6.14 COMMAND_VALUE_START, an
// empty token before the receiver of such a call that is a value, and
// COMMAND_PATH_START, one between the first name of a dotted receiver
// and its first member access;
// §6.9 SUBSCRIPT_OPEN, a `[` on
// the same line as the token before it; §6.11 CAST_CLOSE, a `)` with
// something after it on its line; and §6.12 NOT_IN, the `!in`
// operator. UNTERMINATED_COMMENT (§6.6) is an empty token at the
// first `/*` that has no `*/`, and carries the scanner's state.
// STRING_BODY (§6.10) is never emitted; it is valid only
// inside a string body, where the scanner emits nothing. ERROR_SENTINEL is never emitted either; the parser marks it
// valid only during error recovery, where COMMAND_NAMED_KEY,
// COMMAND_VALUE_START, COMMAND_PATH_START, SUBSCRIPT_OPEN, CAST_CLOSE,
// NOT_IN and the STRING_BODY check are suppressed.
//
// One bit of state, `Scanner.no_comment_close_ahead` (§6.6). Nothing
// else is kept between calls.

#include "tree_sitter/alloc.h"
#include "tree_sitter/parser.h"

#include <string.h>

enum TokenType {
    SLASHY_STRING_START,
    LINE_COMMENT,
    BLOCK_COMMENT,
    GROOVYDOC_COMMENT,
    RECORD_KEYWORD,
    TRAIT_KEYWORD,
    SEALED_KEYWORD,
    COMMAND_NAMED_KEY,
    COMMAND_VALUE_START,
    COMMAND_PATH_START,
    SUBSCRIPT_OPEN,
    CAST_CLOSE,
    NOT_IN,
    UNTERMINATED_COMMENT,
    STRING_BODY,
    ERROR_SENTINEL,
};

// SPECIFICATION.md §6.6 — a `/*` with no `*/` after it is not a
// comment, and finding that out means reading to the end of input.
// Without a memory, a file with N of them was read N times.
//
// The fact is about the input alone: no `*/` at or after one position
// means none after any later one. But a scan that fails leaves no
// token, and the runtime restores the state from the last external
// token before every call, so the scan that learns it emits the empty
// UNTERMINATED_COMMENT token to hold it. That token read to the end of
// input, so any edit after it makes the runtime lex it again, and a
// node lexed in this state is reused only where the new parse is in
// the same state.
// §6.6 records the one reparse this does not cover: being empty, the
// token lets a node before it be reused after the comment is closed.
typedef struct {
    bool no_comment_close_ahead;
} Scanner;

// A failed allocation leaves the scanner without its memory, which
// costs time and nothing else.
static inline bool no_comment_close_ahead(const Scanner *scanner) {
    return scanner != NULL && scanner->no_comment_close_ahead;
}

// Wider than the grammar's `identifier` token (SPECIFICATION.md
// §5.2): every non-ASCII code point counts as an identifier
// character, because the scanner has no Unicode tables. That is safe
// because the scanner never emits a name. It only looks ahead: past the
// name in a declaration header, at the key of a named argument, and at
// words it compares with ASCII keywords. The in-grammar lexer then
// reads any name with the real class. On valid input the two agree:
// outside strings and comments, a non-ASCII character can only be part
// of a name. On other input the name the lexer reads is shorter, and
// the parse fails there.
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

// Consumes a name and keeps its first `size` characters in `word`, to
// compare with a keyword. A non-ASCII character is stored as 0, which
// matches no keyword; cut to a `char` it could. Returns the full length.
static unsigned read_word(TSLexer *lexer, char *word, unsigned size) {
    unsigned len = 0;
    // EOF-bounded: lookahead is 0 at end of input, which fails the
    // predicate.
    while (is_identifier_char(lexer->lookahead)) {
        if (len < size) {
            word[len] = lexer->lookahead <= 0x7F ? (char)lexer->lookahead : 0;
        }
        len++;
        lexer->advance(lexer, false);
    }
    return len;
}

// Whether a word that `read_word` returned is `keyword`. A word longer
// than the buffer differs in length from every keyword compared.
static bool word_is(const char *word, unsigned len, const char *keyword) {
    return len == strlen(keyword) && memcmp(word, keyword, len) == 0;
}

// Whether a word that `read_word` returned is the literal `true`,
// `false` or `null`.
static bool is_literal_word(const char *word, unsigned len) {
    return word_is(word, len, "true") || word_is(word, len, "false")
        || word_is(word, len, "null");
}

// Whether the name that `read_word` just consumed is the key of a named
// argument: `:` follows on the same line, and not `::`, which makes a
// method reference. `case:` and `default:` are switch labels, not keys.
// Apache's `namedArgLabel` accepts both keywords, so `foo default: 1`
// diverges on purpose: the one-liner
// `switch (x) { case 1: foo default: bar }` in statements-switch.txt
// depends on the rejection.
static bool key_colon_follows(TSLexer *lexer, const char *word, unsigned len) {
    if (word_is(word, len, "case") || word_is(word, len, "default")) {
        return false;
    }
    if (!skip_same_line_space(lexer, false)) {
        return false;
    }
    if (lexer->lookahead != ':') {
        return false;
    }
    lexer->advance(lexer, false);
    return lexer->lookahead != ':';
}

// Consumes a name and what follows it on its line, and returns whether
// the name is the key of a named argument.
static bool read_named_key(TSLexer *lexer) {
    // Keeps enough characters to tell `default` from a longer word.
    char word[sizeof "default"];
    unsigned len = read_word(lexer, word, sizeof word);
    return key_colon_follows(lexer, word, len);
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
    if (!read_named_key(lexer)) {
        return false;
    }
    lexer->result_symbol = COMMAND_NAMED_KEY;
    return true;
}

// Whether a member access may start here: the first character of every
// operator `read_member_access` reads.
static inline bool is_member_access_start(int32_t c) {
    return c == '.' || c == '?' || c == '*';
}

// Consumes one of the operators that join the names of a
// parenthesis-free call's receiver: `.`, `?.`, `??.`, `*.`, `.&` and
// `.@`. Returns false, with input consumed, on anything else.
static bool read_member_access(TSLexer *lexer) {
    switch (lexer->lookahead) {
        case '.':
            lexer->advance(lexer, false);
            if (lexer->lookahead == '&' || lexer->lookahead == '@') {
                lexer->advance(lexer, false);
            }
            return true;
        case '?':
            lexer->advance(lexer, false);
            if (lexer->lookahead == '?') {
                lexer->advance(lexer, false);
            }
            break;
        case '*':
            lexer->advance(lexer, false);
            break;
        default:
            return false;
    }
    if (lexer->lookahead != '.') {
        return false;
    }
    lexer->advance(lexer, false);
    return true;
}

// Consumes member accesses, each an operator and a name, for as long as
// one starts here, and sets `last_name` to the first character of the last
// name read. Returns false, with input consumed, at an operator that is not
// one of them or that no name follows.
static bool read_member_accesses(TSLexer *lexer, int32_t *last_name) {
    while (is_member_access_start(lexer->lookahead)) {
        if (!read_member_access(lexer)
            || !is_identifier_start(lexer->lookahead)) {
            return false;
        }
        *last_name = lexer->lookahead;
        read_word(lexer, NULL, 0);
    }
    return true;
}

// Whether the first argument of a parenthesis-free call starts here, on
// the line: after spaces, tabs and line escapes, a number, a string,
// `true`, `false`, `null` or the key of a named argument. A quote is
// left unread. A slashy string is not an argument here: after a name
// Groovy reads `/` as division.
static bool first_argument_follows(TSLexer *lexer) {
    if (!skip_same_line_space(lexer, false)) {
        return false;
    }
    int32_t c = lexer->lookahead;
    if ((c >= '0' && c <= '9') || c == '\'' || c == '"') {
        return true;
    }
    if (!is_identifier_start(c)) {
        return false;
    }
    char argument[sizeof "default"];
    unsigned argument_len = read_word(lexer, argument, sizeof argument);
    return is_literal_word(argument, argument_len)
        || key_colon_follows(lexer, argument, argument_len);
}

// SPECIFICATION.md §6.14 — decides that a parenthesis-free call starts
// here as the value of a declarator or an assignment
// (`def out = sh script: 'make'`, `def r = obj.method 1`): a name and any
// member accesses written without spaces, then on its line a number, a
// string, `true`, `false`, `null` or the key of a named argument. The
// token is empty and sits before the name, which the in-grammar lexer
// reads, as it reads the argument. Taking the decision at the name, not
// after it, keeps a line break hidden by a comment from joining two
// statements: in `def x = foo` followed by `/* c */ 1` on the next line,
// the token after the comment would no longer see the line break.
//
// The caller has skipped whitespace and seen an identifier start.
static bool scan_command_value_start(TSLexer *lexer) {
    lexer->mark_end(lexer);
    char word[sizeof "switch"];
    unsigned len = read_word(lexer, word, sizeof word);
    // The words that start a value as keywords keep that reading.
    if (word_is(word, len, "new") || word_is(word, len, "switch")
        || is_literal_word(word, len)) {
        return false;
    }
    int32_t last_name = 0;
    if (!read_member_accesses(lexer, &last_name)
        || !first_argument_follows(lexer)) {
        return false;
    }
    lexer->result_symbol = COMMAND_VALUE_START;
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

// Whether the string that starts here is the name of a method declared
// with a qualified return type (`java.util.List 'foo'() { }`): it closes
// on this line, has no interpolation, and `(` follows it on the line.
// Groovy reads an interpolated name before `(` as a dynamic call.
static bool method_name_follows(TSLexer *lexer) {
    int32_t quote = lexer->lookahead;
    lexer->advance(lexer, false);
    unsigned closing_quotes = 1;
    if (lexer->lookahead == quote) {
        lexer->advance(lexer, false);
        if (lexer->lookahead == quote) {
            lexer->advance(lexer, false);
            closing_quotes = 3;
        } else {
            closing_quotes = 0;
        }
    }
    // Bounded by the line: a line break or the end of input stops it.
    unsigned quotes_seen = 0;
    while (quotes_seen < closing_quotes) {
        int32_t c = lexer->lookahead;
        if (lexer->eof(lexer) || is_line_break(c) || (quote == '"' && c == '$')) {
            return false;
        }
        lexer->advance(lexer, false);
        if (c == '\\') {
            if (lexer->eof(lexer) || is_line_break(lexer->lookahead)) {
                return false;
            }
            lexer->advance(lexer, false);
        }
        quotes_seen = c == quote ? quotes_seen + 1 : 0;
    }
    return skip_same_line_space(lexer, false) && lexer->lookahead == '(';
}

// SPECIFICATION.md §6.14 — decides that a statement is a
// parenthesis-free call whose receiver has member accesses
// (`obj.method 1`, `a?.b.c x: 1`). The token is empty and sits between
// the receiver's first name, already read, and its first access. Nothing
// can come between the two, so no line break or comment is in play
// there; the accesses must follow each other without spaces, and the
// first argument must start on their line. The caller has seen a member
// access start.
static bool scan_command_path_start(TSLexer *lexer) {
    lexer->mark_end(lexer);
    int32_t last_name = 0;
    if (!read_member_accesses(lexer, &last_name)
        || !first_argument_follows(lexer)) {
        return false;
    }
    // Groovy declares a method when the last name is capitalized
    // (`java.util.List 'foo'() { }`) and calls one otherwise
    // (`obj.m 'a'() { }` is `obj.m('a'({ }))`).
    bool capitalized = last_name >= 'A' && last_name <= 'Z';
    if (capitalized && (lexer->lookahead == '\'' || lexer->lookahead == '"')
        && method_name_follows(lexer)) {
        return false;
    }
    lexer->result_symbol = COMMAND_PATH_START;
    return true;
}

// Skips the trivia Groovy's lexer hides inside a declaration header:
// spaces, tabs, line escapes and block comments, plus line breaks and
// line comments when `newlines` is set. Returns false on a `/` that
// opens neither comment kind (a slashy string or division cannot be
// part of a header), on a `\` that is not a line escape, and on an
// unterminated block comment. `no_close_ahead` says that every block
// comment from here on is unterminated. Finding that out here cannot
// be remembered: the token would have to start at the keyword, which
// is before the position the fact holds from. The comment branch of
// the scan function reaches the same `/*` next and remembers it.
// `crossed_line`, when not NULL, is set once a line break is skipped.
static bool skip_header_trivia(
    TSLexer *lexer,
    bool newlines,
    bool no_close_ahead,
    bool *crossed_line
) {
    for (;;) {
        for (;;) {
            if (!skip_same_line_space(lexer, false)) {
                return false;
            }
            if (!newlines || !is_line_break(lexer->lookahead)) {
                break;
            }
            if (crossed_line != NULL) {
                *crossed_line = true;
            }
            advance(lexer);
        }
        if (lexer->lookahead != '/') {
            return true;
        }
        advance(lexer);
        if (lexer->lookahead == '*') {
            if (no_close_ahead) {
                return false;
            }
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

// Consumes `keyword` and returns whether the input spells it as a whole
// word, with no identifier character after it. On false, input has been
// consumed, so the caller returns false and the runtime rewinds.
static bool consume_keyword(TSLexer *lexer, const char *keyword) {
    for (const char *c = keyword; *c != '\0'; c++) {
        if (lexer->lookahead != *c) {
            return false;
        }
        advance(lexer);
    }
    return !is_identifier_char(lexer->lookahead);
}

// The name of a declaration header, after its keyword. `mark_end` fixes
// the token at the keyword; what is read from here on is look-ahead and
// is not kept. Between the keyword and the name only spaces, tabs, line
// escapes and block comments may appear: `classDeclaration` in
// GroovyParser.g4 has no `nls` there, so the keyword at the end of a
// line is never one. `private Record record` followed by a constructor
// on the next line, and `private String trait` followed by any member,
// stay a field and that member.
static bool scan_header_name(TSLexer *lexer, bool no_close_ahead) {
    lexer->mark_end(lexer);
    if (!is_inline_space(lexer->lookahead) && lexer->lookahead != '/'
        && lexer->lookahead != '\\') {
        return false;
    }
    if (!skip_header_trivia(lexer, false, no_close_ahead, NULL)) {
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
    return true;
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
// Before `(` the rule has `nls`, so line breaks and line comments are
// skipped there too. A command-style call `record Point(1, 2)`
// matches the pattern and misfires; §6.7 lists the residuals.
static bool scan_record_header(TSLexer *lexer, bool no_close_ahead) {
    if (!scan_header_name(lexer, no_close_ahead)) {
        return false;
    }
    if (lexer->lookahead == '<') {
        return true;
    }
    if (!skip_header_trivia(lexer, true, no_close_ahead, NULL)) {
        return false;
    }
    return lexer->lookahead == '(';
}

// SPECIFICATION.md §6.13 — `trait` is a keyword only at the start of a
// trait header: as for `record`, a name on the same line, then, with
// line breaks and comments allowed (the rule has `nls` before each
// clause and the body), `<`, `{` or one of `extends`, `implements` and
// `permits`. Everywhere else the word is an identifier, as Apache's
// `identifier` rule allows. A `<` after a space opens type parameters
// in Groovy but not in the grammar, whose opener is `token.immediate`;
// taking the keyword keeps that an `ERROR` rather than a comparison.
// The scanner has already consumed `trait` when this is called.
static bool scan_trait_header(TSLexer *lexer, bool no_close_ahead) {
    if (!scan_header_name(lexer, no_close_ahead)) {
        return false;
    }
    if (!skip_header_trivia(lexer, true, no_close_ahead, NULL)) {
        return false;
    }
    if (lexer->lookahead == '{' || lexer->lookahead == '<') {
        return true;
    }
    char word[sizeof "implements"];
    unsigned len = read_word(lexer, word, sizeof word);
    return word_is(word, len, "extends") || word_is(word, len, "implements")
        || word_is(word, len, "permits");
}

// The words of `_modifier` in grammar.js, and `default`, which Apache's
// class modifiers add. Apache's member modifiers take all of them, in
// any order, before a nested type declaration.
static bool is_modifier_word(const char *word, unsigned len) {
    return word_is(word, len, "public") || word_is(word, len, "protected")
        || word_is(word, len, "private") || word_is(word, len, "static")
        || word_is(word, len, "abstract") || word_is(word, len, "final")
        || word_is(word, len, "strictfp") || word_is(word, len, "default")
        || word_is(word, len, "synchronized") || word_is(word, len, "native")
        || word_is(word, len, "transient") || word_is(word, len, "volatile");
}

// SPECIFICATION.md §6.13 — `sealed` is a keyword only before `class`,
// `interface` or `trait`, the words the grammar accepts after it.
// Apache's modifiers end in `nls`, so line breaks and comments may come
// between. Everywhere else the word is an identifier. The scanner has
// already consumed `sealed` when this is called.
//
// Groovy's parser also takes `sealed` before another modifier, an
// annotation, `record` or `enum`. The grammar accepts none of them
// after it, so the keyword there only keeps the `ERROR` at the word.
// Read as a name, the word gave a tree with no `ERROR` that dropped the
// modifier, or lost the declaration to an `ERROR` that began at `sealed`.
// The keyword is taken only on the same line: across a line break the
// same words also start the member after a field named `sealed`, which
// the keyword would turn into an `ERROR`.
static bool scan_sealed_target(TSLexer *lexer, bool no_close_ahead) {
    lexer->mark_end(lexer);
    bool crossed_line = false;
    if (!skip_header_trivia(lexer, true, no_close_ahead, &crossed_line)) {
        return false;
    }
    if (lexer->lookahead == '@') {
        return !crossed_line;
    }
    char word[sizeof "synchronized"];
    unsigned len = read_word(lexer, word, sizeof word);
    if (word_is(word, len, "class") || word_is(word, len, "interface")
        || word_is(word, len, "trait")) {
        return true;
    }
    if (crossed_line) {
        return false;
    }
    if (word_is(word, len, "non") && lexer->lookahead == '-') {
        advance(lexer);
        return consume_keyword(lexer, "sealed");
    }
    return is_modifier_word(word, len) || word_is(word, len, "record")
        || word_is(word, len, "enum");
}

// SPECIFICATION.md §6.11 — whether something follows a `)` on its
// line. The `)` is already consumed when this is called.
// `castParExpression castOperandExpression` in GroovyParser.g4 has no
// `nls` between the two, so where a line break can end a statement,
// `x = (a)` followed by `foo()` on the next line is two statements.
// The grammar ranks a cast by this answer; it does not forbid one,
// because inside `(…)` and `[…]` a line break ends nothing. Spaces,
// tabs, line escapes and block comments that close on the line may
// come first. A line comment ends the line.
static bool cast_operand_follows(TSLexer *lexer) {
    lexer->mark_end(lexer);
    for (;;) {
        if (!skip_same_line_space(lexer, false)) {
            return true;
        }
        if (lexer->eof(lexer) || is_line_break(lexer->lookahead)) {
            return false;
        }
        if (lexer->lookahead != '/') {
            return true;
        }
        advance(lexer);
        if (lexer->lookahead == '/') {
            return false;
        }
        if (lexer->lookahead != '*') {
            return true;
        }
        // A block comment that closes on this line is skipped, and the
        // answer depends on what follows it. One that runs past the
        // end of the line counts as something on the line: Groovy's
        // lexer drops the comment with its line breaks, and reading
        // on would scan an unterminated comment to the end of input.
        advance(lexer);
        for (;;) {
            if (lexer->eof(lexer) || is_line_break(lexer->lookahead)) {
                return true;
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
    }
}

void *tree_sitter_groovy_external_scanner_create(void) {
    return ts_calloc(1, sizeof(Scanner));
}

void tree_sitter_groovy_external_scanner_destroy(void *payload) {
    ts_free(payload);
}

// Length of the serialized state while the flag is unset. It must
// stay 0: the runtime passes that length before the first external
// token.
static const unsigned UNSET_STATE_LENGTH = 0;

// Length of the serialized state once the flag is set: one byte.
static const unsigned SET_STATE_LENGTH = 1;

// The value of that byte. Any other is read as unset.
static const char SET_STATE_BYTE = 1;

unsigned tree_sitter_groovy_external_scanner_serialize(
    void *payload,
    char *buffer
) {
    if (!no_comment_close_ahead(payload)) {
        return UNSET_STATE_LENGTH;
    }
    buffer[0] = SET_STATE_BYTE;
    return SET_STATE_LENGTH;
}

void tree_sitter_groovy_external_scanner_deserialize(
    void *payload,
    const char *buffer,
    unsigned length
) {
    Scanner *scanner = payload;
    if (scanner != NULL) {
        scanner->no_comment_close_ahead =
            length == SET_STATE_LENGTH && buffer[0] == SET_STATE_BYTE;
    }
}

bool tree_sitter_groovy_external_scanner_scan(
    void *payload,
    TSLexer *lexer,
    const bool *valid_symbols
) {
    Scanner *scanner = payload;

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

    // SPECIFICATION.md §6.14 — a member access directly after the first
    // name of a statement. No other token starts with one of its
    // characters.
    if (valid_symbols[COMMAND_PATH_START] && !valid_symbols[ERROR_SENTINEL]
        && is_member_access_start(lexer->lookahead)) {
        return scan_command_path_start(lexer);
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
        // The keywords of §6.7 and §6.13 can be valid here too, after a
        // statement-start identifier (`foo` then `record R(…)`). The key
        // wins on the same line: `foo record: 1` is a named argument,
        // while a declaration there is not valid Groovy. A keyword on the
        // next line is not reached here, since a newline is not skipped.
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
    // recovery. The contextual keywords (§6.7, §6.13) and the receiver
    // of a call that is a value (§6.14) share the whitespace skip and
    // are dispatched on their first letter before the `/` branches. A
    // value may start on the line after its `=`, so that receiver is
    // looked for past line breaks too.
    bool wants_comment = valid_symbols[LINE_COMMENT]
        || valid_symbols[BLOCK_COMMENT]
        || valid_symbols[GROOVYDOC_COMMENT];
    bool wants_slashy = valid_symbols[SLASHY_STRING_START];
    bool wants_record = valid_symbols[RECORD_KEYWORD];
    bool wants_trait = valid_symbols[TRAIT_KEYWORD];
    bool wants_sealed = valid_symbols[SEALED_KEYWORD];
    bool wants_value_start = valid_symbols[COMMAND_VALUE_START]
        && !valid_symbols[ERROR_SENTINEL];
    bool wants_cast_close = valid_symbols[CAST_CLOSE]
        && !valid_symbols[ERROR_SENTINEL];
    bool wants_not_in = valid_symbols[NOT_IN]
        && !valid_symbols[ERROR_SENTINEL];
    if (!wants_comment && !wants_slashy && !wants_record && !wants_trait
        && !wants_sealed && !wants_value_start && !wants_cast_close
        && !wants_not_in) {
        return false;
    }

    // A line escape is whitespace too, so a comment, a slashy string or
    // a keyword may follow one.
    while (is_whitespace(lexer->lookahead) || lexer->lookahead == '\\') {
        if (lexer->lookahead == '\\') {
            if (!skip_same_line_space(lexer, true)) {
                return false;
            }
        } else {
            lexer->advance(lexer, true);
        }
    }
    // A letter can never open a comment or a slashy string, so every
    // path out of the keyword branches returns. On `false` the runtime
    // rewinds to the token start and the in-grammar lexer reads the
    // word as an identifier.
    bool no_close_ahead = no_comment_close_ahead(scanner);
    if (lexer->lookahead == 'r' && wants_record) {
        if (!consume_keyword(lexer, "record")
            || !scan_record_header(lexer, no_close_ahead)) {
            return false;
        }
        lexer->result_symbol = RECORD_KEYWORD;
        return true;
    }
    if (lexer->lookahead == 't' && wants_trait) {
        if (!consume_keyword(lexer, "trait")
            || !scan_trait_header(lexer, no_close_ahead)) {
            return false;
        }
        lexer->result_symbol = TRAIT_KEYWORD;
        return true;
    }
    if (lexer->lookahead == 's' && wants_sealed) {
        if (!consume_keyword(lexer, "sealed")
            || !scan_sealed_target(lexer, no_close_ahead)) {
            return false;
        }
        lexer->result_symbol = SEALED_KEYWORD;
        return true;
    }
    if (wants_value_start && is_identifier_start(lexer->lookahead)) {
        return scan_command_value_start(lexer);
    }
    if (lexer->lookahead == '!' && wants_not_in) {
        // SPECIFICATION.md §6.12 — `!in` is an operator only before
        // whitespace or an opening bracket (GroovyLexer.g4 `NOT_IN`),
        // so that `a !inside` is `!` applied to a name. The character
        // after it is read and not taken: `a !in[1, 2]`.
        static const char not_in[] = "!in";
        for (const char *c = not_in; *c != '\0'; c++) {
            if (lexer->lookahead != *c) {
                return false;
            }
            advance(lexer);
        }
        int32_t next = lexer->lookahead;
        if (!is_whitespace(next) && next != '[' && next != '('
            && next != '{') {
            return false;
        }
        lexer->result_symbol = NOT_IN;
        return true;
    }
    if (lexer->lookahead == ')' && wants_cast_close) {
        // On `false` the runtime rewinds and the in-grammar lexer reads
        // a plain `)`. A cast that closes with one ranks below a
        // parenthesized expression.
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

    // UNTERMINATED_COMMENT is empty and ends here. Every other token
    // of this branch moves the end again.
    lexer->mark_end(lexer);
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
        lexer->mark_end(lexer);
        lexer->result_symbol = LINE_COMMENT;
        return true;
    }
    if (next == '*') {
        if (!wants_comment || no_comment_close_ahead(scanner)) {
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
                lexer->mark_end(lexer);
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
                    lexer->mark_end(lexer);
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
        // The runtime drops an empty extra token unless the state
        // changed with it. This one sets the flag, and the check above
        // keeps a second one from being emitted in the same state.
        if (scanner == NULL || !valid_symbols[UNTERMINATED_COMMENT]) {
            return false;
        }
        scanner->no_comment_close_ahead = true;
        lexer->result_symbol = UNTERMINATED_COMMENT;
        return true;
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
