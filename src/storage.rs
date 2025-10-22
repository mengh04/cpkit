use crate::models::Problem;
use anyhow::Result;
use std::collections::HashMap;
use uuid::Uuid;

/// 问题存储管理器（仅内存存储）
pub struct ProblemStore {
    problems: HashMap<Uuid, Problem>,
    current_problem: Option<Uuid>,
}

impl ProblemStore {
    /// 创建新的存储管理器
    pub fn new() -> Result<Self> {
        Ok(Self {
            problems: HashMap::new(),
            current_problem: None,
        })
    }

    /// 添加新问题
    pub fn add_problem(&mut self, problem: Problem) -> Result<()> {
        let id = problem.id;
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
    #[allow(dead_code)]
    pub fn delete_problem(&mut self, id: Uuid) -> Result<()> {
        self.problems.remove(&id);
        if self.current_problem == Some(id) {
            self.current_problem = None;
        }
        Ok(())
    }

    /// 更新当前问题（兼容接口，不再需要保存到磁盘）
    pub fn update_current_problem(&mut self) -> Result<()> {
        // 数据已经在内存中，无需额外操作
        Ok(())
    }

    /// 获取问题数量
    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.problems.len()
    }

    /// 清空所有问题
    #[allow(dead_code)]
    pub fn clear_all(&mut self) -> Result<()> {
        self.problems.clear();
        self.current_problem = None;
        Ok(())
    }
}
