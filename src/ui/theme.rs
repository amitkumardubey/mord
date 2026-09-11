use crate::editor::layout::LinePaintTheme;

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg_app: u32,
    pub bg_editor: u32,
    pub bg_active_line: u32,
    pub bg_code_inline: u32,
    pub bg_code_block: u32,
    pub bg_selection: u32,
    pub text_primary: u32,
    pub text_muted: u32,
    pub text_marker_dimmed: u32,
    pub text_marker_active: u32,
    pub text_accent: u32,
    pub text_heading: u32,
    pub text_link: u32,
    pub border_subtle: u32,
    pub border_quote: u32,
    pub cursor_color: u32,
    pub font_size_base: f32,
    pub line_height_base: f32,
    /// Font-size multipliers for heading levels 1..=6 (index 0 = H1).
    pub heading_scales: [f32; 6],
}

impl Theme {
    /// Sleek Modern Dark Theme inspired by Obsidian & Zed
    pub fn dark() -> Self {
        Self {
            bg_app: 0x18181B,
            bg_editor: 0x121214,
            bg_active_line: 0x1F1F24,
            bg_code_inline: 0x27272A,
            bg_code_block: 0x1E1E22,
            bg_selection: 0x3F3F46,
            text_primary: 0xE4E4E7,
            text_muted: 0x71717A,
            text_marker_dimmed: 0x52525B,
            text_marker_active: 0x38BDF8,
            text_accent: 0x60A5FA,
            text_heading: 0xFAFAFA,
            text_link: 0x38BDF8,
            border_subtle: 0x27272A,
            border_quote: 0x6366F1,
            cursor_color: 0x60A5FA,
            font_size_base: 16.0,
            line_height_base: 26.0,
            heading_scales: [1.85, 1.55, 1.30, 1.15, 1.05, 1.0],
        }
    }

    /// Clean Word-like Light Theme
    pub fn light() -> Self {
        Self {
            bg_app: 0xF4F4F5,
            bg_editor: 0xFFFFFF,
            bg_active_line: 0xF8FAFC,
            bg_code_inline: 0xF1F5F9,
            bg_code_block: 0xF8FAFC,
            bg_selection: 0xBAE6FD,
            text_primary: 0x0F172A,
            text_muted: 0x64748B,
            text_marker_dimmed: 0xCBD5E1,
            text_marker_active: 0x0284C7,
            text_accent: 0x2563EB,
            text_heading: 0x020617,
            text_link: 0x0284C7,
            border_subtle: 0xE2E8F0,
            border_quote: 0x4F46E5,
            cursor_color: 0x2563EB,
            font_size_base: 16.0,
            line_height_base: 26.0,
            heading_scales: [1.85, 1.55, 1.30, 1.15, 1.05, 1.0],
        }
    }

    pub fn heading_scale(&self, level: usize) -> f32 {
        if (1..=6).contains(&level) {
            self.heading_scales[level - 1]
        } else {
            1.0
        }
    }

    pub fn paint_theme(&self) -> LinePaintTheme {
        LinePaintTheme {
            text_primary: self.text_primary,
            text_heading: self.text_heading,
            text_marker_dimmed: self.text_marker_dimmed,
            text_marker_active: self.text_marker_active,
            text_link: self.text_link,
            bg_code_inline: self.bg_code_inline,
            bg_selection: self.bg_selection,
            cursor_color: self.cursor_color,
            code_accent: 0x38BDF8,
            task_checked: 0x34D399,
            font_size_base: self.font_size_base,
            line_height_base: self.line_height_base,
            heading_scales: self.heading_scales,
        }
    }
}
