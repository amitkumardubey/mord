use crate::editor::buffer::DocumentBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize, // Character offset where selection began
    pub head: usize,   // Active cursor character offset
}

impl Selection {
    pub fn point(offset: usize) -> Self {
        Self {
            anchor: offset,
            head: offset,
        }
    }

    pub fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }

    pub fn min(&self) -> usize {
        self.anchor.min(self.head)
    }

    pub fn max(&self) -> usize {
        self.anchor.max(self.head)
    }

    pub fn range(&self) -> std::ops::Range<usize> {
        self.min()..self.max()
    }
}

pub struct CursorManager {
    pub selection: Selection,
    pub preferred_col: Option<usize>, // Retains column during vertical arrow movement
}

impl CursorManager {
    pub fn new() -> Self {
        Self {
            selection: Selection::point(0),
            preferred_col: None,
        }
    }

    pub fn cursor_offset(&self) -> usize {
        self.selection.head
    }

    pub fn set_cursor(&mut self, offset: usize) {
        self.selection = Selection::point(offset);
        self.preferred_col = None;
    }

    pub fn move_left(&mut self, _buffer: &DocumentBuffer, extend_selection: bool) {
        let head = self.selection.head;
        let new_head = if head > 0 { head - 1 } else { 0 };
        if extend_selection {
            self.selection.head = new_head;
        } else {
            if !self.selection.is_collapsed() {
                self.selection = Selection::point(self.selection.min());
            } else {
                self.selection = Selection::point(new_head);
            }
        }
        self.preferred_col = None;
    }

    pub fn move_right(&mut self, buffer: &DocumentBuffer, extend_selection: bool) {
        let max_len = buffer.len_chars();
        let head = self.selection.head;
        let new_head = if head < max_len { head + 1 } else { max_len };
        if extend_selection {
            self.selection.head = new_head;
        } else {
            if !self.selection.is_collapsed() {
                self.selection = Selection::point(self.selection.max());
            } else {
                self.selection = Selection::point(new_head);
            }
        }
        self.preferred_col = None;
    }

    pub fn move_up(&mut self, buffer: &DocumentBuffer, extend_selection: bool) {
        let point = buffer.char_offset_to_point(self.selection.head);
        if point.row == 0 {
            // Already on first line
            let new_head = 0;
            if extend_selection {
                self.selection.head = new_head;
            } else {
                self.selection = Selection::point(new_head);
            }
            return;
        }

        let target_row = point.row - 1;
        let col = self.preferred_col.unwrap_or(point.col);
        self.preferred_col = Some(col);

        let new_head = buffer.point_to_char_offset(target_row, col);
        if extend_selection {
            self.selection.head = new_head;
        } else {
            self.selection = Selection::point(new_head);
        }
    }

    pub fn move_down(&mut self, buffer: &DocumentBuffer, extend_selection: bool) {
        let point = buffer.char_offset_to_point(self.selection.head);
        let max_line = buffer.len_lines().saturating_sub(1);
        if point.row >= max_line {
            // Already on last line
            let new_head = buffer.len_chars();
            if extend_selection {
                self.selection.head = new_head;
            } else {
                self.selection = Selection::point(new_head);
            }
            return;
        }

        let target_row = point.row + 1;
        let col = self.preferred_col.unwrap_or(point.col);
        self.preferred_col = Some(col);

        let new_head = buffer.point_to_char_offset(target_row, col);
        if extend_selection {
            self.selection.head = new_head;
        } else {
            self.selection = Selection::point(new_head);
        }
    }

    pub fn move_to_line_start(&mut self, buffer: &DocumentBuffer, extend_selection: bool) {
        let point = buffer.char_offset_to_point(self.selection.head);
        let line_start = buffer.line_to_char(point.row);
        if extend_selection {
            self.selection.head = line_start;
        } else {
            self.selection = Selection::point(line_start);
        }
        self.preferred_col = None;
    }

    pub fn move_to_line_end(&mut self, buffer: &DocumentBuffer, extend_selection: bool) {
        let point = buffer.char_offset_to_point(self.selection.head);
        let line = buffer.line_to_string(point.row).unwrap_or_default();
        let trimmed_len = line.trim_end_matches(['\r', '\n']).chars().count();
        let line_start = buffer.line_to_char(point.row);
        let line_end = line_start + trimmed_len;

        if extend_selection {
            self.selection.head = line_end;
        } else {
            self.selection = Selection::point(line_end);
        }
        self.preferred_col = None;
    }

    pub fn select_all(&mut self, buffer: &DocumentBuffer) {
        self.selection = Selection::new(0, buffer.len_chars());
        self.preferred_col = None;
    }
}
