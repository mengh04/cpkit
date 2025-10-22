use crate::models::Problem;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// 问题存储管理器
pub struct ProblemStore {
    problems: HashMap<Uuid, Problem>,
    current_problem: Option<Uuid>,
    data_dir: PathBuf,
}

impl ProblemStore {
    /// 创建新的存储管理器
    pub fn new() -> Result<Self> {
        let data_dir = Self::get_data_dir()?;
        fs::create_dir_all(&data_dir)?;

        let mut store = Self {
            problems: HashMap::new(),
            current_problem: None,
            data_dir,
        };

        // 从磁盘加载已保存的问题
        store.load_problems()?;

        Ok(store)
    }

    /// 获取数据存储目录
    fn get_data_dir() -> Result<PathBuf> {
        let mut path = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Unable to get data directory"))?;
        path.push("cpkit-egui");
        path.push("problems");
        Ok(path)
    }

    /// 添加新问题
    pub fn add_problem(&mut self, problem: Problem) -> Result<()> {
        let id = problem.id;
        self.save_problem(&problem)?;
        self.problems.insert(id, problem);
        self.current_problem = Some(id);
        Ok(())
    }

    /// 获取当前问题
    pub fn get_current_problem(&self) -> Option<&Problem> {
        self.current_problem.and_then(|id| self.problems.get(&id))
    }

    /// 获取当前问题（可变）
    pub fn get_current_problem_mut(&mut self) -> Option<&mut Problem> {
        self.current_problem
            .and_then(|id| self.problems.get_mut(&id))
    }

    /// 获取所有问题
    pub fn get_all_problems(&self) -> Vec<&Problem> {
        let mut problems: Vec<_> = self.problems.values().collect();
        problems.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        problems
    }

    /// 删除问题
    pub fn delete_problem(&mut self, id: Uuid) -> Result<()> {
        if let Some(_) = self.problems.remove(&id) {
            let file_path = self.get_problem_file_path(id);
            if file_path.exists() {
                fs::remove_file(file_path)?;
            }

            if self.current_problem == Some(id) {
                self.current_problem = None;
            }
        }
        Ok(())
    }

    /// 保存问题到磁盘
    pub fn save_problem(&self, problem: &Problem) -> Result<()> {
        let file_path = self.get_problem_file_path(problem.id);
        let json = serde_json::to_string_pretty(problem)?;
        fs::write(file_path, json)?;
        Ok(())
    }

    /// 更新当前问题
    pub fn update_current_problem(&mut self) -> Result<()> {
        if let Some(problem) = self.get_current_problem() {
            self.save_problem(problem)?;
        }
        Ok(())
    }

    /// 从磁盘加载所有问题
    fn load_problems(&mut self) -> Result<()> {
        if !self.data_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match fs::read_to_string(&path) {
                    Ok(json) => match serde_json::from_str::<Problem>(&json) {
                        Ok(problem) => {
                            let id = problem.id;
                            self.problems.insert(id, problem);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse problem file {:?}: {}", path, e);
                        }
                    },
                    Err(e) => {
                        tracing::warn!("Failed to read problem file {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取问题文件路径
    fn get_problem_file_path(&self, id: Uuid) -> PathBuf {
        self.data_dir.join(format!("{}.json", id))
    }

    /// 获取问题数量
    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.problems.len()
    }

    /// 清空所有问题
    #[allow(dead_code)]
    pub fn clear_all(&mut self) -> Result<()> {
        let ids: Vec<_> = self.problems.keys().copied().collect();
        for id in ids {
            self.delete_problem(id)?;
        }
        Ok(())
    }
}
