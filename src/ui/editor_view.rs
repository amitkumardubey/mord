use gpui_component::scroll::ScrollableElement;
use gpui_kit::gpui::*;
use crate::editor::{
    buffer::DocumentBuffer,
    decorator::{ConcealMode, Decorator, VisualFontStyle, VisualFontWeight},
    parser::{BlockKind, MarkdownParser, MarkerType},
    selection::CursorManager,
};
use crate::ui::theme::Theme;

pub struct EditorView {
    pub buffer: DocumentBuffer,
    pub cursor: CursorManager,
    pub conceal_mode: ConcealMode,
    pub theme: Theme,
    pub focus_handle: FocusHandle,
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

        Self {
            buffer: DocumentBuffer::from_str(sample_text),
            cursor: CursorManager::new(),
            conceal_mode: ConcealMode::TokenReveal,
            theme: Theme::dark(),
            focus_handle: cx.focus_handle(),
        }
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

    pub fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if !self.cursor.selection.is_collapsed() {
            let range = self.cursor.selection.range();
            self.buffer.delete_range(range.start, range.end);
            self.cursor.set_cursor(range.start);
        }
        let pos = self.cursor.cursor_offset();
        self.buffer.insert(pos, text);
        self.cursor.set_cursor(pos + text.chars().count());
        cx.notify();
    }

    pub fn backspace(&mut self, cx: &mut Context<Self>) {
        if !self.cursor.selection.is_collapsed() {
            let range = self.cursor.selection.range();
            self.buffer.delete_range(range.start, range.end);
            self.cursor.set_cursor(range.start);
            cx.notify();
            return;
        }
        let pos = self.cursor.cursor_offset();
        if pos == 0 {
            return;
        }

        let point = self.buffer.char_offset_to_point(pos);
        let current_line = self.buffer.line_without_newline(point.row).unwrap_or_default();
        let line_start = self.buffer.line_to_char(point.row);

        // If cursor is at the very beginning of a line (point.col == 0)
        if point.col == 0 {
            if point.row > 0 {
                // Merge with previous line, landing exactly at the end of the previous line!
                let prev_line = self.buffer.line_without_newline(point.row - 1).unwrap_or_default();
                let prev_line_start = self.buffer.line_to_char(point.row - 1);
                let prev_line_len = prev_line.chars().count();
                let join_point = prev_line_start + prev_line_len;

                self.buffer.delete_range(join_point, line_start);
                self.cursor.set_cursor(join_point);
                cx.notify();
            }
            return;
        }

        let indent_len = current_line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
        if point.col > 0 && point.col <= indent_len {
            // Unindent: delete up to 4 spaces or 1 tab
            let delete_count = if current_line[..point.col].ends_with("    ") {
                4
            } else {
                1
            };
            self.buffer.delete_range(pos - delete_count, pos);
            self.cursor.set_cursor(pos - delete_count);
            cx.notify();
            return;
        }

        let after_indent = &current_line[indent_len..];
        let prefix_len = if after_indent.starts_with("- [ ] ") || after_indent.starts_with("- [x] ") {
            Some(indent_len + 6)
        } else if after_indent.starts_with("- ") || after_indent.starts_with("* ") || after_indent.starts_with("> ") {
            Some(indent_len + 2)
        } else {
            let digits = after_indent.chars().take_while(|c| c.is_ascii_digit()).count();
            if digits > 0 && after_indent[digits..].starts_with(". ") {
                Some(indent_len + digits + 2)
            } else if after_indent.starts_with('#') {
                let hashes = after_indent.chars().take_while(|&c| c == '#').count();
                if after_indent[hashes..].starts_with(' ') {
                    Some(indent_len + hashes + 1)
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(plen) = prefix_len {
            if point.col <= plen {
                // Backspacing inside or right after prefix removes the prefix, converting line to plain text
                self.buffer.delete_range(line_start, line_start + plen);
                self.cursor.set_cursor(line_start);
                cx.notify();
                return;
            }
        }

        // Normal single-character backspace
        self.buffer.delete_range(pos - 1, pos);
        self.cursor.set_cursor(pos - 1);
        cx.notify();
    }

    pub fn delete_forward(&mut self, cx: &mut Context<Self>) {
        if !self.cursor.selection.is_collapsed() {
            let range = self.cursor.selection.range();
            self.buffer.delete_range(range.start, range.end);
            self.cursor.set_cursor(range.start);
            cx.notify();
            return;
        }
        let pos = self.cursor.cursor_offset();
        if pos < self.buffer.len_chars() {
            self.buffer.delete_range(pos, pos + 1);
            cx.notify();
        }
    }

    pub fn insert_newline(&mut self, cx: &mut Context<Self>) {
        let point = self.buffer.char_offset_to_point(self.cursor.cursor_offset());
        let current_line = self.buffer.line_without_newline(point.row).unwrap_or_default();
        let line_start = self.buffer.line_to_char(point.row);
        
        let indent_len = current_line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
        let indent_str: String = current_line.chars().take(indent_len).collect();
        let after_indent = &current_line[indent_len..];

        let digits = after_indent.chars().take_while(|c| c.is_ascii_digit()).count();
        let prefix = if digits > 0 && after_indent[digits..].starts_with(". ") {
            let num: usize = after_indent[..digits].parse().unwrap_or(1);
            let item_text = after_indent[digits + 2..].trim();
            if item_text.is_empty() {
                // Pressing enter on empty numbered item: clear prefix and exit list
                self.buffer.replace_range(line_start, self.cursor.cursor_offset(), "");
                self.cursor.set_cursor(line_start);
                cx.notify();
                return;
            }
            // Auto-increment numbered list!
            format!("{}{}. ", indent_str, num + 1)
        } else if after_indent.starts_with("- [ ] ") || after_indent.starts_with("- [x] ") {
            let item_text = after_indent[6..].trim();
            if item_text.is_empty() {
                self.buffer.replace_range(line_start, self.cursor.cursor_offset(), "");
                self.cursor.set_cursor(line_start);
                cx.notify();
                return;
            }
            format!("{}- [ ] ", indent_str)
        } else if after_indent.starts_with("- ") || after_indent.starts_with("* ") {
            let item_text = after_indent[2..].trim();
            if item_text.is_empty() {
                self.buffer.replace_range(line_start, self.cursor.cursor_offset(), "");
                self.cursor.set_cursor(line_start);
                cx.notify();
                return;
            }
            format!("{}- ", indent_str)
        } else if after_indent.starts_with("> ") {
            let item_text = after_indent[2..].trim();
            if item_text.is_empty() {
                self.buffer.replace_range(line_start, self.cursor.cursor_offset(), "");
                self.cursor.set_cursor(line_start);
                cx.notify();
                return;
            }
            format!("{}> ", indent_str)
        } else {
            indent_str
        };

        let insert_str = format!("\n{}", prefix);
        self.insert_text(&insert_str, cx);
    }

    pub fn toggle_bold(&mut self, cx: &mut Context<Self>) {
        if self.cursor.selection.is_collapsed() {
            self.insert_text("****", cx);
            self.cursor.set_cursor(self.cursor.cursor_offset() - 2);
        } else {
            let range = self.cursor.selection.range();
            let selected = self.buffer.slice_to_string(range.start, range.end);
            let wrapped = format!("**{}**", selected);
            self.buffer.replace_range(range.start, range.end, &wrapped);
            self.cursor.set_cursor(range.start + wrapped.chars().count());
        }
        cx.notify();
    }

    pub fn toggle_italic(&mut self, cx: &mut Context<Self>) {
        if self.cursor.selection.is_collapsed() {
            self.insert_text("**", cx);
            self.cursor.set_cursor(self.cursor.cursor_offset() - 1);
        } else {
            let range = self.cursor.selection.range();
            let selected = self.buffer.slice_to_string(range.start, range.end);
            let wrapped = format!("*{}*", selected);
            self.buffer.replace_range(range.start, range.end, &wrapped);
            self.cursor.set_cursor(range.start + wrapped.chars().count());
        }
        cx.notify();
    }

    pub fn toggle_code(&mut self, cx: &mut Context<Self>) {
        if self.cursor.selection.is_collapsed() {
            self.insert_text("``", cx);
            self.cursor.set_cursor(self.cursor.cursor_offset() - 1);
        } else {
            let range = self.cursor.selection.range();
            let selected = self.buffer.slice_to_string(range.start, range.end);
            let wrapped = format!("`{}`", selected);
            self.buffer.replace_range(range.start, range.end, &wrapped);
            self.cursor.set_cursor(range.start + wrapped.chars().count());
        }
        cx.notify();
    }

    pub fn toggle_task_at_line(&mut self, line_idx: usize, cx: &mut Context<Self>) {
        if let Some(line) = self.buffer.line_without_newline(line_idx) {
            let line_start = self.buffer.line_to_char(line_idx);
            if line.starts_with("- [ ] ") {
                let replaced = format!("- [x] {}", &line[6..]);
                self.buffer.replace_range(line_start, line_start + line.chars().count(), &replaced);
            } else if line.starts_with("- [x] ") || line.starts_with("- [X] ") {
                let replaced = format!("- [ ] {}", &line[6..]);
                self.buffer.replace_range(line_start, line_start + line.chars().count(), &replaced);
            }
            cx.notify();
        }
    }

    pub fn word_count(&self) -> usize {
        let text = self.buffer.text();
        text.split_whitespace().count()
    }

    pub fn char_count(&self) -> usize {
        self.buffer.len_chars()
    }

    pub fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = &event.keystroke.modifiers;

        if modifiers.control || modifiers.platform {
            match key {
                "z" | "Z" => {
                    if modifiers.shift {
                        if let Some(pos) = self.buffer.redo() {
                            self.cursor.set_cursor(pos);
                            cx.notify();
                        }
                    } else {
                        if let Some(pos) = self.buffer.undo() {
                            self.cursor.set_cursor(pos);
                            cx.notify();
                        }
                    }
                    return;
                }
                "y" | "Y" => {
                    if let Some(pos) = self.buffer.redo() {
                        self.cursor.set_cursor(pos);
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
                cx.notify();
            }
            "right" => {
                self.cursor.move_right(&self.buffer, modifiers.shift);
                cx.notify();
            }
            "up" => {
                self.cursor.move_up(&self.buffer, modifiers.shift);
                cx.notify();
            }
            "down" => {
                self.cursor.move_down(&self.buffer, modifiers.shift);
                cx.notify();
            }
            "home" => {
                self.cursor.move_to_line_start(&self.buffer, modifiers.shift);
                cx.notify();
            }
            "end" => {
                self.cursor.move_to_line_end(&self.buffer, modifiers.shift);
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
                } else if key.chars().count() == 1 && !modifiers.control && !modifiers.platform && !modifiers.alt {
                    self.insert_text(key, cx);
                }
            }
        }
    }
}

impl Render for EditorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cursor_point = self.buffer.char_offset_to_point(self.cursor.cursor_offset());
        let current_row = cursor_point.row;
        let theme = &self.theme;

        let bg_app = rgb(theme.bg_app);
        let bg_editor = rgb(theme.bg_editor);
        let bg_active_line = rgb(theme.bg_active_line);
        let text_primary = rgb(theme.text_primary);
        let text_muted = rgb(theme.text_muted);
        let border_color = rgb(theme.border_subtle);

        let total_lines = self.buffer.len_lines();
        let word_count = self.word_count();
        let char_count = self.char_count();

        // 1. Top Navbar / Toolbar
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
                        div()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .bg(if self.conceal_mode == ConcealMode::TokenReveal { rgb(0x4F46E5) } else { rgb(theme.bg_code_inline) })
                            .text_color(if self.conceal_mode == ConcealMode::TokenReveal { rgb(0xFFFFFF) } else { text_muted })
                            .font_weight(FontWeight::MEDIUM)
                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                                view.set_conceal_mode(ConcealMode::TokenReveal, cx);
                            }))
                            .child("🎯 Token Reveal"),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .bg(if self.conceal_mode == ConcealMode::LineReveal { rgb(0x4F46E5) } else { rgb(theme.bg_code_inline) })
                            .text_color(if self.conceal_mode == ConcealMode::LineReveal { rgb(0xFFFFFF) } else { text_muted })
                            .font_weight(FontWeight::MEDIUM)
                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                                view.set_conceal_mode(ConcealMode::LineReveal, cx);
                            }))
                            .child("⚡ Line Reveal"),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .bg(if self.conceal_mode == ConcealMode::Raw { rgb(0x4F46E5) } else { rgb(theme.bg_code_inline) })
                            .text_color(if self.conceal_mode == ConcealMode::Raw { rgb(0xFFFFFF) } else { text_muted })
                            .font_weight(FontWeight::MEDIUM)
                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                                view.set_conceal_mode(ConcealMode::Raw, cx);
                            }))
                            .child("📝 Raw Markdown"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(border_color)
                            .cursor_pointer()
                            .text_color(text_primary)
                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                                view.toggle_theme(cx);
                            }))
                            .child(if theme.bg_editor == 0x121214 { "☀️ Light" } else { "🌙 Dark" }),
                    ),
            );

        // 2. Editor Document Content
        let mut document_lines = div()
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
            let cursor_col = if is_active { Some(cursor_point.col) } else { None };
            let parsed = MarkdownParser::parse_line(&raw_line);
            let block_kind = parsed.block_kind.clone();
            let decorated = Decorator::decorate_line(row, &parsed, cursor_col, self.conceal_mode);
            let decorated_for_click = decorated.clone();
            let line_start = self.buffer.line_to_char(row);
            let line_len = raw_line.chars().count();
            let block_kind_for_click = block_kind.clone();

            let mut line_row = div()
                .flex()
                .items_center()
                .w_full()
                .py_1()
                .px_2()
                .rounded_md()
                .cursor_text()
                .on_mouse_down(MouseButton::Left, cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                    let win_width = window.viewport_size().width;
                    let doc_max_w = px(860.0);
                    let doc_pad = px(32.0);
                    let doc_left = if win_width > doc_max_w + px(64.0) {
                        (win_width - doc_max_w) / 2.0
                    } else {
                        doc_pad
                    };
                    
                    let quote_offset = if matches!(block_kind_for_click, BlockKind::Blockquote { .. }) { 16.0 } else { 0.0 };
                    let text_start_x = doc_left + px(72.0 + quote_offset);
                    let click_x = event.position.x;
                    let delta_x: f32 = (click_x - text_start_x).into();

                    if delta_x <= 0.0 {
                        view.cursor.set_cursor(line_start);
                    } else {
                        let font_scale = match block_kind_for_click {
                            BlockKind::Heading { level } => match level {
                                1 => 1.85,
                                2 => 1.55,
                                3 => 1.30,
                                _ => 1.0,
                            },
                            _ => 1.0,
                        };

                        let mut current_x: f32 = 0.0;
                        let mut est_visual_col = decorated_for_click.display_text.chars().count();
                        for (idx, ch) in decorated_for_click.display_text.chars().enumerate() {
                            let base_w = match ch {
                                'i' | 'l' | 'j' | 't' | 'r' | 'f' | 'I' | '\'' | '"' | '!' | ':' | ';' | '.' | ',' | ' ' | '|' => 5.0,
                                'm' | 'w' | 'M' | 'W' | '@' | '%' | '&' => 12.5,
                                _ => 8.5,
                            };
                            let w = base_w * font_scale;
                            if delta_x < current_x + (w * 0.55) {
                                est_visual_col = idx;
                                break;
                            }
                            current_x += w;
                        }

                        let buf_col = Decorator::visual_col_to_buffer_col(&decorated_for_click, est_visual_col, line_len);
                        let target = (line_start + buf_col).min(line_start + line_len);
                        view.cursor.set_cursor(target);
                    }
                    cx.notify();
                }));

            if is_active {
                line_row = line_row.bg(bg_active_line);
            }

            // Gutter Line Number
            line_row = line_row.child(
                div()
                    .w_8()
                    .text_xs()
                    .text_color(if is_active { rgb(theme.text_accent) } else { rgb(theme.text_muted) })
                    .font_weight(if is_active { FontWeight::BOLD } else { FontWeight::NORMAL })
                    .child(format!("{}", row + 1)),
            );

            // Blockquote left accent bar
            if matches!(parsed.block_kind, BlockKind::Blockquote { .. }) {
                line_row = line_row.child(
                    div()
                        .w_1()
                        .h_full()
                        .bg(rgb(theme.border_quote))
                        .rounded_full()
                        .mr_3(),
                );
            }

            // Calculate visual column for inline caret insertion
            let target_visual_col = if is_active {
                Decorator::buffer_col_to_visual_col(&decorated, cursor_point.col)
            } else {
                usize::MAX
            };

            // Line Runs Container
            let mut runs_container = div()
                .flex()
                .items_center()
                .flex_wrap()
                .text_color(text_primary);

            let mut caret_rendered = false;
            let mut current_visual_offset = 0;

            let line_scale = match block_kind {
                BlockKind::Heading { level } => match level {
                    1 => 1.85,
                    2 => 1.55,
                    3 => 1.30,
                    _ => 1.0,
                },
                _ => 1.0,
            };

            if decorated.visual_runs.is_empty() {
                if is_active {
                    runs_container = runs_container.child(
                        div()
                            .w(px(2.0))
                            .h(px(20.0 * line_scale))
                            .bg(rgb(theme.cursor_color))
                            .rounded_full(),
                    );
                    caret_rendered = true;
                } else {
                    runs_container = runs_container.child(div().child(" "));
                }
            }

            for run in &decorated.visual_runs {
                let run_len = run.text.chars().count();
                let run_end = current_visual_offset + run_len;

                // Check if caret lands inside this run
                let should_split_for_caret = is_active 
                    && !caret_rendered 
                    && target_visual_col >= current_visual_offset 
                    && target_visual_col <= run_end;

                let apply_styles = |mut d: Div| -> Div {
                    match run.font_weight {
                        VisualFontWeight::Bold => d = d.font_weight(FontWeight::BOLD),
                        VisualFontWeight::ExtraBold => d = d.font_weight(FontWeight::EXTRA_BOLD),
                        VisualFontWeight::Normal => {}
                    }
                    if run.font_style == VisualFontStyle::Italic {
                        d = d.italic();
                    }
                    if run.font_size_scale > 1.4 {
                        d = d.text_xl().text_color(rgb(theme.text_heading));
                    } else if run.font_size_scale > 1.2 {
                        d = d.text_lg().text_color(rgb(theme.text_heading));
                    }
                    if run.is_marker {
                        if is_active {
                            d = d.text_color(rgb(theme.text_marker_active)).font_weight(FontWeight::MEDIUM);
                        } else {
                            d = d.text_color(rgb(theme.text_marker_dimmed));
                        }
                    }
                    if run.is_code {
                        d = d
                            .bg(rgb(theme.bg_code_inline))
                            .px_2()
                            .py_0p5()
                            .rounded_md()
                            .text_sm()
                            .text_color(rgb(0x38BDF8));
                    }
                    if run.link_url.is_some() {
                        d = d.text_color(rgb(theme.text_link)).underline();
                    }
                    d
                };

                if should_split_for_caret {
                    let split_idx = target_visual_col - current_visual_offset;
                    let before_str: String = run.text.chars().take(split_idx).collect();
                    let after_str: String = run.text.chars().skip(split_idx).collect();

                    if !before_str.is_empty() {
                        runs_container = runs_container.child(apply_styles(div().child(before_str)));
                    }

                    // Insert Caret at exact position
                    runs_container = runs_container.child(
                        div()
                            .w(px(2.0))
                            .h(px(20.0 * run.font_size_scale))
                            .bg(rgb(theme.cursor_color))
                            .rounded_full(),
                    );
                    caret_rendered = true;

                    if !after_str.is_empty() {
                        runs_container = runs_container.child(apply_styles(div().child(after_str)));
                    }
                } else {
                    let mut run_div = apply_styles(div().child(run.text.clone()));

                    if let Some(MarkerType::TaskListMarker { checked }) = run.marker_type {
                        run_div = run_div
                            .cursor_pointer()
                            .font_weight(FontWeight::BOLD)
                            .on_mouse_down(MouseButton::Left, cx.listener(move |view, _, _, cx| {
                                view.toggle_task_at_line(row, cx);
                            }));
                        if checked {
                            run_div = run_div.text_color(rgb(0x34D399));
                        }
                    }
                    runs_container = runs_container.child(run_div);
                }

                current_visual_offset = run_end;
            }

            // If caret has not yet been rendered on active line (e.g. at the very end of line)
            if is_active && !caret_rendered {
                runs_container = runs_container.child(
                    div()
                        .w(px(2.0))
                        .h(px(20.0 * line_scale))
                        .bg(rgb(theme.cursor_color))
                        .rounded_full(),
                );
            }

            line_row = line_row.child(runs_container);
            document_lines = document_lines.child(line_row);
        }

        // 3. Status Bar (Bottom)
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
                    .child(format!("Ln {}, Col {}", cursor_point.row + 1, cursor_point.col + 1))
                    .child(format!("{} lines", total_lines)),
            )
            .child(
                div()
                    .child(format!("Mode: {:?}", self.conceal_mode)),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(format!("{} words", word_count))
                    .child(format!("{} characters", char_count)),
            );

        // Assemble root view container with keyboard event capturing
        div()
            .track_focus(&self.focus_handle)
            .key_context("MordEditor")
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, _, cx| {
                view.handle_key_down(event, cx);
            }))
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(bg_app)
            .child(navbar)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .justify_center()
                    .p_8()
                    .child(document_lines),
            )
            .child(status_bar)
    }
}
