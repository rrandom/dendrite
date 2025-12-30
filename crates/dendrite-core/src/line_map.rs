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

    pub fn offset_to_point(&self, offset: usize) -> Point {
        match self.line_starts.binary_search(&offset) {
            Ok(line) => Point { line, col: 0 },
            Err(next_line_idx) => {
                let line = next_line_idx - 1;
                let line_start = self.line_starts[line];
                let col = offset - line_start;
                Point { line, col }
            }
        }
    }
}
