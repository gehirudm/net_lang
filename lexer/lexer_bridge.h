#ifndef NETLANG_BRIDGE_H
#define NETLANG_BRIDGE_H
#include <stddef.h>
typedef struct {
    int kind;
    const char *lexeme;
    size_t length, line, column;
} NetToken;
typedef struct {
    size_t line, column, start_line, start_column;
    size_t comment_line, comment_column;
} NetPosition;
void *netlang_lexer_create(const char *source, size_t length);
int netlang_lexer_next(void *lexer, NetToken *token);
void netlang_lexer_destroy(void *lexer);
#endif
