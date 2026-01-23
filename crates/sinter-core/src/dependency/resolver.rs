//! 依赖解析接口

use crate::models::{Project, DependencySpec};
use crate::deps::Dependency;

/// 依赖解析器trait
pub trait DependencyResolver {
    /// 解析项目依赖
    fn resolve_dependencies(&self, project: &Project) -> Vec<Dependency>;

    /// 解析包含工作空间的依赖
    fn resolve_dependencies_with_workspace(&self, project: &Project, workspace_root: Option<&Project>) -> Vec<Dependency>;
}

/// 获取项目依赖
pub fn get_dependencies(project: &Project) -> Vec<Dependency> {
    project.dependencies
        .iter()
        .filter_map(|(k, spec)| match spec {
            DependencySpec::Simple(version) => {
                Some(Dependency::from_toml_key(k, &version))
            }
            DependencySpec::Detailed(detail) => {
                detail.version.as_ref().map(|v| Dependency::from_toml_key(k, v))
            }
        })
        .collect()
}

/// 获取包含工作空间的依赖
pub fn get_dependencies_with_workspace(project: &Project, workspace_root: Option<&Project>) -> Vec<Dependency> {
    let mut deps = Vec::new();

    for (k, spec) in &project.dependencies {
        match spec {
            DependencySpec::Simple(version) => {
                deps.push(Dependency::from_toml_key(k, &version));
            }
            DependencySpec::Detailed(detail) => {
                if detail.workspace {
                    if let Some(ws) = workspace_root {
                        if let Some(ws_config) = &ws.workspace {
                            if let Some(ws_spec) = ws_config.dependencies.get(k) {
                                match ws_spec {
                                    DependencySpec::Simple(version) => {
                                        deps.push(Dependency::from_toml_key(k, &version));
                                    }
                                    DependencySpec::Detailed(ws_detail) => {
                                        if let Some(version) = &ws_detail.version {
                                            deps.push(Dependency::from_toml_key(k, version));
                                        }
                                    }
                                }
                            } else {
                                eprintln!("Warning: dependency '{}' marked as workspace but not found in workspace root", k);
                            }
                        }
                    }
                } else if let Some(version) = &detail.version {
                    deps.push(Dependency::from_toml_key(k, version));
                }
            }
        }
    }

    deps
}

/// 获取传递依赖
pub async fn get_transitive_dependencies_with_workspace(
    project: &Project,
    workspace_root: Option<&Project>,
    project_dir: &std::path::Path,
) -> anyhow::Result<Vec<Dependency>> {
    eprintln!("DEBUG: Starting get_transitive_dependencies_with_workspace for project at {}", project_dir.display());
    let direct_deps = get_dependencies_with_workspace(project, workspace_root);
    eprintln!("DEBUG: Found {} direct dependencies", direct_deps.len());
    let mut dep_manager = crate::deps::default_dependency_manager().await;
    dep_manager.set_project_dir(project_dir);
    eprintln!("DEBUG: Calling dep_manager.get_transitive_dependencies");
    let result = dep_manager.get_transitive_dependencies(&direct_deps).await;
    eprintln!("DEBUG: get_transitive_dependencies_with_workspace completed");
    result
}