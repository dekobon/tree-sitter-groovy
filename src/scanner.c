// Hand-written external scanner for tree-sitter-groovy.
//
// Handles §6.2 slashy-string-start, §6.6 line/block/groovydoc
// comments, and the contextual `record` (§6.7), `trait` and `sealed`
// (§6.13) keywords. The first two start with `/`; combining their
// dispatch keeps the in-grammar lexer from partially consuming the
// leading `/`. The keywords are look-ahead decisions the grammar
// cannot express: each word is a keyword only when the declaration it
// opens follows it. §6.16 does the same for `var`, `yield`, `in` and
// `as`, each a keyword only before what it declares, takes or opens,
// and for such a word as a named argument's key (CONTEXTUAL_KEY).
// Also handles §6.8
// COMMAND_NAMED_KEY, an empty token before the key of a named
// argument in a parenthesis-free call; §6.14 COMMAND_VALUE_START, an
// empty token before the receiver of such a call that is a value,
// COMMAND_PATH_START, one between the first name of a dotted receiver
// and its first member access, and COMMAND_STATEMENT_START, one before
// a statement's one-name receiver whose first argument is a `new`, an
// array type, such a call in parentheses or a name;
// §6.9 SUBSCRIPT_OPEN, a `[` on
// the same line as the token before it; §6.18 STATEMENT_PAREN, a `(`
// that starts a line after an expression where a statement may start
// and the parentheses can open one; §6.15 ARRAY_VALUE_START, an
// empty token before an array type that is a value (`String[].class`);
// §6.11 CAST_CLOSE, a `)` with
// something after it on its line; §6.12 NOT_IN, the `!in`
// operator; and §6.17 LAMBDA_START and BARE_LAMBDA_START, empty tokens
// before a lambda's parameters in parentheses and before its one name
// without them, when `->` follows. UNTERMINATED_COMMENT (§6.6) is an
// empty token at the
// first `/*` that has no `*/`, and carries the scanner's state.
// STRING_BODY (§6.10) and TYPE_START (§6.15) are never emitted; they
// are valid only inside a string body, where the scanner emits nothing,
// and where a type may start. ERROR_SENTINEL is never emitted either;
// the parser marks it valid only during error recovery, where
// COMMAND_NAMED_KEY, COMMAND_VALUE_START, COMMAND_PATH_START,
// COMMAND_STATEMENT_START, SUBSCRIPT_OPEN, STATEMENT_PAREN,
// ARRAY_VALUE_START, CAST_CLOSE, NOT_IN, LAMBDA_START, BARE_LAMBDA_START
// and the STRING_BODY check are suppressed.
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
    VAR_KEYWORD,
    VAR_NAMES_KEYWORD,
    YIELD_KEYWORD,
    IN_KEYWORD,
    AS_KEYWORD,
    COMMAND_NAMED_KEY,
    CONTEXTUAL_KEY,
    COMMAND_VALUE_START,
    COMMAND_PATH_START,
    COMMAND_STATEMENT_START,
    SUBSCRIPT_OPEN,
    ARRAY_VALUE_START,
    CAST_CLOSE,
    NOT_IN,
    LAMBDA_START,
    BARE_LAMBDA_START,
    UNTERMINATED_COMMENT,
    STRING_BODY,
    TYPE_START,
    STATEMENT_PAREN,
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

// SPECIFICATION.md §6.15 — the element type of an array type that is a
// value is a primitive type or a class, and class names are capitalized
// by convention, as `scan_command_path_start` also assumes. Checking the
// spelling lets a lowercase name, nearly always a variable or a method,
// be turned down after a character or two instead of being read twice.
static inline bool is_capital(int32_t c) {
    return c >= 'A' && c <= 'Z';
}

// Whether a primitive type name or a class name may start with `c`.
static inline bool is_element_type_start(int32_t c) {
    return is_capital(c) || c == 'b' || c == 'c' || c == 'd' || c == 'f'
        || c == 'i' || c == 'l' || c == 's';
}

// Whether a word that `read_word` returned is a primitive type name.
static bool is_primitive_type(const char *word, unsigned len) {
    return word_is(word, len, "boolean") || word_is(word, len, "byte")
        || word_is(word, len, "char") || word_is(word, len, "double")
        || word_is(word, len, "float") || word_is(word, len, "int")
        || word_is(word, len, "long") || word_is(word, len, "short");
}

// Consumes a name and returns whether it is a primitive type name. It
// stops at the first character that no such name has there, so most
// names cost a character or two.
static bool read_primitive_type(TSLexer *lexer) {
    const char *first = NULL;
    const char *second = NULL;
    switch (lexer->lookahead) {
        case 'b':
            first = "boolean";
            second = "byte";
            break;
        case 'c':
            first = "char";
            break;
        case 'd':
            first = "double";
            break;
        case 'f':
            first = "float";
            break;
        case 'i':
            first = "int";
            break;
        case 'l':
            first = "long";
            break;
        case 's':
            first = "short";
            break;
        default:
            return false;
    }
    unsigned i = 0;
    // EOF-bounded: lookahead is 0 at end of input, which fails the
    // predicate. A candidate is dropped at its terminating NUL, which no
    // identifier character equals.
    for (; is_identifier_char(lexer->lookahead); i++) {
        if (first != NULL && first[i] != lexer->lookahead) {
            first = NULL;
        }
        if (second != NULL && second[i] != lexer->lookahead) {
            second = NULL;
        }
        if (first == NULL && second == NULL) {
            return false;
        }
        lexer->advance(lexer, false);
    }
    return (first != NULL && first[i] == '\0')
        || (second != NULL && second[i] == '\0');
}

// Consumes `.Name` for as long as one follows and its name is
// capitalized, as the names of nested types are (`Map.Entry`). Returns
// false, with input consumed, at a `.` that no capitalized name follows.
static bool read_nested_type_names(TSLexer *lexer) {
    while (lexer->lookahead == '.') {
        lexer->advance(lexer, false);
        if (!is_capital(lexer->lookahead)) {
            return false;
        }
        read_word(lexer, NULL, 0);
    }
    return true;
}

// Consumes a name and returns whether it may name the element type of an
// array type: a primitive type name, or a capitalized name and any nested
// type names after it. A lowercase name is read only while it can still
// become a primitive type name.
static bool read_element_type_name(TSLexer *lexer) {
    if (is_capital(lexer->lookahead)) {
        read_word(lexer, NULL, 0);
        return read_nested_type_names(lexer);
    }
    return read_primitive_type(lexer);
}

// Defined with the other look-ahead helpers below.
static bool skip_header_trivia(
    TSLexer *lexer,
    bool newlines,
    bool no_close_ahead,
    bool *crossed_line,
    bool *slash
);
static bool array_dimensions_follow(
    TSLexer *lexer,
    bool type_may_start,
    bool no_close_ahead
);
static bool is_contextual_word(const char *word, unsigned len);
static bool scan_contextual_word(
    TSLexer *lexer,
    const char *word,
    unsigned len,
    const bool *valid_symbols,
    bool no_close_ahead
);

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

// Whether a member access may start here: the first character of every
// operator `read_member_access` reads.
static inline bool is_member_access_start(int32_t c) {
    return c == '.' || c == '?' || c == '*';
}

// Consumes one of the operators that join the names of a
// parenthesis-free call's receiver: `.`, `?.`, `??.`, `*.`, `.&` and
// `.@`, and clears `dot` unless it is `.`. Returns false, with input
// consumed, on anything else.
static bool read_member_access(TSLexer *lexer, bool *dot) {
    if (lexer->lookahead != '.') {
        *dot = false;
    }
    switch (lexer->lookahead) {
        case '.':
            lexer->advance(lexer, false);
            if (lexer->lookahead == '&' || lexer->lookahead == '@') {
                *dot = false;
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
// name read. Sets `nested` to whether every access is `.` and every name
// after one is capitalized, as in the name of a nested type, and
// `ends_in_new` to whether the last name is `new`. Returns false, with
// input consumed, at an operator that is not one of them or that no name
// follows.
static bool read_member_accesses(
    TSLexer *lexer,
    int32_t *last_name,
    bool *nested,
    bool *ends_in_new
) {
    *nested = true;
    *ends_in_new = false;
    while (is_member_access_start(lexer->lookahead)) {
        if (!read_member_access(lexer, nested)
            || !is_identifier_start(lexer->lookahead)) {
            return false;
        }
        *last_name = lexer->lookahead;
        if (!is_capital(*last_name)) {
            *nested = false;
        }
        char name[sizeof "new"];
        *ends_in_new = word_is(name, read_word(lexer, name, sizeof name), "new");
    }
    return true;
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
// An argument after a comma may also be an array type (§6.15,
// `foo 'a', String[]`), which the same name starts. The scanner cannot
// read the name twice, so ARRAY_VALUE_START is decided here as well when
// `wants_array` is set.
//
// §6.16 — the same name may be `var`, `yield`, `in` or `as`, after a
// statement's first name (`it in xs`, `it as T`). Its keyword must end
// at the word, and whether it is a key is known only past the spaces
// after it (`foo in : 1`), so the word ends the token in either case:
// such a key is CONTEXTUAL_KEY, which spans the word and which the
// grammar aliases to `identifier`.
//
// The caller has skipped the same-line space and seen an identifier
// start.
static bool scan_command_named_key(
    TSLexer *lexer,
    const bool *valid_symbols,
    bool wants_array,
    bool type_may_start,
    bool no_close_ahead
) {
    lexer->mark_end(lexer);
    bool capitalized = is_capital(lexer->lookahead);
    char word[sizeof "default"];
    unsigned len = read_word(lexer, word, sizeof word);
    if (is_contextual_word(word, len)) {
        lexer->mark_end(lexer);
        if (!skip_same_line_space(lexer, false)) {
            return false;
        }
        if (lexer->lookahead == ':') {
            lexer->advance(lexer, false);
            lexer->result_symbol = CONTEXTUAL_KEY;
            return valid_symbols[CONTEXTUAL_KEY] && lexer->lookahead != ':';
        }
        return scan_contextual_word(lexer, word, len, valid_symbols,
                                    no_close_ahead);
    }
    if (lexer->lookahead == '.') {
        // A key is one name; only a nested type name can go on.
        if (!wants_array || !capitalized || !read_nested_type_names(lexer)
            || !array_dimensions_follow(lexer, type_may_start, no_close_ahead)) {
            return false;
        }
        lexer->result_symbol = ARRAY_VALUE_START;
        return true;
    }
    if (key_colon_follows(lexer, word, len)) {
        lexer->result_symbol = COMMAND_NAMED_KEY;
        return true;
    }
    if (wants_array && lexer->lookahead == '['
        && (capitalized || is_primitive_type(word, len))
        && array_dimensions_follow(lexer, type_may_start, no_close_ahead)) {
        lexer->result_symbol = ARRAY_VALUE_START;
        return true;
    }
    return false;
}

// Whether the `new` just consumed opens an object or array creation: the
// name of the created type follows, after Groovy's trivia, which
// `new nls creator` in GroovyParser.g4 allows to include line breaks.
// `x.y new` followed by `(c)` on the next line is not one.
static bool creation_follows(TSLexer *lexer, bool no_close_ahead) {
    return skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)
        && is_identifier_start(lexer->lookahead);
}

// SPECIFICATION.md §6.15 — whether the name that `read_word` just
// consumed starts an array type that is a value (`String[].class`,
// `int[]::new`, `Map.Entry[].class`): a capitalized name and any nested
// type names, or a primitive type name, then dimensions followed by a
// member access or `::`. A bare `String[]` is a type, as where a type
// may start.
static bool array_value_follows(
    TSLexer *lexer,
    const char *word,
    unsigned len,
    bool capitalized,
    bool no_close_ahead
) {
    if (lexer->lookahead == '.') {
        if (!capitalized || !read_nested_type_names(lexer)) {
            return false;
        }
    } else if (!capitalized && !is_primitive_type(word, len)) {
        return false;
    }
    return array_dimensions_follow(lexer, true, no_close_ahead);
}

static bool parenthesized_command_follows(
    TSLexer *lexer,
    bool no_close_ahead
);
static bool call_arguments_follow(TSLexer *lexer, bool no_close_ahead);

// Defined after is_grammar_keyword, which they call.
static bool is_value_keyword(const char *word, unsigned len, int32_t next);
static bool is_reserved_lambda_name(const char *word, unsigned len);
static bool is_reserved_bare_lambda_name(const char *word, unsigned len);

// SPECIFICATION.md §6.14 — the most the look-ahead after a name argument
// reads of a list (`parameters_follow`, `call_arguments_follow`), in items:
// a word, a number, a string, a comment, a line break or a punctuation
// character. Malformed input can restart a statement at every element of a
// long list (`{ bar a, bar b, … }` with no `->`), and each restart reads
// ahead from there, so without a bound the parse takes time quadratic in
// the list. With it each restart reads at most this many items, and the
// parse stays linear. A list longer than this after a lowercase-typed
// first closure parameter is read as a call's arguments; real lists are a
// few items.
static const unsigned NAME_ARGUMENT_LOOKAHEAD_ITEMS = 256;

static bool is_modifier_word(const char *word, unsigned len);

// The code points of the upper-case letters `is_upper_letter` knows
// beyond A to Z: the capitals of Latin-1 (`À` to `Þ`, but for the
// multiplication sign `×`), of Greek (`Α` to `Ω`, but for the unassigned
// code point between `Ρ` and `Σ`) and of Cyrillic (`Ѐ` to `Я`).
enum {
    LATIN1_CAPITAL_FIRST = 0xC0,
    LATIN1_CAPITAL_LAST = 0xDE,
    MULTIPLICATION_SIGN = 0xD7,
    GREEK_CAPITAL_FIRST = 0x391,
    GREEK_CAPITAL_LAST = 0x3A9,
    GREEK_CAPITAL_UNASSIGNED = 0x3A2,
    CYRILLIC_CAPITAL_FIRST = 0x400,
    CYRILLIC_CAPITAL_LAST = 0x42F,
};

// SPECIFICATION.md §6.14 — whether `c` is an upper-case letter, which
// Apache's isInvalidLocalVariableDeclaration asks of a receiver's first
// letter (`Character.isUpperCase`): A to Z, and the capitals of Latin-1,
// Greek and Cyrillic. A capital of any other script reads as lower case;
// the scanner has no Unicode tables.
static inline bool is_upper_letter(int32_t c) {
    return is_capital(c)
        || (c >= LATIN1_CAPITAL_FIRST && c <= LATIN1_CAPITAL_LAST
            && c != MULTIPLICATION_SIGN)
        || (c >= GREEK_CAPITAL_FIRST && c <= GREEK_CAPITAL_LAST
            && c != GREEK_CAPITAL_UNASSIGNED)
        || (c >= CYRILLIC_CAPITAL_FIRST && c <= CYRILLIC_CAPITAL_LAST);
}

// SPECIFICATION.md §6.14 — whether a receiver, one name that `read_word`
// returned, may instead be the type of a declaration: a primitive type name,
// `void` or a name with an upper-case first letter (`upper`). That is how
// Apache's isInvalidLocalVariableDeclaration (the same from Groovy 3 to 6)
// tells `String s` and `int x` from the call `foo bar`; for a dotted name it
// looks at the last name (`foo.Bar x`, `path_takes_names`). Such a receiver
// takes no name as its first argument, in a value as well, so
// `Point(int x)` and `foo(String a)` hold no call.
static bool may_name_type(const char *word, unsigned len, bool upper) {
    return upper || is_primitive_type(word, len) || word_is(word, len, "void");
}

// Whether a receiver that ends in member accesses, read by
// `read_member_accesses`, takes a name as its first argument: its last name
// does not start with an upper-case letter (`a.b c`, but `foo.Bar x`
// declares) and is not `new` (`this.new Inner()` creates an inner class's
// instance, Apache `DOT NEW creator`).
static inline bool path_takes_names(int32_t last_name, bool ends_in_new) {
    return !is_upper_letter(last_name) && !ends_in_new;
}

// After `!`, which the caller has seen: a negation, the first argument of a
// call (`foo !x`), unless the operator is `!=`, or `!in` or `!instanceof`
// before what makes them operators (§6.12): whitespace or, after `!in`, a
// bracket.
static bool negation_follows(TSLexer *lexer) {
    advance(lexer);
    if (lexer->lookahead == '=') {
        return false;
    }
    if (lexer->lookahead != 'i') {
        return true;
    }
    char word[sizeof "instanceof"];
    unsigned len = read_word(lexer, word, sizeof word);
    int32_t next = lexer->lookahead;
    if (word_is(word, len, "in")) {
        return !is_whitespace(next) && next != '[' && next != '('
            && next != '{';
    }
    return !(word_is(word, len, "instanceof") && is_whitespace(next));
}

// Whether a name that `read_word` returned, with `next` the character after
// it, may be a call's first argument: no keyword Groovy reserves, and
// neither `in` nor `as`, which continue the receiver as operators.
static bool name_may_be_argument(const char *word, unsigned len, int32_t next) {
    return !is_value_keyword(word, len, next) && !word_is(word, len, "in")
        && !word_is(word, len, "as");
}

// Whether a name argument spelled `word` starts what Groovy may read as a
// declaration or a statement of its own when a name follows it: `var`,
// `yield`, `record`, `trait` and `sealed`. `foo var x` is left as before,
// `foo` and the declaration `var x`; Groovy reads `foo(var).x`.
static bool is_declaring_word(const char *word, unsigned len) {
    return word_is(word, len, "var") || word_is(word, len, "yield")
        || word_is(word, len, "record") || word_is(word, len, "trait")
        || word_is(word, len, "sealed");
}

// Whether a number or a string starts with `c`.
static inline bool is_literal_start(int32_t c) {
    return (c >= '0' && c <= '9') || c == '\'' || c == '"';
}

static bool first_argument_follows(
    TSLexer *lexer,
    bool parenthesized,
    bool names,
    bool statement,
    bool no_close_ahead
);

// The trivia after a name argument, which Groovy's lexer hides: spaces,
// tabs, line escapes and block comments, which may span lines; a line
// comment is read to the end of its line. Returns false where the look-ahead
// must stop: at a `/` that opens no comment (division, consumed), and at a
// block comment that does not close (§6.6). A comment is no trivia between
// the receiver and the name argument (§6.14), only after the argument.
static bool skip_argument_trivia(TSLexer *lexer, bool no_close_ahead) {
    bool slash = false;
    if (!skip_header_trivia(lexer, false, no_close_ahead, NULL, &slash)) {
        return false;
    }
    if (!slash) {
        return true;
    }
    if (lexer->lookahead != '/') {
        return false;
    }
    while (!lexer->eof(lexer) && !is_line_break(lexer->lookahead)) {
        advance(lexer);
    }
    return true;
}

// After a name argument that `.` or `[` follows (`foo x.y`, `foo xs[0]`):
// its member accesses, then on the line no first argument as the grammar
// takes it after a name (see `continuation_follows`). In Groovy
// `foo x.y 'a'` is `foo(x.y).a`, a chain continuation, which is left as
// before: `foo` and the call `x.y 'a'`. A subscript is not read past, so
// `foo xs[0] 1` is the call `foo xs[0]` and the statement `1`.
static bool dotted_argument_rest(TSLexer *lexer, bool no_close_ahead) {
    int32_t last_name = 0;
    bool nested = false;
    bool ends_in_new = false;
    if (!read_member_accesses(lexer, &last_name, &nested, &ends_in_new)) {
        return true;
    }
    return !first_argument_follows(lexer, false, false, false, no_close_ahead);
}

// After a name argument, at a name that follows it: whether that name
// starts a chain continuation, which is left as before (§5.5 item 4) —
// a literal word, a key, a creation or an array type value, the first
// argument as the grammar takes it after a name (`foo bar true`,
// `foo bar baz: 1`, `foo bar new Baz()`, `foo bar String[].class`), or a
// name and `=` (`foo bar x = 1`, Groovy's `foo.bar(x = 1)`); in a value
// any name (`def r = a b c` is Groovy's `a(b).c`, and the rest would be no
// member of a class body). After `var`, `yield`, `record`, `trait` or
// `sealed` (`declaring`) any name does. So does a name before parentheses
// that hold parameters or a parenthesis-free call (`call_arguments_follow`):
// `foo bar baz(int a) { }` stays `foo` and a method, which Groovy reads as
// `foo(bar).baz(…)`; inside the one parenthesis already read (`nested`)
// that is not looked into. `in`, `as` and `instanceof` go on with the
// argument as operators, unless a key (`foo bar as: 1`).
static bool continuation_follows(
    TSLexer *lexer,
    bool statement,
    bool declaring,
    bool nested,
    bool no_close_ahead
) {
    bool capitalized = is_capital(lexer->lookahead);
    char word[sizeof "instanceof"];
    unsigned len = read_word(lexer, word, sizeof word);
    if (word_is(word, len, "in") || word_is(word, len, "as")
        || word_is(word, len, "instanceof")) {
        return key_colon_follows(lexer, word, len);
    }
    if (declaring || is_literal_word(word, len)) {
        return true;
    }
    if (word_is(word, len, "new")) {
        return key_colon_follows(lexer, word, len)
            || creation_follows(lexer, no_close_ahead);
    }
    if (lexer->lookahead == '.' || lexer->lookahead == '[') {
        return array_value_follows(lexer, word, len, capitalized,
                                   no_close_ahead)
            || !statement;
    }
    if (key_colon_follows(lexer, word, len)) {
        return true;
    }
    if (lexer->lookahead == '=') {
        advance(lexer);
        return lexer->lookahead != '=' && lexer->lookahead != '~';
    }
    if (lexer->lookahead == '(' && !nested) {
        return !call_arguments_follow(lexer, no_close_ahead);
    }
    return !statement;
}

// At the end of closure parameters, past a line break or a comment: whether
// `->` follows after Groovy's trivia (whitespace, line breaks and comments),
// as Groovy allows them before it. The trivia is read once per call; no
// statement can start inside it, so no restart reads it again.
static bool arrow_follows(TSLexer *lexer, bool no_close_ahead) {
    if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)
        || lexer->lookahead != '-') {
        return false;
    }
    advance(lexer);
    return lexer->lookahead == '>';
}

// Reads a string that closes on its line, from its quote. Returns false at
// a line break or the end of input before the close.
static bool skip_quoted(TSLexer *lexer) {
    int32_t quote = lexer->lookahead;
    advance(lexer);
    while (lexer->lookahead != quote) {
        if (lexer->eof(lexer) || is_line_break(lexer->lookahead)) {
            return false;
        }
        if (lexer->lookahead == '\\') {
            advance(lexer);
            if (lexer->eof(lexer) || is_line_break(lexer->lookahead)) {
                return false;
            }
        }
        advance(lexer);
    }
    advance(lexer);
    return true;
}

// Reads a block comment from its `*`, the `/` before it consumed, to its
// close. Returns false at the end of input, where it does not close.
static bool skip_comment_body(TSLexer *lexer) {
    advance(lexer);
    for (;;) {
        if (lexer->eof(lexer)) {
            return false;
        }
        int32_t c = lexer->lookahead;
        advance(lexer);
        if (c == '*' && lexer->lookahead == '/') {
            advance(lexer);
            return true;
        }
    }
}

// After the `,` of `foo a,` in a statement: whether `->` ends the list,
// which makes it a closure's parameters, the first of type `foo`
// (`{ foo a, b -> }`, `{ foo a, @A b = 1 -> }`). The elements are read as
// Groovy's tokens, bracket depth counted, so that a default value may hold
// anything; comments are trivia. A line break ends the list unless a comma,
// a modifier, an operator or an open bracket comes before it, or `->` after
// it: a call's last argument ends at it (`foo a, b` then the next
// statement). `;`, a closing bracket with none open and the end of the
// budget end the look-ahead with no `->`.
static bool parameters_follow(TSLexer *lexer, bool no_close_ahead) {
    unsigned depth = 0;
    bool continued = true;
    for (unsigned items = 0; items < NAME_ARGUMENT_LOOKAHEAD_ITEMS; items++) {
        if (!skip_same_line_space(lexer, false) || lexer->eof(lexer)) {
            return false;
        }
        int32_t c = lexer->lookahead;
        if (is_line_break(c)) {
            if (depth == 0 && !continued) {
                return arrow_follows(lexer, no_close_ahead);
            }
            advance(lexer);
            continue;
        }
        if (c == '/') {
            advance(lexer);
            if (lexer->lookahead == '/') {
                while (!lexer->eof(lexer) && !is_line_break(lexer->lookahead)) {
                    advance(lexer);
                }
                continue;
            }
            if (lexer->lookahead == '*') {
                if (no_close_ahead || !skip_comment_body(lexer)) {
                    return false;
                }
                continue;
            }
            continued = true;
            continue;
        }
        if (c == '-') {
            advance(lexer);
            if (lexer->lookahead == '>' && depth == 0) {
                return true;
            }
            continued = true;
            continue;
        }
        if (c == '\'' || c == '"') {
            if (!skip_quoted(lexer)) {
                return false;
            }
            continued = false;
            continue;
        }
        if (is_identifier_char(c)) {
            char word[sizeof "synchronized"];
            unsigned len = read_word(lexer, word, sizeof word);
            continued = is_modifier_word(word, len) || word_is(word, len, "def");
            continue;
        }
        if (c == '(' || c == '[' || c == '{') {
            depth++;
        } else if (c == ')' || c == ']' || c == '}') {
            if (depth == 0) {
                return false;
            }
            depth--;
        } else if (c == ';' && depth == 0) {
            return false;
        }
        advance(lexer);
        continued = c != ')' && c != ']' && c != '}';
    }
    return false;
}

// After a name argument and the spaces after it, which `key_colon_follows`
// has skipped: whether the name is all of the first argument's start, which
// it is unless what follows, past comments (`skip_argument_trivia`), makes
// the input something else:
//   - `=` in a statement: `foo bar = 1` declares `bar`, by Apache's rule
//     (`==` and `=~` are operators of the argument);
//   - `->`, or a list that ends in `->`, in a statement, also on a later
//     line: closure parameters, the first of type `foo`;
//   - a literal, or a name that `continuation_follows` takes: in Groovy a
//     further call in the chain, left as before (§5.5 item 4);
//   - `(` with a parenthesis-free call or typed parameters in it: in
//     Groovy `foo bar(baz 1)` is `foo.bar(baz(1))`, and Groovy rejects
//     `foo bar(int a) { }`, which stays a method here. Inside the one
//     parenthesis the look-ahead reads (`nested`), the call is taken as it
//     is, so that no part of the input is read more than a bounded number
//     of times.
// `declaring` is set when the name argument is `var`, `yield`, `record`,
// `trait` or `sealed` (`is_declaring_word`).
static bool name_argument_rest(
    TSLexer *lexer,
    bool statement,
    bool nested,
    bool declaring,
    bool no_close_ahead
) {
    if (!skip_argument_trivia(lexer, no_close_ahead)) {
        return true;
    }
    int32_t c = lexer->lookahead;
    if (c == '(') {
        return nested || call_arguments_follow(lexer, no_close_ahead);
    }
    if (is_literal_start(c)) {
        return false;
    }
    if (is_identifier_start(c)) {
        return !continuation_follows(lexer, statement, declaring, nested,
                                     no_close_ahead);
    }
    if (c == '-') {
        advance(lexer);
        return lexer->lookahead != '>';
    }
    if (!statement) {
        return true;
    }
    if (c == ',') {
        return !parameters_follow(lexer, no_close_ahead);
    }
    if (c == '=') {
        advance(lexer);
        return lexer->lookahead == '=' || lexer->lookahead == '~';
    }
    if (is_line_break(c)) {
        return !arrow_follows(lexer, no_close_ahead);
    }
    return true;
}

// Whether the first argument of a parenthesis-free call starts here, on
// the line: after spaces, tabs and line escapes, a number, a string,
// `true`, `false`, `null`, an object or array creation, the key of a
// named argument or an array type that is a value; or, when
// `parenthesized` is set, such a call in parentheses
// (`parenthesized_command_follows`). When `names` is set, also a name, as
// `name_may_be_argument` and `name_argument_rest` take it, a negation or
// `~`: Apache's argumentList after any expression that a name, `!` or `~`
// cannot continue. `statement` adds the rules of a statement. A quote is
// left unread. A slashy string is not an argument here: after a name
// Groovy reads `/` as division.
static bool first_argument_follows(
    TSLexer *lexer,
    bool parenthesized,
    bool names,
    bool statement,
    bool no_close_ahead
) {
    if (!skip_same_line_space(lexer, false)) {
        return false;
    }
    int32_t c = lexer->lookahead;
    if (is_literal_start(c)) {
        return true;
    }
    if (c == '(') {
        return parenthesized
            && parenthesized_command_follows(lexer, no_close_ahead);
    }
    if (names && c == '!') {
        return negation_follows(lexer);
    }
    if (names && c == '~') {
        return true;
    }
    if (!is_identifier_start(c)) {
        return false;
    }
    bool capitalized = is_capital(c);
    char argument[sizeof "synchronized"];
    unsigned argument_len = read_word(lexer, argument, sizeof argument);
    if (is_literal_word(argument, argument_len)) {
        return true;
    }
    if (word_is(argument, argument_len, "new")) {
        return key_colon_follows(lexer, argument, argument_len)
            || creation_follows(lexer, no_close_ahead);
    }
    int32_t next = lexer->lookahead;
    if (names && (next == '.' || next == '[')) {
        return name_may_be_argument(argument, argument_len, next)
            && dotted_argument_rest(lexer, no_close_ahead);
    }
    // A key is one name; only a nested type name can go on.
    if (next != '.') {
        if (key_colon_follows(lexer, argument, argument_len)) {
            return true;
        }
        if (names) {
            return name_may_be_argument(argument, argument_len, next)
                && name_argument_rest(lexer, statement, !parenthesized,
                                      is_declaring_word(argument, argument_len),
                                      no_close_ahead);
        }
        if (lexer->lookahead != '[') {
            return false;
        }
    }
    return array_value_follows(lexer, argument, argument_len, capitalized,
                               no_close_ahead);
}

// SPECIFICATION.md §6.14 — whether a parenthesis-free call in
// parentheses starts here, as the first argument of another one:
// Groovy reads `println(foo 1)` as `println` called with `(foo 1)`
// (Apache `expressionInPar` as the first element of `argumentList`).
// After the `(` and Groovy's trivia, which inside parentheses includes
// line breaks, the receiver and the first argument that
// COMMAND_VALUE_START reads there. That token opens the inner call, and
// the scanner decides it again once the parser is past the `(`. The
// look-ahead goes into one parenthesis only, so that each part of the
// input is read a bounded number of times: `println(bar(foo 1))` is no
// call of `println`.
static bool parenthesized_command_follows(
    TSLexer *lexer,
    bool no_close_ahead
) {
    lexer->advance(lexer, false);
    if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)
        || !is_identifier_start(lexer->lookahead)) {
        return false;
    }
    bool upper = is_upper_letter(lexer->lookahead);
    char word[sizeof "synchronized"];
    unsigned len = read_word(lexer, word, sizeof word);
    if (is_value_keyword(word, len, lexer->lookahead)) {
        return false;
    }
    bool accesses = is_member_access_start(lexer->lookahead);
    int32_t last_name = 0;
    bool nested = false;
    bool ends_in_new = false;
    if (!read_member_accesses(lexer, &last_name, &nested, &ends_in_new)) {
        return false;
    }
    bool names = accesses ? path_takes_names(last_name, ends_in_new)
        : !may_name_type(word, len, upper);
    return first_argument_follows(lexer, false, names, false, no_close_ahead);
}

// After a type in parentheses, which `may_name_type` or a dotted name with
// an upper-case last name starts: whether its type arguments, dimensions
// and varargs dots, then a name, follow, which makes the parentheses a
// parameter list. Groovy's trivia may come between, line breaks included,
// as inside parentheses. Counts the items it reads in `items`.
static bool parameter_name_follows(
    TSLexer *lexer,
    unsigned *items,
    bool no_close_ahead
) {
    unsigned depth = 0;
    for (; *items < NAME_ARGUMENT_LOOKAHEAD_ITEMS; (*items)++) {
        if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)) {
            return false;
        }
        int32_t c = lexer->lookahead;
        if (is_identifier_start(c)) {
            if (depth == 0) {
                return true;
            }
            read_word(lexer, NULL, 0);
            continue;
        }
        if (c == '<') {
            depth++;
        } else if (c == '>' && depth > 0) {
            depth--;
        } else if (c != '[' && c != ']' && c != '?' && c != '.'
                   && (depth == 0 || (c != ',' && c != '&'))) {
            return false;
        }
        advance(lexer);
    }
    return false;
}

// After a name argument, at the `(` that follows it (`foo bar(x)`):
// whether the parentheses hold the arguments of a call of that name, which
// is then all of the first argument, as in Groovy (`foo(bar(x))`). They do
// not when they hold a parenthesis-free call (`foo bar(baz 1)`, Groovy's
// `foo.bar(baz(1))`, also `bar(println` then `List)` on the next line, as
// Groovy's lexer drops line breaks there) or parameters
// (`foo bar(int a) { }`, `foo bar(a, String... b) { }`, a method
// declaration here, which Groovy rejects): an element that is a type, as
// `may_name_type` reads it, followed by a name, or that starts with an
// annotation, `final` or `def`. The elements are read up to the first one
// that is neither a parameter nor a lone name, within the budget of
// `NAME_ARGUMENT_LOOKAHEAD_ITEMS`; at its end they are arguments.
static bool call_arguments_follow(TSLexer *lexer, bool no_close_ahead) {
    unsigned items = 0;
    for (bool first = true; items < NAME_ARGUMENT_LOOKAHEAD_ITEMS;
         first = false, items++) {
        advance(lexer);
        if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)
            || lexer->lookahead == '@') {
            return false;
        }
        if (!is_identifier_start(lexer->lookahead)) {
            return true;
        }
        bool upper = is_upper_letter(lexer->lookahead);
        char word[sizeof "synchronized"];
        unsigned len = read_word(lexer, word, sizeof word);
        if (word_is(word, len, "final") || word_is(word, len, "def")) {
            return false;
        }
        if (is_value_keyword(word, len, lexer->lookahead)) {
            return true;
        }
        bool accesses = is_member_access_start(lexer->lookahead);
        int32_t last_name = 0;
        bool nested = false;
        bool ends_in_new = false;
        bool read = read_member_accesses(lexer, &last_name, &nested,
                                         &ends_in_new);
        // `String... a`: the accesses stop at the second dot.
        bool varargs = !read && lexer->lookahead == '.';
        if (!read && !varargs) {
            return true;
        }
        bool type = accesses && last_name != 0
            ? !path_takes_names(last_name, ends_in_new)
            : may_name_type(word, len, upper);
        if (type) {
            return !parameter_name_follows(lexer, &items, no_close_ahead);
        }
        if (varargs) {
            return true;
        }
        if (first && first_argument_follows(lexer, false, true, false,
                                            no_close_ahead)) {
            return false;
        }
        if (!skip_same_line_space(lexer, false)) {
            return true;
        }
        if (is_line_break(lexer->lookahead)) {
            return !skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)
                || !is_identifier_start(lexer->lookahead);
        }
        if (lexer->lookahead != ',') {
            return true;
        }
    }
    return true;
}

// SPECIFICATION.md §6.17 — after the name of a lambda's one parameter
// without parentheses and the spaces after it: whether `->` follows, also
// after line breaks and comments (Apache `NL* ARROW`). Only a `-`, a line
// break or a `/` may start the arrow or the trivia before it; at any other
// character this returns false and reads nothing. Otherwise it sets `*read`,
// as it consumes input. None of the three starts the first argument of a
// call (`first_argument_follows`), so a value that is no lambda there is no
// call either.
static bool bare_lambda_arrow_follows(
    TSLexer *lexer,
    bool no_close_ahead,
    bool *read
) {
    int32_t c = lexer->lookahead;
    if (c != '-' && c != '/' && !is_line_break(c)) {
        return false;
    }
    *read = true;
    return arrow_follows(lexer, no_close_ahead);
}

// SPECIFICATION.md §6.17 — decides BARE_LAMBDA_START, an empty token before
// the name of a lambda's one parameter without parentheses (`x -> x`),
// where no call that is a value may start: in an argument, the value of a
// map entry and a closure parameter's default. The name is read once, so
// ARRAY_VALUE_START, valid at the same place (`foo(String[].class)`), is
// decided here too. A name that cannot be a parameter's is checked only
// once the arrow is found, so that the other names cost no comparison.
// The caller has skipped whitespace and seen an identifier start.
static bool scan_bare_lambda_start(
    TSLexer *lexer,
    bool wants_array,
    bool type_may_start,
    bool no_close_ahead
) {
    bool capitalized = is_capital(lexer->lookahead);
    char word[sizeof "synchronized"];
    unsigned len = read_word(lexer, word, sizeof word);
    if (lexer->lookahead != '.' && lexer->lookahead != '[') {
        if (!skip_same_line_space(lexer, false)) {
            return false;
        }
        bool read = false;
        if (bare_lambda_arrow_follows(lexer, no_close_ahead, &read)) {
            lexer->result_symbol = BARE_LAMBDA_START;
            return !is_reserved_bare_lambda_name(word, len);
        }
        if (read) {
            return false;
        }
    }
    // `read_element_type_name`, from the end of the name.
    if (!wants_array
        || (capitalized ? !read_nested_type_names(lexer)
                        : !is_primitive_type(word, len))
        || !array_dimensions_follow(lexer, type_may_start, no_close_ahead)) {
        return false;
    }
    lexer->result_symbol = ARRAY_VALUE_START;
    return true;
}

// SPECIFICATION.md §6.14 — decides that a parenthesis-free call starts
// here as a value: of a declarator or an assignment
// (`def out = sh script: 'make'`, `def r = obj.method 1`), or in
// parentheses or an interpolation (`(foo 1)`, `"${foo 1}"`): a name and any
// member accesses written without spaces, then on its line the first
// argument (`first_argument_follows`), a name included unless the receiver
// may name a type (`def x = foo bar`, but not `def x = Foo bar`). The
// token is empty and sits before the name, which the in-grammar lexer
// reads, as it reads the argument. Taking the decision at the name, not
// after it, keeps a line break hidden by a comment from joining two
// statements: in `def x = foo` followed by `/* c */ 1` on the next line,
// the token after the comment would no longer see the line break.
//
// §6.15 — the same name may start an array type that is a value
// (`def c = String[].class`), and ARRAY_VALUE_START, which sits at the
// same place, is decided here too: the two need different things after
// the names, `[` and an argument.
//
// The caller has skipped whitespace and seen an identifier start.
static bool scan_value_start(
    TSLexer *lexer,
    bool wants_command,
    bool wants_bare_lambda,
    bool wants_array,
    bool type_may_start,
    bool no_close_ahead
) {
    lexer->mark_end(lexer);
    if (!wants_command) {
        if (wants_bare_lambda) {
            return scan_bare_lambda_start(lexer, wants_array, type_may_start,
                                          no_close_ahead);
        }
        if (!read_element_type_name(lexer)
            || !array_dimensions_follow(lexer, type_may_start, no_close_ahead)) {
            return false;
        }
        lexer->result_symbol = ARRAY_VALUE_START;
        return true;
    }
    bool capitalized = is_capital(lexer->lookahead);
    bool upper = is_upper_letter(lexer->lookahead);
    char word[sizeof "synchronized"];
    unsigned len = read_word(lexer, word, sizeof word);
    if (is_value_keyword(word, len, lexer->lookahead)) {
        return false;
    }
    bool accesses = is_member_access_start(lexer->lookahead);
    if (wants_bare_lambda && !accesses && lexer->lookahead != '[') {
        if (!skip_same_line_space(lexer, false)) {
            return false;
        }
        bool read = false;
        if (bare_lambda_arrow_follows(lexer, no_close_ahead, &read)) {
            lexer->result_symbol = BARE_LAMBDA_START;
            return !is_reserved_bare_lambda_name(word, len);
        }
        if (read) {
            return false;
        }
    }
    int32_t last_name = 0;
    bool nested = false;
    bool ends_in_new = false;
    if (!read_member_accesses(lexer, &last_name, &nested, &ends_in_new)
        || !skip_same_line_space(lexer, false)) {
        return false;
    }
    if (lexer->lookahead == '[') {
        bool element = capitalized
            ? nested
            : !accesses && is_primitive_type(word, len);
        if (!wants_array || !element
            || !array_dimensions_follow(lexer, type_may_start, no_close_ahead)) {
            return false;
        }
        lexer->result_symbol = ARRAY_VALUE_START;
        return true;
    }
    bool names = accesses ? path_takes_names(last_name, ends_in_new)
        : !may_name_type(word, len, upper);
    if (!first_argument_follows(lexer, true, names, false, no_close_ahead)) {
        return false;
    }
    lexer->result_symbol = COMMAND_VALUE_START;
    return true;
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
// first argument must start on their line. A name is one when the last
// name of the receiver is not capitalized, Apache's rule for a statement
// (`a.b c` calls, `foo.Bar x` declares). The caller has seen a member
// access start.
static bool scan_command_path_start(TSLexer *lexer, bool no_close_ahead) {
    lexer->mark_end(lexer);
    int32_t last_name = 0;
    bool nested = false;
    bool ends_in_new = false;
    if (!read_member_accesses(lexer, &last_name, &nested, &ends_in_new)
        || !skip_same_line_space(lexer, false)) {
        return false;
    }
    bool quoted = lexer->lookahead == '\'' || lexer->lookahead == '"';
    if (!first_argument_follows(lexer, true,
                                path_takes_names(last_name, ends_in_new), true,
                                no_close_ahead)) {
        return false;
    }
    // Groovy declares a method when the last name is capitalized
    // (`java.util.List 'foo'() { }`) and calls one otherwise
    // (`obj.m 'a'() { }` is `obj.m('a'({ }))`).
    bool capitalized = last_name >= 'A' && last_name <= 'Z';
    if (capitalized && quoted && method_name_follows(lexer)) {
        return false;
    }
    lexer->result_symbol = COMMAND_PATH_START;
    return true;
}

// Skips the trivia Groovy's lexer hides inside a declaration header:
// spaces, tabs, line escapes and block comments, plus line breaks and
// line comments when `newlines` is set. Returns false on a `/` that
// opens neither comment kind (a slashy string or division cannot be
// part of a header), unless `slash` is not NULL: that `/` is then
// consumed, `*slash` is set and the result is true. Returns false on a
// `\` that is not a line escape, and on an
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
    bool *crossed_line,
    bool *slash
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
        } else if (slash != NULL) {
            *slash = true;
            return true;
        } else {
            return false;
        }
    }
}

// SPECIFICATION.md §6.15 — whether the dimensions of an array type start
// here, after the name: on the name's line `[`, then `]` with Groovy's
// trivia between, as inside brackets its lexer drops line breaks and
// comments. `String[0]` and `a[i]` are subscripts.
//
// Where a type may also start (TYPE_START valid: at a statement, after
// `(` or after `return`, which a statement may follow), `String[] xs`
// and `(String[]) x` keep their reading. There every dimension is read,
// each further `[` on the line of the `]` before it, and what follows
// them must be what no type is: a member access (`.class`, `.&new`) or
// a method reference (`::new`).
static bool array_dimensions_follow(
    TSLexer *lexer,
    bool type_may_start,
    bool no_close_ahead
) {
    do {
        if (!skip_same_line_space(lexer, false) || lexer->lookahead != '[') {
            return false;
        }
        advance(lexer);
        if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)
            || lexer->lookahead != ']') {
            return false;
        }
        if (!type_may_start) {
            return true;
        }
        advance(lexer);
        if (!skip_same_line_space(lexer, false)) {
            return false;
        }
    } while (lexer->lookahead == '[');
    if (lexer->lookahead == ':') {
        advance(lexer);
        return lexer->lookahead == ':';
    }
    if (lexer->lookahead != '.') {
        return false;
    }
    advance(lexer);
    return lexer->lookahead == '&' || is_identifier_start(lexer->lookahead);
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
    if (!skip_header_trivia(lexer, false, no_close_ahead, NULL, NULL)) {
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
    if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)) {
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
    if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)) {
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
    if (!skip_header_trivia(lexer, true, no_close_ahead, &crossed_line, NULL)) {
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

// SPECIFICATION.md §6.16 — whether `instanceof` and a type follow, read
// from here: a name's test. Groovy never takes `instanceof` as a name, so
// `var instanceof T` and `yield instanceof T` test the word before it.
// Where no type follows (`var instanceof = 1`, which Groovy rejects), the
// word before it keeps the reading it had when it was always a keyword.
static bool instanceof_test_follows(TSLexer *lexer, bool no_close_ahead) {
    return consume_keyword(lexer, "instanceof")
        && skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)
        && is_identifier_start(lexer->lookahead);
}

// SPECIFICATION.md §6.16 — whether an expression starts here, after
// Groovy's trivia, line breaks and comments included: what `yield` and
// `in` need after them to be keywords. The characters are those an
// expression of the grammar can start with, except that `instanceof` or
// `!instanceof` with a type after it tests the word before it. `!`, `+`
// and `-` start one unless they are `!=`, `+=`, `-=` or `->`. `++` and
// `--` start one only when an operand follows them on their line, so
// `in++` and `yield++` update a name, also when `foo()` follows on the
// next line, as in Groovy; they are read in a loop, not by recursion,
// which a long run of them would turn into a deep stack. `()` starts
// none, so `yield()` is a call. A `/` starts a slashy string unless a
// space, `=` or a second `/` (a line comment, read on the line of a `++`)
// follows it.
static bool expression_follows(TSLexer *lexer, bool no_close_ahead) {
    bool same_line = false;
    for (;;) {
        bool slash = false;
        if (!skip_header_trivia(lexer, !same_line, no_close_ahead, NULL,
                                &slash)) {
            return false;
        }
        int32_t c = lexer->lookahead;
        if (slash) {
            return !lexer->eof(lexer) && !is_whitespace(c) && c != '='
                && c != '/';
        }
        if (c == 'i') {
            return !instanceof_test_follows(lexer, no_close_ahead);
        }
        if (is_identifier_start(c) || (c >= '0' && c <= '9') || c == '\''
            || c == '"' || c == '[' || c == '{' || c == '~') {
            return true;
        }
        if (c != '(' && c != '!' && c != '+' && c != '-') {
            return false;
        }
        advance(lexer);
        if (c == '(') {
            return !skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)
                || lexer->lookahead != ')';
        }
        if (c == '!') {
            // `!instanceof` is one operator only before whitespace, as the
            // grammar's token is (GroovyLexer.g4 `NOT_INSTANCEOF`).
            if (lexer->lookahead == 'i') {
                return !consume_keyword(lexer, "instanceof")
                    || !is_whitespace(lexer->lookahead)
                    || !skip_header_trivia(lexer, true, no_close_ahead, NULL,
                                           NULL)
                    || !is_identifier_start(lexer->lookahead);
            }
            return lexer->lookahead != '=';
        }
        if (lexer->lookahead != c) {
            return lexer->lookahead != '=' && lexer->lookahead != '>';
        }
        advance(lexer);
        same_line = true;
    }
}

// SPECIFICATION.md §6.16 — the names of a multi-assignment declaration
// opened by `var` alone, from its `(`, then `)` and `=`: `var (a, b) = l`.
// `var(a, b)` and `var (a, b).foo()` call a method named `var`. Only what
// a list of names and types holds is read: names, `,`, `*` (a rest
// binding), `:` (a keyed pair), `.`, `<`, `>`, `?`, `[` and `]`, with
// Groovy's trivia. Any other character ends the look-ahead, so it reads
// no `(` and cannot read the same text twice. Line breaks are allowed
// inside the parentheses and before the `=`, as in Groovy.
static bool names_assignment_follows(TSLexer *lexer, bool no_close_ahead) {
    advance(lexer);
    for (;;) {
        if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)) {
            return false;
        }
        int32_t c = lexer->lookahead;
        if (is_identifier_start(c)) {
            read_word(lexer, NULL, 0);
            continue;
        }
        if (c == ')') {
            break;
        }
        if (c != ',' && c != '*' && c != ':' && c != '.' && c != '<'
            && c != '>' && c != '?' && c != '[' && c != ']') {
            return false;
        }
        advance(lexer);
    }
    advance(lexer);
    if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)
        || lexer->lookahead != '=') {
        return false;
    }
    advance(lexer);
    return lexer->lookahead != '=' && lexer->lookahead != '~';
}

// SPECIFICATION.md §6.16 — `var` declares the name that follows it, on
// the line or a later one (`var x = 1`, `def f(var x)`), as it did when
// the word was always a keyword. `var instanceof T` tests `var`, as in
// Groovy (`instanceof_test_follows`). Before `(` it opens a
// multi-assignment: where `var` alone may open one (VAR_NAMES_KEYWORD
// valid), only when the names, `)` and `=` follow; after `final`, always.
// Anywhere else the word is a name.
static bool scan_var(
    TSLexer *lexer,
    const bool *valid_symbols,
    bool no_close_ahead
) {
    if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)) {
        return false;
    }
    if (is_identifier_start(lexer->lookahead)) {
        if (lexer->lookahead == 'i'
            && instanceof_test_follows(lexer, no_close_ahead)) {
            return false;
        }
        lexer->result_symbol = VAR_KEYWORD;
        return valid_symbols[VAR_KEYWORD];
    }
    if (lexer->lookahead != '(') {
        return false;
    }
    if (valid_symbols[VAR_NAMES_KEYWORD]) {
        lexer->result_symbol = VAR_NAMES_KEYWORD;
        return names_assignment_follows(lexer, no_close_ahead);
    }
    lexer->result_symbol = VAR_KEYWORD;
    return valid_symbols[VAR_KEYWORD];
}

// Whether `word` is one of the contextual keywords of §6.16.
static bool is_contextual_word(const char *word, unsigned len) {
    return word_is(word, len, "var") || word_is(word, len, "yield")
        || word_is(word, len, "in") || word_is(word, len, "as");
}

// Whether `c` may start one of the words of §6.16 whose token is valid.
static inline bool contextual_word_may_start(
    int32_t c,
    const bool *valid_symbols
) {
    switch (c) {
        case 'v':
            return valid_symbols[VAR_KEYWORD]
                || valid_symbols[VAR_NAMES_KEYWORD];
        case 'y':
            return valid_symbols[YIELD_KEYWORD];
        case 'i':
            return valid_symbols[IN_KEYWORD];
        case 'a':
            return valid_symbols[AS_KEYWORD];
        default:
            return false;
    }
}

// Consumes the word that starts here for as long as it can still be
// `var`, `yield`, `in`, `as` or `int`, and returns which of them it is,
// or NULL. The words that start a statement after another one (`if`,
// `it`, `implementation`, `apply`) are turned down at their first or
// second character, and are not read twice.
static const char *read_contextual_word(TSLexer *lexer) {
    const char *word = NULL;
    switch (lexer->lookahead) {
        case 'v':
            word = "var";
            break;
        case 'y':
            word = "yield";
            break;
        case 'i':
            word = "in";
            break;
        case 'a':
            word = "as";
            break;
        default:
            return NULL;
    }
    for (const char *c = word; *c != '\0'; c++) {
        if (lexer->lookahead != *c) {
            return NULL;
        }
        advance(lexer);
    }
    if (word[0] == 'i' && lexer->lookahead == 't') {
        advance(lexer);
        word = "int";
    }
    return is_identifier_char(lexer->lookahead) ? NULL : word;
}

// SPECIFICATION.md §6.16 — decides the keyword for `var`, `yield`, `in`
// or `as`, which the caller has consumed and marked as the token's end.
// Apache's `identifier` rule lists each word, so each is a name wherever
// its construct does not follow: `yield` and `in` need an expression
// after them, `as` a type, `var` a name or a multi-assignment's names.
// Line breaks and comments before those count as nothing, as they did
// when the words were grammar literals. On false the in-grammar lexer
// reads the word as an `identifier`.
static bool scan_contextual_word(
    TSLexer *lexer,
    const char *word,
    unsigned len,
    const bool *valid_symbols,
    bool no_close_ahead
) {
    if (word_is(word, len, "var")) {
        return (valid_symbols[VAR_KEYWORD] || valid_symbols[VAR_NAMES_KEYWORD])
            && scan_var(lexer, valid_symbols, no_close_ahead);
    }
    if (word_is(word, len, "yield")) {
        lexer->result_symbol = YIELD_KEYWORD;
        return valid_symbols[YIELD_KEYWORD]
            && expression_follows(lexer, no_close_ahead);
    }
    if (word_is(word, len, "in")) {
        lexer->result_symbol = IN_KEYWORD;
        return valid_symbols[IN_KEYWORD]
            && expression_follows(lexer, no_close_ahead);
    }
    if (word_is(word, len, "as")) {
        lexer->result_symbol = AS_KEYWORD;
        return valid_symbols[AS_KEYWORD]
            && skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)
            && is_identifier_start(lexer->lookahead);
    }
    return false;
}

// The keywords of grammar.js that a name can spell. After
// COMMAND_STATEMENT_START the parser takes only a name, so the in-grammar
// lexer would read any of them as one: `return new Foo()` would call a
// method named `return`. The contextual words (§6.7, §6.13, §6.16) are
// not keywords there and are decided before this list is read.
// bindings/rust/tests/scanner_keywords.rs checks the words against
// src/grammar.json.
static bool is_grammar_keyword(const char *word, unsigned len) {
    switch (word[0]) {
        case '_':
            return word_is(word, len, "_");
        case 'a':
            return word_is(word, len, "abstract")
                || word_is(word, len, "assert");
        case 'b':
            return word_is(word, len, "break");
        case 'c':
            return word_is(word, len, "case") || word_is(word, len, "catch")
                || word_is(word, len, "class")
                || word_is(word, len, "continue");
        case 'd':
            return word_is(word, len, "def") || word_is(word, len, "default")
                || word_is(word, len, "do");
        case 'e':
            return word_is(word, len, "else") || word_is(word, len, "enum")
                || word_is(word, len, "extends");
        case 'f':
            return word_is(word, len, "false") || word_is(word, len, "final")
                || word_is(word, len, "finally") || word_is(word, len, "for");
        case 'i':
            return word_is(word, len, "if")
                || word_is(word, len, "implements")
                || word_is(word, len, "import")
                || word_is(word, len, "instanceof")
                || word_is(word, len, "interface");
        case 'n':
            return word_is(word, len, "native") || word_is(word, len, "new")
                || word_is(word, len, "null");
        case 'p':
            return word_is(word, len, "package")
                || word_is(word, len, "permits")
                || word_is(word, len, "pipeline")
                || word_is(word, len, "private")
                || word_is(word, len, "protected")
                || word_is(word, len, "public");
        case 'r':
            return word_is(word, len, "return");
        case 's':
            return word_is(word, len, "static")
                || word_is(word, len, "strictfp")
                || word_is(word, len, "super") || word_is(word, len, "switch")
                || word_is(word, len, "synchronized");
        case 't':
            return word_is(word, len, "throw") || word_is(word, len, "throws")
                || word_is(word, len, "transient")
                || word_is(word, len, "true") || word_is(word, len, "try");
        case 'v':
            return word_is(word, len, "volatile");
        case 'w':
            return word_is(word, len, "while");
        default:
            return false;
    }
}

// Whether a word that `read_word` returned, with `next` the character
// after it, is a keyword that cannot be the first name of a value's
// receiver: a keyword of grammar.js that Groovy reserves, among them the
// words that start a value as keywords (`new`, `switch`, `true`, `false`,
// `null`). Groovy calls no method named `return` in `def x = return 1`,
// and reads the braces of an interpolation that holds one as a closure
// with a statement. It takes `super`, `permits`, `pipeline` and `_` as
// names, and `static` before a `.` (Apache `pathExpression`), so
// `def x = super.foo 1` and `def x = static.foo 1` are calls.
static bool is_value_keyword(const char *word, unsigned len, int32_t next) {
    if (word_is(word, len, "static")) {
        return next != '.';
    }
    return is_grammar_keyword(word, len) && !word_is(word, len, "super")
        && !word_is(word, len, "permits") && !word_is(word, len, "pipeline")
        && !word_is(word, len, "_");
}

// SPECIFICATION.md §6.17 — whether a word that `read_word` returned cannot
// name a lambda's parameter: a keyword of grammar.js that Groovy reserves
// (`null`, `new`, `if`; `_`, `permits` and `pipeline` are names), a
// primitive type name or `void`, which Groovy's `variableDeclaratorId`
// takes only to report the mistake. Taken as a parameter, the in-grammar
// lexer would read any of them as a name.
static bool is_reserved_lambda_name(const char *word, unsigned len) {
    if (is_primitive_type(word, len) || word_is(word, len, "void")) {
        return true;
    }
    return is_grammar_keyword(word, len) && !word_is(word, len, "_")
        && !word_is(word, len, "permits") && !word_is(word, len, "pipeline");
}

// The same for the one parameter without parentheses, where `this` is no
// name either; with them a type may come first (`(String this) -> 1`).
static bool is_reserved_bare_lambda_name(const char *word, unsigned len) {
    return is_reserved_lambda_name(word, len) || word_is(word, len, "this");
}

// Whether the first argument of a statement's parenthesis-free call that
// opens with COMMAND_STATEMENT_START starts here, after the spaces after
// the receiver: an object or array creation, an array type that is a
// value, or such a call in parentheses (`println(foo 1)`); and, when the
// receiver takes `names`, a name, a negation or `~` as
// `first_argument_follows` takes them in a statement (`println x`,
// `foo !x`). A literal and the key of a named argument (`foo 1`,
// `foo a: 1`, `foo new: 1`) are not: the grammar's form takes them. Without
// `names`, a lowercase name is read only while it can still be `new` or a
// primitive type name.
static bool statement_argument_follows(
    TSLexer *lexer,
    bool names,
    bool no_close_ahead
) {
    int32_t c = lexer->lookahead;
    if (c == '(') {
        return parenthesized_command_follows(lexer, no_close_ahead);
    }
    if (!names) {
        if (c == 'n') {
            return consume_keyword(lexer, "new")
                && creation_follows(lexer, no_close_ahead);
        }
        return read_element_type_name(lexer)
            && array_dimensions_follow(lexer, true, no_close_ahead);
    }
    if (c == '!') {
        return negation_follows(lexer);
    }
    if (c == '~') {
        return true;
    }
    if (!is_identifier_start(c)) {
        return false;
    }
    char word[sizeof "synchronized"];
    unsigned len = read_word(lexer, word, sizeof word);
    if (word_is(word, len, "new")) {
        return creation_follows(lexer, no_close_ahead);
    }
    int32_t next = lexer->lookahead;
    if (next == '.' || next == '[') {
        return name_may_be_argument(word, len, next)
            && dotted_argument_rest(lexer, no_close_ahead);
    }
    return !key_colon_follows(lexer, word, len)
        && name_may_be_argument(word, len, next)
        && name_argument_rest(lexer, true, false, is_declaring_word(word, len),
                              no_close_ahead);
}

// SPECIFICATION.md §6.14 — decides at the first name of a statement,
// before it, whether the statement is a parenthesis-free call whose
// receiver is that name and whose first argument starts with `new`, is
// an array type that is a value (`foo new Bar()`, `foo String[].class`) or
// is a name (`println x`), on the receiver's line. A name is one only after
// a receiver that may not name a type (`may_name_type`) and is no
// contextual word. Literal and key arguments are matched in the
// grammar instead (§5.5 item 3), and a dotted receiver by
// COMMAND_PATH_START. The token is empty and the in-grammar lexer reads
// the name. Deciding here rather than after the name keeps a comment
// that opens the next line from hiding the line break: after `foo` and
// `/* c */` on the next line the scanner no longer sees one.
//
// The same name may be a contextual keyword (§6.7, §6.13, §6.16) or start
// an array type that is a value (§6.15), whose tokens are valid at a
// statement start as well. The scanner cannot read the name twice, so
// each is decided here, as the branches of the scan function decide it
// where no statement may start.
//
// The caller has skipped whitespace and seen an identifier start.
static bool scan_statement_start(
    TSLexer *lexer,
    const bool *valid_symbols,
    bool wants_array,
    bool type_may_start,
    bool no_close_ahead
) {
    lexer->mark_end(lexer);
    bool capitalized = is_capital(lexer->lookahead);
    bool upper = is_upper_letter(lexer->lookahead);
    char word[sizeof "synchronized"];
    unsigned len = read_word(lexer, word, sizeof word);
    if (word_is(word, len, "record") && valid_symbols[RECORD_KEYWORD]) {
        lexer->result_symbol = RECORD_KEYWORD;
        return scan_record_header(lexer, no_close_ahead);
    }
    if (word_is(word, len, "trait") && valid_symbols[TRAIT_KEYWORD]) {
        lexer->result_symbol = TRAIT_KEYWORD;
        return scan_trait_header(lexer, no_close_ahead);
    }
    if (word_is(word, len, "sealed") && valid_symbols[SEALED_KEYWORD]) {
        lexer->result_symbol = SEALED_KEYWORD;
        return scan_sealed_target(lexer, no_close_ahead);
    }
    if (is_contextual_word(word, len)
        && contextual_word_may_start(word[0], valid_symbols)) {
        lexer->mark_end(lexer);
        return scan_contextual_word(lexer, word, len, valid_symbols,
                                    no_close_ahead);
    }
    // A keyword is no receiver or element type of an array type, so the
    // look-ahead past one would decide nothing, except before a call in
    // parentheses for the keywords Groovy takes as names: `super(foo 1)` in
    // a constructor calls, as `this(foo 1)` does. Before `new` or an array
    // type value they stay the name and then the argument.
    if (is_grammar_keyword(word, len)) {
        if (is_value_keyword(word, len, lexer->lookahead)
            || !skip_same_line_space(lexer, false) || lexer->lookahead != '('
            || !parenthesized_command_follows(lexer, no_close_ahead)) {
            return false;
        }
        lexer->result_symbol = COMMAND_STATEMENT_START;
        return true;
    }
    if (lexer->lookahead == '.') {
        // A dotted receiver is decided after the first name.
        if (!wants_array || !capitalized || !read_nested_type_names(lexer)
            || !array_dimensions_follow(lexer, type_may_start, no_close_ahead)) {
            return false;
        }
        lexer->result_symbol = ARRAY_VALUE_START;
        return true;
    }
    if (!skip_same_line_space(lexer, false)) {
        return false;
    }
    if (lexer->lookahead == '[') {
        if (!wants_array || (!capitalized && !is_primitive_type(word, len))
            || !array_dimensions_follow(lexer, type_may_start, no_close_ahead)) {
            return false;
        }
        lexer->result_symbol = ARRAY_VALUE_START;
        return true;
    }
    // `var`, `yield`, `in` or `as` whose own token was not valid here takes
    // no name either: whether that token is valid depends on the token
    // before it (`in` and `as` follow an expression), which an incremental
    // reparse need not restore, so the reading would too.
    bool names = !may_name_type(word, len, upper)
        && !is_contextual_word(word, len);
    if (!statement_argument_follows(lexer, names, no_close_ahead)) {
        return false;
    }
    lexer->result_symbol = COMMAND_STATEMENT_START;
    return true;
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

// SPECIFICATION.md §6.17 — the most the look-ahead for a lambda's
// parameters reads, in items: a word, a string, a comment, an annotation or
// a punctuation character. It runs at every `(` that may start an
// expression, and the `(` of a default value may start one too, so without a
// bound `(a = (a = (a = …` would be read in time quadratic in its depth.
// With it each `(` reads at most this many items, and the parse stays
// linear. A parameter list longer than this is read as no lambda's; real
// ones are a few items.
static const unsigned LAMBDA_PARAMETERS_LOOKAHEAD_ITEMS = 256;

// Counts one item of the look-ahead for a lambda's parameters. Returns false
// once the budget is spent.
static inline bool spend_lambda_item(unsigned *items) {
    if (*items >= LAMBDA_PARAMETERS_LOOKAHEAD_ITEMS) {
        return false;
    }
    (*items)++;
    return true;
}

// Reads Groovy's tokens, counting brackets, up to a `)` with no bracket
// open, and, when `comma` is set, a `,` with none open; it leaves that
// character. That is a parameter's default value (`comma`) or the arguments
// of an annotation, from after their `(`. Strings that close on their line
// and comments are items of their own, and line breaks are trivia, as inside
// parentheses for Groovy's lexer. Returns false at the end of input, an
// unterminated block comment, a closing bracket that does not match and the
// end of the budget.
static bool skip_lambda_tokens(
    TSLexer *lexer,
    unsigned *items,
    bool comma,
    bool no_close_ahead
) {
    unsigned depth = 0;
    while (spend_lambda_item(items)) {
        bool slash = false;
        if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, &slash)
            || lexer->eof(lexer)) {
            return false;
        }
        if (slash) {
            continue;
        }
        int32_t c = lexer->lookahead;
        if (c == '\'' || c == '"') {
            if (!skip_quoted(lexer)) {
                return false;
            }
            continue;
        }
        if (is_identifier_char(c)) {
            read_word(lexer, NULL, 0);
            continue;
        }
        if (c == '(' || c == '[' || c == '{') {
            depth++;
        } else if (c == ')' || c == ']' || c == '}') {
            if (depth == 0) {
                return c == ')';
            }
            depth--;
        } else if (c == ',' && depth == 0 && comma) {
            return true;
        }
        advance(lexer);
    }
    return false;
}

// Reads one parameter of a lambda, with the trivia around it, and leaves
// what follows it: annotations, each a name and arguments in parentheses;
// words (`final`, `def`, a type's names, the parameter's name) and the
// punctuation of a type (`.`, `...`, `[]`, type arguments in `<…>`); then a
// default value after `=`. Apart from the name being last, it does not check
// the order: what it takes and `formal_parameter` does not is an `ERROR`
// either way. Returns false where no parameter can be, as at the operator of
// `(a + b)`, the call of `(a.b())` and the literal of `(1)`, and where a
// keyword is in it (`(if)`). Where no name ends it (`(String[])`, `(a.b)`,
// `(int)`, `(final)`), it clears `*named` and goes on: those are a cast's
// types, and the caller reads on past the `)` all the same.
static bool lambda_parameter_follows(
    TSLexer *lexer,
    unsigned *items,
    bool *named_parameter,
    bool no_close_ahead
) {
    for (;;) {
        if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)) {
            return false;
        }
        if (lexer->lookahead != '@') {
            break;
        }
        if (!spend_lambda_item(items)) {
            return false;
        }
        advance(lexer);
        if (!is_identifier_start(lexer->lookahead)) {
            return false;
        }
        read_word(lexer, NULL, 0);
        while (lexer->lookahead == '.') {
            advance(lexer);
            if (!is_identifier_start(lexer->lookahead)) {
                return false;
            }
            read_word(lexer, NULL, 0);
        }
        if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)) {
            return false;
        }
        if (lexer->lookahead == '(') {
            advance(lexer);
            if (!skip_lambda_tokens(lexer, items, false, no_close_ahead)) {
                return false;
            }
            advance(lexer);
        }
    }
    unsigned angles = 0;
    // Whether the last item is a word that may name the parameter, and how
    // many dots come right before the current item: after one, a word is
    // part of a qualified type (`a.b.C`); after three, a name (`String... a`).
    bool named = false;
    unsigned dots = 0;
    for (;;) {
        int32_t c = lexer->lookahead;
        if (is_identifier_start(c)) {
            char word[sizeof "synchronized"];
            unsigned len = read_word(lexer, word, sizeof word);
            bool modifier = word_is(word, len, "final")
                || word_is(word, len, "def");
            if (!modifier && is_reserved_lambda_name(word, len)
                && !is_primitive_type(word, len)) {
                return false;
            }
            named = !modifier && angles == 0 && dots != 1
                && !is_reserved_lambda_name(word, len);
            dots = 0;
        } else if (c == '.') {
            named = false;
            dots++;
            advance(lexer);
        } else if (c == '<' || (c == '>' && angles > 0) || c == '[' || c == ']'
                   || (angles > 0 && (c == ',' || c == '?' || c == '&'))) {
            if (c == '<') {
                angles++;
            } else if (c == '>') {
                angles--;
            }
            named = false;
            dots = 0;
            advance(lexer);
        } else {
            break;
        }
        if (!spend_lambda_item(items)
            || !skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)) {
            return false;
        }
    }
    if (angles > 0) {
        return false;
    }
    if (!named) {
        *named_parameter = false;
    }
    if (lexer->lookahead != '=') {
        return true;
    }
    advance(lexer);
    if (lexer->lookahead == '=' || lexer->lookahead == '~') {
        return false;
    }
    return skip_lambda_tokens(lexer, items, true, no_close_ahead);
}

// SPECIFICATION.md §6.17 — whether the `(` here opens a lambda's parameters:
// what the parentheses hold reads as parameters (`lambda_parameter_follows`)
// and `->` follows the `)`, after Groovy's trivia, line breaks included
// (Apache `NL* ARROW`). The `(` is not consumed when this is called.
//
// A cast's type without a name (`(int)`, `(a.b.C)`) is read through the `)`
// and the trivia after it as well, and turned down only then. The runtime
// counts what the scanner read against the `(`, so an edit in that trivia,
// such as closing a comment that did not close (§6.6), makes it lex the `(`
// again, as it does after `(String)`.
static bool lambda_parameters_follow(TSLexer *lexer, bool no_close_ahead) {
    unsigned items = 0;
    bool named = true;
    advance(lexer);
    if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)) {
        return false;
    }
    if (lexer->lookahead != ')') {
        for (;;) {
            if (!lambda_parameter_follows(lexer, &items, &named,
                                          no_close_ahead)) {
                return false;
            }
            if (lexer->lookahead == ')') {
                break;
            }
            if (lexer->lookahead != ',') {
                return false;
            }
            advance(lexer);
        }
    }
    advance(lexer);
    return arrow_follows(lexer, no_close_ahead) && named;
}

// SPECIFICATION.md §6.18 — at a `(` that starts a line after an expression,
// where a statement may start: whether the parentheses can open that
// statement, which Groovy reads there as its lexer ends the one before at
// the line break. They cannot when they are empty, when an element is a
// spread (`*a`) or a named argument (`a: 1`), when a `,` follows them, or
// when they hold a `,` with no bracket open, unless they hold only names
// and `=` follows them (`(a, b) = l`, a multi-assignment) or the first
// element is a parenthesis-free call (`(foo 1, 2)`: a name that is no value
// keyword, any member accesses, and on its line a name other than `in`, `as`
// and `instanceof`, a number or a string). Groovy rejects such a line; Java,
// which much Groovy code is written like, reads a call (`foo` followed by
// `(a, b);`), and so does the grammar, as before. When `->` follows them,
// Groovy appends the lambda to the expression (`pathElement`'s
// `nls closureOrLambdaExpression`), and they are the arguments here as
// before. The elements are read as Groovy's tokens within the budget of
// the lambda look-ahead. At its end, and at a `/` or a dollar-slashy string
// that opens an element, whose end the look-ahead does not look for, the
// parentheses open the statement. The token is the `(` alone.
static bool parentheses_open_statement(TSLexer *lexer, bool no_close_ahead) {
    advance(lexer);
    lexer->mark_end(lexer);
    unsigned items = 0;
    bool names = true;
    bool comma = false;
    bool command = false;
    for (;;) {
        if (!spend_lambda_item(&items)
            || !skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)) {
            return true;
        }
        if (lexer->lookahead == ')') {
            if (!comma) {
                return false;
            }
            names = false;
            break;
        }
        if (lexer->lookahead == '*') {
            return false;
        }
        if (is_identifier_start(lexer->lookahead)) {
            char word[sizeof "synchronized"];
            unsigned len = read_word(lexer, word, sizeof word);
            if (word_is(word, len, "$") && lexer->lookahead == '/') {
                return true;
            }
            bool receiver = !comma
                && !is_value_keyword(word, len, lexer->lookahead);
            bool bare = !is_member_access_start(lexer->lookahead);
            if (!bare) {
                names = false;
                int32_t last_name = word[0];
                bool nested = false;
                bool ends_in_new = false;
                receiver = receiver
                    && read_member_accesses(lexer, &last_name, &nested,
                                            &ends_in_new);
            }
            if (!skip_same_line_space(lexer, false)) {
                return true;
            }
            int32_t c = lexer->lookahead;
            if (c == ':' && bare && !command) {
                advance(lexer);
                if (lexer->lookahead != ':') {
                    return false;
                }
            } else if (receiver && is_identifier_start(c)) {
                char next_word[sizeof "instanceof"];
                unsigned next_len =
                    read_word(lexer, next_word, sizeof next_word);
                names = false;
                command = !word_is(next_word, next_len, "in")
                    && !word_is(next_word, next_len, "as")
                    && !word_is(next_word, next_len, "instanceof");
            } else if (receiver && is_literal_start(c)) {
                command = true;
            }
        }
        if (lexer->lookahead != ',' && lexer->lookahead != ')') {
            names = false;
            if (!skip_lambda_tokens(lexer, &items, true, no_close_ahead)) {
                return true;
            }
        }
        if (lexer->lookahead == ')') {
            break;
        }
        comma = true;
        advance(lexer);
    }
    if (comma && !names && !command) {
        return false;
    }
    advance(lexer);
    if (!skip_header_trivia(lexer, true, no_close_ahead, NULL, NULL)) {
        return true;
    }
    int32_t next = lexer->lookahead;
    if (next == ',') {
        return false;
    }
    if (next == '-') {
        advance(lexer);
        if (lexer->lookahead == '>') {
            return false;
        }
    }
    if (!comma || command) {
        return true;
    }
    if (next != '=') {
        return false;
    }
    advance(lexer);
    return lexer->lookahead != '=' && lexer->lookahead != '~';
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
        return scan_command_path_start(lexer, no_comment_close_ahead(scanner));
    }

    // Same-line tokens: SUBSCRIPT_OPEN (§5.10) and COMMAND_NAMED_KEY
    // (§5.5). Both skip only spaces, tabs and line escapes, the
    // same-line whitespace of Groovy's lexer; at a newline we fall
    // through to the branches below. Their first characters (`[` vs. an
    // identifier start) are disjoint, so one whitespace skip serves
    // both. STATEMENT_PAREN (§6.18) shares the skip to turn down a `(`
    // on the line, which leaves the branches below a `(` only after a
    // line break. NOT_IN is valid exactly after an expression. Only rules
    // that may open a statement take STATEMENT_PAREN, so after an
    // expression it is valid exactly where a statement may start: inside
    // parentheses and brackets, where Groovy's lexer drops line breaks, it
    // is not, and the `(` on a later line opens a call's arguments there.
    bool wants_subscript = valid_symbols[SUBSCRIPT_OPEN];
    bool wants_statement_paren = valid_symbols[STATEMENT_PAREN]
        && valid_symbols[NOT_IN] && !valid_symbols[ERROR_SENTINEL];
    bool wants_named_key = valid_symbols[COMMAND_NAMED_KEY];
    // §6.15 — TYPE_START is never emitted; it is valid where a type may
    // start, which narrows ARRAY_VALUE_START there.
    bool wants_array = valid_symbols[ARRAY_VALUE_START]
        && !valid_symbols[ERROR_SENTINEL];
    bool type_may_start = valid_symbols[TYPE_START];
    if ((wants_subscript || wants_named_key || wants_statement_paren)
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
        if (wants_statement_paren && lexer->lookahead == '(') {
            return false;
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
            return scan_command_named_key(lexer, valid_symbols, wants_array,
                                          type_may_start,
                                          no_comment_close_ahead(scanner));
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
    // of a call that is a value or a statement (§6.14) share the
    // whitespace skip and are dispatched on their first letter before the
    // `/` branches. A value may start on the line after its `=`, and a
    // statement on the line after the one before it, so those receivers
    // are looked for past line breaks too.
    bool wants_comment = valid_symbols[LINE_COMMENT]
        || valid_symbols[BLOCK_COMMENT]
        || valid_symbols[GROOVYDOC_COMMENT];
    bool wants_slashy = valid_symbols[SLASHY_STRING_START];
    bool wants_record = valid_symbols[RECORD_KEYWORD];
    bool wants_trait = valid_symbols[TRAIT_KEYWORD];
    bool wants_sealed = valid_symbols[SEALED_KEYWORD];
    bool wants_value_start = valid_symbols[COMMAND_VALUE_START]
        && !valid_symbols[ERROR_SENTINEL];
    bool wants_statement_start = valid_symbols[COMMAND_STATEMENT_START]
        && !valid_symbols[ERROR_SENTINEL];
    bool wants_cast_close = valid_symbols[CAST_CLOSE]
        && !valid_symbols[ERROR_SENTINEL];
    bool wants_not_in = valid_symbols[NOT_IN]
        && !valid_symbols[ERROR_SENTINEL];
    // §6.17 — NOT_IN is valid only after an expression, where a `(` opens a
    // call's arguments (`foo(a) -> b`, an `ERROR`). A lambda may start there
    // only as the next statement, which is left unread: reading the
    // arguments of every such call would cost more than the rare lambda
    // statement after an expression is worth (`foo()` then `(a) -> a` on the
    // next line, which Groovy also reads as a call).
    bool wants_lambda = valid_symbols[LAMBDA_START]
        && !valid_symbols[ERROR_SENTINEL] && !valid_symbols[NOT_IN];
    bool wants_bare_lambda = valid_symbols[BARE_LAMBDA_START]
        && !valid_symbols[ERROR_SENTINEL];
    if (!wants_comment && !wants_slashy && !wants_record && !wants_trait
        && !wants_sealed && !wants_value_start && !wants_statement_start
        && !wants_array && !wants_cast_close && !wants_not_in && !wants_lambda
        && !wants_bare_lambda
        && !valid_symbols[VAR_KEYWORD] && !valid_symbols[VAR_NAMES_KEYWORD]
        && !valid_symbols[YIELD_KEYWORD] && !valid_symbols[IN_KEYWORD]
        && !valid_symbols[AS_KEYWORD]) {
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
    // §6.14 — a statement's first name; it decides the keywords and the
    // array type below as well.
    if (wants_statement_start && is_identifier_start(lexer->lookahead)) {
        return scan_statement_start(lexer, valid_symbols, wants_array,
                                    type_may_start, no_close_ahead);
    }
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
        // §6.15 — `short` is the one element type name that shares a
        // first letter with these keywords, and a statement may start
        // with `short[].class` where `sealed` may start too. The array
        // token is empty and ends here; the keyword moves the end again.
        lexer->mark_end(lexer);
        advance(lexer);
        if (lexer->lookahead == 'h') {
            if (!wants_array || !consume_keyword(lexer, "hort")
                || !array_dimensions_follow(lexer, type_may_start,
                                            no_close_ahead)) {
                return false;
            }
            lexer->result_symbol = ARRAY_VALUE_START;
            return true;
        }
        if (!consume_keyword(lexer, "ealed")
            || !scan_sealed_target(lexer, no_close_ahead)) {
            return false;
        }
        lexer->result_symbol = SEALED_KEYWORD;
        return true;
    }
    // §6.16 — `var`, `yield`, `in` and `as`. `int` is the one element
    // type name (§6.15) that shares a first letter with them, and it may
    // start a statement where `in` may follow the one before, so it is
    // decided here too; the array token is empty and ends here.
    if (contextual_word_may_start(lexer->lookahead, valid_symbols)) {
        lexer->mark_end(lexer);
        const char *word = read_contextual_word(lexer);
        if (word == NULL) {
            return false;
        }
        unsigned len = (unsigned)strlen(word);
        if (word_is(word, len, "int")) {
            if (!wants_array
                || !array_dimensions_follow(lexer, type_may_start,
                                            no_close_ahead)) {
                return false;
            }
            lexer->result_symbol = ARRAY_VALUE_START;
            return true;
        }
        lexer->mark_end(lexer);
        return scan_contextual_word(lexer, word, len, valid_symbols,
                                    no_close_ahead);
    }
    if (((wants_value_start || wants_bare_lambda)
         && is_identifier_start(lexer->lookahead))
        || (wants_array && is_element_type_start(lexer->lookahead))) {
        return scan_value_start(lexer, wants_value_start, wants_bare_lambda,
                                wants_array, type_may_start, no_close_ahead);
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
    if (lexer->lookahead == '(' && wants_statement_paren) {
        // On `false` the runtime rewinds and the in-grammar lexer reads
        // the `(` of a call's arguments.
        if (!parentheses_open_statement(lexer, no_close_ahead)) {
            return false;
        }
        lexer->result_symbol = STATEMENT_PAREN;
        return true;
    }
    if (lexer->lookahead == '(' && wants_lambda) {
        // The token is empty and ends here. On `false` the runtime rewinds
        // and the in-grammar lexer reads the `(`.
        lexer->mark_end(lexer);
        if (!lambda_parameters_follow(lexer, no_close_ahead)) {
            return false;
        }
        lexer->result_symbol = LAMBDA_START;
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
