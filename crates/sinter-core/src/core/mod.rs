//! 核心模块
//!
//! 包含插件系统的核心 trait 和应用结构

pub mod app;
pub mod handler;

pub use app::Sinter;
pub use handler::CommandHandler;
