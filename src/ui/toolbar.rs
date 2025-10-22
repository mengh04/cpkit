use egui::{Button, Color32, RichText, Ui};

/// 工具栏面板
pub struct Toolbar;

impl Toolbar {
    /// 渲染工具栏
    pub fn ui(
        ui: &mut Ui,
        source_file: &mut String,
        on_run_all: &mut bool,
        on_stop: &mut bool,
        on_add_test: &mut bool,
        on_clear_results: &mut bool,
        on_open_file: &mut bool,
        has_problem: bool,
        is_running: bool,
    ) {
        ui.vertical(|ui| {
            // 第一行：标题和状态
            ui.horizontal(|ui| {
                ui.heading("🏆 CPKit");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if is_running {
                        ui.spinner();
                        ui.label(RichText::new("Running...").color(Color32::LIGHT_BLUE));
                    } else if has_problem {
                        ui.label(RichText::new("✓ Ready").color(Color32::GREEN));
                    } else {
                        ui.label(RichText::new("Waiting...").color(Color32::GRAY));
                    }
                });
            });

            ui.add_space(4.0);

            // 第二行：源文件
            ui.horizontal(|ui| {
                ui.label("Source File:");
                ui.add(
                    egui::TextEdit::singleline(source_file)
                        .desired_width(ui.available_width() - 180.0)
                        .hint_text("接收题目后自动生成...")
                        .interactive(false),
                );

                // 打开/创建文件按钮
                ui.add_enabled_ui(
                    has_problem && !source_file.is_empty() && !is_running,
                    |ui| {
                        if ui
                            .add(
                                Button::new(RichText::new("📝 打开文件").color(Color32::WHITE))
                                    .fill(Color32::from_rgb(0, 150, 100)),
                            )
                            .on_hover_text("创建并用 Zed 打开源文件")
                            .clicked()
                        {
                            *on_open_file = true;
                        }
                    },
                );
            });

            ui.add_space(4.0);

            // 第三行：操作按钮
            ui.horizontal(|ui| {
                // Run buttons - allow running even without problem as long as source file exists
                ui.add_enabled_ui(!source_file.is_empty() && !is_running, |ui| {
                    if ui
                        .add(
                            Button::new(RichText::new("▶ Run All").color(Color32::WHITE))
                                .fill(Color32::from_rgb(0, 120, 212)),
                        )
                        .on_hover_text("Run all test cases")
                        .clicked()
                    {
                        *on_run_all = true;
                    }
                });

                // Stop button
                ui.add_enabled_ui(is_running, |ui| {
                    if ui
                        .add(
                            Button::new(RichText::new("⏹ Stop").color(Color32::WHITE))
                                .fill(Color32::from_rgb(200, 0, 0)),
                        )
                        .clicked()
                    {
                        *on_stop = true;
                    }
                });

                ui.separator();

                // Add/Clear buttons
                // Allow adding tests even without a problem (as long as source file exists)
                ui.add_enabled_ui(!source_file.is_empty() && !is_running, |ui| {
                    if ui
                        .button("➕ Add Test")
                        .on_hover_text("Add empty test case")
                        .clicked()
                    {
                        *on_add_test = true;
                    }
                });

                ui.add_enabled_ui(has_problem && !is_running, |ui| {
                    if ui
                        .button("🗑 Clear Results")
                        .on_hover_text("Clear all test results")
                        .clicked()
                    {
                        *on_clear_results = true;
                    }
                });
            });
        });
    }
}
