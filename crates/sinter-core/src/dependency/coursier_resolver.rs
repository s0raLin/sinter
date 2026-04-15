//! Coursier依赖解析器

use super::resolver::DependencyResolver;
use crate::deps::Dependency;
use crate::models::Project;

pub struct CoursierResolver;

impl DependencyResolver for CoursierResolver {
    fn resolve_dependencies(&self, project: &Project) -> Vec<Dependency> {
        // TODO: 实现Coursier依赖解析
        Vec::new()
    }

    fn resolve_dependencies_with_workspace(
        &self,
        project: &Project,
        workspace_root: Option<&Project>,
    ) -> Vec<Dependency> {
        // TODO: 实现包含工作空间的Coursier依赖解析
        Vec::new()
    }
}
