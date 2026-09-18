//! Shared rendering for lexer and parser diagnostics.
pub fn render(filename: &str, source: &str, line: usize, column: usize, message: &str) -> String {
    render_span(
        filename,
        source,
        crate::source::Span::point(line, column),
        message,
    )
}

/// Render the first line of a half-open range, with a continuation note for
/// multiline spans. Invalid/out-of-source ranges are clamped for safe rendering.
pub fn render_span(
    filename: &str,
    source: &str,
    span: crate::source::Span,
    message: &str,
) -> String {
    let line = span.start.line.max(1);
    let column = span.start.column.max(1);
    let source_line = source
        .split('\n')
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .trim_end_matches('\r');
    // Lexer columns count UTF-8 bytes. Convert the prefix to a display column,
    // expanding tabs consistently in both the source line and caret padding.
    let byte_column = column.saturating_sub(1);
    let display_width = |byte_limit| {
        source_line
            .char_indices()
            .take_while(|(offset, _)| *offset < byte_limit)
            .map(|(_, ch)| if ch == '\t' { 4 } else { 1 })
            .sum::<usize>()
    };
    let width = display_width(byte_column);
    let end_byte = if span.end.line > line {
        source_line.len()
    } else if span.end.line == line {
        span.end.column.saturating_sub(1)
    } else {
        byte_column
    };
    let marker_width = display_width(end_byte).saturating_sub(width).max(1);
    let gutter = line.to_string().len();
    let mut output = format!(
        "error: {message}\n {:>gutter$}--> {filename}:{line}:{column}\n{line} | {}\n {:>gutter$}| {}^{}\n",
        "",
        source_line.replace('\t', "    "),
        "",
        " ".repeat(width),
        "~".repeat(marker_width - 1),
    );
    if span.end.line > line {
        output.push_str(&format!(
            "  = span continues to {}:{}\n",
            span.end.line, span.end.column
        ));
    }
    output
}
