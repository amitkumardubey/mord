use crate::editor::buffer::{DocumentBuffer, TextPoint};
use crate::editor::decorator::{DecoratedLine, Decorator};
use crate::editor::offset::{BufferCol, BufferOffset, VisualCol};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: BufferOffset,
    pub head: BufferOffset,
}

impl Selection {
    pub fn point(offset: BufferOffset) -> Self {
        Self {
            anchor: offset,
            head: offset,
        }
    }

    pub fn new(anchor: BufferOffset, head: BufferOffset) -> Self {
        Self { anchor, head }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }

    pub fn min(&self) -> BufferOffset {
        self.anchor.min(self.head)
    }

    pub fn max(&self) -> BufferOffset {
        self.anchor.max(self.head)
    }

    pub fn range(&self) -> std::ops::Range<usize> {
        self.min().get()..self.max().get()
    }
}

pub struct CursorManager {
    pub selection: Selection,
    /// Visual column retained during vertical arrow movement (not a buffer column).
    pub preferred_col: Option<VisualCol>,
}

impl CursorManager {
    pub fn new() -> Self {
        Self {
            selection: Selection::point(BufferOffset(0)),
            preferred_col: None,
        }
    }

    pub fn cursor_offset(&self) -> BufferOffset {
        self.selection.head
    }

    pub fn set_cursor(&mut self, offset: BufferOffset) {
        self.selection = Selection::point(offset);
        self.preferred_col = None;
    }

    pub fn set_head(&mut self, offset: BufferOffset, extend: bool) {
        if extend {
            self.selection.head = offset;
        } else {
            self.selection = Selection::point(offset);
        }
        self.preferred_col = None;
    }

    pub fn move_left(&mut self, buffer: &DocumentBuffer, extend_selection: bool) {
        let head = self.selection.head;
        let text = buffer.text();
        let new_head = BufferOffset(previous_grapheme_char_offset(&text, head.get()));
        if extend_selection {
            self.selection.head = new_head;
        } else if !self.selection.is_collapsed() {
            self.selection = Selection::point(self.selection.min());
        } else {
            self.selection = Selection::point(new_head);
        }
        self.preferred_col = None;
    }

    pub fn move_right(&mut self, buffer: &DocumentBuffer, extend_selection: bool) {
        let max_len = BufferOffset(buffer.len_chars());
        let head = self.selection.head;
        let text = buffer.text();
        let next = BufferOffset(next_grapheme_char_offset(&text, head.get()));
        let new_head = next.min(max_len);
        if extend_selection {
            self.selection.head = new_head;
        } else if !self.selection.is_collapsed() {
            self.selection = Selection::point(self.selection.max());
        } else {
            self.selection = Selection::point(new_head);
        }
        self.preferred_col = None;
    }

    /// Move up using a **visual** preferred column mapped through each line's `char_map`.
    pub fn move_up(
        &mut self,
        buffer: &DocumentBuffer,
        current_decorated: &DecoratedLine,
        target_decorated: &DecoratedLine,
        extend_selection: bool,
    ) {
        let point = buffer.char_offset_to_point(self.selection.head.get());
        if point.row == 0 {
            let new_head = BufferOffset(0);
            if extend_selection {
                self.selection.head = new_head;
            } else {
                self.selection = Selection::point(new_head);
            }
            return;
        }

        let visual = self
            .preferred_col
            .unwrap_or_else(|| VisualCol(Decorator::buffer_col_to_visual_col(current_decorated, point.col)));
        self.preferred_col = Some(visual);

        let target_row = point.row - 1;
        let buf_col = Decorator::visual_col_to_buffer_col(
            target_decorated,
            visual.get(),
            line_char_len(buffer, target_row),
        );
        let new_head = BufferOffset(buffer.point_to_char_offset(target_row, buf_col));
        if extend_selection {
            self.selection.head = new_head;
        } else {
            self.selection = Selection::point(new_head);
        }
    }

    /// Move down using a **visual** preferred column mapped through each line's `char_map`.
    pub fn move_down(
        &mut self,
        buffer: &DocumentBuffer,
        current_decorated: &DecoratedLine,
        target_decorated: &DecoratedLine,
        extend_selection: bool,
    ) {
        let point = buffer.char_offset_to_point(self.selection.head.get());
        let max_line = buffer.len_lines().saturating_sub(1);
        if point.row >= max_line {
            let new_head = BufferOffset(buffer.len_chars());
            if extend_selection {
                self.selection.head = new_head;
            } else {
                self.selection = Selection::point(new_head);
            }
            return;
        }

        let visual = self
            .preferred_col
            .unwrap_or_else(|| VisualCol(Decorator::buffer_col_to_visual_col(current_decorated, point.col)));
        self.preferred_col = Some(visual);

        let target_row = point.row + 1;
        let buf_col = Decorator::visual_col_to_buffer_col(
            target_decorated,
            visual.get(),
            line_char_len(buffer, target_row),
        );
        let new_head = BufferOffset(buffer.point_to_char_offset(target_row, buf_col));
        if extend_selection {
            self.selection.head = new_head;
        } else {
            self.selection = Selection::point(new_head);
        }
    }

    pub fn move_to_line_start(&mut self, buffer: &DocumentBuffer, extend_selection: bool) {
        let point = buffer.char_offset_to_point(self.selection.head.get());
        let line_start = BufferOffset(buffer.line_to_char(point.row));
        if extend_selection {
            self.selection.head = line_start;
        } else {
            self.selection = Selection::point(line_start);
        }
        self.preferred_col = None;
    }

    pub fn move_to_line_end(&mut self, buffer: &DocumentBuffer, extend_selection: bool) {
        let point = buffer.char_offset_to_point(self.selection.head.get());
        let line = buffer.line_to_string(point.row).unwrap_or_default();
        let trimmed_len = line.trim_end_matches(['\r', '\n']).chars().count();
        let line_start = buffer.line_to_char(point.row);
        let line_end = BufferOffset(line_start + trimmed_len);

        if extend_selection {
            self.selection.head = line_end;
        } else {
            self.selection = Selection::point(line_end);
        }
        self.preferred_col = None;
    }

    pub fn select_all(&mut self, buffer: &DocumentBuffer) {
        self.selection = Selection::new(BufferOffset(0), BufferOffset(buffer.len_chars()));
        self.preferred_col = None;
    }

    pub fn cursor_point(&self, buffer: &DocumentBuffer) -> TextPoint {
        buffer.char_offset_to_point(self.selection.head.get())
    }

    pub fn cursor_buffer_col(&self, buffer: &DocumentBuffer) -> BufferCol {
        BufferCol(self.cursor_point(buffer).col)
    }
}

fn line_char_len(buffer: &DocumentBuffer, row: usize) -> usize {
    buffer
        .line_without_newline(row)
        .map(|s| s.chars().count())
        .unwrap_or(0)
}

/// Char offset of the start of the grapheme before `char_offset` (or 0).
pub fn previous_grapheme_char_offset(text: &str, char_offset: usize) -> usize {
    use unicode_segmentation::UnicodeSegmentation;
    if char_offset == 0 {
        return 0;
    }
    let byte = crate::editor::offset::char_to_byte_index(text, char_offset);
    let prev_byte = text
        .grapheme_indices(true)
        .map(|(i, _)| i)
        .take_while(|&i| i < byte)
        .last()
        .unwrap_or(0);
    crate::editor::offset::byte_to_char_index(text, prev_byte)
}

/// Char offset after the grapheme at `char_offset` (or end).
pub fn next_grapheme_char_offset(text: &str, char_offset: usize) -> usize {
    use unicode_segmentation::UnicodeSegmentation;
    let total = text.chars().count();
    if char_offset >= total {
        return total;
    }
    let byte = crate::editor::offset::char_to_byte_index(text, char_offset);
    let next_byte = text
        .grapheme_indices(true)
        .find(|(i, _)| *i > byte)
        .map(|(i, _)| i)
        .unwrap_or(text.len());
    crate::editor::offset::byte_to_char_index(text, next_byte)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::decorator::{ConcealMode, Decorator};
    use crate::editor::parser::MarkdownParser;

    fn decorate(line: &str, cursor_col: Option<usize>) -> DecoratedLine {
        let parsed = MarkdownParser::parse_line(line);
        let scale = match parsed.block_kind {
            crate::editor::BlockKind::Heading { level: 3 } => 1.30,
            crate::editor::BlockKind::Heading { level: 1 } => 1.85,
            _ => 1.0,
        };
        Decorator::decorate_line(0, &parsed, cursor_col, ConcealMode::Live, scale, false)
    }

    #[test]
    fn up_down_from_concealed_heading_keeps_visual_column() {
        let buffer = DocumentBuffer::from_str("### My Title\nHi");
        let mut cursor = CursorManager::new();
        // Buffer col 4 = 'M' in "My Title" (after concealed "### ")
        let heading_start = buffer.line_to_char(0);
        cursor.set_cursor(BufferOffset(heading_start + 4));

        let heading_dec = decorate("### My Title", None);
        assert_eq!(heading_dec.display_text, "My Title");
        assert_eq!(Decorator::buffer_col_to_visual_col(&heading_dec, 4), 0);

        let para_dec = decorate("Hi", None);
        cursor.move_down(&buffer, &heading_dec, &para_dec, false);

        let point = buffer.char_offset_to_point(cursor.cursor_offset().get());
        assert_eq!(point.row, 1);
        // Visual col 0 on "Hi" → buffer col 0, not buffer col 4
        assert_eq!(point.col, 0);
        assert_eq!(cursor.preferred_col, Some(VisualCol(0)));

        // Moving back up should land on 'M' again (visual 0 → buffer 4)
        let heading_dec2 = decorate("### My Title", None);
        let para_dec2 = decorate("Hi", Some(0));
        cursor.move_up(&buffer, &para_dec2, &heading_dec2, false);
        let point = buffer.char_offset_to_point(cursor.cursor_offset().get());
        assert_eq!(point.row, 0);
        assert_eq!(point.col, 4);
    }
}
