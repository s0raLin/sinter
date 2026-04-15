//! 构建后端抽象

use crate::models::Project;

/// 构建后端trait
pub trait BuildBackend {
    /// 构建项目
    fn build(&self, project: &Project, output_dir: &std::path::Path) -> anyhow::Result<()>;

    /// 运行项目
    fn run(&self, project: &Project, args: &[String]) -> anyhow::Result<()>;

    /// 测试项目
    fn test(&self, project: &Project) -> anyhow::Result<()>;
}
