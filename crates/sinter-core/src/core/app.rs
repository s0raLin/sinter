//! Sinter 应用构建器 + 命令执行器
//!
//! 使用函数式 Builder 模式构建应用，支持链式注册插件

use crate::cli::Cli;
use crate::cli::commands;
use crate::core::handler::CommandHandler;
use crate::toolkit::path::PathManager;

/// Sinter 应用构建器 — 支持链式调用注册插件
pub struct Sinter {
    plugins: Vec<Box<dyn CommandHandler>>,
}

impl Sinter {
    pub fn new() -> Self { Self { plugins: Vec::new() } }

    /// 注册单个插件
    pub fn plugin<H: CommandHandler + 'static>(mut self, handler: H) -> Self {
        self.plugins.push(Box::new(handler));
        self
    }

    /// 批量注册插件
    pub fn plugins<H: CommandHandler + 'static, I: IntoIterator<Item = H>>(mut self, handlers: I) -> Self {
        for handler in handlers { self.plugins.push(Box::new(handler)); }
        self
    }

    /// 运行应用
    pub async fn run(self) -> anyhow::Result<()> {
        let cli = Cli::parse_with_plugins(&self.plugins);
        let cwd = PathManager::current_dir().await?;
        Executor::new(self.plugins).execute(cli, cwd).await
    }
}

impl Default for Sinter {
    fn default() -> Self { Self::new() }
}

// ━━━━━━━━━━━━━━━━━━━ Executor (was runtime/executor.rs) ━━━━━━━━━━━━━━━━━━━

/// 命令执行器 — 分发 CLI 命令到对应的处理器
pub struct Executor {
    plugins: Vec<Box<dyn CommandHandler>>,
}

impl Executor {
    pub fn new(plugins: Vec<Box<dyn CommandHandler>>) -> Self { Self { plugins } }

    pub async fn execute(&self, cli: Cli, cwd: PathManager) -> anyhow::Result<()> {
        // 首先检查是否是插件命令
        if let Some((command_name, matches)) = cli.raw_matches.subcommand() {
            if let Some(handler) = self.plugins.iter().find(|cmd| cmd.name() == command_name) {
                return handler.execute(matches, &cwd.to_path_buf()).await;
            }
        }

        // 处理内置命令
        if let Some(command) = cli.command {
            commands::execute_command(command, &cwd).await?;
        } else {
            commands::execute_default(&cwd).await?;
        }
        Ok(())
    }
}
