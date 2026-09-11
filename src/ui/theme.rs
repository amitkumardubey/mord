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
}

impl Theme {
    /// Sleek Modern Dark Theme inspired by Obsidian & Zed
    pub fn dark() -> Self {
        Self {
            bg_app: 0x18181B,             // Zinc 900
            bg_editor: 0x121214,          // Deep dark editor background
            bg_active_line: 0x1F1F24,     // Subtle highlight for active line
            bg_code_inline: 0x27272A,     // Zinc 800 pill background
            bg_code_block: 0x1E1E22,      // Code block background
            bg_selection: 0x3F3F46,       // Selection tint
            text_primary: 0xE4E4E7,       // Zinc 200 (crisp readable text)
            text_muted: 0x71717A,         // Zinc 500
            text_marker_dimmed: 0x52525B, // Zinc 600 (faded syntax tokens)
            text_marker_active: 0x38BDF8, // Sky 400 (active editable markers)
            text_accent: 0x60A5FA,        // Blue 400
            text_heading: 0xFAFAFA,       // Zinc 50 (extra crisp headings)
            text_link: 0x38BDF8,          // Sky 400
            border_subtle: 0x27272A,      // Zinc 800
            border_quote: 0x6366F1,       // Indigo 500 (quote left bar)
            cursor_color: 0x60A5FA,       // Electric blue caret
            font_size_base: 16.0,
            line_height_base: 26.0,
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
        }
    }
}
