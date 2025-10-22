use crate::models::ExecutionResult;
use anyhow::{Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// 代码执行器（仅支持 C++）
pub struct Executor;

impl Executor {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// 编译 C++ 代码，输出为 a.exe
    pub fn compile(&self, source_file: &Path) -> Result<PathBuf> {
        let output_file = PathBuf::from("a.exe");

        // 尝试查找编译器
        let compiler = self.find_cpp_compiler()?;

        let status = Command::new(compiler)
            .args(&[
                source_file.to_str().unwrap(),
                "-o",
                "a.exe",
                "-O2",
                "-std=c++17",
                "-Wall",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .context("Cannot execute C++ compiler")?;

        if !status.success() {
            anyhow::bail!("C++ compilation failed");
        }

        Ok(output_file)
    }

    /// 执行程序
    pub fn execute(
        &self,
        executable: &Path,
        input: &str,
        time_limit: Duration,
    ) -> Result<ExecutionResult> {
        let start = Instant::now();

        // 在 Windows 上使用绝对路径
        let exe_path = std::env::current_dir()?.join(executable);

        let mut child = Command::new(&exe_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context(format!("Cannot start program: {:?}", exe_path))?;

        // 写入输入
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes())?;
        }

        // 等待程序结束或超时
        let output = std::thread::spawn(move || child.wait_with_output())
            .join()
            .map_err(|_| anyhow::anyhow!("Execution thread crashed"))??;

        let execution_time = start.elapsed();

        // Check for timeout
        if execution_time > time_limit {
            return Ok(ExecutionResult {
                output: String::new(),
                exit_code: -1,
                execution_time,
                memory_used: None,
                error: Some("Timeout".to_string()),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let error = if !stderr.is_empty() {
            Some(stderr)
        } else {
            None
        };

        Ok(ExecutionResult {
            output: stdout,
            exit_code: output.status.code().unwrap_or(-1),
            execution_time,
            memory_used: None,
            error,
        })
    }

    /// Find C++ compiler
    fn find_cpp_compiler(&self) -> Result<String> {
        // Try common C++ compilers
        for compiler in &["g++", "clang++", "cl"] {
            if which::which(compiler).is_ok() {
                return Ok(compiler.to_string());
            }
        }
        anyhow::bail!("C++ compiler not found (g++, clang++, cl)")
    }

    /// 清理 a.exe
    pub fn cleanup(&self) {
        let exe_path = PathBuf::from("a.exe");
        if exe_path.exists() {
            let _ = std::fs::remove_file(exe_path);
            tracing::info!("已删除临时文件 a.exe");
        }
    }
}
