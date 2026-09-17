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
<COMMENT><<EOF>> { yyextra->start_line = yyextra->comment_line; yyextra->start_column = yyextra->comment_column; return -2; }
"let" return 6;
"fn" return 7;
"return" return 8;
"if" return 9;
"else" return 10;
"while" return 11;
"for" return 12;
"in" return 13;
"match" return 14;
"parallel" return 15;
"true" return 16;
"false" return 17;
"null" return 18;
"GET" return 19;
"POST" return 20;
"PUT" return 21;
"PATCH" return 22;
"DELETE" return 23;
"HEAD" return 24;
"+" return 25;
"-" return 26;
"*" return 27;
"/" return 28;
"%" return 29;
"=" return 30;
"==" return 31;
"!" return 32;
"!=" return 33;
"<" return 34;
"<=" return 35;
">" return 36;
">=" return 37;
"&&" return 38;
"||" return 39;
"(" return 40;
")" return 41;
"{" return 42;
"}" return 43;
"[" return 44;
"]" return 45;
"," return 46;
":" return 47;
";" return 48;
"." return 49;
"=>" return 50;
[0-9]+(ms|s|m) return 5;
[0-9]+\.[0-9]+ return 3;
[0-9]+ return 2;
[a-zA-Z_][a-zA-Z0-9_]* return 1;
\"([^\"\\\n\r]|\\[ntr\"\\])*\" return 4;
\"([^\"\\\n\r]|\\[^\n\r])*\" return -3;
\"([^\"\\\n\r]|\\[^\n\r])* return -4;
. return -1;
<<EOF>> { yyextra->start_line = yyextra->line; yyextra->start_column = yyextra->column; return 0; }
%%
