//! Sinter - Scala 项目构建工具
//!
//! 这是一个类似 Cargo 的 Scala 项目管理和构建工具。

// 数据模型
pub mod models;

// 配置管理
pub mod config;

// 构建系统
pub mod build;

// 工作空间管理
pub mod workspace;

// IDE支持
pub mod ide;

// 命令行接口
pub mod cli;

// 运行时
pub mod runtime;

// 核心模块
pub mod core;

// 功能模块
pub mod deps;

// 工具包
pub mod toolkit;

// 错误处理
pub mod error;

// 公共 API
pub use cli::{Cli, Commands, WorkspaceCommands};
pub use core::{CommandHandler, Sinter};
