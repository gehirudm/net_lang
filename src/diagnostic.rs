//! Shared rendering for lexer and parser diagnostics.
pub fn render(filename: &str, source: &str, line: usize, column: usize, message: &str) -> String {
    let source_line = source
        .split('\n')
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .trim_end_matches('\r');
    // Lexer columns count UTF-8 bytes. Convert the prefix to a display column,
    // expanding tabs consistently in both the source line and caret padding.
    let byte_column = column.saturating_sub(1);
    let width = source_line
        .char_indices()
        .take_while(|(offset, _)| *offset < byte_column)
        .map(|(_, ch)| if ch == '\t' { 4 } else { 1 })
        .sum::<usize>();
    let gutter = line.to_string().len();
    format!(
        "error: {message}\n {:>gutter$}--> {filename}:{line}:{column}\n{line} | {}\n {:>gutter$}| {}^\n",
        "",
        source_line.replace('\t', "    "),
        "",
        " ".repeat(width),
    )
}
