#ifndef NETLANG_BRIDGE_H
#define NETLANG_BRIDGE_H
#include <stddef.h>
/* Stable token ABI: keep these values aligned with Rust TokenKind. */
typedef enum {
    NET_EOF = 0,
    NET_IDENTIFIER = 1,
    NET_INTEGER = 2,
    NET_FLOAT = 3,
    NET_STRING = 4,
    NET_DURATION = 5,
    NET_LET = 6,
    NET_FN = 7,
    NET_RETURN = 8,
    NET_IF = 9,
    NET_ELSE = 10,
    NET_WHILE = 11,
    NET_FOR = 12,
    NET_IN = 13,
    NET_MATCH = 14,
    NET_PARALLEL = 15,
    NET_TRUE = 16,
    NET_FALSE = 17,
    NET_NULL = 18,
    NET_GET = 19,
    NET_POST = 20,
    NET_PUT = 21,
    NET_PATCH = 22,
    NET_DELETE = 23,
    NET_HEAD = 24,
    NET_PLUS = 25,
    NET_MINUS = 26,
    NET_STAR = 27,
    NET_SLASH = 28,
    NET_PERCENT = 29,
    NET_EQUAL = 30,
    NET_EQUAL_EQUAL = 31,
    NET_BANG = 32,
    NET_BANG_EQUAL = 33,
    NET_LESS = 34,
    NET_LESS_EQUAL = 35,
    NET_GREATER = 36,
    NET_GREATER_EQUAL = 37,
    NET_AND_AND = 38,
    NET_OR_OR = 39,
    NET_LEFT_PAREN = 40,
    NET_RIGHT_PAREN = 41,
    NET_LEFT_BRACE = 42,
    NET_RIGHT_BRACE = 43,
    NET_LEFT_BRACKET = 44,
    NET_RIGHT_BRACKET = 45,
    NET_COMMA = 46,
    NET_COLON = 47,
    NET_SEMICOLON = 48,
    NET_DOT = 49,
    NET_FAT_ARROW = 50,
    NET_TCP = 51,
    NET_UDP = 52,
    NET_SEND = 53,
    NET_RECEIVE = 54,
    NET_TO = 55,
    NET_USING = 56,
    NET_BREAK = 57,
    NET_CONTINUE = 58,
    NET_ARROW = 59,
    NET_UNEXPECTED_CHARACTER = -1,
    NET_UNTERMINATED_COMMENT = -2,
    NET_INVALID_ESCAPE = -3,
    NET_UNTERMINATED_STRING = -4
} NetTokenKind;

typedef struct {
    int kind;
    const char *lexeme;
    size_t length, line, column;
} NetToken;
typedef struct {
    size_t line, column, start_line, start_column;
    size_t comment_line, comment_column;
} NetPosition;
/* Source is copied. Length excludes any terminator and may include NUL bytes.
 * Returns NULL if the scanner cannot be initialized or the input is too large.
 * Token text is borrowed until next/destroy; copy it before another call.
 * next returns the token kind, zero for EOF, or a negative lexical error.
 * Each instance owns its scanner and source buffer; destroy it exactly once.
 */
void *netlang_lexer_create(const char *source, size_t length);
int netlang_lexer_next(void *lexer, NetToken *token);
void netlang_lexer_destroy(void *lexer);
#endif
