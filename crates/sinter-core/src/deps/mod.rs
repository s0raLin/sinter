pub mod add;
pub mod deps;
pub mod manager;

pub use add::add_dependency;
pub use deps::Dependency;
pub use manager::{default_dependency_manager, DependencyManager, ScalaCliDependencyManager};

use crate::models::{DependencySpec, Project};

pub fn get_dependencies(project: &Project) -> Vec<Dependency> {
    project
        .dependencies
        .iter()
        .filter_map(|(k, spec)| match spec {
            DependencySpec::Simple(version) => Some(Dependency::from_toml_key(k, &version)),
            DependencySpec::Detailed(detail) => detail
                .version
                .as_ref()
                .map(|v| Dependency::from_toml_key(k, v)),
        })
        .collect()
}

pub fn get_dependencies_with_workspace(
    project: &Project,
    workspace_root: Option<&Project>,
) -> Vec<Dependency> {
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

pub async fn get_transitive_dependencies_with_workspace(
    project: &Project,
    workspace_root: Option<&Project>,
    project_dir: &std::path::Path,
) -> anyhow::Result<Vec<Dependency>> {
    let direct_deps = get_dependencies_with_workspace(project, workspace_root);
    let mut dep_manager = default_dependency_manager();
    dep_manager.set_project_dir(project_dir);
    dep_manager.get_transitive_dependencies(&direct_deps).await
}
