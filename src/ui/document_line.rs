//! One document line: GPUI soft-wrapped shaped text, overlay caret, selection.

use crate::editor::decorator::DecoratedLine;
use crate::editor::layout::{
    build_shaped_line_input, line_selection_visual_range, position_for_visual_col, selection_bounds,
    LinePaintTheme,
};
use crate::editor::offset::VisualCol;
use gpui_kit::gpui::{
    fill, point, px, size, App, AvailableSpace, Bounds, Element, ElementId, GlobalElementId,
    IntoElement, LayoutId, PaintQuad, Pixels, Style, TextAlign, Window, WrappedLine,
};
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

/// Cached bounds + wrap metrics for hit-testing (reshape on click with same wrap width).
#[derive(Clone)]
pub struct LinePaintCache {
    pub bounds: Bounds<Pixels>,
    pub wrap_width: Pixels,
    pub line_height: Pixels,
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
    pub selection: Option<Range<usize>>,
    pub line_caches: LineCacheMap,
}

pub struct PrepaintState {
    wrapped: WrappedLine,
    display_text: String,
    caret: Option<PaintQuad>,
    selection: Vec<PaintQuad>,
    line_height: Pixels,
    wrap_width: Pixels,
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
        _cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let decorated = self.decorated.clone();
        let paint_theme = self.paint_theme;
        let is_active = self.is_active;

        let mut style = Style::default();
        style.size.width = gpui_kit::gpui::relative(1.).into();

        let layout_id = window.request_measured_layout(style, move |known, available, window, _cx| {
            let wrap_width = known.width.or(match available.width {
                AvailableSpace::Definite(w) => Some(w),
                _ => None,
            });
            let input = build_shaped_line_input(
                &decorated,
                &paint_theme,
                window.text_style().font(),
                is_active,
            );
            let height = match window.text_system().shape_text(
                input.text.clone(),
                input.font_size,
                &input.runs,
                wrap_width,
                None,
            ) {
                Ok(lines) => lines
                    .first()
                    .map(|line| line.size(input.line_height).height)
                    .unwrap_or(input.line_height),
                Err(_) => input.line_height,
            };
            let width = wrap_width.unwrap_or_else(|| match available.width {
                AvailableSpace::Definite(w) => w,
                _ => px(0.0),
            });
            size(width, height)
        });
        (layout_id, ())
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
        let wrap_width = bounds.size.width;
        let wrapped = window
            .text_system()
            .shape_text(
                input.text.clone(),
                input.font_size,
                &input.runs,
                Some(wrap_width),
                None,
            )
            .ok()
            .and_then(|mut lines| lines.pop())
            .or_else(|| {
                window
                    .text_system()
                    .shape_text(input.text, input.font_size, &input.runs, None, None)
                    .ok()
                    .and_then(|mut lines| lines.pop())
            })
            .expect("shape_text must succeed for document line");

        let display_text = self.decorated.display_text.clone();
        let line_height = input.line_height;

        let selection = self
            .selection
            .as_ref()
            .and_then(|sel| {
                line_selection_visual_range(
                    &self.decorated,
                    self.line_start,
                    self.line_len,
                    sel.start,
                    sel.end,
                )
            })
            .map(|visual| {
                selection_bounds(
                    &wrapped,
                    &display_text,
                    visual,
                    bounds.origin,
                    line_height,
                )
                .into_iter()
                .map(|b| fill(b, LinePaintTheme::hsla(self.paint_theme.bg_selection)))
                .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let caret = if self.is_active && self.caret_visible {
            let col = self.caret_visual_col.unwrap_or(VisualCol(0));
            let local = position_for_visual_col(&wrapped, &display_text, col, line_height);
            Some(fill(
                Bounds::new(
                    point(bounds.left() + local.x, bounds.top() + local.y),
                    size(px(2.0), line_height),
                ),
                LinePaintTheme::hsla(self.paint_theme.cursor_color),
            ))
        } else {
            None
        };

        PrepaintState {
            wrapped,
            display_text,
            caret,
            selection,
            line_height,
            wrap_width,
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
        for quad in prepaint.selection.drain(..) {
            window.paint_quad(quad);
        }

        let _ = prepaint.wrapped.paint(
            bounds.origin,
            prepaint.line_height,
            TextAlign::Left,
            Some(bounds),
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
                wrap_width: prepaint.wrap_width,
                line_height: prepaint.line_height,
                display_text: prepaint.display_text.clone(),
                line_len: self.line_len,
            },
        );
    }
}
