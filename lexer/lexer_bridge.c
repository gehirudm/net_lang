#include "lexer_bridge.h"
#include <stdlib.h>
#include <limits.h>
typedef void *yyscan_t;
typedef struct yy_buffer_state *YY_BUFFER_STATE;
int yylex_init_extra(NetPosition *, yyscan_t *);
int yylex_destroy(yyscan_t);
YY_BUFFER_STATE yy_scan_bytes(const char *, int, yyscan_t);
int yylex(yyscan_t);
char *yyget_text(yyscan_t);
int yyget_leng(yyscan_t);
typedef struct { yyscan_t scanner; NetPosition position; } NetLexer;
void *netlang_lexer_create(const char *source, size_t length) {
    if (length > INT_MAX) return NULL;
    NetLexer *lexer = calloc(1, sizeof(*lexer));
    if (!lexer) return NULL;
    lexer->position.line = lexer->position.column = 1;
    if (yylex_init_extra(&lexer->position, &lexer->scanner)) { free(lexer); return NULL; }
    yy_scan_bytes(source, (int)length, lexer->scanner);
    return lexer;
}
int netlang_lexer_next(void *raw, NetToken *token) {
    NetLexer *lexer = raw;
    int kind = yylex(lexer->scanner);
    token->kind = kind;
    token->lexeme = kind <= 0 ? "" : yyget_text(lexer->scanner);
    token->length = kind <= 0 ? 0 : (size_t)yyget_leng(lexer->scanner);
    token->line = lexer->position.start_line;
    token->column = lexer->position.start_column;
    return kind;
}
void netlang_lexer_destroy(void *raw) {
    if (!raw) return;
    NetLexer *lexer = raw;
    yylex_destroy(lexer->scanner);
    free(lexer);
}
