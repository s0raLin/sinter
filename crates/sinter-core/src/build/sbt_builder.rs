//! SBT构建器

use super::backend::BuildBackend;
use crate::models::Project;

pub struct SbtBuilder;

impl BuildBackend for SbtBuilder {
    fn build(&self, project: &Project, output_dir: &std::path::Path) -> anyhow::Result<()> {
        // TODO: 实现SBT构建逻辑
        Ok(())
    }

    fn run(&self, project: &Project, args: &[String]) -> anyhow::Result<()> {
        // TODO: 实现SBT运行逻辑
        Ok(())
    }

    fn test(&self, project: &Project) -> anyhow::Result<()> {
        // TODO: 实现SBT测试逻辑
        Ok(())
    }
}
