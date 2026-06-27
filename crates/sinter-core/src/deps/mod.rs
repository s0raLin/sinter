//! 依赖管理 — 解析、验证和传递依赖处理

use crate::models::{DependencySpec, Project};

pub mod manager;

pub use manager::{default_dependency_manager, DependencyManager, ScalaCliDependencyManager};

// ━━━━━━━━━━━━━━━━━━━ Dependency struct ━━━━━━━━━━━━━━━━━━━

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub enum Dependency {
    Maven {
        group: String,
        artifact: String,
        version: String,
        is_scala: bool,
    },
    Sbt {
        path: String,
    },
}

impl Dependency {
    pub fn from_toml_key(key: &str, version: &str) -> Self {
        if key.starts_with("sbt:") || (key.contains("/") && !key.contains("::") && !key.contains(":")) {
            let path = if key.starts_with("sbt:") { key[4..].to_string() } else { key.to_string() };
            Self::Sbt { path }
        } else {
            let is_scala_format = key.contains("::");
            let (group, artifact) = if is_scala_format {
                let parts: Vec<&str> = key.split("::").collect();
                if parts.len() >= 2 { (parts[0].to_string(), parts[1].to_string()) }
                else { ("".to_string(), key.to_string()) }
            } else if key.contains(":") {
                let parts: Vec<&str> = key.splitn(2, ':').collect();
                if parts.len() >= 2 { (parts[0].to_string(), parts[1].to_string()) }
                else { ("".to_string(), key.to_string()) }
            } else {
                ("".to_string(), key.to_string())
            };
            let is_scala = artifact.contains("_2.") || artifact.contains("_3");
            Self::Maven { group, artifact, version: version.to_string(), is_scala }
        }
    }

    pub fn coord(&self) -> String {
        match self {
            Dependency::Maven { group, artifact, version, .. } => format!("{}:{}:{}", group, artifact, version),
            Dependency::Sbt { path } => format!("sbt:{}", path),
        }
    }

    pub fn is_sbt(&self) -> bool { matches!(self, Dependency::Sbt { .. }) }

    pub fn sbt_path(&self) -> Option<&str> {
        match self { Dependency::Sbt { path } => Some(path), _ => None }
    }
}

// ━━━━━━━━━━━━━━━━━━━ Public helpers ━━━━━━━━━━━━━━━━━━━

pub fn get_dependencies(project: &Project) -> Vec<Dependency> {
    project.dependencies.iter().filter_map(|(k, spec)| match spec {
        DependencySpec::Simple(version) => Some(Dependency::from_toml_key(k, version)),
        DependencySpec::Detailed(detail) => detail.version.as_ref().map(|v| Dependency::from_toml_key(k, v)),
    }).collect()
}

pub fn get_dependencies_with_workspace(project: &Project, workspace_root: Option<&Project>) -> Vec<Dependency> {
    let mut deps = Vec::new();
    for (k, spec) in &project.dependencies {
        match spec {
            DependencySpec::Simple(version) => { deps.push(Dependency::from_toml_key(k, version)); }
            DependencySpec::Detailed(detail) => {
                if detail.workspace {
                    if let Some(ws) = workspace_root {
                        if let Some(ws_config) = &ws.workspace {
                            if let Some(ws_spec) = ws_config.dependencies.get(k) {
                                match ws_spec {
                                    DependencySpec::Simple(version) => { deps.push(Dependency::from_toml_key(k, version)); }
                                    DependencySpec::Detailed(ws_detail) => {
                                        if let Some(version) = &ws_detail.version { deps.push(Dependency::from_toml_key(k, version)); }
                                    }
                                }
                            }
                        }
                    }
                } else if let Some(version) = &detail.version { deps.push(Dependency::from_toml_key(k, version)); }
            }
        }
    }
    deps
}

pub async fn get_transitive_dependencies_with_workspace(
    project: &Project, workspace_root: Option<&Project>, project_dir: &std::path::Path,
) -> anyhow::Result<Vec<Dependency>> {
    let direct_deps = get_dependencies_with_workspace(project, workspace_root);
    let mut dep_manager = default_dependency_manager();
    dep_manager.set_project_dir(project_dir);
    dep_manager.get_transitive_dependencies(&direct_deps).await
}

// ━━━━━━━━━━━━━━━━━━━ add_dependency ━━━━━━━━━━━━━━━━━━━

use std::path::Path;

pub async fn add_dependency(project_dir: &Path, dep_spec: &str) -> anyhow::Result<()> {
    let project = crate::config::load_project(project_dir)?;
    let manifest_path = project_dir.join("project.toml");
    let is_workspace_root = project.workspace.is_some();

    if dep_spec.starts_with("sbt:") || (dep_spec.contains("/") && !dep_spec.contains("::")) {
        let sbt_path = if dep_spec.starts_with("sbt:") { dep_spec[4..].to_string() } else { dep_spec.to_string() };
        let sbt_project_path = project_dir.join(&sbt_path);
        if !sbt_project_path.exists() { anyhow::bail!("sbt project path does not exist: {}", sbt_path); }
        let key = format!("sbt:{}", sbt_path);
        if is_workspace_root {
            crate::config::add_workspace_dependency_to_manifest(&manifest_path, &key, "")?;
        } else {
            crate::config::add_dependency_to_manifest(&manifest_path, &key, "")?;
        }
        return Ok(());
    }

    let (artifact, scala_ver, version) = parse_dep_spec(dep_spec, &project.package.scala_version).await?;
    let full_key = if artifact.contains(':') && !artifact.contains("::") {
        artifact.clone()
    } else if !scala_ver.is_empty() && scala_ver != "latest" {
        format!("{}_{}", artifact, scala_ver)
    } else {
        artifact.clone()
    };

    let dep_manager = default_dependency_manager();
    let dep = Dependency::from_toml_key(&full_key, &version);
    if let Err(e) = dep_manager.validate_dependency(&dep).await {
        anyhow::bail!("Failed to validate dependency {}: {}", full_key, e);
    }
    if let Err(e) = dep_manager.prepare_dependencies(&[dep.clone()], &project_dir.join("target")).await {
        anyhow::bail!("Failed to download dependency {}: {}", full_key, e);
    }
    if is_workspace_root {
        crate::config::add_workspace_dependency_to_manifest(&manifest_path, &full_key, &version)?;
    } else {
        crate::config::add_dependency_to_manifest(&manifest_path, &full_key, &version)?;
    }
    Ok(())
}

async fn parse_dep_spec(spec: &str, default_scala_version: &str) -> anyhow::Result<(String, String, String)> {
    let is_scala_format = spec.contains("::");
    let (group, artifact_version, is_scala) = if is_scala_format {
        let parts: Vec<&str> = spec.split("::").collect();
        if parts.len() != 2 { anyhow::bail!("依赖格式无效，请使用完整格式：group::artifact:version"); }
        (parts[0], parts[1], true)
    } else {
        let parts: Vec<&str> = spec.split(':').collect();
        if parts.len() != 3 { anyhow::bail!("依赖格式无效，请使用完整格式：group::artifact:version"); }
        return Ok((format!("{}:{}", parts[0], parts[1]), "".to_string(), parts[2].to_string()));
    };

    let av_parts: Vec<&str> = artifact_version.split(':').collect();
    if av_parts.len() != 2 { anyhow::bail!("依赖格式无效，请使用完整格式：group::artifact:version"); }
    let artifact_with_scala = av_parts[0];
    let version = av_parts[1];
    if artifact_with_scala.contains("::") { anyhow::bail!("依赖格式无效，artifact 不应包含 '::'"); }

    let artifact_parts: Vec<&str> = artifact_with_scala.split('@').collect();
    let (artifact, scala_ver) = if artifact_parts.len() == 2 {
        (artifact_parts[0].to_string(), artifact_parts[1])
    } else {
        (artifact_with_scala.to_string(), default_scala_version)
    };
    let full_artifact = format!("{}::{}", group, artifact);
    if version.is_empty() || version == "latest" { anyhow::bail!("必须明确指定版本，不允许使用 'latest'"); }
    Ok((full_artifact, scala_ver.to_string(), version.to_string()))
}
