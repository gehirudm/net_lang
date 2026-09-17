%option reentrant noyywrap noinput nounput nodefault
%option extra-type="NetPosition *"
%x COMMENT
%{
#include "lexer_bridge.h"
#define YY_USER_ACTION do { \
    yyextra->start_line = yyextra->line; \
    yyextra->start_column = yyextra->column; \
    for (int i = 0; i < yyleng; ++i) { \
        if (yytext[i] == '\n') { yyextra->line++; yyextra->column = 1; } \
        else { yyextra->column++; } \
    } \
} while (0);
%}
%%
[ \t\r\n]+ ;
"//"[^\n]* ;
"/*" { yyextra->comment_line = yyextra->start_line; yyextra->comment_column = yyextra->start_column; BEGIN(COMMENT); }
<COMMENT>"*/" BEGIN(INITIAL);
<COMMENT>[^*\n]+ ;
<COMMENT>"*" ;
<COMMENT>\n ;
<COMMENT><<EOF>> { yyextra->start_line = yyextra->comment_line; yyextra->start_column = yyextra->comment_column; return NET_UNTERMINATED_COMMENT; }
"let" return NET_LET;
"fn" return NET_FN;
"return" return NET_RETURN;
"if" return NET_IF;
"else" return NET_ELSE;
"while" return NET_WHILE;
"for" return NET_FOR;
"in" return NET_IN;
"match" return NET_MATCH;
"parallel" return NET_PARALLEL;
"true" return NET_TRUE;
"false" return NET_FALSE;
"null" return NET_NULL;
"GET" return NET_GET;
"POST" return NET_POST;
"PUT" return NET_PUT;
"PATCH" return NET_PATCH;
"DELETE" return NET_DELETE;
"HEAD" return NET_HEAD;
"TCP" return NET_TCP;
"UDP" return NET_UDP;
"SEND" return NET_SEND;
"RECEIVE" return NET_RECEIVE;
"TO" return NET_TO;
"using" return NET_USING;
"+" return NET_PLUS;
"-" return NET_MINUS;
"*" return NET_STAR;
"/" return NET_SLASH;
"%" return NET_PERCENT;
"=" return NET_EQUAL;
"==" return NET_EQUAL_EQUAL;
"!" return NET_BANG;
"!=" return NET_BANG_EQUAL;
"<" return NET_LESS;
"<=" return NET_LESS_EQUAL;
">" return NET_GREATER;
">=" return NET_GREATER_EQUAL;
"&&" return NET_AND_AND;
"||" return NET_OR_OR;
"(" return NET_LEFT_PAREN;
")" return NET_RIGHT_PAREN;
"{" return NET_LEFT_BRACE;
"}" return NET_RIGHT_BRACE;
"[" return NET_LEFT_BRACKET;
"]" return NET_RIGHT_BRACKET;
"," return NET_COMMA;
":" return NET_COLON;
";" return NET_SEMICOLON;
"." return NET_DOT;
"=>" return NET_FAT_ARROW;
[0-9]+(ms|s|m) return NET_DURATION;
[0-9]+\.[0-9]+ return NET_FLOAT;
[0-9]+ return NET_INTEGER;
[a-zA-Z_][a-zA-Z0-9_]* return NET_IDENTIFIER;
\"([^\"\\\n\r]|\\[ntr\"\\])*\" return NET_STRING;
\"([^\"\\\n\r]|\\[^\n\r])*\" return NET_INVALID_ESCAPE;
\"([^\"\\\n\r]|\\[^\n\r])* return NET_UNTERMINATED_STRING;
. return NET_UNEXPECTED_CHARACTER;
<<EOF>> { yyextra->start_line = yyextra->line; yyextra->start_column = yyextra->column; return NET_EOF; }
%%
