use ropey::Rope;
use tower_lsp::lsp_types::{Position, Range};
use tree_sitter::{InputEdit, Point};

// use crate::lsp_files::JinjaVariable;

pub trait ToInputEdit {
    fn to_point(&self, position: Position) -> Point;
    fn to_char(&self, position: Position) -> usize;
    fn to_position(&self, offset: usize) -> Position;
    fn to_input_edit(&self, range: Range, text: &str) -> InputEdit;
}

impl ToInputEdit for Rope {
    fn to_point(&self, position: Position) -> Point {
        Point::new(position.line as usize, position.character as usize)
    }

    fn to_char(&self, position: Position) -> usize {
        let start_line = self.line_to_char(position.line as usize);
        start_line + position.character as usize
    }

    fn to_position(&self, mut offset: usize) -> Position {
        offset = offset.min(self.len_bytes());
        let mut low = 0usize;
        let mut high = self.len_lines();
        if high == 0 {
            return Position {
                line: 0,
                character: offset as u32,
            };
        }
        while low < high {
            let mid = low + (high - low) / 2;
            if self.line_to_byte(mid) > offset {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        let line = low - 1;
        let character = offset - self.line_to_byte(line);
        Position::new(line as u32, character as u32)
    }

    fn to_input_edit(&self, range: Range, text: &str) -> InputEdit {
        let start = range.start;
        let end = range.end;

        let start_byte = self.to_char(start);
        let start_position = self.to_point(start);

        let new_end_byte = start_byte + text.len();
        let new_end_position = self.to_position(new_end_byte);
        let new_end_position = self.to_point(new_end_position);

        let old_end_byte = self.to_char(end);
        let old_end_position = self.to_point(end);

        InputEdit {
            start_byte,
            old_end_byte,
            new_end_byte,
            start_position,
            old_end_position,
            new_end_position,
        }
    }
}

pub fn to_position2(point: Point) -> Position {
    Position::new(point.row as u32, point.column as u32)
}
