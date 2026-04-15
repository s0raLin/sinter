pub mod add;
pub mod deps;
pub mod manager;

pub use add::add_dependency;
pub use deps::Dependency;
pub use manager::{default_dependency_manager, DependencyManager, ScalaCliDependencyManager};
