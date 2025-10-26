use gpui::*;
use gpui_component::{button::Button, *};

mod test_case_panel;
use test_case_panel::TestCasePanel;

struct HelloWorld {
    test_case_panels: Vec<Entity<TestCasePanel>>,
}

impl HelloWorld {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let test_case_panels: Vec<Entity<TestCasePanel>> = vec![];

        Self { test_case_panels }
    }
}

impl Render for HelloWorld {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_4()
            .child(
                Button::new("my-button")
                    .label("添加测试样例")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.test_case_panels
                            .push(cx.new(|cx| TestCasePanel::new(window, cx)));
                    })),
            )
            .child(
                div()
                    .id("test-cases-container")
                    .flex_1()
                    .scrollable(Axis::Vertical)
                    .children(self.test_case_panels.iter_mut().map(|panel| panel.clone())),
            )
    }
}

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
                let view = cx.new(|cx| HelloWorld::new(window, cx));
                cx.new(|cx| Root::new(view.into(), window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    })
}
