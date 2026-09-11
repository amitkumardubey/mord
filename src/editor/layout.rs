//! Layout helpers: visual runs → GPUI TextRuns, and UTF-8 ↔ visual column mapping.
//!
//! Paint lives in `ui::document_line`. Hit-testing uses these helpers with a
//! shaped line produced by GPUI (never guessed glyph widths).

use crate::editor::decorator::{
    DecoratedLine, VisualFontStyle, VisualFontWeight, VisualRun,
};
use crate::editor::offset::{byte_to_char_index, char_to_byte_index, BufferCol, VisualCol};
use crate::editor::parser::MarkerType;
use gpui_kit::gpui::{
    point, px, rgb, size, Bounds, Font, FontStyle, FontWeight, Hsla, Pixels, Point, SharedString,
    StrikethroughStyle, TextRun, UnderlineStyle, WrappedLine,
};
use std::ops::Range;

/// Colors / metrics needed to turn visual runs into TextRuns (no UI dependency).
#[derive(Clone, Copy)]
pub struct LinePaintTheme {
    pub text_primary: u32,
    pub text_heading: u32,
    pub text_marker_dimmed: u32,
    pub text_marker_active: u32,
    pub text_link: u32,
    pub bg_code_inline: u32,
    pub bg_selection: u32,
    pub cursor_color: u32,
    pub code_accent: u32,
    pub task_checked: u32,
    pub font_size_base: f32,
    pub line_height_base: f32,
    pub heading_scales: [f32; 6],
}

impl LinePaintTheme {
    pub fn hsla(hex: u32) -> Hsla {
        Hsla::from(rgb(hex))
    }

    pub fn heading_scale(&self, level: usize) -> f32 {
        if (1..=6).contains(&level) {
            self.heading_scales[level - 1]
        } else {
            1.0
        }
    }
}

/// Prepared text for shaping one document line.
#[derive(Clone)]
pub struct ShapedLineInput {
    pub text: SharedString,
    pub runs: Vec<TextRun>,
    pub font_size: Pixels,
    pub line_height: Pixels,
}

/// Result of mapping a click onto a decorated line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineHitResult {
    pub visual_col: VisualCol,
    pub buffer_col: BufferCol,
    /// True when the hit landed on a task-list checkbox glyph.
    pub on_task_marker: bool,
}

pub fn line_font_size(decorated: &DecoratedLine, theme: &LinePaintTheme) -> Pixels {
    let scale = decorated
        .visual_runs
        .iter()
        .map(|r| r.font_size_scale)
        .fold(1.0_f32, f32::max);
    px(theme.font_size_base * scale)
}

pub fn line_height(decorated: &DecoratedLine, theme: &LinePaintTheme) -> Pixels {
    let scale = decorated
        .visual_runs
        .iter()
        .map(|r| r.font_size_scale)
        .fold(1.0_f32, f32::max);
    px(theme.line_height_base * scale.max(1.0))
}

/// Build GPUI text runs from a decorated line. Run lengths are UTF-8 bytes.
pub fn build_shaped_line_input(
    decorated: &DecoratedLine,
    theme: &LinePaintTheme,
    base_font: Font,
    is_active_line: bool,
) -> ShapedLineInput {
    let text: SharedString = if decorated.display_text.is_empty() {
        " ".into()
    } else {
        decorated.display_text.clone().into()
    };

    let font_size = line_font_size(decorated, theme);
    let line_height = line_height(decorated, theme);

    if decorated.display_text.is_empty() || decorated.visual_runs.is_empty() {
        let run = TextRun {
            len: text.len(),
            font: base_font,
            color: LinePaintTheme::hsla(theme.text_primary),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        return ShapedLineInput {
            text,
            runs: vec![run],
            font_size,
            line_height,
        };
    }

    let mut runs = Vec::with_capacity(decorated.visual_runs.len());
    for run in &decorated.visual_runs {
        let byte_len = run.text.len();
        if byte_len == 0 {
            continue;
        }
        runs.push(visual_run_to_text_run(
            run,
            theme,
            &base_font,
            is_active_line,
            byte_len,
        ));
    }

    let covered: usize = runs.iter().map(|r| r.len).sum();
    if covered < text.len() {
        runs.push(TextRun {
            len: text.len() - covered,
            font: base_font,
            color: LinePaintTheme::hsla(theme.text_primary),
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }

    ShapedLineInput {
        text,
        runs,
        font_size,
        line_height,
    }
}

fn visual_run_to_text_run(
    run: &VisualRun,
    theme: &LinePaintTheme,
    base_font: &Font,
    is_active_line: bool,
    byte_len: usize,
) -> TextRun {
    use crate::editor::decorator::VisualRole;

    let mut font = base_font.clone();
    match run.font_weight {
        VisualFontWeight::Bold | VisualFontWeight::ExtraBold => {
            font.weight = FontWeight::BOLD;
        }
        VisualFontWeight::Normal => {}
    }
    if run.font_style == VisualFontStyle::Italic {
        font.style = FontStyle::Italic;
    }

    let mut color = LinePaintTheme::hsla(theme.text_primary);
    match run.role {
        VisualRole::Heading => {
            color = LinePaintTheme::hsla(theme.text_heading);
        }
        VisualRole::StructuralMarker | VisualRole::InlineMarker => {
            color = if is_active_line {
                LinePaintTheme::hsla(theme.text_marker_active)
            } else {
                LinePaintTheme::hsla(theme.text_marker_dimmed)
            };
        }
        VisualRole::Code => {
            color = LinePaintTheme::hsla(theme.code_accent);
        }
        VisualRole::Link => {
            color = LinePaintTheme::hsla(theme.text_link);
        }
        VisualRole::TaskGlyph => {
            if matches!(
                run.marker_type,
                Some(MarkerType::TaskListMarker { checked: true })
            ) {
                color = LinePaintTheme::hsla(theme.task_checked);
            }
        }
        VisualRole::Emphasis | VisualRole::Plain => {}
    }

    let background_color = if run.role == VisualRole::Code {
        Some(LinePaintTheme::hsla(theme.bg_code_inline))
    } else {
        None
    };

    let underline = if run.role == VisualRole::Link || run.link_url.is_some() {
        Some(UnderlineStyle {
            thickness: px(1.0),
            color: Some(color),
            wavy: false,
        })
    } else {
        None
    };

    let strikethrough = if run.is_strikethrough {
        Some(StrikethroughStyle {
            thickness: px(1.0),
            color: Some(color),
        })
    } else {
        None
    };

    TextRun {
        len: byte_len,
        font,
        color,
        background_color,
        underline,
        strikethrough,
    }
}

fn byte_index_from_closest(result: Result<usize, usize>) -> usize {
    match result {
        Ok(i) | Err(i) => i,
    }
}

pub fn visual_col_for_position(
    wrapped: &WrappedLine,
    display_text: &str,
    local: Point<Pixels>,
    line_height: Pixels,
) -> VisualCol {
    if display_text.is_empty() {
        return VisualCol(0);
    }
    let byte_idx = byte_index_from_closest(wrapped.closest_index_for_position(local, line_height));
    VisualCol(byte_to_char_index(
        display_text,
        byte_idx.min(display_text.len()),
    ))
}

pub fn position_for_visual_col(
    wrapped: &WrappedLine,
    display_text: &str,
    col: VisualCol,
    line_height: Pixels,
) -> Point<Pixels> {
    if display_text.is_empty() {
        return point(px(0.0), px(0.0));
    }
    let byte_idx = char_to_byte_index(display_text, col.get());
    wrapped
        .position_for_index(byte_idx, line_height)
        .unwrap_or_else(|| point(px(0.0), px(0.0)))
}

/// Map a document-level selection onto visual char columns within one line.
pub fn line_selection_visual_range(
    decorated: &DecoratedLine,
    line_start: usize,
    line_len: usize,
    sel_start: usize,
    sel_end: usize,
) -> Option<Range<VisualCol>> {
    if sel_start == sel_end {
        return None;
    }
    let line_end = line_start + line_len;
    if sel_end <= line_start || sel_start >= line_end {
        return None;
    }
    let local_start = sel_start.saturating_sub(line_start).min(line_len);
    let local_end = sel_end.saturating_sub(line_start).min(line_len);
    if local_start >= local_end {
        return None;
    }
    let v_start =
        crate::editor::decorator::Decorator::buffer_col_to_visual_col(decorated, local_start);
    let v_end = crate::editor::decorator::Decorator::buffer_col_to_visual_col(decorated, local_end);
    Some(VisualCol(v_start)..VisualCol(v_end.max(v_start)))
}

/// Selection highlight rects for a visual range, accounting for soft wrap.
pub fn selection_bounds(
    wrapped: &WrappedLine,
    display_text: &str,
    visual: Range<VisualCol>,
    origin: Point<Pixels>,
    line_height: Pixels,
) -> Vec<Bounds<Pixels>> {
    if display_text.is_empty() || visual.start >= visual.end {
        return Vec::new();
    }
    let byte_start = char_to_byte_index(display_text, visual.start.get());
    let byte_end = char_to_byte_index(display_text, visual.end.get());
    let Some(p0) = wrapped.position_for_index(byte_start, line_height) else {
        return Vec::new();
    };
    let Some(p1) = wrapped.position_for_index(byte_end, line_height) else {
        return Vec::new();
    };
    let wrap_w = wrapped.width().max(px(2.0));

    if p0.y == p1.y {
        return vec![Bounds::from_corners(
            point(origin.x + p0.x, origin.y + p0.y),
            point(
                origin.x + p1.x.max(p0.x + px(2.0)),
                origin.y + p0.y + line_height,
            ),
        )];
    }

    let mut rects = Vec::new();
    // First wrapped row: from caret x to wrap width
    rects.push(Bounds::from_corners(
        point(origin.x + p0.x, origin.y + p0.y),
        point(origin.x + wrap_w, origin.y + p0.y + line_height),
    ));
    // Middle full rows
    let mut y = p0.y + line_height;
    while y < p1.y {
        rects.push(Bounds::new(
            point(origin.x, origin.y + y),
            size(wrap_w, line_height),
        ));
        y = y + line_height;
    }
    // Last wrapped row
    rects.push(Bounds::from_corners(
        point(origin.x, origin.y + p1.y),
        point(
            origin.x + p1.x.max(px(2.0)),
            origin.y + p1.y + line_height,
        ),
    ));
    rects
}

pub fn hit_test_wrapped_line(
    wrapped: &WrappedLine,
    decorated: &DecoratedLine,
    line_len: usize,
    local: Point<Pixels>,
    line_height: Pixels,
) -> LineHitResult {
    let visual_col = visual_col_for_position(wrapped, &decorated.display_text, local, line_height);
    let buffer_col = BufferCol(crate::editor::decorator::Decorator::visual_col_to_buffer_col(
        decorated,
        visual_col.get(),
        line_len,
    ));
    let on_task_marker = point_on_task_marker(decorated, visual_col);
    LineHitResult {
        visual_col,
        buffer_col,
        on_task_marker,
    }
}

fn point_on_task_marker(decorated: &DecoratedLine, visual_col: VisualCol) -> bool {
    use crate::editor::decorator::VisualRole;
    let mut offset = 0usize;
    for run in &decorated.visual_runs {
        let run_len = run.text.chars().count();
        let run_end = offset + run_len;
        if visual_col.get() >= offset && visual_col.get() < run_end {
            return run.role == VisualRole::TaskGlyph
                || matches!(run.marker_type, Some(MarkerType::TaskListMarker { .. }));
        }
        offset = run_end;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::decorator::{ConcealMode, Decorator};
    use crate::editor::parser::MarkdownParser;

    #[test]
    fn selection_range_on_concealed_heading() {
        let parsed = MarkdownParser::parse_line("### My Title");
        let decorated = Decorator::decorate_line(0, &parsed, None, ConcealMode::Live, 1.30, false);
        let range = line_selection_visual_range(&decorated, 0, 12, 4, 6).unwrap();
        assert_eq!(range.start, VisualCol(0));
        assert_eq!(range.end, VisualCol(2));
    }
}
