//! 命令处理器 trait
//!
//! 所有命令（包括内置命令和插件）都需要实现这个 trait

use async_trait::async_trait;
use clap::{ArgMatches, Command};
use std::path::PathBuf;

/// 命令处理器 trait
///
/// 实现这个 trait 就可以创建一个新的命令。
/// 无论是内置命令还是插件，都使用相同的接口。
#[async_trait]
pub trait CommandHandler: Send + Sync {
    /// 返回命令名称
    ///
    /// 例如：`"new"`, `"build"`, `"jsp"` 等
    fn name(&self) -> &'static str;

    /// 返回命令描述
    ///
    /// 这个描述会显示在帮助信息中
    fn about(&self) -> &'static str;

    /// 配置命令参数
    ///
    /// 默认实现会设置命令名称和描述。
    /// 如果需要添加参数，可以重写这个方法。
    fn configure(&self, cmd: Command) -> Command {
        cmd.name(self.name()).about(self.about())
    }

    /// 执行命令逻辑
    ///
    /// 这是命令的核心逻辑，当用户运行命令时会被调用。
    async fn execute(&self, matches: &ArgMatches, cwd: &PathBuf) -> anyhow::Result<()>;
}
