use crate::editor::{
    buffer::DocumentBuffer,
    decorator::{ConcealMode, Decorator},
    layout::hit_test_wrapped_line,
    offset::BufferOffset,
    parser::{BlockKind, MarkdownParser},
    prefix::{parse_prefix, LinePrefixKind},
    selection::CursorManager,
};
use crate::ui::document_line::{DocumentLine, LineCacheMap};
use crate::ui::theme::Theme;
use gpui_kit::gpui::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

pub struct EditorView {
    pub buffer: DocumentBuffer,
    pub cursor: CursorManager,
    pub conceal_mode: ConcealMode,
    pub theme: Theme,
    pub focus_handle: FocusHandle,
    pub dragging: bool,
    pub cursor_visible: bool,
    blink_task: Task<()>,
    line_caches: LineCacheMap,
    scroll_handle: ScrollHandle,
}

impl EditorView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let sample_text = r#"# Welcome to Mord

Mord is a **high-performance** Word-like *live inline preview* Markdown editor built with Rust and GPUI.

### Key Capabilities
1. High-speed GPU accelerated text layout
2. Token-level live inline syntax reveal
3. Accurate mouse click positioning
4. Smart auto-indenting lists and task lists

### Checklist
- [x] Try clicking anywhere inside a word on this line
- [ ] Test numbered list auto-continuation (press Enter at the end of an item)
- [ ] Toggle Dark/Light mode using the top button

> "Simplicity is prerequisite for reliability." — Edsger W. Dijkstra

Here is some `inline code` and a link to [GPUI Kit](https://gpui-kit.com).

Type anywhere to see live inline syntax formatting in action!
"#;

        let focus_handle = cx.focus_handle();
        let mut view = Self {
            buffer: DocumentBuffer::from_str(sample_text),
            cursor: CursorManager::new(),
            conceal_mode: ConcealMode::TokenReveal,
            theme: Theme::dark(),
            focus_handle: focus_handle.clone(),
            dragging: false,
            cursor_visible: true,
            blink_task: Task::ready(()),
            line_caches: Rc::new(RefCell::new(HashMap::new())),
            scroll_handle: ScrollHandle::new(),
        };
        view.start_blink(cx);
        view
    }

    fn start_blink(&mut self, cx: &mut Context<Self>) {
        self.cursor_visible = true;
        self.blink_task = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(530))
                    .await;
                if this
                    .update(cx, |view, cx| {
                        view.cursor_visible = !view.cursor_visible;
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
    }

    fn reset_blink(&mut self, cx: &mut Context<Self>) {
        self.cursor_visible = true;
        self.start_blink(cx);
    }

    pub fn focus_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        self.reset_blink(cx);
    }

    pub fn set_conceal_mode(&mut self, mode: ConcealMode, cx: &mut Context<Self>) {
        self.conceal_mode = mode;
        cx.notify();
    }

    pub fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        if self.theme.bg_editor == 0x121214 {
            self.theme = Theme::light();
        } else {
            self.theme = Theme::dark();
        }
        cx.notify();
    }

    fn parsed_document(&self) -> Vec<crate::editor::ParsedLine> {
        let lines: Vec<String> = (0..self.buffer.len_lines())
            .map(|i| self.buffer.line_without_newline(i).unwrap_or_default())
            .collect();
        MarkdownParser::parse_document(&lines)
    }

    fn decorate_row(&self, row: usize) -> crate::editor::DecoratedLine {
        let parsed_doc = self.parsed_document();
        self.decorate_parsed_row(row, &parsed_doc)
    }

    fn decorate_parsed_row(
        &self,
        row: usize,
        parsed_doc: &[crate::editor::ParsedLine],
    ) -> crate::editor::DecoratedLine {
        let cursor_point = self
            .buffer
            .char_offset_to_point(self.cursor.cursor_offset().get());
        let cursor_col = if row == cursor_point.row {
            Some(cursor_point.col)
        } else {
            None
        };
        let parsed = parsed_doc.get(row).cloned().unwrap_or_else(|| {
            MarkdownParser::parse_line(
                &self.buffer.line_without_newline(row).unwrap_or_default(),
            )
        });
        Decorator::decorate_line(row, &parsed, cursor_col, self.conceal_mode)
    }

    fn scroll_caret_into_view(&self) {
        let row = self
            .buffer
            .char_offset_to_point(self.cursor.cursor_offset().get())
            .row;
        self.scroll_handle.scroll_to_item(row);
    }

    pub fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if !self.cursor.selection.is_collapsed() {
            let range = self.cursor.selection.range();
            self.buffer.replace_range(range.start, range.end, text);
            self.cursor
                .set_cursor(BufferOffset(range.start + text.chars().count()));
        } else {
            let pos = self.cursor.cursor_offset().get();
            self.buffer.insert(pos, text);
            self.cursor
                .set_cursor(BufferOffset(pos + text.chars().count()));
        }
        self.reset_blink(cx);
        self.scroll_caret_into_view();
        cx.notify();
    }

    pub fn backspace(&mut self, cx: &mut Context<Self>) {
        if !self.cursor.selection.is_collapsed() {
            let range = self.cursor.selection.range();
            self.buffer.delete_range(range.start, range.end);
            self.cursor.set_cursor(BufferOffset(range.start));
            self.reset_blink(cx);
            cx.notify();
            return;
        }
        let pos = self.cursor.cursor_offset().get();
        if pos == 0 {
            return;
        }

        let point = self.buffer.char_offset_to_point(pos);
        let current_line = self
            .buffer
            .line_without_newline(point.row)
            .unwrap_or_default();
        let line_start = self.buffer.line_to_char(point.row);

        if point.col == 0 {
            if point.row > 0 {
                let prev_line = self
                    .buffer
                    .line_without_newline(point.row - 1)
                    .unwrap_or_default();
                let prev_line_start = self.buffer.line_to_char(point.row - 1);
                let prev_line_len = prev_line.chars().count();
                let join_point = prev_line_start + prev_line_len;

                self.buffer.delete_range(join_point, line_start);
                self.cursor.set_cursor(BufferOffset(join_point));
                self.reset_blink(cx);
                cx.notify();
            }
            return;
        }

        let indent_len = current_line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .count();
        if point.col > 0 && point.col <= indent_len {
            let delete_count = if current_line[..point.col].ends_with("    ") {
                4
            } else {
                1
            };
            self.buffer.delete_range(pos - delete_count, pos);
            self.cursor.set_cursor(BufferOffset(pos - delete_count));
            self.reset_blink(cx);
            cx.notify();
            return;
        }

        if let Some(prefix) = parse_prefix(&current_line) {
            if point.col <= prefix.prefix_len {
                self.buffer
                    .delete_range(line_start, line_start + prefix.prefix_len);
                self.cursor.set_cursor(BufferOffset(line_start));
                self.reset_blink(cx);
                cx.notify();
                return;
            }
        }

        // Grapheme-aware single delete
        let text = self.buffer.text();
        let prev = previous_grapheme_char_offset(&text, pos);
        self.buffer.delete_range(prev, pos);
        self.cursor.set_cursor(BufferOffset(prev));
        self.reset_blink(cx);
        cx.notify();
    }

    pub fn delete_forward(&mut self, cx: &mut Context<Self>) {
        if !self.cursor.selection.is_collapsed() {
            let range = self.cursor.selection.range();
            self.buffer.delete_range(range.start, range.end);
            self.cursor.set_cursor(BufferOffset(range.start));
            self.reset_blink(cx);
            cx.notify();
            return;
        }
        let pos = self.cursor.cursor_offset().get();
        let len = self.buffer.len_chars();
        if pos < len {
            let text = self.buffer.text();
            let next = next_grapheme_char_offset(&text, pos);
            self.buffer.delete_range(pos, next);
            self.reset_blink(cx);
            cx.notify();
        }
    }

    pub fn insert_newline(&mut self, cx: &mut Context<Self>) {
        let point = self
            .buffer
            .char_offset_to_point(self.cursor.cursor_offset().get());
        let current_line = self
            .buffer
            .line_without_newline(point.row)
            .unwrap_or_default();
        let line_start = self.buffer.line_to_char(point.row);

        let prefix_text = if let Some(prefix) = parse_prefix(&current_line) {
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
                self.buffer
                    .replace_range(line_start, self.cursor.cursor_offset().get(), "");
                self.cursor.set_cursor(BufferOffset(line_start));
                self.reset_blink(cx);
                cx.notify();
                return;
            }
            prefix.continue_prefix().unwrap_or_else(|| {
                current_line
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect()
            })
        } else {
            current_line
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect()
        };

        let insert_str = format!("\n{}", prefix_text);
        self.insert_text(&insert_str, cx);
    }

    pub fn toggle_bold(&mut self, cx: &mut Context<Self>) {
        if self.cursor.selection.is_collapsed() {
            self.insert_text("****", cx);
            self.cursor
                .set_cursor(self.cursor.cursor_offset() - 2);
        } else {
            let range = self.cursor.selection.range();
            let selected = self.buffer.slice_to_string(range.start, range.end);
            let wrapped = format!("**{}**", selected);
            self.buffer.replace_range(range.start, range.end, &wrapped);
            self.cursor
                .set_cursor(BufferOffset(range.start + wrapped.chars().count()));
        }
        self.reset_blink(cx);
        cx.notify();
    }

    pub fn toggle_italic(&mut self, cx: &mut Context<Self>) {
        if self.cursor.selection.is_collapsed() {
            self.insert_text("**", cx);
            self.cursor
                .set_cursor(self.cursor.cursor_offset() - 1);
        } else {
            let range = self.cursor.selection.range();
            let selected = self.buffer.slice_to_string(range.start, range.end);
            let wrapped = format!("*{}*", selected);
            self.buffer.replace_range(range.start, range.end, &wrapped);
            self.cursor
                .set_cursor(BufferOffset(range.start + wrapped.chars().count()));
        }
        self.reset_blink(cx);
        cx.notify();
    }

    pub fn toggle_code(&mut self, cx: &mut Context<Self>) {
        if self.cursor.selection.is_collapsed() {
            self.insert_text("``", cx);
            self.cursor
                .set_cursor(self.cursor.cursor_offset() - 1);
        } else {
            let range = self.cursor.selection.range();
            let selected = self.buffer.slice_to_string(range.start, range.end);
            let wrapped = format!("`{}`", selected);
            self.buffer.replace_range(range.start, range.end, &wrapped);
            self.cursor
                .set_cursor(BufferOffset(range.start + wrapped.chars().count()));
        }
        self.reset_blink(cx);
        cx.notify();
    }

    pub fn toggle_task_at_line(&mut self, line_idx: usize, cx: &mut Context<Self>) {
        if let Some(line) = self.buffer.line_without_newline(line_idx) {
            if let Some(prefix) = parse_prefix(&line) {
                if let Some(replaced) = prefix.toggle_task_line(&line) {
                    let line_start = self.buffer.line_to_char(line_idx);
                    self.buffer.replace_range(
                        line_start,
                        line_start + line.chars().count(),
                        &replaced,
                    );
                    cx.notify();
                }
            }
        }
    }

    pub fn word_count(&self) -> usize {
        self.buffer.text().split_whitespace().count()
    }

    pub fn char_count(&self) -> usize {
        self.buffer.len_chars()
    }

    fn hit_test_at(
        &self,
        window: &mut Window,
        line_idx: usize,
        position: Point<Pixels>,
    ) -> Option<(BufferOffset, bool)> {
        let cache = self.line_caches.borrow().get(&line_idx).cloned()?;
        let local = point(
            position.x - cache.bounds.left(),
            position.y - cache.bounds.top(),
        );
        let decorated = self.decorate_row(line_idx);
        let paint = self.theme.paint_theme();
        let input = crate::editor::layout::build_shaped_line_input(
            &decorated,
            &paint,
            window.text_style().font(),
            true,
        );
        let wrapped = window
            .text_system()
            .shape_text(
                input.text,
                input.font_size,
                &input.runs,
                Some(cache.wrap_width),
                None,
            )
            .ok()
            .and_then(|mut lines| lines.pop())?;
        let hit = hit_test_wrapped_line(
            &wrapped,
            &decorated,
            cache.line_len,
            local,
            cache.line_height,
        );
        let line_start = self.buffer.line_to_char(line_idx);
        let offset =
            BufferOffset((line_start + hit.buffer_col.get()).min(line_start + cache.line_len));
        Some((offset, hit.on_task_marker))
    }

    pub fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = &event.keystroke.modifiers;

        if modifiers.control || modifiers.platform {
            match key {
                "z" | "Z" => {
                    if modifiers.shift {
                        if let Some(pos) = self.buffer.redo() {
                            self.cursor.set_cursor(BufferOffset(pos));
                            self.reset_blink(cx);
                            cx.notify();
                        }
                    } else if let Some(pos) = self.buffer.undo() {
                        self.cursor.set_cursor(BufferOffset(pos));
                        self.reset_blink(cx);
                        cx.notify();
                    }
                    return;
                }
                "y" | "Y" => {
                    if let Some(pos) = self.buffer.redo() {
                        self.cursor.set_cursor(BufferOffset(pos));
                        self.reset_blink(cx);
                        cx.notify();
                    }
                    return;
                }
                "a" | "A" => {
                    self.cursor.select_all(&self.buffer);
                    cx.notify();
                    return;
                }
                "b" | "B" => {
                    self.toggle_bold(cx);
                    return;
                }
                "i" | "I" => {
                    self.toggle_italic(cx);
                    return;
                }
                "e" | "E" => {
                    self.toggle_code(cx);
                    return;
                }
                _ => {}
            }
        }

        match key {
            "backspace" => self.backspace(cx),
            "delete" => self.delete_forward(cx),
            "enter" => self.insert_newline(cx),
            "left" => {
                self.cursor.move_left(&self.buffer, modifiers.shift);
                self.reset_blink(cx);
                self.scroll_caret_into_view();
                cx.notify();
            }
            "right" => {
                self.cursor.move_right(&self.buffer, modifiers.shift);
                self.reset_blink(cx);
                self.scroll_caret_into_view();
                cx.notify();
            }
            "up" => {
                let point = self
                    .buffer
                    .char_offset_to_point(self.cursor.cursor_offset().get());
                if point.row > 0 {
                    let current = self.decorate_row(point.row);
                    let target = self.decorate_row(point.row - 1);
                    self.cursor
                        .move_up(&self.buffer, &current, &target, modifiers.shift);
                } else {
                    self.cursor.set_head(BufferOffset(0), modifiers.shift);
                }
                self.reset_blink(cx);
                self.scroll_caret_into_view();
                cx.notify();
            }
            "down" => {
                let point = self
                    .buffer
                    .char_offset_to_point(self.cursor.cursor_offset().get());
                let max_line = self.buffer.len_lines().saturating_sub(1);
                if point.row < max_line {
                    let current = self.decorate_row(point.row);
                    let target = self.decorate_row(point.row + 1);
                    self.cursor
                        .move_down(&self.buffer, &current, &target, modifiers.shift);
                } else {
                    self.cursor
                        .set_head(BufferOffset(self.buffer.len_chars()), modifiers.shift);
                }
                self.reset_blink(cx);
                self.scroll_caret_into_view();
                cx.notify();
            }
            "home" => {
                self.cursor.move_to_line_start(&self.buffer, modifiers.shift);
                self.reset_blink(cx);
                cx.notify();
            }
            "end" => {
                self.cursor.move_to_line_end(&self.buffer, modifiers.shift);
                self.reset_blink(cx);
                cx.notify();
            }
            "tab" => {
                self.insert_text("    ", cx);
            }
            _ => {
                if let Some(ch) = &event.keystroke.key_char {
                    if !modifiers.control && !modifiers.platform && !modifiers.alt {
                        self.insert_text(ch, cx);
                    }
                } else if key.chars().count() == 1
                    && !modifiers.control
                    && !modifiers.platform
                    && !modifiers.alt
                {
                    self.insert_text(key, cx);
                }
            }
        }
    }
}

impl Focusable for EditorView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EditorView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cursor_point = self
            .buffer
            .char_offset_to_point(self.cursor.cursor_offset().get());
        let current_row = cursor_point.row;
        let theme = &self.theme;
        let paint_theme = theme.paint_theme();

        let bg_app = rgb(theme.bg_app);
        let bg_editor = rgb(theme.bg_editor);
        let bg_active_line = rgb(theme.bg_active_line);
        let text_primary = rgb(theme.text_primary);
        let text_muted = rgb(theme.text_muted);
        let border_color = rgb(theme.border_subtle);

        let total_lines = self.buffer.len_lines();
        let parsed_doc = self.parsed_document();
        let word_count = self.word_count();
        let char_count = self.char_count();
        let sel_range = if self.cursor.selection.is_collapsed() {
            None
        } else {
            Some(self.cursor.selection.range())
        };
        let caret_visible = self.cursor_visible && self.focus_handle.is_focused(window);
        let line_caches = self.line_caches.clone();

        // Clear stale caches each frame; paint will refill.
        line_caches.borrow_mut().clear();

        let navbar = div()
            .flex()
            .items_center()
            .justify_between()
            .px_6()
            .py_3()
            .border_b_1()
            .border_color(border_color)
            .bg(bg_app)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w_7()
                            .h_7()
                            .rounded_md()
                            .bg(rgb(0x4F46E5))
                            .text_color(rgb(0xFFFFFF))
                            .font_weight(FontWeight::BOLD)
                            .child("M"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_color(text_primary)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Mord"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .child("Word-like Live Markdown"),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .p_1()
                    .rounded_lg()
                    .bg(rgb(theme.bg_code_inline))
                    .gap_1()
                    .child(
                        mode_chip("Token", self.conceal_mode == ConcealMode::TokenReveal, theme)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|view, _, _, cx| {
                                    view.set_conceal_mode(ConcealMode::TokenReveal, cx);
                                }),
                            ),
                    )
                    .child(
                        mode_chip("Line", self.conceal_mode == ConcealMode::LineReveal, theme)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|view, _, _, cx| {
                                    view.set_conceal_mode(ConcealMode::LineReveal, cx);
                                }),
                            ),
                    )
                    .child(
                        mode_chip("Raw", self.conceal_mode == ConcealMode::Raw, theme)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|view, _, _, cx| {
                                    view.set_conceal_mode(ConcealMode::Raw, cx);
                                }),
                            ),
                    ),
            )
            .child(
                div()
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(border_color)
                    .cursor_pointer()
                    .text_color(text_primary)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|view, _, _, cx| view.toggle_theme(cx)),
                    )
                    .child(if theme.bg_editor == 0x121214 {
                        "Light"
                    } else {
                        "Dark"
                    }),
            );

        let mut document_lines = div()
            .id("document-lines")
            .flex()
            .flex_col()
            .w_full()
            .max_w(px(860.0))
            .bg(bg_editor)
            .p_8()
            .rounded_xl()
            .border_1()
            .border_color(border_color)
            .gap_1();

        for row in 0..total_lines {
            let raw_line = self.buffer.line_without_newline(row).unwrap_or_default();
            let is_active = row == current_row;
            let decorated = self.decorate_parsed_row(row, &parsed_doc);
            let line_start = self.buffer.line_to_char(row);
            let line_len = raw_line.chars().count();
            let block_kind = decorated.block_kind.clone();

            let caret_visual = if is_active {
                Some(crate::editor::offset::VisualCol(
                    Decorator::buffer_col_to_visual_col(&decorated, cursor_point.col),
                ))
            } else {
                None
            };

            let mut line_row = div()
                .id(("line", row))
                .flex()
                .items_start()
                .w_full()
                .py_1()
                .px_2()
                .rounded_md()
                .cursor_text()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                        view.focus_editor(window, cx);
                        if let Some((offset, on_task)) =
                            view.hit_test_at(window, row, event.position)
                        {
                            if on_task && !event.modifiers.shift {
                                view.toggle_task_at_line(row, cx);
                                return;
                            }
                            view.dragging = true;
                            view.cursor
                                .set_head(offset, event.modifiers.shift);
                            view.reset_blink(cx);
                            cx.notify();
                        }
                    }),
                )
                .on_mouse_move(cx.listener(move |view, event: &MouseMoveEvent, window, cx| {
                    if view.dragging {
                        if let Some((offset, _)) = view.hit_test_at(window, row, event.position) {
                            view.cursor.set_head(offset, true);
                            view.reset_blink(cx);
                            cx.notify();
                        }
                    }
                }))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|view, _: &MouseUpEvent, _, cx| {
                        view.dragging = false;
                        cx.notify();
                    }),
                );

            if is_active {
                line_row = line_row.bg(bg_active_line);
            }

            line_row = line_row.child(
                div()
                    .w_8()
                    .pt_1()
                    .text_xs()
                    .text_color(if is_active {
                        rgb(theme.text_accent)
                    } else {
                        rgb(theme.text_muted)
                    })
                    .font_weight(if is_active {
                        FontWeight::BOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .child(format!("{}", row + 1)),
            );

            if matches!(block_kind, BlockKind::Blockquote { .. }) {
                line_row = line_row.child(
                    div()
                        .w_1()
                        .h_full()
                        .min_h(px(20.0))
                        .bg(rgb(theme.border_quote))
                        .rounded_full()
                        .mr_3(),
                );
            }

            line_row = line_row.child(
                div().flex_1().min_w_0().child(DocumentLine {
                    line_index: row,
                    decorated,
                    line_len,
                    line_start,
                    paint_theme,
                    is_active,
                    caret_visual_col: caret_visual,
                    caret_visible: caret_visible && is_active,
                    selection: sel_range.clone(),
                    line_caches: line_caches.clone(),
                }),
            );

            document_lines = document_lines.child(line_row);
        }

        let mode_label = match self.conceal_mode {
            ConcealMode::TokenReveal => "Token Reveal",
            ConcealMode::LineReveal => "Line Reveal",
            ConcealMode::Raw => "Raw",
        };

        let status_bar = div()
            .flex()
            .items_center()
            .justify_between()
            .px_6()
            .py_2()
            .border_t_1()
            .border_color(border_color)
            .bg(bg_app)
            .text_xs()
            .text_color(text_muted)
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(format!(
                        "Ln {}, Col {}",
                        cursor_point.row + 1,
                        cursor_point.col + 1
                    ))
                    .child(format!("{} lines", total_lines)),
            )
            .child(div().child(format!("Mode: {}", mode_label)))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(format!("{} words", word_count))
                    .child(format!("{} characters", char_count)),
            );

        div()
            .track_focus(&self.focus_handle)
            .key_context("MordEditor")
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, _, cx| {
                view.handle_key_down(event, cx);
            }))
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|view, _: &MouseUpEvent, _, _cx| {
                    view.dragging = false;
                }),
            )
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(bg_app)
            .child(navbar)
            .child(
                div()
                    .id("editor-scroll")
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .justify_center()
                    .p_8()
                    .child(document_lines),
            )
            .child(status_bar)
    }
}

fn mode_chip(label: &'static str, active: bool, theme: &Theme) -> Div {
    div()
        .px_3()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .bg(if active {
            rgb(0x4F46E5)
        } else {
            rgb(theme.bg_code_inline)
        })
        .text_color(if active {
            rgb(0xFFFFFF)
        } else {
            rgb(theme.text_muted)
        })
        .font_weight(FontWeight::MEDIUM)
        .child(label)
}

fn previous_grapheme_char_offset(text: &str, char_offset: usize) -> usize {
    crate::editor::selection::previous_grapheme_char_offset(text, char_offset)
}

fn next_grapheme_char_offset(text: &str, char_offset: usize) -> usize {
    crate::editor::selection::next_grapheme_char_offset(text, char_offset)
}
