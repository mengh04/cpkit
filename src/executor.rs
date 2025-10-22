use crate::models::ExecutionResult;
use anyhow::{Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// 代码执行器（仅支持 C++）
pub struct Executor;

impl Executor {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// 获取可执行文件名（跨平台）
    fn get_executable_name() -> &'static str {
        #[cfg(windows)]
        {
            "a.exe"
        }
        #[cfg(not(windows))]
        {
            "a.out"
        }
    }

    /// 编译 C++ 代码
    pub fn compile(
        &self,
        source_file: &Path,
        stop_signal: Option<Arc<AtomicBool>>,
    ) -> Result<PathBuf> {
        let output_file = PathBuf::from(Self::get_executable_name());

        // 尝试查找编译器
        let compiler = self.find_cpp_compiler()?;

        let mut cmd = Command::new(compiler);
        cmd.args(&[
            source_file.to_str().unwrap(),
            "-o",
            Self::get_executable_name(),
            "-O2",
            "-std=c++20",
            "-Wall",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

        // 在 Windows 上隐藏控制台窗口
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd.spawn().context("Cannot execute C++ compiler")?;

        // 轮询检查编译进程状态和停止信号
        let check_interval = Duration::from_millis(50);
        loop {
            // 检查停止信号
            if let Some(ref signal) = stop_signal {
                if signal.load(Ordering::Relaxed) {
                    tracing::info!("收到停止信号，终止编译进程");
                    let _ = child.kill();
                    let _ = child.wait();
                    anyhow::bail!("编译被用户中断");
                }
            }

            // 检查编译进程是否结束
            match child.try_wait()? {
                Some(status) => {
                    if !status.success() {
                        anyhow::bail!("C++ compilation failed");
                    }
                    break;
                }
                None => {
                    std::thread::sleep(check_interval);
                }
            }
        }

        Ok(output_file)
    }

    /// 执行程序
    pub fn execute(
        &self,
        executable: &Path,
        input: &str,
        stop_signal: Option<Arc<AtomicBool>>,
    ) -> Result<ExecutionResult> {
        let start = Instant::now();

        // 在 Windows 上使用绝对路径
        let exe_path = std::env::current_dir()?.join(executable);

        let mut cmd = Command::new(&exe_path);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // 在 Windows 上隐藏控制台窗口
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd
            .spawn()
            .context(format!("Cannot start program: {:?}", exe_path))?;

        // 写入输入
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes())?;
        }

        // 使用轮询方式检查停止信号
        let check_interval = Duration::from_millis(50);
        loop {
            // 检查停止信号
            if let Some(ref signal) = stop_signal {
                if signal.load(Ordering::Relaxed) {
                    tracing::info!("收到停止信号，强制终止进程");
                    let _ = child.kill();
                    let _ = child.wait();
                    return Ok(ExecutionResult {
                        output: String::new(),
                        exit_code: -1,
                        execution_time: start.elapsed(),
                        memory_used: None,
                        error: Some("被用户中断".to_string()),
                    });
                }
            }

            // 尝试非阻塞地检查进程是否结束
            match child.try_wait()? {
                Some(status) => {
                    // 进程已结束，收集输出
                    let output = child.wait_with_output()?;
                    let execution_time = start.elapsed();

                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                    let error = if !stderr.is_empty() {
                        Some(stderr)
                    } else {
                        None
                    };

                    return Ok(ExecutionResult {
                        output: stdout,
                        exit_code: status.code().unwrap_or(-1),
                        execution_time,
                        memory_used: None,
                        error,
                    });
                }
                None => {
                    // 进程还在运行，等待一小段时间后再检查
                    std::thread::sleep(check_interval);
                }
            }
        }
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

    /// 清理编译产物
    pub fn cleanup(&self) {
        let exe_path = PathBuf::from(Self::get_executable_name());
        if exe_path.exists() {
            let _ = std::fs::remove_file(&exe_path);
            tracing::info!("已删除临时文件 {}", exe_path.display());
        }
    }
}
