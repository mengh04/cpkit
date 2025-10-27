use gpui::*;
use gpui_component::*;

mod test_case_card;
mod test_case_panel;

use crate::test_case_panel::TestCasePanel;

fn main() {
    let app = Application::new();

    app.run(move |cx| {
        gpui_component::init(cx);

        let window_size = size(px(400.0), px(850.0));
        let window_bounds = Bounds::centered(None, window_size, cx);

        cx.spawn(async move |cx| {
            let window_option = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                ..Default::default()
            };

            cx.open_window(window_option, |window, cx| {
                let view = cx.new(|cx| TestCasePanel::new(window, cx));
                cx.new(|cx| Root::new(view.into(), window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    })
}
