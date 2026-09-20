# Shared Net-lang grammar

`netlang.tmLanguage.json` is the TextMate grammar used by the documentation site's
Shiki highlighter. The future VS Code extension should reference this same file;
do not fork a second grammar. Its scope is `source.netlang`, with `.net` association.

This grammar highlights implemented syntax, including protocol metadata `using`.
It does not validate programs or implement future syntax. Type names are contextual,
not globally reserved keywords. Strings deliberately have no interpolation scope.
Comments are non-nested. Compiler behavior remains defined by Flex and the Rust parser.

Run `npm test --prefix docs` to test highlighting boundaries and supported tokens.
