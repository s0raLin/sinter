//! IDE支持模块
//!
//! 提供IDE集成功能

pub mod bsp_setup;
pub mod classpath_generator;

// Re-export for convenience
pub use bsp_setup::*;
pub use classpath_generator::*;
