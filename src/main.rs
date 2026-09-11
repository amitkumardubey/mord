pub mod editor;
pub mod ui;

use gpui_component::Root;
use gpui_kit::gpui::*;
use ui::EditorView;

fn main() {
    let app = gpui_kit::application();

    app.run(move |cx| {
        gpui_kit::init(cx);
        gpui_component::init(cx);

        let window_size = size(px(1120.0), px(800.0));
        let window_bounds = Bounds::centered(None, window_size, cx);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Mord - Word-like Live Markdown Editor".into()),
                appears_transparent: false,
                ..Default::default()
            }),
            window_min_size: Some(size(px(640.0), px(480.0))),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let editor_view = cx.new(|cx| EditorView::new(cx));
            cx.new(|cx| Root::new(editor_view, window, cx))
        })
        .expect("Failed to open main Mord window");

        cx.activate(true);
    });
}
