use gpui::*;
use gpui_component::{
    input::{InputState, TextInput},
    label::Label,
    *,
};

pub struct TestCasePanel {
    stdin_input: Entity<InputState>,
    expected_input: Entity<InputState>,
}

impl TestCasePanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
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
