use crate::models::{TestCase, TestStatus};
use arboard::Clipboard;
use egui::{Color32, RichText, ScrollArea, TextEdit, Ui};
use uuid::Uuid;

/// 测试面板
pub struct TestPanel {
    collapsed_tests: std::collections::HashSet<Uuid>,
    run_test_id: Option<Uuid>,                           // 要运行的测试ID
    input_heights: std::collections::HashMap<Uuid, f32>, // 每个测试的输入框高度
    expected_heights: std::collections::HashMap<Uuid, f32>, // 每个测试的期望输出框高度
    last_test_status: std::collections::HashMap<Uuid, TestStatus>, // 记录上次的测试状态
}

impl Default for TestPanel {
    fn default() -> Self {
        Self {
            collapsed_tests: std::collections::HashSet::new(),
            run_test_id: None,
            input_heights: std::collections::HashMap::new(),
            expected_heights: std::collections::HashMap::new(),
            last_test_status: std::collections::HashMap::new(),
        }
    }
}

impl TestPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// 渲染测试面板
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        tests: &mut Vec<TestCase>,
        on_delete_test: &mut Option<Uuid>,
        on_add_test: bool,
    ) {
        // 重置运行测试ID
        self.run_test_id = None;
        // 直接添加空测试点
        if on_add_test {
            let new_test = TestCase::new(String::new(), String::new());
            tests.push(new_test);
        }

        ui.vertical(|ui| {
            // 标题栏
            ui.horizontal(|ui| {
                ui.heading("Test Cases");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{} tests", tests.len()))
                            .size(12.0)
                            .color(Color32::GRAY),
                    );
                });
            });

            ui.separator();

            if tests.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(30.0);
                    ui.label(
                        RichText::new("No Test Cases")
                            .size(14.0)
                            .color(Color32::GRAY),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Click '➕ Add Test' button above to add custom test cases")
                            .size(11.0)
                            .color(Color32::DARK_GRAY),
                    );
                });
            } else {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut i = 0;
                        for test in tests.iter_mut() {
                            i += 1;

                            // 只在状态改变时自动折叠/展开
                            let last_status = self.last_test_status.get(&test.id).copied();
                            if last_status != Some(test.status) {
                                // 状态发生了变化
                                if test.status == TestStatus::Accepted {
                                    self.collapsed_tests.insert(test.id);
                                } else if test.status != TestStatus::Pending {
                                    // 其他非 Pending 状态（失败、错误等）自动展开
                                    self.collapsed_tests.remove(&test.id);
                                }
                                // 更新记录的状态
                                self.last_test_status.insert(test.id, test.status);
                            }

                            self.render_test_case(ui, test, i, on_delete_test);
                        }
                    });
            }
        });
    }

    /// 渲染单个测试用例
    fn render_test_case(
        &mut self,
        ui: &mut Ui,
        test: &mut TestCase,
        index: usize,
        on_delete_test: &mut Option<Uuid>,
    ) {
        let is_collapsed = self.collapsed_tests.contains(&test.id);

        let frame = egui::Frame::none()
            .fill(Color32::from_rgb(30, 30, 30))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 60)))
            .inner_margin(10.0)
            .outer_margin(egui::Margin::symmetric(0.0, 4.0))
            .rounding(6.0);

        frame.show(ui, |ui| {
            // 测试用例头部
            ui.horizontal(|ui| {
                // Collapse button
                let collapse_icon = if is_collapsed { "▶" } else { "▼" };
                if ui.button(RichText::new(collapse_icon).size(12.0)).clicked() {
                    if is_collapsed {
                        self.collapsed_tests.remove(&test.id);
                    } else {
                        self.collapsed_tests.insert(test.id);
                    }
                }

                // Status icon and title
                ui.label(
                    RichText::new(format!("Test #{}", index))
                        .size(13.0)
                        .strong()
                        .color(Color32::WHITE),
                );

                // Status text
                if test.status != TestStatus::Pending {
                    ui.label(
                        RichText::new(format!("- {}", test.status.text()))
                            .size(12.0)
                            .color(test.status.color()),
                    );
                }

                // Execution time
                if let Some(time) = test.execution_time {
                    ui.separator();
                    ui.label(
                        RichText::new(format!("⏱ {:.0}ms", time.as_millis()))
                            .size(11.0)
                            .color(Color32::GRAY),
                    );
                }

                // Right side buttons
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(RichText::new("🗑").color(Color32::from_rgb(200, 0, 0)))
                        .on_hover_text("Delete this test")
                        .clicked()
                    {
                        *on_delete_test = Some(test.id);
                    }

                    // 运行按钮
                    if ui
                        .button(RichText::new("▶").color(Color32::from_rgb(0, 200, 0)))
                        .on_hover_text("Run this test")
                        .clicked()
                    {
                        self.run_test_id = Some(test.id);
                    }
                });
            });

            // 只在未折叠时显示详细内容
            if !is_collapsed {
                ui.add_space(6.0);

                // Input section
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Input:")
                            .size(11.0)
                            .color(Color32::LIGHT_GRAY),
                    );

                    // 粘贴按钮
                    if ui
                        .button(RichText::new("📋 Paste").size(10.0))
                        .on_hover_text("Paste from clipboard")
                        .clicked()
                    {
                        if let Ok(mut clipboard) = Clipboard::new() {
                            if let Ok(text) = clipboard.get_text() {
                                test.input = text;
                            }
                        }
                    }
                });
                ui.add_space(2.0);

                let input_height = self.input_heights.entry(test.id).or_insert(100.0);

                egui::Frame::none()
                    .fill(Color32::from_rgb(20, 20, 20))
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(50, 50, 50)))
                    .inner_margin(6.0)
                    .rounding(3.0)
                    .show(ui, |ui| {
                        ui.set_height(*input_height);
                        ScrollArea::vertical()
                            .id_source(format!("input_{}", test.id))
                            .show(ui, |ui| {
                                ui.add(
                                    TextEdit::multiline(&mut test.input)
                                        .desired_width(f32::INFINITY)
                                        .font(egui::TextStyle::Monospace)
                                        .hint_text("Enter test input here..."),
                                );
                            });
                    });

                // 拖拽调整输入框高度
                let resize_response = ui.allocate_rect(
                    egui::Rect::from_min_size(
                        ui.cursor().left_top(),
                        egui::vec2(ui.available_width(), 6.0),
                    ),
                    egui::Sense::drag(),
                );

                ui.painter().hline(
                    ui.cursor().left()..=ui.cursor().right(),
                    ui.cursor().top() + 3.0,
                    egui::Stroke::new(1.0, Color32::from_rgb(100, 100, 100)),
                );

                if resize_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                }

                if resize_response.dragged() {
                    *input_height =
                        (*input_height + resize_response.drag_delta().y).clamp(50.0, 500.0);
                }

                ui.add_space(6.0);

                // Expected output section
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Expected Output:")
                            .size(11.0)
                            .color(Color32::LIGHT_GRAY),
                    );

                    // 粘贴按钮
                    if ui
                        .button(RichText::new("📋 Paste").size(10.0))
                        .on_hover_text("Paste from clipboard")
                        .clicked()
                    {
                        if let Ok(mut clipboard) = Clipboard::new() {
                            if let Ok(text) = clipboard.get_text() {
                                test.expected_output = text;
                            }
                        }
                    }
                });
                ui.add_space(2.0);

                let expected_height = self.expected_heights.entry(test.id).or_insert(100.0);

                egui::Frame::none()
                    .fill(Color32::from_rgb(20, 20, 20))
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(50, 50, 50)))
                    .inner_margin(6.0)
                    .rounding(3.0)
                    .show(ui, |ui| {
                        ui.set_height(*expected_height);
                        ScrollArea::vertical()
                            .id_source(format!("expected_{}", test.id))
                            .show(ui, |ui| {
                                ui.add(
                                    TextEdit::multiline(&mut test.expected_output)
                                        .desired_width(f32::INFINITY)
                                        .font(egui::TextStyle::Monospace)
                                        .hint_text("Enter expected output here..."),
                                );
                            });
                    });

                // 拖拽调整期望输出框高度
                let resize_response = ui.allocate_rect(
                    egui::Rect::from_min_size(
                        ui.cursor().left_top(),
                        egui::vec2(ui.available_width(), 6.0),
                    ),
                    egui::Sense::drag(),
                );

                ui.painter().hline(
                    ui.cursor().left()..=ui.cursor().right(),
                    ui.cursor().top() + 3.0,
                    egui::Stroke::new(1.0, Color32::from_rgb(100, 100, 100)),
                );

                if resize_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                }

                if resize_response.dragged() {
                    *expected_height =
                        (*expected_height + resize_response.drag_delta().y).clamp(50.0, 500.0);
                }

                // Actual output (if any)
                if let Some(actual_output) = &test.actual_output {
                    ui.add_space(6.0);

                    ui.label(
                        RichText::new("Actual Output:")
                            .size(11.0)
                            .color(Color32::LIGHT_GRAY),
                    );
                    ui.add_space(2.0);

                    let output_color = match test.status {
                        TestStatus::Accepted => Color32::from_rgb(0, 50, 0),
                        TestStatus::WrongAnswer => Color32::from_rgb(50, 0, 0),
                        _ => Color32::from_rgb(20, 20, 20),
                    };

                    egui::Frame::none()
                        .fill(output_color)
                        .stroke(egui::Stroke::new(
                            1.0,
                            match test.status {
                                TestStatus::Accepted => Color32::from_rgb(0, 150, 0),
                                TestStatus::WrongAnswer => Color32::from_rgb(150, 0, 0),
                                _ => Color32::from_rgb(50, 50, 50),
                            },
                        ))
                        .inner_margin(6.0)
                        .rounding(3.0)
                        .show(ui, |ui| {
                            ScrollArea::vertical()
                                .id_source(format!("actual_{}", test.id))
                                .max_height(100.0)
                                .show(ui, |ui| {
                                    ui.add(
                                        TextEdit::multiline(&mut actual_output.as_str())
                                            .desired_width(f32::INFINITY)
                                            .font(egui::TextStyle::Monospace)
                                            .interactive(false),
                                    );
                                });
                        });
                }

                // Error message (if any)
                if let Some(error) = &test.error_message {
                    ui.add_space(6.0);

                    egui::Frame::none()
                        .fill(Color32::from_rgb(50, 20, 20))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(150, 0, 0)))
                        .inner_margin(6.0)
                        .rounding(3.0)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("❌ Error: {}", error))
                                    .size(11.0)
                                    .color(Color32::from_rgb(255, 100, 100))
                                    .monospace(),
                            );
                        });
                }
            }
        });
    }

    pub fn get_run_test_id(&self) -> Option<Uuid> {
        self.run_test_id
    }
}
