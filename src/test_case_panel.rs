use gpui::*;
use gpui_component::{button::Button, *};

use crate::test_case_card::TestCaseCard;

pub struct TestCasePanel {
    test_case_cards: Vec<Entity<TestCaseCard>>,
}

impl TestCasePanel {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let test_case_cards: Vec<Entity<TestCaseCard>> = vec![];

        Self { test_case_cards }
    }
}

impl Render for TestCasePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_4()
            .child(
                Button::new("my-button")
                    .label("添加测试样例")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.test_case_cards
                            .push(cx.new(|cx| TestCaseCard::new(window, cx)));
                    })),
            )
            .child(
                div()
                    .id("test-cases-container")
                    .flex_1()
                    .scrollable(Axis::Vertical)
                    .children(self.test_case_cards.iter_mut().map(|panel| panel.clone())),
            )
    }
}
