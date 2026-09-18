//! Source locations shared by compiler stages without depending on the lexer.

/// One-based line and UTF-8 byte column, matching the Flex bridge's positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// A half-open source range. Empty ranges represent insertion points or EOF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn point(line: usize, column: usize) -> Self {
        let position = Position { line, column };
        Self {
            start: position,
            end: position,
        }
    }

    /// Advance over raw source text, not decoded string contents. Newlines reset
    /// byte columns; carriage returns count as bytes, as in the Flex scanner.
    pub fn from_text(start: Position, text: &str) -> Self {
        let mut end = start;
        for byte in text.bytes() {
            if byte == b'\n' {
                end.line = end.line.saturating_add(1);
                end.column = 1;
            } else {
                end.column = end.column.saturating_add(1);
            }
        }
        Self { start, end }
    }
}
