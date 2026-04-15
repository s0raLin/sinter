//! 依赖管理模块
//!
//! 提供依赖解析和管理功能

pub mod coursier_resolver;
pub mod resolver;
pub mod sbt_resolver;
pub mod scala_cli_resolver;

// Re-export for convenience
pub use resolver::*;
