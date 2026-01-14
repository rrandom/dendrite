use crate::model::Point;

pub struct LineMap {
    line_starts: Vec<usize>,
}

impl LineMap {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, c) in text.char_indices() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    pub fn offset_to_point(&self, text: &str, offset: usize) -> Point {
        match self.line_starts.binary_search(&offset) {
            Ok(line) => Point { line: line as u32, col: 0 },
            Err(next_line_idx) => {
                let line = next_line_idx - 1;
                let line_start = self.line_starts[line];
                let line_text = &text[line_start..offset];
                let col = line_text.encode_utf16().count();
                Point { line: line as u32, col: col as u32 }
            }
        }
    }

    pub fn point_to_offset(&self, text: &str, point: Point) -> Option<usize> {
        let line_start = *self.line_starts.get(point.line as usize)?;
        let mut current_col = 0u32;

        for (i, c) in text[line_start..].char_indices() {
            if current_col == point.col {
                return Some(line_start + i);
            }
            if c == '\n' {
                break;
            }
            current_col += c.len_utf16() as u32;
        }

        if current_col == point.col {
            return Some(text.len()); // or end of line segment
        }

        None
    }
}
