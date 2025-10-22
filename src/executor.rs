use crate::models::{ExecutionResult, Language};
use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// 代码执行器
pub struct Executor {
    #[allow(dead_code)]
    work_dir: std::path::PathBuf,
}

impl Executor {
    pub fn new() -> Result<Self> {
        let work_dir = std::env::current_dir()?;
        Ok(Self { work_dir })
    }

    /// 编译源代码
    pub fn compile(&self, source_file: &Path, language: Language) -> Result<std::path::PathBuf> {
        match language {
            Language::Cpp => self.compile_cpp(source_file),
            Language::Rust => self.compile_rust(source_file),
            Language::C => self.compile_c(source_file),
            Language::Java => self.compile_java(source_file),
            Language::Python => Ok(source_file.to_path_buf()), // Python 不需要编译
        }
    }

    /// 编译 C++ 代码
    fn compile_cpp(&self, source_file: &Path) -> Result<std::path::PathBuf> {
        let output_file = source_file.with_extension("exe");

        // 尝试查找编译器
        let compiler = self.find_cpp_compiler()?;

        let status = Command::new(compiler)
            .args(&[
                source_file.to_str().unwrap(),
                "-o",
                output_file.to_str().unwrap(),
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

    /// 编译 C 代码
    fn compile_c(&self, source_file: &Path) -> Result<std::path::PathBuf> {
        let output_file = source_file.with_extension("exe");

        let compiler = self.find_c_compiler()?;

        let status = Command::new(compiler)
            .args(&[
                source_file.to_str().unwrap(),
                "-o",
                output_file.to_str().unwrap(),
                "-O2",
                "-std=c11",
                "-Wall",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .context("Cannot execute C compiler")?;

        if !status.success() {
            anyhow::bail!("C compilation failed");
        }

        Ok(output_file)
    }

    /// 编译 Rust 代码
    fn compile_rust(&self, source_file: &Path) -> Result<std::path::PathBuf> {
        let output_file = source_file.with_extension("exe");

        let status = Command::new("rustc")
            .args(&[
                source_file.to_str().unwrap(),
                "-o",
                output_file.to_str().unwrap(),
                "-O",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .context("Cannot execute Rust compiler")?;

        if !status.success() {
            anyhow::bail!("Rust compilation failed");
        }

        Ok(output_file)
    }

    /// 编译 Java 代码
    fn compile_java(&self, source_file: &Path) -> Result<std::path::PathBuf> {
        let status = Command::new("javac")
            .arg(source_file.to_str().unwrap())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .context("Cannot execute Java compiler")?;

        if !status.success() {
            anyhow::bail!("Java compilation failed");
        }

        // Java 返回 class 文件路径
        Ok(source_file.with_extension("class"))
    }

    /// 执行程序
    pub fn execute(
        &self,
        executable: &Path,
        input: &str,
        language: Language,
        time_limit: Duration,
    ) -> Result<ExecutionResult> {
        let start = Instant::now();

        let mut child = match language {
            Language::Python => Command::new("python")
                .arg(executable)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .context("Cannot start Python interpreter")?,
            Language::Java => {
                let class_name = executable
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .context("Invalid Java class name")?;

                let dir = executable.parent().context("Cannot get directory")?;

                Command::new("java")
                    .current_dir(dir)
                    .arg(class_name)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .context("Cannot start Java Virtual Machine")?
            }
            _ => Command::new(executable)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .context("Cannot start program")?,
        };

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

    /// Find C compiler
    fn find_c_compiler(&self) -> Result<String> {
        // Try common C compilers
        for compiler in &["gcc", "clang", "cl"] {
            if which::which(compiler).is_ok() {
                return Ok(compiler.to_string());
            }
        }
        anyhow::bail!("C compiler not found (gcc, clang, cl)")
    }

    /// Clean up build artifacts
    pub fn cleanup(&self, files: &[std::path::PathBuf]) {
        for file in files {
            if file.exists() {
                let _ = std::fs::remove_file(file);
            }
        }
    }
}
