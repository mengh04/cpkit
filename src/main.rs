use gpui::*;
use gpui_component::{
    button::Button,
    input::{InputState, TextInput},
    label::Label,
    *,
};

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

struct TestCasePanel {
    stdin_input: Entity<InputState>,
    expected_input: Entity<InputState>,
}

impl TestCasePanel {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut create_input = || {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .auto_grow(1, 10)
                    .soft_wrap(false)
            })
        };

        Self {
            stdin_input: create_input(),
            expected_input: create_input(),
        }
    }
}

impl Render for TestCasePanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_2().p_4().children([
            Label::new("标准输入 (stdin):").into_any_element(),
            TextInput::new(&self.stdin_input)
                .w_full()
                .into_any_element(),
            Label::new("期望输出 (expected):").into_any_element(),
            TextInput::new(&self.expected_input)
                .w_full()
                .into_any_element(),
        ])
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
