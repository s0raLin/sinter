//! 数据模型和DTO
//!
//! 定义 Sinter 的数据传输对象（DTO）和领域模型

pub mod dependency;
pub mod directory;
pub mod library;
pub mod project;
pub mod workspace;

// Re-export for convenience
pub use dependency::{DependencyDetail, DependencyDto, DependencySpec};
pub use directory::Directory;
pub use library::{Library, LibraryType};
pub use project::{Package, Project, ProjectDto};
pub use workspace::{Workspace, WorkspaceDto};
