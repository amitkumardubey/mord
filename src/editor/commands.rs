//! Markdown edits as pure functions: one `Edit` per command, no GPUI.

use crate::editor::buffer::DocumentBuffer;
use crate::editor::prefix::{parse_prefix, LinePrefixKind};

const INDENT: &str = "    ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub new_text: String,
    pub caret: usize,
}

impl Edit {
    pub fn apply(self, buffer: &mut DocumentBuffer) -> usize {
        buffer.replace_range(self.start, self.end, &self.new_text);
        self.caret
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapKind {
    Bold,
    Italic,
    Code,
}

impl WrapKind {
    fn mark(self) -> &'static str {
        match self {
            WrapKind::Bold => "**",
            WrapKind::Italic => "*",
            WrapKind::Code => "`",
        }
    }
}

/// Wrap or unwrap the selection. Collapsed caret does nothing (no empty `****`).
pub fn wrap_marks(
    buffer: &DocumentBuffer,
    sel_start: usize,
    sel_end: usize,
    kind: WrapKind,
) -> Option<Edit> {
    let mut start = sel_start.min(sel_end);
    let mut end = sel_start.max(sel_end);
    let len = buffer.len_chars();
    start = start.min(len);
    end = end.min(len);
    let mark = kind.mark();
    let mark_len = mark.chars().count();

    if start == end {
        return None;
    }

    let selected = buffer.slice_to_string(start, end);
    if selected.starts_with(mark)
        && selected.ends_with(mark)
        && selected.chars().count() >= mark_len * 2
    {
        let inner: String = selected
            .chars()
            .skip(mark_len)
            .take(selected.chars().count() - mark_len * 2)
            .collect();
        let caret = start + inner.chars().count();
        return Some(Edit {
            start,
            end,
            new_text: inner,
            caret,
        });
    }

    let wrapped = format!("{}{}{}", mark, selected, mark);
    let caret = start + wrapped.chars().count();
    Some(Edit {
        start,
        end,
        new_text: wrapped,
        caret,
    })
}

/// Set heading level on a line (`0` = paragraph). Rewrites the line prefix as one `Edit`.
pub fn set_heading_level(buffer: &DocumentBuffer, line_idx: usize, level: usize) -> Edit {
    let level = level.min(6);
    let line = buffer.line_without_newline(line_idx).unwrap_or_default();
    let line_start = buffer.line_to_char(line_idx);
    let line_len = line.chars().count();

    let content = if let Some(prefix) = parse_prefix(&line) {
        match prefix.kind {
            LinePrefixKind::Heading { .. }
            | LinePrefixKind::Blockquote { .. }
            | LinePrefixKind::Unordered { .. }
            | LinePrefixKind::Ordered { .. }
            | LinePrefixKind::Task { .. } => prefix.content_owned(&line),
        }
    } else {
        line.clone()
    };

    let new_line = if level == 0 {
        content
    } else {
        format!("{} {}", "#".repeat(level), content)
    };
    let new_prefix_len = if level == 0 { 0 } else { level + 1 };
    Edit {
        start: line_start,
        end: line_start + line_len,
        new_text: new_line,
        caret: line_start + new_prefix_len,
    }
}

/// Enter: continue list/task/quote, or exit an empty item.
pub fn insert_newline(buffer: &DocumentBuffer, caret: usize) -> Edit {
    let point = buffer.char_offset_to_point(caret);
    let current_line = buffer.line_without_newline(point.row).unwrap_or_default();
    let line_start = buffer.line_to_char(point.row);

    if let Some(prefix) = parse_prefix(&current_line) {
        let content = prefix.content_owned(&current_line);
        let item_empty = content.trim().is_empty()
            && matches!(
                prefix.kind,
                LinePrefixKind::Task { .. }
                    | LinePrefixKind::Unordered { .. }
                    | LinePrefixKind::Ordered { .. }
                    | LinePrefixKind::Blockquote { .. }
            );
        if item_empty {
            // Exit list: strip prefix, caret at line start.
            return Edit {
                start: line_start,
                end: caret,
                new_text: String::new(),
                caret: line_start,
            };
        }
        let prefix_text = prefix.continue_prefix().unwrap_or_else(|| {
            current_line
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect()
        });
        let new_text = format!("\n{}", prefix_text);
        let caret_after = caret + new_text.chars().count();
        return Edit {
            start: caret,
            end: caret,
            new_text,
            caret: caret_after,
        };
    }

    let indent: String = current_line
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let new_text = format!("\n{}", indent);
    let caret_after = caret + new_text.chars().count();
    Edit {
        start: caret,
        end: caret,
        new_text,
        caret: caret_after,
    }
}

/// Backspace: selection delete, join lines, strip prefix, outdent, or grapheme delete.
pub fn backspace(buffer: &DocumentBuffer, sel_start: usize, sel_end: usize) -> Option<Edit> {
    let mut start = sel_start.min(sel_end);
    let mut end = sel_start.max(sel_end);
    let len = buffer.len_chars();
    start = start.min(len);
    end = end.min(len);

    if start != end {
        return Some(Edit {
            start,
            end,
            new_text: String::new(),
            caret: start,
        });
    }

    let pos = start;
    if pos == 0 {
        return None;
    }

    let point = buffer.char_offset_to_point(pos);
    let current_line = buffer.line_without_newline(point.row).unwrap_or_default();
    let line_start = buffer.line_to_char(point.row);

    if point.col == 0 {
        if point.row == 0 {
            return None;
        }
        let prev_line = buffer.line_without_newline(point.row - 1).unwrap_or_default();
        let prev_line_start = buffer.line_to_char(point.row - 1);
        let join_point = prev_line_start + prev_line.chars().count();
        return Some(Edit {
            start: join_point,
            end: line_start,
            new_text: String::new(),
            caret: join_point,
        });
    }

    let indent_len = current_line
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .count();
    if point.col > 0 && point.col <= indent_len {
        let delete_count = if current_line
            .chars()
            .take(point.col)
            .collect::<String>()
            .ends_with(INDENT)
        {
            4
        } else {
            1
        };
        let del_start = pos - delete_count;
        return Some(Edit {
            start: del_start,
            end: pos,
            new_text: String::new(),
            caret: del_start,
        });
    }

    if let Some(prefix) = parse_prefix(&current_line) {
        if point.col <= prefix.prefix_len {
            return Some(Edit {
                start: line_start,
                end: line_start + prefix.prefix_len,
                new_text: String::new(),
                caret: line_start,
            });
        }
    }

    let text = buffer.text();
    let prev = crate::editor::selection::previous_grapheme_char_offset(&text, pos);
    Some(Edit {
        start: prev,
        end: pos,
        new_text: String::new(),
        caret: prev,
    })
}

/// Toggle task checkbox on a line; returns None if not a task.
pub fn toggle_task(buffer: &DocumentBuffer, line_idx: usize) -> Option<Edit> {
    let line = buffer.line_without_newline(line_idx)?;
    let prefix = parse_prefix(&line)?;
    let replaced = prefix.toggle_task_line(&line)?;
    let line_start = buffer.line_to_char(line_idx);
    let line_len = line.chars().count();
    Some(Edit {
        start: line_start,
        end: line_start + line_len,
        new_text: replaced,
        caret: line_start, // caller may preserve caret; default line start
    })
}

/// Indent lines covering [sel_start, sel_end) by one step (4 spaces at line start).
pub fn indent_lines(buffer: &DocumentBuffer, sel_start: usize, sel_end: usize) -> Edit {
    indent_or_outdent(buffer, sel_start, sel_end, true)
}

/// Outdent lines covering [sel_start, sel_end) by one indent step.
pub fn outdent_lines(buffer: &DocumentBuffer, sel_start: usize, sel_end: usize) -> Edit {
    indent_or_outdent(buffer, sel_start, sel_end, false)
}

fn indent_or_outdent(
    buffer: &DocumentBuffer,
    sel_start: usize,
    sel_end: usize,
    indent: bool,
) -> Edit {
    let a = sel_start.min(sel_end).min(buffer.len_chars());
    let mut b = sel_start.max(sel_end).min(buffer.len_chars());
    // If selection ends at a line start and is non-empty, don't include that line.
    if a < b && b > 0 {
        let end_point = buffer.char_offset_to_point(b);
        if end_point.col == 0 {
            b -= 1;
        }
    }
    let start_row = buffer.char_to_line(a);
    let end_row = buffer.char_to_line(b);
    let range_start = buffer.line_to_char(start_row);
    let range_end = if end_row + 1 < buffer.len_lines() {
        buffer.line_to_char(end_row + 1)
    } else {
        buffer.len_chars()
    };

    let mut out = String::new();
    let mut caret_delta = 0isize;
    let indent_chars = INDENT.chars().count() as isize;

    for row in start_row..=end_row {
        let line = buffer.line_without_newline(row).unwrap_or_default();
        let with_nl = if row + 1 < buffer.len_lines() || buffer.text().ends_with('\n') {
            // Preserve newline if this line had one in the slice
            let line_start = buffer.line_to_char(row);
            let line_end = if row + 1 < buffer.len_lines() {
                buffer.line_to_char(row + 1)
            } else {
                buffer.len_chars()
            };
            buffer.slice_to_string(line_start, line_end)
        } else {
            line.clone()
        };

        if indent {
            out.push_str(INDENT);
            out.push_str(&with_nl);
            caret_delta += indent_chars;
        } else {
            let stripped = strip_one_indent(&with_nl);
            let removed = with_nl.chars().count() as isize - stripped.chars().count() as isize;
            caret_delta -= removed;
            out.push_str(&stripped);
        }
    }

    let new_caret = if indent {
        (a as isize + indent_chars).max(0) as usize
    } else {
        let strip_on_first = {
            let first = buffer.line_without_newline(start_row).unwrap_or_default();
            let full = if start_row + 1 < buffer.len_lines() {
                buffer.slice_to_string(
                    buffer.line_to_char(start_row),
                    buffer.line_to_char(start_row + 1),
                )
            } else {
                first
            };
            full.chars().count() as isize - strip_one_indent(&full).chars().count() as isize
        };
        (a as isize - strip_on_first).max(range_start as isize) as usize
    };

    let _ = caret_delta;
    Edit {
        start: range_start,
        end: range_end,
        new_text: out,
        caret: new_caret,
    }
}

fn strip_one_indent(line: &str) -> String {
    if line.starts_with(INDENT) {
        line.chars().skip(4).collect()
    } else if line.starts_with('\t') {
        line.chars().skip(1).collect()
    } else if line.starts_with(' ') {
        let n = line.chars().take_while(|&c| c == ' ').count().min(4);
        line.chars().skip(n).collect()
    } else {
        line.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(buffer: &mut DocumentBuffer, edit: Edit) -> usize {
        edit.apply(buffer)
    }

    #[test]
    fn continue_star_task() {
        let mut buf = DocumentBuffer::from_str("* [ ] Hello");
        let caret = buf.len_chars();
        let edit = insert_newline(&buf, caret);
        apply(&mut buf, edit);
        assert_eq!(buf.text(), "* [ ] Hello\n* [ ] ");
    }

    #[test]
    fn exit_empty_task_item() {
        let mut buf = DocumentBuffer::from_str("* [ ] ");
        let caret = buf.len_chars();
        let edit = insert_newline(&buf, caret);
        apply(&mut buf, edit);
        assert_eq!(buf.text(), "");
    }

    #[test]
    fn wrap_and_unwrap_bold() {
        let mut buf = DocumentBuffer::from_str("hello");
        let edit = wrap_marks(&buf, 0, 5, WrapKind::Bold).unwrap();
        apply(&mut buf, edit);
        assert_eq!(buf.text(), "**hello**");
        let edit = wrap_marks(&buf, 0, buf.len_chars(), WrapKind::Bold).unwrap();
        apply(&mut buf, edit);
        assert_eq!(buf.text(), "hello");
    }

    #[test]
    fn collapsed_wrap_does_nothing() {
        let buf = DocumentBuffer::from_str("hello");
        assert!(wrap_marks(&buf, 2, 2, WrapKind::Bold).is_none());
        assert!(wrap_marks(&buf, 2, 2, WrapKind::Italic).is_none());
        assert!(wrap_marks(&buf, 2, 2, WrapKind::Code).is_none());
    }

    #[test]
    fn set_heading_level_wrap_unwrap() {
        let mut buf = DocumentBuffer::from_str("My Title");
        let edit = set_heading_level(&buf, 0, 3);
        apply(&mut buf, edit);
        assert_eq!(buf.text(), "### My Title");

        let edit = set_heading_level(&buf, 0, 1);
        apply(&mut buf, edit);
        assert_eq!(buf.text(), "# My Title");

        let edit = set_heading_level(&buf, 0, 0);
        apply(&mut buf, edit);
        assert_eq!(buf.text(), "My Title");
    }

    #[test]
    fn set_heading_from_list_strips_marker() {
        let mut buf = DocumentBuffer::from_str("- item");
        let edit = set_heading_level(&buf, 0, 2);
        apply(&mut buf, edit);
        assert_eq!(buf.text(), "## item");
    }

    #[test]
    fn indent_line() {
        let mut buf = DocumentBuffer::from_str("item");
        let edit = indent_lines(&buf, 0, 0);
        apply(&mut buf, edit);
        assert_eq!(buf.text(), "    item");
    }

    #[test]
    fn toggle_indented_star_task() {
        let mut buf = DocumentBuffer::from_str("  * [ ] Hi");
        let edit = toggle_task(&buf, 0).unwrap();
        apply(&mut buf, edit);
        assert_eq!(buf.text(), "  * [x] Hi");
    }
}
