use crate::competitive_companion::SharedProblemStore;
use crate::judge::Judge;
use crate::models::{TestCase, TestStatus};

use crate::ui::{TestPanel, Toolbar};
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;
use uuid::Uuid;

/// 应用消息
#[derive(Debug, Clone)]
enum AppMessage {
    ProblemsUpdated(Vec<ProblemData>),
    CurrentProblemChanged(Option<Uuid>, Vec<TestCase>),
    RunCompleted,
}

/// 问题数据（可跨线程传递）
#[derive(Debug, Clone)]
struct ProblemData {
    id: Uuid,
    name: String,
}

/// CPKit 主应用
pub struct CPKitApp {
    problem_store: SharedProblemStore,
    test_panel: TestPanel,

    // UI 状态
    source_file: String,
    is_running: bool,

    // 缓存数据
    cached_problems: Vec<ProblemData>,
    cached_current_id: Option<Uuid>,
    cached_tests: Vec<TestCase>,

    // 运行时状态
    last_error: Option<String>,

    // 事件标志
    pending_run_all: bool,
    pending_run_test_id: Option<Uuid>,
    pending_stop: bool,
    pending_add_test: bool,
    pending_open_file: bool,

    // 消息通道
    tx: Sender<AppMessage>,
    rx: Receiver<AppMessage>,

    // 刷新标志
    frame_count: u64,
}

impl CPKitApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, problem_store: SharedProblemStore) -> Self {
        Self::setup_custom_fonts(&_cc.egui_ctx);
        Self::setup_custom_style(&_cc.egui_ctx);

        let (tx, rx) = channel();

        let app = Self {
            problem_store: problem_store.clone(),
            test_panel: TestPanel::new(),
            source_file: String::new(),
            is_running: false,
            cached_problems: Vec::new(),
            cached_current_id: None,
            cached_tests: Vec::new(),
            last_error: None,
            pending_run_all: false,
            pending_run_test_id: None,
            pending_stop: false,
            pending_add_test: false,
            pending_open_file: false,
            tx: tx.clone(),
            rx,
            frame_count: 0,
        };

        // 启动后台任务定期同步数据
        let store = problem_store.clone();
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;

                let store_lock = store.lock().await;

                // 获取问题列表
                let problems: Vec<ProblemData> = store_lock
                    .get_all_problems()
                    .iter()
                    .map(|p| ProblemData {
                        id: p.id,
                        name: p.name.clone(),
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
        let mut fonts = egui::FontDefinitions::default();

        // 添加中文字体
        // 方法1: 使用系统自带的中文字体（推荐）
        #[cfg(target_os = "windows")]
        {
            // Windows 系统字体路径
            if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
                fonts.font_data.insert(
                    "microsoft_yahei".to_owned(),
                    egui::FontData::from_owned(font_data),
                );
                // 将微软雅黑设置为默认字体的第一优先级
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "microsoft_yahei".to_owned());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "microsoft_yahei".to_owned());
            }
        }

        #[cfg(target_os = "macos")]
        {
            // macOS 系统字体路径
            if let Ok(font_data) = std::fs::read("/System/Library/Fonts/PingFang.ttc") {
                fonts
                    .font_data
                    .insert("pingfang".to_owned(), egui::FontData::from_owned(font_data));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "pingfang".to_owned());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "pingfang".to_owned());
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Linux 系统字体路径（以 Noto Sans CJK 为例）
            if let Ok(font_data) =
                std::fs::read("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc")
            {
                fonts.font_data.insert(
                    "noto_sans_cjk".to_owned(),
                    egui::FontData::from_owned(font_data),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "noto_sans_cjk".to_owned());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "noto_sans_cjk".to_owned());
            }
        }

        // 方法2: 使用项目内嵌的字体文件
        // 取消下面注释并将字体文件放到 fonts 目录
        /*
        if let Ok(font_data) = std::fs::read("fonts/SourceHanSansSC-Regular.otf") {
            fonts.font_data.insert(
                "source_han_sans".to_owned(),
                egui::FontData::from_owned(font_data),
            );
            fonts.families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "source_han_sans".to_owned());
            fonts.families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "source_han_sans".to_owned());
        }
        */

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
    fn process_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AppMessage::ProblemsUpdated(problems) => {
                    self.cached_problems = problems;
                }
                AppMessage::CurrentProblemChanged(id, tests) => {
                    let problem_changed = id != self.cached_current_id;
                    self.cached_current_id = id;
                    self.cached_tests = tests;

                    // 当接收到新题目时，自动设置source_file为题目名.cpp并打开文件
                    if problem_changed && id.is_some() {
                        if let Some(problem) =
                            self.cached_problems.iter().find(|p| Some(p.id) == id)
                        {
                            // 生成安全的文件名（移除特殊字符）
                            let safe_name = problem
                                .name
                                .chars()
                                .map(|c| match c {
                                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                                    _ => c,
                                })
                                .collect::<String>();
                            self.source_file = format!("{}.cpp", safe_name);
                            tracing::info!("新题目接收，自动设置源文件为: {}", self.source_file);

                            // 自动创建并打开文件
                            self.pending_open_file = true;
                        }
                    }

                    // 立即请求重绘
                    ctx.request_repaint();
                }
                AppMessage::RunCompleted => {
                    self.is_running = false;
                    ctx.request_repaint();
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

        let store = self.problem_store.clone();
        let tx = self.tx.clone();

        self.is_running = true;
        self.last_error = None;

        tokio::spawn(async move {
            let time_limit;
            let problem_id;
            let test_count;

            {
                let mut store_lock = store.lock().await;
                if let Some(problem) = store_lock.get_current_problem_mut() {
                    time_limit = Duration::from_millis(problem.time_limit);
                    problem_id = problem.id;
                    test_count = problem.tests.len();

                    // 重置所有测试状态
                    for test in problem.tests.iter_mut() {
                        test.reset();
                    }
                } else {
                    let _ = tx.send(AppMessage::RunCompleted);
                    return;
                }
            }

            match Judge::new() {
                Ok(judge) => {
                    // 逐个运行测试
                    for i in 0..test_count {
                        {
                            let mut store_lock = store.lock().await;
                            if let Some(problem) = store_lock.get_current_problem_mut() {
                                if let Err(e) = judge
                                    .judge_test(&source_path, &mut problem.tests[i], time_limit)
                                    .await
                                {
                                    tracing::error!("测试执行失败: {}", e);
                                    problem.tests[i].status = TestStatus::RuntimeError;
                                    problem.tests[i].error_message =
                                        Some(format!("执行错误: {}", e));
                                }

                                // 克隆测试数据
                                let tests_clone = problem.tests.clone();

                                // 保存更新
                                let _ = store_lock.update_current_problem();

                                // 发送更新消息
                                let _ = tx.send(AppMessage::CurrentProblemChanged(
                                    Some(problem_id),
                                    tests_clone,
                                ));
                            }
                        }
                        ctx.request_repaint();
                    }
                }
                Err(e) => {
                    tracing::error!("初始化判断器失败: {}", e);
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

        let store = self.problem_store.clone();
        let tx = self.tx.clone();

        self.is_running = true;
        self.last_error = None;

        tokio::spawn(async move {
            let time_limit;
            let problem_id;
            let test_index;

            {
                let mut store_lock = store.lock().await;
                if let Some(problem) = store_lock.get_current_problem_mut() {
                    time_limit = Duration::from_millis(problem.time_limit);
                    problem_id = problem.id;

                    if let Some(idx) = problem.tests.iter().position(|t| t.id == test_id) {
                        test_index = idx;
                        problem.tests[test_index].reset();
                    } else {
                        let _ = tx.send(AppMessage::RunCompleted);
                        return;
                    }
                } else {
                    let _ = tx.send(AppMessage::RunCompleted);
                    return;
                }
            }

            match Judge::new() {
                Ok(judge) => {
                    let mut store_lock = store.lock().await;
                    if let Some(problem) = store_lock.get_current_problem_mut() {
                        if let Err(e) = judge
                            .judge_test(&source_path, &mut problem.tests[test_index], time_limit)
                            .await
                        {
                            tracing::error!("测试执行失败: {}", e);
                            problem.tests[test_index].status = TestStatus::RuntimeError;
                            problem.tests[test_index].error_message =
                                Some(format!("执行错误: {}", e));
                        }

                        // 克隆测试数据
                        let tests_clone = problem.tests.clone();

                        let _ = store_lock.update_current_problem();

                        // 测试完成后立即发送更新消息
                        let _ = tx.send(AppMessage::CurrentProblemChanged(
                            Some(problem_id),
                            tests_clone,
                        ));
                    }
                }
                Err(e) => {
                    tracing::error!("初始化判断器失败: {}", e);
                }
            }

            let _ = tx.send(AppMessage::RunCompleted);
            ctx.request_repaint();
        });
    }

    /// 创建并打开源文件
    fn create_and_open_file(&mut self) {
        if self.source_file.is_empty() {
            self.last_error = Some("没有设置源文件名".to_string());
            return;
        }

        let source_path = PathBuf::from(&self.source_file);

        // 如果文件不存在，创建空文件
        if !source_path.exists() {
            match std::fs::write(&source_path, "") {
                Ok(_) => {
                    tracing::info!("创建新文件: {}", self.source_file);
                }
                Err(e) => {
                    self.last_error = Some(format!("创建文件失败: {}", e));
                    return;
                }
            }
        }

        // 使用 zed 编辑器打开文件
        match std::process::Command::new("zed").arg(&source_path).spawn() {
            Ok(_) => {
                tracing::info!("使用 zed 打开文件: {}", self.source_file);
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(format!(
                    "打开编辑器失败: {}。请确保 zed 已安装并在 PATH 中",
                    e
                ));
            }
        }
    }

    /// 渲染主界面
    fn render_ui(&mut self, ctx: &egui::Context) {
        // 顶部工具栏
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(4.0);

            let mut run_all = false;
            let mut stop = false;
            let mut add_test = false;
            let mut clear_results = false;
            let mut open_file = false;

            let has_problem = self.cached_current_id.is_some();

            Toolbar::ui(
                ui,
                &mut self.source_file,
                &mut run_all,
                &mut stop,
                &mut add_test,
                &mut clear_results,
                &mut open_file,
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
            if open_file {
                self.pending_open_file = true;
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
            let has_problem = self.cached_current_id.is_some();
            let has_tests = has_problem && !self.cached_tests.is_empty();

            if has_tests || (has_problem && self.pending_add_test) {
                let mut on_delete_test = None;
                let mut tests = self.cached_tests.clone();

                self.test_panel
                    .ui(ui, &mut tests, &mut on_delete_test, self.pending_add_test);

                self.pending_add_test = false;

                // 检查是否有测试需要运行
                if let Some(test_id) = self.test_panel.get_run_test_id() {
                    self.pending_run_test_id = Some(test_id);
                }

                // 检测测试点是否改变
                if tests != self.cached_tests {
                    self.cached_tests = tests.clone();

                    let store = self.problem_store.clone();
                    tokio::spawn(async move {
                        let mut store_lock = store.lock().await;
                        if let Some(problem) = store_lock.get_current_problem_mut() {
                            problem.tests = tests;
                            let _ = store_lock.update_current_problem();
                        }
                    });
                }

                // 处理删除测试
                if let Some(test_id) = on_delete_test {
                    self.cached_tests.retain(|t| t.id != test_id);

                    let store = self.problem_store.clone();
                    let tests_clone = self.cached_tests.clone();
                    tokio::spawn(async move {
                        let mut store_lock = store.lock().await;
                        if let Some(problem) = store_lock.get_current_problem_mut() {
                            problem.tests = tests_clone;
                            let _ = store_lock.update_current_problem();
                        }
                    });
                }
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("Welcome to CPKit");
                    ui.add_space(20.0);
                    if has_problem {
                        ui.label("Click '➕ Add Test' to add custom test cases");
                    } else {
                        ui.label("Use Competitive Companion browser extension to import problems");
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
        self.process_messages(ctx);

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

        // 处理打开文件请求
        if self.pending_open_file {
            self.pending_open_file = false;
            self.create_and_open_file();
        }

        // 渲染 UI
        self.render_ui(ctx);

        // 如果正在运行，请求持续重绘
        if self.is_running {
            ctx.request_repaint();
        }

        // 定期请求重绘以接收新数据（如 Competitive Companion 的题目）
        ctx.request_repaint_after(Duration::from_millis(100));

        self.frame_count += 1;
    }
}
