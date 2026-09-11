use crate::editor::parser::{BlockKind, MarkerType, ParsedLine, SpanStyle, StyledSpan};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcealMode {
    /// Live: closed marks hidden. Line prefixes stay visible while composing
    /// (typing) that line, not merely because the caret is on it. Inline markers
    /// stay visible until the wrap has content.
    Live,
    /// Source Raw: 1:1 with the rope; light syntax color; no type scale or glyph substitutes.
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualRole {
    Plain,
    StructuralMarker,
    InlineMarker,
    Emphasis,
    Code,
    Link,
    TaskGlyph,
    Heading,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualRun {
    pub text: String,
    pub original_char_range: Range<usize>,
    pub role: VisualRole,
    pub font_weight: VisualFontWeight,
    pub font_style: VisualFontStyle,
    /// Relative to base font size; layout may also consult Theme heading table via BlockKind.
    pub font_size_scale: f32,
    pub is_strikethrough: bool,
    pub marker_type: Option<MarkerType>,
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
    pub display_text: String,
    pub char_map: Vec<usize>,
}

pub struct Decorator;

impl Decorator {
    pub fn decorate_line(
        line_idx: usize,
        parsed: &ParsedLine,
        cursor_col: Option<usize>,
        mode: ConcealMode,
        heading_scale: f32,
        reveal_line_markers: bool,
    ) -> DecoratedLine {
        let is_active_line = cursor_col.is_some();
        let mut visual_runs = Vec::new();
        let mut display_text = String::new();
        let mut char_map = Vec::new();

        let is_raw = mode == ConcealMode::Raw;
        let base_scale = if is_raw { 1.0 } else { heading_scale };

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
                        ConcealMode::Live => {
                            live_show_marker(marker_type, span, reveal_line_markers)
                        }
                    };

                    if should_show {
                        push_run(
                            &mut display_text,
                            &mut char_map,
                            &mut visual_runs,
                            VisualRun {
                                text: span_str.clone(),
                                original_char_range: start..end,
                                role: if is_structural_marker(marker_type) {
                                    VisualRole::StructuralMarker
                                } else {
                                    VisualRole::InlineMarker
                                },
                                font_weight: VisualFontWeight::Normal,
                                font_style: VisualFontStyle::Normal,
                                font_size_scale: base_scale,
                                is_strikethrough: false,
                                marker_type: Some(marker_type.clone()),
                                link_url: None,
                            },
                        );
                    } else {
                        // Inactive line: list/task glyphs; heading/quote/HR/inlines omitted.
                        match marker_type {
                            MarkerType::ListPrefix => {
                                push_run(
                                    &mut display_text,
                                    &mut char_map,
                                    &mut visual_runs,
                                    VisualRun {
                                        text: "• ".to_string(),
                                        original_char_range: start..end,
                                        role: VisualRole::StructuralMarker,
                                        font_weight: VisualFontWeight::Bold,
                                        font_style: VisualFontStyle::Normal,
                                        font_size_scale: base_scale,
                                        is_strikethrough: false,
                                        marker_type: Some(marker_type.clone()),
                                        link_url: None,
                                    },
                                );
                            }
                            MarkerType::OrderedListPrefix { number } => {
                                let num_str = format!("{}. ", number);
                                push_run(
                                    &mut display_text,
                                    &mut char_map,
                                    &mut visual_runs,
                                    VisualRun {
                                        text: num_str,
                                        original_char_range: start..end,
                                        role: VisualRole::StructuralMarker,
                                        font_weight: VisualFontWeight::Bold,
                                        font_style: VisualFontStyle::Normal,
                                        font_size_scale: base_scale,
                                        is_strikethrough: false,
                                        marker_type: Some(marker_type.clone()),
                                        link_url: None,
                                    },
                                );
                            }
                            MarkerType::TaskListMarker { checked } => {
                                let check_symbol = if *checked { "☑ " } else { "☐ " };
                                push_run(
                                    &mut display_text,
                                    &mut char_map,
                                    &mut visual_runs,
                                    VisualRun {
                                        text: check_symbol.to_string(),
                                        original_char_range: start..end,
                                        role: VisualRole::TaskGlyph,
                                        font_weight: VisualFontWeight::Normal,
                                        font_style: VisualFontStyle::Normal,
                                        font_size_scale: base_scale,
                                        is_strikethrough: false,
                                        marker_type: Some(marker_type.clone()),
                                        link_url: None,
                                    },
                                );
                            }
                            _ => {}
                        }
                    }
                }
                SpanStyle::Plain => {
                    let (weight, role) = if is_raw {
                        match parsed.block_kind {
                            BlockKind::Heading { .. } => {
                                (VisualFontWeight::Normal, VisualRole::Heading)
                            }
                            _ => (VisualFontWeight::Normal, VisualRole::Plain),
                        }
                    } else {
                        match parsed.block_kind {
                            BlockKind::Heading { .. } => {
                                (VisualFontWeight::Bold, VisualRole::Heading)
                            }
                            _ => (VisualFontWeight::Normal, VisualRole::Plain),
                        }
                    };
                    push_run(
                        &mut display_text,
                        &mut char_map,
                        &mut visual_runs,
                        VisualRun {
                            text: span_str,
                            original_char_range: start..end,
                            role,
                            font_weight: weight,
                            font_style: VisualFontStyle::Normal,
                            font_size_scale: base_scale,
                            is_strikethrough: false,
                            marker_type: None,
                            link_url: None,
                        },
                    );
                }
                SpanStyle::Bold => {
                    push_run(
                        &mut display_text,
                        &mut char_map,
                        &mut visual_runs,
                        VisualRun {
                            text: span_str,
                            original_char_range: start..end,
                            role: VisualRole::Emphasis,
                            font_weight: if is_raw {
                                VisualFontWeight::Normal
                            } else {
                                VisualFontWeight::Bold
                            },
                            font_style: VisualFontStyle::Normal,
                            font_size_scale: base_scale,
                            is_strikethrough: false,
                            marker_type: None,
                            link_url: None,
                        },
                    );
                }
                SpanStyle::Italic => {
                    push_run(
                        &mut display_text,
                        &mut char_map,
                        &mut visual_runs,
                        VisualRun {
                            text: span_str,
                            original_char_range: start..end,
                            role: VisualRole::Emphasis,
                            font_weight: VisualFontWeight::Normal,
                            font_style: if is_raw {
                                VisualFontStyle::Normal
                            } else {
                                VisualFontStyle::Italic
                            },
                            font_size_scale: base_scale,
                            is_strikethrough: false,
                            marker_type: None,
                            link_url: None,
                        },
                    );
                }
                SpanStyle::BoldItalic => {
                    push_run(
                        &mut display_text,
                        &mut char_map,
                        &mut visual_runs,
                        VisualRun {
                            text: span_str,
                            original_char_range: start..end,
                            role: VisualRole::Emphasis,
                            font_weight: if is_raw {
                                VisualFontWeight::Normal
                            } else {
                                VisualFontWeight::Bold
                            },
                            font_style: if is_raw {
                                VisualFontStyle::Normal
                            } else {
                                VisualFontStyle::Italic
                            },
                            font_size_scale: base_scale,
                            is_strikethrough: false,
                            marker_type: None,
                            link_url: None,
                        },
                    );
                }
                SpanStyle::InlineCode => {
                    push_run(
                        &mut display_text,
                        &mut char_map,
                        &mut visual_runs,
                        VisualRun {
                            text: span_str,
                            original_char_range: start..end,
                            role: VisualRole::Code,
                            font_weight: VisualFontWeight::Normal,
                            font_style: VisualFontStyle::Normal,
                            font_size_scale: if is_raw {
                                base_scale
                            } else {
                                base_scale * 0.95
                            },
                            is_strikethrough: false,
                            marker_type: None,
                            link_url: None,
                        },
                    );
                }
                SpanStyle::Strikethrough => {
                    push_run(
                        &mut display_text,
                        &mut char_map,
                        &mut visual_runs,
                        VisualRun {
                            text: span_str,
                            original_char_range: start..end,
                            role: VisualRole::Emphasis,
                            font_weight: VisualFontWeight::Normal,
                            font_style: VisualFontStyle::Normal,
                            font_size_scale: base_scale,
                            is_strikethrough: !is_raw,
                            marker_type: None,
                            link_url: None,
                        },
                    );
                }
                SpanStyle::LinkText { url } => {
                    push_run(
                        &mut display_text,
                        &mut char_map,
                        &mut visual_runs,
                        VisualRun {
                            text: span_str,
                            original_char_range: start..end,
                            role: VisualRole::Link,
                            font_weight: VisualFontWeight::Normal,
                            font_style: VisualFontStyle::Normal,
                            font_size_scale: base_scale,
                            is_strikethrough: false,
                            marker_type: None,
                            link_url: Some(url.clone()),
                        },
                    );
                }
            }
        }

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

    pub fn visual_col_to_buffer_col(
        decorated: &DecoratedLine,
        visual_col: usize,
        line_len: usize,
    ) -> usize {
        if decorated.char_map.is_empty() {
            return 0;
        }
        if visual_col >= decorated.char_map.len() {
            return line_len;
        }
        decorated.char_map[visual_col]
    }

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

/// Line prefixes: visible while composing (typing) the line, not on click.
/// Inlines: visible until the pair has inner content. Closed links hide.
fn live_show_marker(marker: &MarkerType, span: &StyledSpan, reveal_line_markers: bool) -> bool {
    match marker {
        MarkerType::CodeFence => true,
        MarkerType::HeadingPrefix
        | MarkerType::BlockquotePrefix
        | MarkerType::ListPrefix
        | MarkerType::OrderedListPrefix { .. }
        | MarkerType::TaskListMarker { .. }
        | MarkerType::HorizontalRule => reveal_line_markers,
        MarkerType::BoldMarker
        | MarkerType::ItalicMarker
        | MarkerType::StrikethroughMarker
        | MarkerType::InlineCodeMarker => !inline_wrap_has_content(span),
        MarkerType::LinkBracket | MarkerType::LinkUrl => false,
    }
}

fn inline_wrap_has_content(span: &StyledSpan) -> bool {
    let mark_len = span.char_range.end.saturating_sub(span.char_range.start);
    let group_len = span.group_range.end.saturating_sub(span.group_range.start);
    group_len > mark_len.saturating_mul(2)
}

fn is_structural_marker(marker: &MarkerType) -> bool {
    matches!(
        marker,
        MarkerType::HeadingPrefix
            | MarkerType::BlockquotePrefix
            | MarkerType::ListPrefix
            | MarkerType::OrderedListPrefix { .. }
            | MarkerType::TaskListMarker { .. }
            | MarkerType::HorizontalRule
            | MarkerType::CodeFence
    )
}

fn push_run(
    display_text: &mut String,
    char_map: &mut Vec<usize>,
    visual_runs: &mut Vec<VisualRun>,
    run: VisualRun,
) {
    let start = run.original_char_range.start;
    let end = run.original_char_range.end;
    let src_len = end.saturating_sub(start);
    let vis_len = run.text.chars().count();
    for (i, c) in run.text.chars().enumerate() {
        display_text.push(c);
        if vis_len == src_len {
            char_map.push(start + i);
        } else if matches!(run.role, VisualRole::TaskGlyph) {
            char_map.push(start);
        } else {
            char_map.push((start + i).min(end.saturating_sub(1)));
        }
    }
    visual_runs.push(run);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::parser::MarkdownParser;

    fn scale_for(parsed: &ParsedLine) -> f32 {
        match parsed.block_kind {
            BlockKind::Heading { level } => match level {
                1 => 1.85,
                2 => 1.55,
                3 => 1.30,
                4 => 1.15,
                5 => 1.05,
                _ => 1.0,
            },
            _ => 1.0,
        }
    }

    fn decorate(
        line: &str,
        cursor_col: Option<usize>,
        mode: ConcealMode,
    ) -> DecoratedLine {
        decorate_reveal(line, cursor_col, mode, false)
    }

    fn decorate_reveal(
        line: &str,
        cursor_col: Option<usize>,
        mode: ConcealMode,
        reveal_line_markers: bool,
    ) -> DecoratedLine {
        let parsed = MarkdownParser::parse_line(line);
        let scale = if mode == ConcealMode::Raw {
            1.0
        } else {
            scale_for(&parsed)
        };
        Decorator::decorate_line(0, &parsed, cursor_col, mode, scale, reveal_line_markers)
    }

    #[test]
    fn inactive_heading_conceals_prefix() {
        let decorated = decorate("### My Title", None, ConcealMode::Live);
        assert_eq!(decorated.display_text, "My Title");
        assert_eq!(decorated.visual_runs[0].role, VisualRole::Heading);
        assert_eq!(decorated.visual_runs[0].font_size_scale, 1.30);
        assert_eq!(Decorator::visual_col_to_buffer_col(&decorated, 0, 12), 4);
        assert_eq!(Decorator::visual_col_to_buffer_col(&decorated, 8, 12), 12);
    }

    #[test]
    fn live_heading_click_does_not_reveal_prefix() {
        let dec = decorate("### My Title", Some(6), ConcealMode::Live);
        assert_eq!(dec.display_text, "My Title");
        assert_eq!(Decorator::visual_col_to_buffer_col(&dec, 0, 12), 4);
        assert_eq!(Decorator::visual_col_to_buffer_col(&dec, 99, 12), 12);
    }

    #[test]
    fn live_heading_shows_prefix_while_composing() {
        let dec = decorate_reveal("### My Title", Some(6), ConcealMode::Live, true);
        assert_eq!(dec.display_text, "### My Title");
        assert_eq!(Decorator::visual_col_to_buffer_col(&dec, 0, 12), 0);
        assert_eq!(Decorator::visual_col_to_buffer_col(&dec, 4, 12), 4);
        assert_eq!(Decorator::visual_col_to_buffer_col(&dec, 99, 12), 12);
    }

    #[test]
    fn live_heading_hides_closed_inline_marks() {
        let line = "### Title with **bold**";
        let parsed = MarkdownParser::parse_line(line);
        let scale = scale_for(&parsed);
        let dec = Decorator::decorate_line(0, &parsed, Some(6), ConcealMode::Live, scale, false);
        assert_eq!(dec.display_text, "Title with bold");
        assert!(!dec.display_text.contains("**"));
        let dec2 = Decorator::decorate_line(0, &parsed, Some(18), ConcealMode::Live, scale, true);
        assert!(dec2.display_text.starts_with("### "));
        assert!(!dec2.display_text.contains("**"));
        assert!(dec2.display_text.contains("bold"));
    }

    #[test]
    fn live_incomplete_marks_stay_plain_visible() {
        let dec = decorate("hello *world", Some(7), ConcealMode::Live);
        assert_eq!(dec.display_text, "hello *world");
        let dec2 = decorate("hello `code", Some(7), ConcealMode::Live);
        assert_eq!(dec2.display_text, "hello `code");
    }

    #[test]
    fn live_empty_wrap_stays_visible() {
        // Adjacent ** parse as an empty italic pair — still composing, keep markers
        let dec = decorate("hello **world", Some(8), ConcealMode::Live);
        assert_eq!(dec.display_text, "hello **world");
        let dec2 = decorate("**", Some(2), ConcealMode::Live);
        assert_eq!(dec2.display_text, "**");
    }

    #[test]
    fn live_inline_marks_stay_hidden() {
        let line = "Hello **bold** and *italic*";
        let dec_bold = decorate(line, Some(9), ConcealMode::Live);
        assert_eq!(dec_bold.display_text, "Hello bold and italic");
        let dec_italic = decorate(line, Some(23), ConcealMode::Live);
        assert_eq!(dec_italic.display_text, "Hello bold and italic");
        let dec_plain = decorate(line, Some(2), ConcealMode::Live);
        assert_eq!(dec_plain.display_text, "Hello bold and italic");
    }

    #[test]
    fn live_list_source_while_composing() {
        let writing = decorate_reveal("- item", Some(2), ConcealMode::Live, true);
        assert_eq!(writing.display_text, "- item");
        let clicked = decorate("- item", Some(2), ConcealMode::Live);
        assert_eq!(clicked.display_text, "• item");
        let inactive = decorate("- item", None, ConcealMode::Live);
        assert_eq!(inactive.display_text, "• item");
    }

    #[test]
    fn live_task_source_while_composing() {
        let writing = decorate_reveal("- [ ] Todo", Some(6), ConcealMode::Live, true);
        assert!(writing.display_text.contains("[ ]"));
        assert!(!writing.display_text.contains('☐'));
        let clicked = decorate("- [ ] Todo", Some(6), ConcealMode::Live);
        assert!(clicked.display_text.starts_with("☐ "));
        assert!(!clicked.display_text.contains("[ ]"));
        let inactive = decorate("- [ ] Todo", None, ConcealMode::Live);
        assert!(inactive.display_text.starts_with("☐ "));
        assert!(!inactive.display_text.contains("[ ]"));
    }

    #[test]
    fn ordered_list_display() {
        let decorated = decorate("1. First item", None, ConcealMode::Live);
        assert_eq!(decorated.display_text, "1. First item");
        assert_eq!(
            decorated.visual_runs[0].role,
            VisualRole::StructuralMarker
        );
    }

    #[test]
    fn raw_heading_is_identity_no_scale() {
        let dec = decorate("### My Title", Some(0), ConcealMode::Raw);
        assert_eq!(dec.display_text, "### My Title");
        assert_eq!(dec.visual_runs[0].role, VisualRole::StructuralMarker);
        for run in &dec.visual_runs {
            assert_eq!(run.font_size_scale, 1.0);
        }
        // 1:1 mapping
        assert_eq!(Decorator::visual_col_to_buffer_col(&dec, 0, 12), 0);
        assert_eq!(Decorator::visual_col_to_buffer_col(&dec, 4, 12), 4);
    }

    #[test]
    fn raw_list_shows_source_not_glyphs() {
        let dec = decorate("- item", None, ConcealMode::Raw);
        assert_eq!(dec.display_text, "- item");
        assert!(!dec.display_text.contains('•'));
    }

    #[test]
    fn raw_no_emphasis_typefaces() {
        let dec = decorate("**bold** and *italic*", None, ConcealMode::Raw);
        assert_eq!(dec.display_text, "**bold** and *italic*");
        for run in &dec.visual_runs {
            assert_eq!(run.font_weight, VisualFontWeight::Normal);
            assert_eq!(run.font_style, VisualFontStyle::Normal);
        }
    }

    #[test]
    fn live_quote_prefix_while_composing() {
        let writing = decorate_reveal("> quoted", Some(2), ConcealMode::Live, true);
        assert_eq!(writing.display_text, "> quoted");
        let clicked = decorate("> quoted", Some(2), ConcealMode::Live);
        assert_eq!(clicked.display_text, "quoted");
        let inactive = decorate("> quoted", None, ConcealMode::Live);
        assert_eq!(inactive.display_text, "quoted");
    }

    #[test]
    fn live_fence_stays_visible() {
        let dec = decorate("```rust", Some(0), ConcealMode::Live);
        assert_eq!(dec.display_text, "```rust");
    }
}
