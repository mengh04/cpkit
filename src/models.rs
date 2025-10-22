use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// 问题信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub id: Uuid,
    pub name: String,
    pub group: String,
    pub url: String,
    pub interactive: bool,
    pub memory_limit: u64, // MB
    pub time_limit: u64,   // ms
    pub tests: Vec<TestCase>,
    pub source_file: Option<String>,
    pub language: Language,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
}

impl Problem {
    pub fn new(name: String, group: String, url: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            group,
            url,
            interactive: false,
            memory_limit: 256,
            time_limit: 2000,
            tests: Vec::new(),
            source_file: None,
            language: Language::Cpp,
            created_at: Utc::now(),
            last_run: None,
        }
    }

    pub fn add_test(&mut self, input: String, output: String) {
        self.tests.push(TestCase::new(input, output));
    }
}

/// 测试用例
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestCase {
    pub id: Uuid,
    pub input: String,
    pub expected_output: String,
    pub actual_output: Option<String>,
    pub status: TestStatus,
    pub execution_time: Option<Duration>,
    pub memory_used: Option<u64>,
    pub error_message: Option<String>,
}

impl TestCase {
    pub fn new(input: String, expected_output: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            input,
            expected_output,
            actual_output: None,
            status: TestStatus::Pending,
            execution_time: None,
            memory_used: None,
            error_message: None,
        }
    }

    pub fn reset(&mut self) {
        self.actual_output = None;
        self.status = TestStatus::Pending;
        self.execution_time = None;
        self.memory_used = None;
        self.error_message = None;
    }
}

/// 测试状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Pending,
    Running,
    Accepted,
    WrongAnswer,
    RuntimeError,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    CompilationError,
}

impl TestStatus {
    pub fn icon(&self) -> &str {
        match self {
            TestStatus::Pending => "⏳",
            TestStatus::Running => "▶",
            TestStatus::Accepted => "✓",
            TestStatus::WrongAnswer => "✗",
            TestStatus::RuntimeError => "⚠",
            TestStatus::TimeLimitExceeded => "⏱",
            TestStatus::MemoryLimitExceeded => "💾",
            TestStatus::CompilationError => "🔨",
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            TestStatus::Pending => egui::Color32::GRAY,
            TestStatus::Running => egui::Color32::LIGHT_BLUE,
            TestStatus::Accepted => egui::Color32::GREEN,
            TestStatus::WrongAnswer => egui::Color32::RED,
            TestStatus::RuntimeError => egui::Color32::from_rgb(255, 165, 0),
            TestStatus::TimeLimitExceeded => egui::Color32::YELLOW,
            TestStatus::MemoryLimitExceeded => egui::Color32::GOLD,
            TestStatus::CompilationError => egui::Color32::DARK_RED,
        }
    }

    pub fn text(&self) -> &str {
        match self {
            TestStatus::Pending => "Pending",
            TestStatus::Running => "Running",
            TestStatus::Accepted => "Accepted",
            TestStatus::WrongAnswer => "Wrong Answer",
            TestStatus::RuntimeError => "Runtime Error",
            TestStatus::TimeLimitExceeded => "Time Limit Exceeded",
            TestStatus::MemoryLimitExceeded => "Memory Limit Exceeded",
            TestStatus::CompilationError => "Compilation Error",
        }
    }
}

/// 编程语言
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    Cpp,
    Rust,
    Python,
    Java,
    C,
}

impl Language {
    #[allow(dead_code)]
    pub fn file_extension(&self) -> &str {
        match self {
            Language::Cpp => "cpp",
            Language::Rust => "rs",
            Language::Python => "py",
            Language::Java => "java",
            Language::C => "c",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Language::Cpp => "C++",
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::Java => "Java",
            Language::C => "C",
        }
    }

    pub fn all() -> &'static [Language] {
        &[
            Language::Cpp,
            Language::Rust,
            Language::Python,
            Language::Java,
            Language::C,
        ]
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Competitive Companion 发送的数据格式
#[derive(Debug, Clone, Deserialize)]
pub struct CompetitiveCompanionData {
    pub name: String,
    pub group: String,
    pub url: String,
    pub interactive: bool,
    #[serde(rename = "memoryLimit")]
    pub memory_limit: u64,
    #[serde(rename = "timeLimit")]
    pub time_limit: u64,
    pub tests: Vec<CompetitiveCompanionTest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompetitiveCompanionTest {
    pub input: String,
    pub output: String,
}

impl From<CompetitiveCompanionData> for Problem {
    fn from(data: CompetitiveCompanionData) -> Self {
        let mut problem = Problem::new(data.name, data.group, data.url);
        problem.interactive = data.interactive;
        problem.memory_limit = data.memory_limit;
        problem.time_limit = data.time_limit;

        for test in data.tests {
            problem.add_test(test.input, test.output);
        }

        problem
    }
}

/// 执行结果
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub output: String,
    pub exit_code: i32,
    pub execution_time: Duration,
    pub memory_used: Option<u64>,
    pub error: Option<String>,
}

impl ExecutionResult {
    #[allow(dead_code)]
    pub fn is_success(&self) -> bool {
        self.exit_code == 0 && self.error.is_none()
    }
}
