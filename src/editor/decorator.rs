use crate::editor::parser::{BlockKind, MarkerType, ParsedLine, SpanStyle};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcealMode {
    /// Only the specific token directly under the cursor reveals markers, rest of line stays concealed
    TokenReveal,
    /// The entire active line where the cursor resides reveals all its markers
    LineReveal,
    /// Show all raw markdown syntax markers without hiding
    Raw,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualRun {
    pub text: String,
    pub original_char_range: Range<usize>, // Range in the source line
    pub font_weight: VisualFontWeight,
    pub font_style: VisualFontStyle,
    pub font_size_scale: f32, // Relative to base font size (e.g. 1.8 for H1)
    pub is_marker: bool,
    pub marker_type: Option<MarkerType>,
    pub is_code: bool,
    pub is_strikethrough: bool,
    pub link_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualFontWeight {
    Normal,
    Bold,
    ExtraBold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualFontStyle {
    Normal,
    Italic,
}

#[derive(Debug, Clone)]
pub struct DecoratedLine {
    pub line_index: usize,
    pub block_kind: BlockKind,
    pub is_active_line: bool,
    pub visual_runs: Vec<VisualRun>,
    pub display_text: String, // Flattened text as rendered on screen
    pub char_map: Vec<usize>, // Maps visual char index -> buffer char index in line
}

pub struct Decorator;

impl Decorator {
    pub fn decorate_line(
        line_idx: usize,
        parsed: &ParsedLine,
        cursor_col: Option<usize>,
        mode: ConcealMode,
    ) -> DecoratedLine {
        let is_active_line = cursor_col.is_some();
        let mut visual_runs = Vec::new();
        let mut display_text = String::new();
        let mut char_map = Vec::new();

        let base_scale: f32 = match parsed.block_kind {
            BlockKind::Heading { level } => match level {
                1 => 1.85,
                2 => 1.55,
                3 => 1.30,
                4 => 1.15,
                5 => 1.05,
                _ => 1.0,
            },
            _ => 1.0,
        };

        let chars: Vec<char> = parsed.raw_text.chars().collect();

        for span in &parsed.spans {
            let start = span.char_range.start.min(chars.len());
            let end = span.char_range.end.min(chars.len());
            if start >= end {
                continue;
            }
            let span_str: String = chars[start..end].iter().collect();

            match &span.style {
                SpanStyle::Marker(marker_type) => {
                    let should_show = match mode {
                        ConcealMode::Raw => true,
                        ConcealMode::LineReveal => is_active_line,
                        ConcealMode::TokenReveal => {
                            if let Some(col) = cursor_col {
                                col >= span.group_range.start && col <= span.group_range.end
                            } else {
                                false
                            }
                        }
                    };

                    if should_show {
                        // Visible marker
                        let run = VisualRun {
                            text: span_str.clone(),
                            original_char_range: start..end,
                            font_weight: VisualFontWeight::Normal,
                            font_style: VisualFontStyle::Normal,
                            font_size_scale: if is_active_line { base_scale.min(1.2) } else { base_scale },
                            is_marker: true,
                            marker_type: Some(marker_type.clone()),
                            is_code: false,
                            is_strikethrough: false,
                            link_url: None,
                        };
                        for (i, c) in span_str.chars().enumerate() {
                            display_text.push(c);
                            char_map.push(start + i);
                        }
                        visual_runs.push(run);
                    } else {
                        // Concealed marker in inactive line!
                        // For special markers like checklist or bullet, we can replace them with a visual character
                        match marker_type {
                            MarkerType::ListPrefix => {
                                let bullet = "• ";
                                display_text.push_str(bullet);
                                for _ in bullet.chars() {
                                    char_map.push(start);
                                }
                                visual_runs.push(VisualRun {
                                    text: bullet.to_string(),
                                    original_char_range: start..end,
                                    font_weight: VisualFontWeight::Bold,
                                    font_style: VisualFontStyle::Normal,
                                    font_size_scale: base_scale,
                                    is_marker: false,
                                    marker_type: Some(marker_type.clone()),
                                    is_code: false,
                                    is_strikethrough: false,
                                    link_url: None,
                                });
                            }
                            MarkerType::OrderedListPrefix { number } => {
                                let num_str = format!("{}. ", number);
                                for (i, c) in num_str.chars().enumerate() {
                                    display_text.push(c);
                                    char_map.push((start + i).min(end.saturating_sub(1)));
                                }
                                visual_runs.push(VisualRun {
                                    text: num_str,
                                    original_char_range: start..end,
                                    font_weight: VisualFontWeight::Bold,
                                    font_style: VisualFontStyle::Normal,
                                    font_size_scale: base_scale,
                                    is_marker: false,
                                    marker_type: Some(marker_type.clone()),
                                    is_code: false,
                                    is_strikethrough: false,
                                    link_url: None,
                                });
                            }
                            MarkerType::TaskListMarker { checked } => {
                                let check_symbol = if *checked { "☑ " } else { "☐ " };
                                display_text.push_str(check_symbol);
                                for _ in check_symbol.chars() {
                                    char_map.push(start);
                                }
                                visual_runs.push(VisualRun {
                                    text: check_symbol.to_string(),
                                    original_char_range: start..end,
                                    font_weight: VisualFontWeight::Normal,
                                    font_style: VisualFontStyle::Normal,
                                    font_size_scale: base_scale,
                                    is_marker: false,
                                    marker_type: Some(marker_type.clone()),
                                    is_code: false,
                                    is_strikethrough: false,
                                    link_url: None,
                                });
                            }
                            _ => {
                                // Completely concealed: 0 visual characters
                                // Does not add to display_text or char_map
                            }
                        }
                    }
                }
                SpanStyle::Plain => {
                    let weight = match parsed.block_kind {
                        BlockKind::Heading { level } if level <= 3 => VisualFontWeight::Bold,
                        _ => VisualFontWeight::Normal,
                    };
                    for (i, c) in span_str.chars().enumerate() {
                        display_text.push(c);
                        char_map.push(start + i);
                    }
                    visual_runs.push(VisualRun {
                        text: span_str,
                        original_char_range: start..end,
                        font_weight: weight,
                        font_style: VisualFontStyle::Normal,
                        font_size_scale: base_scale,
                        is_marker: false,
                        marker_type: None,
                        is_code: false,
                        is_strikethrough: false,
                        link_url: None,
                    });
                }
                SpanStyle::Bold => {
                    for (i, c) in span_str.chars().enumerate() {
                        display_text.push(c);
                        char_map.push(start + i);
                    }
                    visual_runs.push(VisualRun {
                        text: span_str,
                        original_char_range: start..end,
                        font_weight: VisualFontWeight::Bold,
                        font_style: VisualFontStyle::Normal,
                        font_size_scale: base_scale,
                        is_marker: false,
                        marker_type: None,
                        is_code: false,
                        is_strikethrough: false,
                        link_url: None,
                    });
                }
                SpanStyle::Italic => {
                    for (i, c) in span_str.chars().enumerate() {
                        display_text.push(c);
                        char_map.push(start + i);
                    }
                    visual_runs.push(VisualRun {
                        text: span_str,
                        original_char_range: start..end,
                        font_weight: VisualFontWeight::Normal,
                        font_style: VisualFontStyle::Italic,
                        font_size_scale: base_scale,
                        is_marker: false,
                        marker_type: None,
                        is_code: false,
                        is_strikethrough: false,
                        link_url: None,
                    });
                }
                SpanStyle::BoldItalic => {
                    for (i, c) in span_str.chars().enumerate() {
                        display_text.push(c);
                        char_map.push(start + i);
                    }
                    visual_runs.push(VisualRun {
                        text: span_str,
                        original_char_range: start..end,
                        font_weight: VisualFontWeight::Bold,
                        font_style: VisualFontStyle::Italic,
                        font_size_scale: base_scale,
                        is_marker: false,
                        marker_type: None,
                        is_code: false,
                        is_strikethrough: false,
                        link_url: None,
                    });
                }
                SpanStyle::InlineCode => {
                    for (i, c) in span_str.chars().enumerate() {
                        display_text.push(c);
                        char_map.push(start + i);
                    }
                    visual_runs.push(VisualRun {
                        text: span_str,
                        original_char_range: start..end,
                        font_weight: VisualFontWeight::Normal,
                        font_style: VisualFontStyle::Normal,
                        font_size_scale: base_scale * 0.95,
                        is_marker: false,
                        marker_type: None,
                        is_code: true,
                        is_strikethrough: false,
                        link_url: None,
                    });
                }
                SpanStyle::Strikethrough => {
                    for (i, c) in span_str.chars().enumerate() {
                        display_text.push(c);
                        char_map.push(start + i);
                    }
                    visual_runs.push(VisualRun {
                        text: span_str,
                        original_char_range: start..end,
                        font_weight: VisualFontWeight::Normal,
                        font_style: VisualFontStyle::Normal,
                        font_size_scale: base_scale,
                        is_marker: false,
                        marker_type: None,
                        is_code: false,
                        is_strikethrough: true,
                        link_url: None,
                    });
                }
                SpanStyle::LinkText { url } => {
                    for (i, c) in span_str.chars().enumerate() {
                        display_text.push(c);
                        char_map.push(start + i);
                    }
                    visual_runs.push(VisualRun {
                        text: span_str,
                        original_char_range: start..end,
                        font_weight: VisualFontWeight::Normal,
                        font_style: VisualFontStyle::Normal,
                        font_size_scale: base_scale,
                        is_marker: false,
                        marker_type: None,
                        is_code: false,
                        is_strikethrough: false,
                        link_url: Some(url.clone()),
                    });
                }
            }
        }

        // If line is completely empty, add an empty mapping
        if display_text.is_empty() {
            char_map.push(0);
        }

        DecoratedLine {
            line_index: line_idx,
            block_kind: parsed.block_kind.clone(),
            is_active_line,
            visual_runs,
            display_text,
            char_map,
        }
    }

    /// Map a visual character index (from mouse hit-test or visual position) to raw buffer index in line
    pub fn visual_col_to_buffer_col(decorated: &DecoratedLine, visual_col: usize, line_len: usize) -> usize {
        if decorated.char_map.is_empty() {
            return 0;
        }
        if visual_col >= decorated.char_map.len() {
            return line_len;
        }
        decorated.char_map[visual_col]
    }

    /// Map a buffer character index in the line to closest visual character index
    pub fn buffer_col_to_visual_col(decorated: &DecoratedLine, buffer_col: usize) -> usize {
        if decorated.char_map.is_empty() {
            return 0;
        }
        for (visual_idx, &buf_idx) in decorated.char_map.iter().enumerate() {
            if buf_idx >= buffer_col {
                return visual_idx;
            }
        }
        decorated.char_map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::parser::MarkdownParser;

    #[test]
    fn test_line_reveal_shows_markers() {
        let parsed = MarkdownParser::parse_line("### My Title");
        let decorated = Decorator::decorate_line(0, &parsed, Some(0), ConcealMode::LineReveal);

        // Active line retains "### " marker
        assert!(decorated.is_active_line);
        assert_eq!(decorated.display_text, "### My Title");
        assert_eq!(decorated.visual_runs.len(), 2);
        assert!(decorated.visual_runs[0].is_marker);
        assert_eq!(decorated.visual_runs[0].text, "### ");

        // Coordinate mapping is 1:1
        assert_eq!(Decorator::visual_col_to_buffer_col(&decorated, 0, 12), 0);
        assert_eq!(Decorator::visual_col_to_buffer_col(&decorated, 4, 12), 4);
        assert_eq!(Decorator::visual_col_to_buffer_col(&decorated, 12, 12), 12);
        assert_eq!(Decorator::buffer_col_to_visual_col(&decorated, 12), 12);
    }

    #[test]
    fn test_inactive_line_conceals_markers() {
        let parsed = MarkdownParser::parse_line("### My Title");
        let decorated = Decorator::decorate_line(0, &parsed, None, ConcealMode::TokenReveal);

        // Inactive line conceals "### "
        assert!(!decorated.is_active_line);
        assert_eq!(decorated.display_text, "My Title");
        assert_eq!(decorated.visual_runs.len(), 1);
        assert!(!decorated.visual_runs[0].is_marker);
        assert_eq!(decorated.visual_runs[0].text, "My Title");
        assert_eq!(decorated.visual_runs[0].font_weight, VisualFontWeight::Bold);
        assert_eq!(decorated.visual_runs[0].font_size_scale, 1.30);

        // Coordinate projection: visual col 0 maps to buffer col 4 ("M" in "My Title")
        assert_eq!(Decorator::visual_col_to_buffer_col(&decorated, 0, 12), 4);
        assert_eq!(Decorator::visual_col_to_buffer_col(&decorated, 3, 12), 7);
        // Past the visual text maps to line_len (12)
        assert_eq!(Decorator::visual_col_to_buffer_col(&decorated, 8, 12), 12);
    }

    #[test]
    fn test_token_level_proximity_reveal() {
        let line = "Hello **bold** and *italic*";
        let parsed = MarkdownParser::parse_line(line);

        // 1. Cursor is inside "bold" (column 9):
        // Only **bold** should reveal its markers; *italic* stays concealed!
        let dec_bold = Decorator::decorate_line(0, &parsed, Some(9), ConcealMode::TokenReveal);
        assert_eq!(dec_bold.display_text, "Hello **bold** and italic");

        // 2. Cursor moves into "italic" (column 23):
        // Only *italic* should reveal its markers; **bold** is now concealed!
        let dec_italic = Decorator::decorate_line(0, &parsed, Some(23), ConcealMode::TokenReveal);
        assert_eq!(dec_italic.display_text, "Hello bold and *italic*");

        // 3. Cursor is in plain text "Hello " (column 2):
        // Both bold and italic stay concealed!
        let dec_plain = Decorator::decorate_line(0, &parsed, Some(2), ConcealMode::TokenReveal);
        assert_eq!(dec_plain.display_text, "Hello bold and italic");
    }

    #[test]
    fn test_ordered_list_display() {
        let parsed = MarkdownParser::parse_line("1. First item");
        let decorated = Decorator::decorate_line(0, &parsed, None, ConcealMode::TokenReveal);

        assert_eq!(decorated.display_text, "1. First item");
        assert_eq!(decorated.visual_runs.len(), 2);
        // Numbered list prefix is prominent (is_marker: false)
        assert!(!decorated.visual_runs[0].is_marker);
        assert_eq!(decorated.visual_runs[0].text, "1. ");

        // Visual to buffer mapping
        assert_eq!(Decorator::visual_col_to_buffer_col(&decorated, 0, 13), 0);
        assert_eq!(Decorator::visual_col_to_buffer_col(&decorated, 3, 13), 3);
        assert_eq!(Decorator::visual_col_to_buffer_col(&decorated, 20, 13), 13);
        
        // Buffer to visual mapping
        assert_eq!(Decorator::buffer_col_to_visual_col(&decorated, 13), 13);
    }
}

