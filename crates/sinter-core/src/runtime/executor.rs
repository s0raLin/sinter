//! 命令执行器
//!
//! 负责将解析后的 CLI 命令分发到对应的处理器

use crate::cli::{builtin, Cli};
use crate::core::handler::CommandHandler;
use crate::toolkit::path::PathManager;

/// 命令执行器
///
/// 负责执行命令，包括：
/// 1. 插件命令的执行
/// 2. 内置命令的执行
pub struct Executor {
    plugins: Vec<Box<dyn CommandHandler>>,
}

impl Executor {
    /// 创建新的执行器
    pub fn new(plugins: Vec<Box<dyn CommandHandler>>) -> Self {
        Self { plugins }
    }

    /// 执行命令
    ///
    /// 根据 CLI 解析结果，执行对应的命令
    pub async fn execute(&self, cli: Cli, cwd: PathManager) -> anyhow::Result<()> {
        // 首先检查是否是插件命令
        if let Some((command_name, matches)) = cli.raw_matches.subcommand() {
            if let Some(handler) = self.plugins.iter().find(|cmd| cmd.name() == command_name) {
                return handler.execute(matches, &cwd.to_path_buf()).await;
            }
        }

        // 处理内置命令
        if let Some(command) = cli.command {
            builtin::execute_command(command, &cwd).await?;
        } else {
            // 没有提供命令，尝试运行默认行为
            builtin::execute_default(&cwd).await?;
        }

        Ok(())
    }
}
