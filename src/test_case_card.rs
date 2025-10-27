use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    button::Button,
    input::{InputState, TextInput},
    label::Label,
    *,
};

pub struct TestCaseCard {
    expended: bool,
    stdin_input: Entity<InputState>,
    expected_input: Entity<InputState>,
}

impl TestCaseCard {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut create_input = || {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .auto_grow(1, 10)
                    .soft_wrap(false)
            })
        };

        Self {
            expended: true,
            stdin_input: create_input(),
            expected_input: create_input(),
        }
    }
}

impl Render for TestCaseCard {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .child(
                Button::new("toggle-expended")
                    .label(if self.expended { "收起" } else { "展开" })
                    .on_click(cx.listener(|this, _, _, _| {
                        this.expended = !this.expended;
                    })),
            )
            .when(self.expended, |this| {
                this.child(
                    v_flex().gap_2().p_4().children([
                        Label::new("标准输入 (stdin):").into_any_element(),
                        TextInput::new(&self.stdin_input)
                            .w_full()
                            .into_any_element(),
                        Label::new("期望输出 (expected):").into_any_element(),
                        TextInput::new(&self.expected_input)
                            .w_full()
                            .into_any_element(),
                    ]),
                )
            })
    }
}
