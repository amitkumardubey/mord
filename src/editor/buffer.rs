use ropey::Rope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPoint {
    pub row: usize,
    pub col: usize, // Unicode scalar (char) index in line; graphemes are a later concern
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditAction {
    pub start_byte: usize,
    pub old_text: String,
    pub new_text: String,
}

pub struct DocumentBuffer {
    rope: Rope,
    undo_stack: Vec<EditAction>,
    redo_stack: Vec<EditAction>,
}

impl DocumentBuffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn from_str(initial: &str) -> Self {
        Self {
            rope: Rope::from_str(initial),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line_to_string(&self, line_idx: usize) -> Option<String> {
        if line_idx < self.rope.len_lines() {
            Some(self.rope.line(line_idx).to_string())
        } else {
            None
        }
    }

    pub fn line_without_newline(&self, line_idx: usize) -> Option<String> {
        self.line_to_string(line_idx).map(|s| {
            s.trim_end_matches(['\r', '\n']).to_string()
        })
    }

    pub fn char_to_line(&self, char_idx: usize) -> usize {
        self.rope.char_to_line(char_idx.min(self.rope.len_chars()))
    }

    pub fn line_to_char(&self, line_idx: usize) -> usize {
        self.rope.line_to_char(line_idx.min(self.rope.len_lines()))
    }

    pub fn point_to_char_offset(&self, row: usize, col: usize) -> usize {
        let max_line = self.rope.len_lines().saturating_sub(1);
        let actual_row = row.min(max_line);
        let line_start = self.rope.line_to_char(actual_row);
        let line = self.rope.line(actual_row);
        let line_len_without_nl = line.len_chars() - line_trailing_newline_len(&line);
        let actual_col = col.min(line_len_without_nl);
        line_start + actual_col
    }

    pub fn char_offset_to_point(&self, char_idx: usize) -> TextPoint {
        let clamped = char_idx.min(self.rope.len_chars());
        let row = self.rope.char_to_line(clamped);
        let line_start = self.rope.line_to_char(row);
        let col = clamped - line_start;
        TextPoint { row, col }
    }

    pub fn slice_to_string(&self, start_char: usize, end_char: usize) -> String {
        let start = start_char.min(self.rope.len_chars());
        let end = end_char.min(self.rope.len_chars());
        if start >= end {
            return String::new();
        }
        self.rope.slice(start..end).to_string()
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        let char_idx = char_idx.min(self.rope.len_chars());
        self.rope.insert(char_idx, text);
        self.undo_stack.push(EditAction {
            start_byte: char_idx,
            old_text: String::new(),
            new_text: text.to_string(),
        });
        self.redo_stack.clear();
    }

    pub fn delete_range(&mut self, start_char: usize, end_char: usize) -> String {
        let start = start_char.min(self.rope.len_chars());
        let end = end_char.min(self.rope.len_chars());
        if start >= end {
            return String::new();
        }
        let old_text = self.rope.slice(start..end).to_string();
        self.rope.remove(start..end);
        self.undo_stack.push(EditAction {
            start_byte: start,
            old_text: old_text.clone(),
            new_text: String::new(),
        });
        self.redo_stack.clear();
        old_text
    }

    pub fn replace_range(&mut self, start_char: usize, end_char: usize, text: &str) {
        let start = start_char.min(self.rope.len_chars());
        let end = end_char.min(self.rope.len_chars());
        let old_text = self.rope.slice(start..end).to_string();
        self.rope.remove(start..end);
        self.rope.insert(start, text);
        self.undo_stack.push(EditAction {
            start_byte: start,
            old_text,
            new_text: text.to_string(),
        });
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> Option<usize> {
        if let Some(action) = self.undo_stack.pop() {
            let start = action.start_byte;
            let inserted_len = action.new_text.chars().count();
            if inserted_len > 0 {
                self.rope.remove(start..start + inserted_len);
            }
            if !action.old_text.is_empty() {
                self.rope.insert(start, &action.old_text);
            }
            let cursor = start + action.old_text.chars().count();
            self.redo_stack.push(action);
            Some(cursor)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<usize> {
        if let Some(action) = self.redo_stack.pop() {
            let start = action.start_byte;
            let removed_len = action.old_text.chars().count();
            if removed_len > 0 {
                self.rope.remove(start..start + removed_len);
            }
            if !action.new_text.is_empty() {
                self.rope.insert(start, &action.new_text);
            }
            let cursor = start + action.new_text.chars().count();
            self.undo_stack.push(action);
            Some(cursor)
        } else {
            None
        }
    }
}

fn line_trailing_newline_len(line: &ropey::RopeSlice) -> usize {
    let len = line.len_chars();
    if len == 0 {
        return 0;
    }
    let last = line.char(len - 1);
    if last == '\n' {
        if len > 1 && line.char(len - 2) == '\r' {
            2
        } else {
            1
        }
    } else if last == '\r' {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_operations() {
        let mut doc = DocumentBuffer::from_str("Hello World\nSecond line");
        assert_eq!(doc.len_lines(), 2);
        assert_eq!(doc.line_without_newline(0).unwrap(), "Hello World");
        
        doc.insert(5, " Beautiful");
        assert_eq!(doc.line_without_newline(0).unwrap(), "Hello Beautiful World");
        
        doc.undo();
        assert_eq!(doc.line_without_newline(0).unwrap(), "Hello World");
        
        doc.redo();
        assert_eq!(doc.line_without_newline(0).unwrap(), "Hello Beautiful World");
    }
}
