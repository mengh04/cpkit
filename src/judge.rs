use crate::executor::Executor;
use crate::models::{ExecutionResult, Language, TestCase, TestStatus};
use anyhow::Result;
use std::path::Path;
use std::time::Duration;

/// 测试判断器
pub struct Judge {
    executor: Executor,
}

impl Judge {
    pub fn new() -> Result<Self> {
        Ok(Self {
            executor: Executor::new()?,
        })
    }

    /// 判断单个测试用例
    pub async fn judge_test(
        &self,
        source_file: &Path,
        language: Language,
        test: &mut TestCase,
        time_limit: Duration,
    ) -> Result<()> {
        test.status = TestStatus::Running;

        // 编译代码
        let executable = match self.executor.compile(source_file, language) {
            Ok(exe) => exe,
            Err(e) => {
                test.status = TestStatus::CompilationError;
                test.error_message = Some(format!("Compilation failed: {}", e));
                return Ok(());
            }
        };

        // 执行代码
        let result = self
            .executor
            .execute(&executable, &test.input, language, time_limit)?;

        // 更新测试结果
        self.update_test_from_result(test, result);

        // 清理编译产物
        if language != Language::Python {
            self.executor.cleanup(&[executable]);
        }

        Ok(())
    }

    /// 根据执行结果更新测试用例状态
    fn update_test_from_result(&self, test: &mut TestCase, result: ExecutionResult) {
        test.execution_time = Some(result.execution_time);
        test.memory_used = result.memory_used;
        test.actual_output = Some(result.output.clone());

        if let Some(error) = result.error {
            if error.contains("Timeout") {
                test.status = TestStatus::TimeLimitExceeded;
            } else {
                test.status = TestStatus::RuntimeError;
                test.error_message = Some(error);
            }
            return;
        }

        if result.exit_code != 0 {
            test.status = TestStatus::RuntimeError;
            test.error_message = Some(format!(
                "Program exited abnormally, exit code: {}",
                result.exit_code
            ));
            return;
        }

        // 比较输出
        if self.compare_output(&test.expected_output, &result.output) {
            test.status = TestStatus::Accepted;
        } else {
            test.status = TestStatus::WrongAnswer;
        }
    }

    /// 比较输出是否匹配
    fn compare_output(&self, expected: &str, actual: &str) -> bool {
        // 规范化输出（去除首尾空白，统一行尾）
        let expected_normalized = self.normalize_output(expected);
        let actual_normalized = self.normalize_output(actual);

        expected_normalized == actual_normalized
    }

    /// 规范化输出
    fn normalize_output(&self, output: &str) -> String {
        output
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }

    /// 判断所有测试用例
    #[allow(dead_code)]
    pub async fn judge_all_tests(
        &self,
        source_file: &Path,
        language: Language,
        tests: &mut [TestCase],
        time_limit: Duration,
    ) -> Result<JudgeStatistics> {
        let mut stats = JudgeStatistics::default();
        stats.total = tests.len();

        for test in tests.iter_mut() {
            self.judge_test(source_file, language, test, time_limit)
                .await?;

            match test.status {
                TestStatus::Accepted => stats.passed += 1,
                TestStatus::WrongAnswer => stats.wrong_answer += 1,
                TestStatus::RuntimeError => stats.runtime_error += 1,
                TestStatus::TimeLimitExceeded => stats.time_limit_exceeded += 1,
                TestStatus::MemoryLimitExceeded => stats.memory_limit_exceeded += 1,
                TestStatus::CompilationError => stats.compilation_error += 1,
                _ => {}
            }
        }

        Ok(stats)
    }
}

/// 判断统计信息
#[derive(Debug, Default, Clone)]
#[allow(dead_code)]
pub struct JudgeStatistics {
    pub total: usize,
    pub passed: usize,
    pub wrong_answer: usize,
    pub runtime_error: usize,
    pub time_limit_exceeded: usize,
    pub memory_limit_exceeded: usize,
    pub compilation_error: usize,
}

impl JudgeStatistics {
    #[allow(dead_code)]
    pub fn all_passed(&self) -> bool {
        self.passed == self.total && self.total > 0
    }

    #[allow(dead_code)]
    pub fn success_rate(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.passed as f32 / self.total as f32) * 100.0
        }
    }
}
