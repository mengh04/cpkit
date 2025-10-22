use crate::competitive_companion::SharedProblemStore;
use crate::judge::Judge;
use crate::models::{Language, TestCase, TestStatus};
use crate::storage::ProblemStore;
use crate::ui::{TestPanel, Toolbar};
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use uuid::Uuid;

/// 应用消息
#[derive(Debug, Clone)]
enum AppMessage {
    ProblemsUpdated(Vec<ProblemData>),
    CurrentProblemChanged(Option<Uuid>, Vec<TestCase>),
    #[allow(dead_code)]
    TestsUpdated(Vec<TestCase>),
    SourceFileTestsUpdated(Vec<TestCase>), // 源文件测试点更新
    RunCompleted,
}

/// 问题数据（可跨线程传递）
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ProblemData {
    id: Uuid,
    name: String,
    group: String,
    tests_len: usize,
    time_limit: u64,
    memory_limit: u64,
    passed: usize,
}

/// CPKit 主应用
pub struct CPKitApp {
    problem_store: SharedProblemStore,
    test_panel: TestPanel,

    // UI 状态
    current_language: Language,
    source_file: String,
    previous_source_file: String, // 用于检测源文件变化
    is_running: bool,

    // 缓存数据
    cached_problems: Vec<ProblemData>,
    cached_current_id: Option<Uuid>,
    cached_tests: Vec<TestCase>,
    source_file_tests: Vec<TestCase>, // 基于源文件的测试点

    // 运行时状态
    last_error: Option<String>,

    // 事件标志
    pending_run_all: bool,
    pending_run_test_id: Option<Uuid>, // 待运行的测试ID
    pending_stop: bool,
    pending_add_test: bool,

    // 消息通道
    tx: Sender<AppMessage>,
    rx: Receiver<AppMessage>,

    // 刷新标志
    frame_count: u64,
}

impl CPKitApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        problem_store: SharedProblemStore,
        source_file: Option<String>,
    ) -> Self {
        Self::setup_custom_fonts(&_cc.egui_ctx);
        Self::setup_custom_style(&_cc.egui_ctx);

        let (tx, rx) = channel();

        let source_file_str = source_file.clone().unwrap_or_default();

        // 尝试从源文件加载测试点
        let source_file_tests = if !source_file_str.is_empty() {
            tracing::info!("启动时加载测试点，源文件: {}", source_file_str);
            let path = PathBuf::from(&source_file_str);
            match ProblemStore::load_tests_from_source_file(&path) {
                Ok(tests) => {
                    tracing::info!("✅ 成功加载 {} 个测试点", tests.len());
                    tests
                }
                Err(e) => {
                    tracing::warn!("⚠️ 加载测试点失败: {}", e);
                    Vec::new()
                }
            }
        } else {
            tracing::info!("启动时未指定源文件，跳过加载测试点");
            Vec::new()
        };

        let app = Self {
            problem_store: problem_store.clone(),
            test_panel: TestPanel::new(),
            current_language: Language::Cpp,
            source_file: source_file_str.clone(),
            previous_source_file: source_file_str.clone(),
            is_running: false,
            cached_problems: Vec::new(),
            cached_current_id: None,
            cached_tests: Vec::new(),
            source_file_tests,
            last_error: None,
            pending_run_all: false,
            pending_run_test_id: None,
            pending_stop: false,
            pending_add_test: false,
            tx: tx.clone(),
            rx,
            frame_count: 0,
        };

        // 启动后台任务定期同步数据
        let store = problem_store.clone();
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;

                let store_lock = store.lock().await;

                // 获取问题列表
                let problems: Vec<ProblemData> = store_lock
                    .get_all_problems()
                    .iter()
                    .map(|p| {
                        let passed = p
                            .tests
                            .iter()
                            .filter(|t| t.status == TestStatus::Accepted)
                            .count();
                        ProblemData {
                            id: p.id,
                            name: p.name.clone(),
                            group: p.group.clone(),
                            tests_len: p.tests.len(),
                            time_limit: p.time_limit,
                            memory_limit: p.memory_limit,
                            passed,
                        }
                    })
                    .collect();

                let current_id = store_lock.get_current_problem().map(|p| p.id);
                let tests = store_lock
                    .get_current_problem()
                    .map(|p| p.tests.clone())
                    .unwrap_or_default();

                drop(store_lock);

                let _ = tx_clone.send(AppMessage::ProblemsUpdated(problems));
                let _ = tx_clone.send(AppMessage::CurrentProblemChanged(current_id, tests));
            }
        });

        app
    }

    /// 设置自定义字体
    fn setup_custom_fonts(ctx: &egui::Context) {
        let fonts = egui::FontDefinitions::default();
        ctx.set_fonts(fonts);
    }

    /// 设置自定义样式
    fn setup_custom_style(ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();

        // 暗色主题
        style.visuals = egui::Visuals::dark();
        style.visuals.window_rounding = egui::Rounding::same(8.0);
        style.visuals.menu_rounding = egui::Rounding::same(6.0);

        ctx.set_style(style);
    }

    /// 处理接收到的消息
    fn process_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AppMessage::ProblemsUpdated(problems) => {
                    self.cached_problems = problems;
                }
                AppMessage::CurrentProblemChanged(id, tests) => {
                    let problem_changed = id != self.cached_current_id;
                    self.cached_current_id = id;
                    self.cached_tests = tests;

                    // 当检测到新的 problem 时，保存测试点到源文件
                    if problem_changed && id.is_some() && !self.source_file.is_empty() {
                        tracing::info!("检测到新 problem，保存测试点到源文件");

                        // 设置 problem 的 source_file 并保存
                        let store = self.problem_store.clone();
                        let source_file = self.source_file.clone();
                        let tests = self.cached_tests.clone();
                        tokio::spawn(async move {
                            let mut store_lock = store.lock().await;
                            if let Some(problem) = store_lock.get_current_problem_mut() {
                                if problem.source_file.is_none() {
                                    problem.source_file = Some(source_file.clone());
                                    let _ = store_lock.update_current_problem();
                                    tracing::info!(
                                        "已设置 problem 的 source_file: {}",
                                        source_file
                                    );
                                }
                            }
                            drop(store_lock);

                            // 保存测试点到源文件
                            let path = PathBuf::from(&source_file);
                            if let Err(e) = ProblemStore::save_tests_to_source_file(&path, &tests) {
                                tracing::error!("保存测试点到源文件失败: {}", e);
                            } else {
                                tracing::info!("✅ 已保存 {} 个测试点到源文件", tests.len());
                            }
                        });
                    }
                }
                AppMessage::TestsUpdated(tests) => {
                    self.cached_tests = tests;
                }
                AppMessage::SourceFileTestsUpdated(tests) => {
                    self.source_file_tests = tests;
                }
                AppMessage::RunCompleted => {
                    self.is_running = false;
                }
            }
        }
    }

    /// 运行所有测试
    fn run_all_tests(&mut self, ctx: egui::Context) {
        if self.is_running {
            return;
        }

        let source_file = self.source_file.clone();
        let source_path = PathBuf::from(&source_file);

        if !source_path.exists() {
            self.last_error = Some(format!("源文件不存在: {}", source_file));
            return;
        }

        let language = self.current_language;
        let store = self.problem_store.clone();
        let tx = self.tx.clone();

        // 优先使用源文件的测试点
        let use_source_file_tests = !self.source_file_tests.is_empty();
        let source_file_tests = if use_source_file_tests {
            Some(self.source_file_tests.clone())
        } else {
            None
        };

        self.is_running = true;
        self.last_error = None;

        // 在异步任务中运行测试
        tokio::spawn(async move {
            if use_source_file_tests {
                // 使用源文件测试
                if let Some(mut tests) = source_file_tests {
                    let time_limit = Duration::from_millis(2000); // 默认时间限制

                    // 重置所有测试状态
                    for test in tests.iter_mut() {
                        test.reset();
                    }

                    match Judge::new() {
                        Ok(judge) => {
                            // 逐个运行测试
                            for test in tests.iter_mut() {
                                if let Err(e) = judge
                                    .judge_test(&source_path, language, test, time_limit)
                                    .await
                                {
                                    tracing::error!("测试执行失败: {}", e);
                                    test.status = TestStatus::RuntimeError;
                                    test.error_message = Some(format!("执行错误: {}", e));
                                }

                                ctx.request_repaint();
                            }

                            // 每次测试完成后发送更新
                            let _ = tx.send(AppMessage::SourceFileTestsUpdated(tests.clone()));

                            // 保存更新后的测试到源文件
                            if let Err(e) =
                                ProblemStore::save_tests_to_source_file(&source_path, &tests)
                            {
                                tracing::error!("保存测试结果失败: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("初始化判断器失败: {}", e);
                        }
                    }
                }
            } else {
                // 使用problem的测试
                let mut store_lock = store.lock().await;

                if let Some(problem) = store_lock.get_current_problem_mut() {
                    let time_limit = Duration::from_millis(problem.time_limit);

                    // 重置所有测试状态
                    for test in problem.tests.iter_mut() {
                        test.reset();
                    }

                    match Judge::new() {
                        Ok(judge) => {
                            // 逐个运行测试
                            for test in problem.tests.iter_mut() {
                                if let Err(e) = judge
                                    .judge_test(&source_path, language, test, time_limit)
                                    .await
                                {
                                    tracing::error!("测试执行失败: {}", e);
                                    test.status = TestStatus::RuntimeError;
                                    test.error_message = Some(format!("执行错误: {}", e));
                                }

                                // 触发 UI 更新
                                ctx.request_repaint();
                            }

                            // 保存更新后的问题
                            let _ = store_lock.update_current_problem();

                            // 同时保存到源文件
                            if let Some(problem) = store_lock.get_current_problem() {
                                if let Err(e) = ProblemStore::save_tests_to_source_file(
                                    &source_path,
                                    &problem.tests,
                                ) {
                                    tracing::error!("保存测试结果到源文件失败: {}", e);
                                } else {
                                    tracing::info!("已保存测试结果到源文件");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("初始化判断器失败: {}", e);
                        }
                    }
                }
            }

            let _ = tx.send(AppMessage::RunCompleted);
            ctx.request_repaint();
        });
    }

    /// 运行单个测试
    fn run_single_test(&mut self, ctx: egui::Context, test_id: Uuid) {
        if self.is_running {
            return;
        }

        let source_file = self.source_file.clone();
        let source_path = PathBuf::from(&source_file);

        if !source_path.exists() {
            self.last_error = Some(format!("源文件不存在: {}", source_file));
            return;
        }

        let language = self.current_language;
        let store = self.problem_store.clone();
        let tx = self.tx.clone();

        // 优先使用源文件的测试点
        let use_source_file_tests = !self.source_file_tests.is_empty();
        let source_file_tests = if use_source_file_tests {
            Some(self.source_file_tests.clone())
        } else {
            None
        };

        self.is_running = true;
        self.last_error = None;

        tokio::spawn(async move {
            if use_source_file_tests {
                // 使用源文件测试
                if let Some(mut tests) = source_file_tests {
                    let time_limit = Duration::from_millis(2000);

                    if let Some(test) = tests.iter_mut().find(|t| t.id == test_id) {
                        test.reset();

                        match Judge::new() {
                            Ok(judge) => {
                                if let Err(e) = judge
                                    .judge_test(&source_path, language, test, time_limit)
                                    .await
                                {
                                    tracing::error!("测试执行失败: {}", e);
                                    test.status = TestStatus::RuntimeError;
                                    test.error_message = Some(format!("执行错误: {}", e));
                                }
                            }
                            Err(e) => {
                                tracing::error!("初始化判断器失败: {}", e);
                            }
                        }
                    }

                    // 保存测试结果
                    if let Err(e) = ProblemStore::save_tests_to_source_file(&source_path, &tests) {
                        tracing::error!("保存测试结果失败: {}", e);
                    } else {
                        tracing::info!("保存单个测试结果到源文件");
                    }

                    // 发送更新消息
                    let _ = tx.send(AppMessage::SourceFileTestsUpdated(tests.clone()));
                }
            } else {
                // 使用problem的测试
                let mut store_lock = store.lock().await;

                if let Some(problem) = store_lock.get_current_problem_mut() {
                    let time_limit = Duration::from_millis(problem.time_limit);

                    if let Some(test) = problem.tests.iter_mut().find(|t| t.id == test_id) {
                        test.reset();

                        match Judge::new() {
                            Ok(judge) => {
                                if let Err(e) = judge
                                    .judge_test(&source_path, language, test, time_limit)
                                    .await
                                {
                                    tracing::error!("测试执行失败: {}", e);
                                    test.status = TestStatus::RuntimeError;
                                    test.error_message = Some(format!("执行错误: {}", e));
                                }

                                let _ = store_lock.update_current_problem();

                                // 同时保存到源文件
                                if let Some(problem) = store_lock.get_current_problem() {
                                    if let Some(ref sf) = problem.source_file {
                                        let sf_path = PathBuf::from(sf);
                                        if let Err(e) = ProblemStore::save_tests_to_source_file(
                                            &sf_path,
                                            &problem.tests,
                                        ) {
                                            tracing::error!("保存单个测试结果到源文件失败: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("初始化判断器失败: {}", e);
                            }
                        }
                    }
                }
            }

            let _ = tx.send(AppMessage::RunCompleted);
            ctx.request_repaint();
        });
    }

    /// 检查源文件是否改变，如果改变则加载对应的测试点
    fn check_source_file_changed(&mut self) {
        if self.source_file != self.previous_source_file {
            self.previous_source_file = self.source_file.clone();

            // 加载新源文件的测试点
            if !self.source_file.is_empty() {
                let path = PathBuf::from(&self.source_file);
                self.source_file_tests =
                    ProblemStore::load_tests_from_source_file(&path).unwrap_or_default();
                tracing::info!(
                    "Loaded {} tests from source file",
                    self.source_file_tests.len()
                );
            } else {
                self.source_file_tests.clear();
            }
        }
    }

    /// 保存测试点到源文件
    fn save_tests_to_source_file(&self, tests: &[TestCase]) {
        if !self.source_file.is_empty() {
            tracing::info!(
                "==> 准备保存测试点: 源文件={}, 测试数量={}",
                self.source_file,
                tests.len()
            );
            let path = PathBuf::from(&self.source_file);
            if let Err(e) = ProblemStore::save_tests_to_source_file(&path, tests) {
                tracing::error!("❌ Failed to save tests to source file: {}", e);
            } else {
                tracing::info!("✅ Saved {} tests to source file", tests.len());
            }
        } else {
            tracing::warn!("⚠️ 源文件为空，无法保存测试点");
        }
    }

    /// 渲染主界面
    fn render_ui(&mut self, ctx: &egui::Context) {
        // 检查源文件是否改变
        self.check_source_file_changed();

        // 顶部工具栏
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(4.0);

            let mut run_all = false;
            let mut stop = false;
            let mut add_test = false;
            let mut clear_results = false;

            let has_problem = self.cached_current_id.is_some();

            Toolbar::ui(
                ui,
                &mut self.current_language,
                &mut self.source_file,
                &mut run_all,
                &mut stop,
                &mut add_test,
                &mut clear_results,
                has_problem,
                self.is_running,
            );

            if run_all {
                self.pending_run_all = true;
            }
            if stop {
                self.pending_stop = true;
            }
            if add_test {
                self.pending_add_test = true;
            }

            // 处理清除结果
            if clear_results {
                let store = self.problem_store.clone();
                tokio::spawn(async move {
                    let mut store_lock = store.lock().await;
                    if let Some(problem) = store_lock.get_current_problem_mut() {
                        for test in problem.tests.iter_mut() {
                            test.reset();
                        }
                        let _ = store_lock.update_current_problem();
                    }
                });
            }

            ui.add_space(4.0);
        });

        // 底部状态栏
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if let Some(error) = &self.last_error {
                    ui.label(
                        egui::RichText::new(format!("❌ {}", error))
                            .color(egui::Color32::from_rgb(255, 100, 100)),
                    );
                } else {
                    ui.label(egui::RichText::new("Ready").color(egui::Color32::GRAY));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("CPKit v0.1.0")
                            .size(10.0)
                            .color(egui::Color32::DARK_GRAY),
                    );
                });
            });
            ui.add_space(2.0);
        });

        // 中央测试面板
        egui::CentralPanel::default().show(ctx, |ui| {
            // 只要有源文件就允许显示和编辑测试点
            let has_source_file = !self.source_file.is_empty();
            let has_problem = self.cached_current_id.is_some();

            // 优先使用源文件的测试点，只有源文件没有测试点时才使用 problem 的测试点
            let use_source_file_tests = !self.source_file_tests.is_empty();
            let has_tests = use_source_file_tests || (has_problem && !self.cached_tests.is_empty());

            if (has_tests || self.pending_add_test) && has_source_file {
                let mut on_delete_test = None;
                let mut tests = if use_source_file_tests {
                    self.source_file_tests.clone()
                } else {
                    self.cached_tests.clone()
                };

                self.test_panel
                    .ui(ui, &mut tests, &mut on_delete_test, self.pending_add_test);

                self.pending_add_test = false;

                // 检查是否有测试需要运行
                if let Some(test_id) = self.test_panel.get_run_test_id() {
                    self.pending_run_test_id = Some(test_id);
                }

                // 检测测试点是否改变
                let tests_changed = if use_source_file_tests {
                    tests != self.source_file_tests
                } else {
                    tests != self.cached_tests
                };

                if tests_changed {
                    // 无论哪种模式，都立即保存到源文件
                    tracing::info!("检测到测试点变化，保存中...");
                    self.save_tests_to_source_file(&tests);

                    if use_source_file_tests {
                        self.source_file_tests = tests.clone();

                        // 如果有 problem，同时更新 problem 的测试点
                        if has_problem {
                            let store = self.problem_store.clone();
                            let tests_clone = tests.clone();
                            let source_file = self.source_file.clone();
                            tokio::spawn(async move {
                                let mut store_lock = store.lock().await;
                                if let Some(problem) = store_lock.get_current_problem_mut() {
                                    problem.tests = tests_clone.clone();
                                    let _ = store_lock.update_current_problem();

                                    // 同时保存到源文件
                                    if !source_file.is_empty() {
                                        let path = PathBuf::from(&source_file);
                                        let _ = ProblemStore::save_tests_to_source_file(
                                            &path,
                                            &tests_clone,
                                        );
                                    }
                                }
                            });
                        }
                    } else if has_problem {
                        // 同时保存到 problem
                        let store = self.problem_store.clone();
                        let tests_clone = tests.clone();
                        let source_file = self.source_file.clone();
                        tokio::spawn(async move {
                            let mut store_lock = store.lock().await;
                            if let Some(problem) = store_lock.get_current_problem_mut() {
                                problem.tests = tests_clone.clone();
                                let _ = store_lock.update_current_problem();

                                // 同时保存到源文件
                                if !source_file.is_empty() {
                                    let path = PathBuf::from(&source_file);
                                    let _ = ProblemStore::save_tests_to_source_file(
                                        &path,
                                        &tests_clone,
                                    );
                                }
                            }
                        });
                        self.cached_tests = tests;
                    }
                }

                // 处理删除测试
                if let Some(test_id) = on_delete_test {
                    if use_source_file_tests {
                        // 从源文件测试中删除
                        self.source_file_tests.retain(|t| t.id != test_id);
                        self.save_tests_to_source_file(&self.source_file_tests);

                        // 如果有 problem，同时从 problem 中删除
                        if has_problem {
                            let store = self.problem_store.clone();
                            let source_file = self.source_file.clone();
                            let tests_clone = self.source_file_tests.clone();
                            tokio::spawn(async move {
                                let mut store_lock = store.lock().await;
                                if let Some(problem) = store_lock.get_current_problem_mut() {
                                    problem.tests.retain(|t| t.id != test_id);
                                    let _ = store_lock.update_current_problem();

                                    // 同时保存到源文件
                                    if !source_file.is_empty() {
                                        let path = PathBuf::from(&source_file);
                                        let _ = ProblemStore::save_tests_to_source_file(
                                            &path,
                                            &tests_clone,
                                        );
                                    }
                                }
                            });
                        }
                    } else if has_problem {
                        // 从 problem 和源文件中删除
                        self.cached_tests.retain(|t| t.id != test_id);
                        self.save_tests_to_source_file(&self.cached_tests);

                        let store = self.problem_store.clone();
                        let source_file = self.source_file.clone();
                        let tests_clone = self.cached_tests.clone();
                        tokio::spawn(async move {
                            let mut store_lock = store.lock().await;
                            if let Some(problem) = store_lock.get_current_problem_mut() {
                                problem.tests.retain(|t| t.id != test_id);
                                let _ = store_lock.update_current_problem();

                                // 同时保存到源文件
                                if !source_file.is_empty() {
                                    let path = PathBuf::from(&source_file);
                                    let _ = ProblemStore::save_tests_to_source_file(
                                        &path,
                                        &tests_clone,
                                    );
                                }
                            }
                        });
                    }
                }
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("Welcome to CPKit");
                    ui.add_space(20.0);
                    if has_source_file {
                        ui.label("Click '➕ Add Test' to add custom test cases");
                    } else {
                        ui.label("Select a source file to start adding test cases");
                        ui.add_space(10.0);
                        ui.label(
                            "Or use Competitive Companion browser extension to import problems",
                        );
                    }
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("Listening on port: 10043")
                            .monospace()
                            .color(egui::Color32::GRAY),
                    );
                });
            }
        });
    }
}

impl eframe::App for CPKitApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 处理消息
        self.process_messages();

        // 处理待处理的运行请求
        if self.pending_run_all {
            self.pending_run_all = false;
            self.run_all_tests(ctx.clone());
        }

        // 处理从test_panel触发的单个测试运行
        if let Some(test_id) = self.pending_run_test_id.take() {
            self.run_single_test(ctx.clone(), test_id);
        }

        if self.pending_stop {
            self.pending_stop = false;
            self.is_running = false;
        }

        // 渲染 UI
        self.render_ui(ctx);

        // 如果正在运行，请求持续重绘
        if self.is_running {
            ctx.request_repaint();
        }

        self.frame_count += 1;
    }
}
