//! Source location types for error reporting.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A location in source text, used for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub column: usize,
    /// 0-based byte offset into the source string.
    pub offset: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// A range in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: SourceLocation,
    pub end: SourceLocation,
}

/// Convert a byte offset in `source` to a `SourceLocation` with line/column.
pub fn offset_to_location(source: &str, offset: usize) -> SourceLocation {
    let offset = offset.min(source.len());
    let mut line = 1;
    let mut col = 1;

    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    SourceLocation {
        line,
        column: col,
        offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line() {
        let src = "SELECT * FROM logs";
        let loc = offset_to_location(src, 7);
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 8);
    }

    #[test]
    fn multi_line() {
        let src = "SELECT *\nFROM logs";
        // 'F' in FROM is at offset 9
        let loc = offset_to_location(src, 9);
        assert_eq!(loc.line, 2);
        assert_eq!(loc.column, 1);
    }

    #[test]
    fn offset_zero() {
        let loc = offset_to_location("hello", 0);
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 1);
    }

    #[test]
    fn offset_past_end() {
        let loc = offset_to_location("hi", 100);
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 3);
    }

    #[test]
    fn display() {
        let loc = SourceLocation {
            line: 3,
            column: 12,
            offset: 42,
        };
        assert_eq!(loc.to_string(), "line 3, column 12");
    }
}
