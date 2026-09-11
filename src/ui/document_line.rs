//! One document line: GPUI-shaped text, overlay caret, selection highlight.

use crate::editor::decorator::DecoratedLine;
use crate::editor::layout::{
    build_shaped_line_input, line_selection_visual_range, x_for_visual_col, LinePaintTheme,
};
use crate::editor::offset::VisualCol;
use gpui_kit::gpui::{
    fill, point, px, size, App, Bounds, Element, ElementId, GlobalElementId, IntoElement, LayoutId,
    PaintQuad, Pixels, ShapedLine, Style, TextAlign, Window,
};
use std::ops::Range;
use std::rc::Rc;
use std::cell::RefCell;

/// Cached shaped line + bounds for hit-testing after paint.
#[derive(Clone)]
pub struct LinePaintCache {
    pub bounds: Bounds<Pixels>,
    pub shaped: ShapedLine,
    pub display_text: String,
    pub line_len: usize,
}

pub type LineCacheMap = Rc<RefCell<std::collections::HashMap<usize, LinePaintCache>>>;

pub struct DocumentLine {
    pub line_index: usize,
    pub decorated: DecoratedLine,
    pub line_len: usize,
    pub line_start: usize,
    pub paint_theme: LinePaintTheme,
    pub is_active: bool,
    pub caret_visual_col: Option<VisualCol>,
    pub caret_visible: bool,
    /// Document-level selection range (char offsets), if any.
    pub selection: Option<Range<usize>>,
    pub line_caches: LineCacheMap,
}

pub struct PrepaintState {
    shaped: ShapedLine,
    display_text: String,
    caret: Option<PaintQuad>,
    selection: Option<PaintQuad>,
    line_height: Pixels,
}

impl IntoElement for DocumentLine {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for DocumentLine {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Integer(self.line_index as u64))
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui_kit::gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let input = build_shaped_line_input(
            &self.decorated,
            &self.paint_theme,
            window.text_style().font(),
            self.is_active,
        );
        let mut style = Style::default();
        style.size.width = gpui_kit::gpui::relative(1.).into();
        style.size.height = input.line_height.into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui_kit::gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        let input = build_shaped_line_input(
            &self.decorated,
            &self.paint_theme,
            window.text_style().font(),
            self.is_active,
        );
        let shaped = window.text_system().shape_line(
            input.text.clone(),
            input.font_size,
            &input.runs,
            None,
        );
        let display_text = self.decorated.display_text.clone();
        let line_height = input.line_height;

        let selection = self.selection.as_ref().and_then(|sel| {
            let visual = line_selection_visual_range(
                &self.decorated,
                self.line_start,
                self.line_len,
                sel.start,
                sel.end,
            )?;
            let x0 = x_for_visual_col(&shaped, &display_text, visual.start);
            let x1 = x_for_visual_col(&shaped, &display_text, visual.end);
            Some(fill(
                Bounds::from_corners(
                    point(bounds.left() + x0, bounds.top()),
                    point(bounds.left() + x1.max(x0 + px(2.0)), bounds.bottom()),
                ),
                LinePaintTheme::hsla(self.paint_theme.bg_selection),
            ))
        });

        let caret = if self.is_active && self.caret_visible {
            let col = self.caret_visual_col.unwrap_or(VisualCol(0));
            let x = x_for_visual_col(&shaped, &display_text, col);
            Some(fill(
                Bounds::new(
                    point(bounds.left() + x, bounds.top()),
                    size(px(2.0), line_height),
                ),
                LinePaintTheme::hsla(self.paint_theme.cursor_color),
            ))
        } else {
            None
        };

        PrepaintState {
            shaped,
            display_text,
            caret,
            selection,
            line_height,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui_kit::gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }

        let shaped = prepaint.shaped.clone();
        let _ = shaped.paint(
            bounds.origin,
            prepaint.line_height,
            TextAlign::Left,
            None,
            window,
            cx,
        );

        if let Some(caret) = prepaint.caret.take() {
            window.paint_quad(caret);
        }

        self.line_caches.borrow_mut().insert(
            self.line_index,
            LinePaintCache {
                bounds,
                shaped,
                display_text: prepaint.display_text.clone(),
                line_len: self.line_len,
            },
        );
    }
}
