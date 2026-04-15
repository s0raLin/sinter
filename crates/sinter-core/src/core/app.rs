//! Sinter 应用构建器
//!
//! 使用函数式 Builder 模式构建应用，支持链式注册插件

use crate::cli::Cli;
use crate::core::handler::CommandHandler;
use crate::runtime::Executor;
use crate::toolkit::path::PathManager;

/// Sinter 应用构建器
///
/// 使用 Builder 模式，支持链式调用注册插件：
///
/// ```rust
/// Sinter::new()
///     .plugin(my_plugin())
///     .plugin(another_plugin())
///     .run()
///     .await?;
/// ```
pub struct Sinter {
    plugins: Vec<Box<dyn CommandHandler>>,
}

impl Sinter {
    /// 创建新的 Sinter 应用
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// 注册单个插件
    ///
    /// 支持链式调用
    pub fn plugin<H: CommandHandler + 'static>(mut self, handler: H) -> Self {
        self.plugins.push(Box::new(handler));
        self
    }

    /// 批量注册插件
    ///
    /// 可以传入一个迭代器，一次性注册多个插件
    pub fn plugins<H: CommandHandler + 'static, I: IntoIterator<Item = H>>(
        mut self,
        handlers: I,
    ) -> Self {
        for handler in handlers {
            self.plugins.push(Box::new(handler));
        }
        self
    }

    /// 运行应用
    ///
    /// 解析命令行参数，执行对应的命令
    pub async fn run(self) -> anyhow::Result<()> {
        let cli = Cli::parse_with_plugins(&self.plugins);
        let cwd = PathManager::current_dir().await?;
        Executor::new(self.plugins).execute(cli, cwd).await
    }
}

impl Default for Sinter {
    fn default() -> Self {
        Self::new()
    }
}
