use tower_lsp_f::lsp_types::{Position, Range};

#[inline]
pub fn pos(line: u32, col: u32) -> Position {
    Position {
        line,
        character: col,
    }
}

#[inline]
pub fn range_on_line(line: u32, col_start: u32, col_end: u32) -> Range {
    Range {
        start: pos(line, col_start),
        end: pos(line, col_end),
    }
}

#[inline]
pub fn zero_range(line: u32, col: u32) -> Range {
    range_on_line(line, col, col)
}

pub fn source_range(r: &vn_core::SourceRange) -> Range {
    Range {
        start: pos(r.start.line.saturating_sub(1), r.start.column),
        end: pos(r.end.line.saturating_sub(1), r.end.column),
    }
}
