//! Sinter - Scala 项目构建工具
//!
//! 这是一个类似 Cargo 的 Scala 项目管理和构建工具。

pub mod models;
pub mod config;
pub mod build;
pub mod deps;
pub mod ide;
pub mod cli;
pub mod core;
pub mod toolkit;

// 公共 API
pub use cli::{Cli, Commands, WorkspaceCommands};
pub use core::{CommandHandler, Sinter};
