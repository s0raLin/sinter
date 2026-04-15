use crate::deps::deps::Dependency;
use anyhow::anyhow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::process::Command;

// --- 核心 Trait ---

#[async_trait::async_trait]
pub trait DependencyManager: Send + Sync {
    async fn prepare_dependencies(
        &self,
        deps: &[Dependency],
        target_dir: &Path,
    ) -> anyhow::Result<()>;

    fn get_build_args(&self, deps: &[Dependency]) -> Vec<String>;

    fn get_run_args(&self, deps: &[Dependency]) -> Vec<String>;

    async fn validate_dependency(&self, dep: &Dependency) -> anyhow::Result<()>;

    async fn get_transitive_dependencies(
        &self,
        deps: &[Dependency],
    ) -> anyhow::Result<Vec<Dependency>>;

    fn set_project_dir(&mut self, project_dir: &Path);
}

// --- 辅助函数 ---

/// 检查命令是否可用
async fn check_command_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 解析单行 Maven 坐标（group:artifact:version）
fn parse_maven_coord(line: &str) -> Option<Dependency> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let parts: Vec<&str> = line.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let (group, artifact, version) = (parts[0], parts[1], parts[2]);
    let is_scala = artifact.contains("_2.13")
        || artifact.contains("_2.12")
        || artifact.contains("_3");
    Some(Dependency::Maven {
        group: group.to_string(),
        artifact: artifact.to_string(),
        version: version.to_string(),
        is_scala,
    })
}

// --- ScalaCli 实现 ---

pub struct ScalaCliDependencyManager {
    project_dir: Option<PathBuf>,
}

impl ScalaCliDependencyManager {
    pub fn new() -> Self {
        Self { project_dir: None }
    }
}

impl Default for ScalaCliDependencyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DependencyManager for ScalaCliDependencyManager {
    async fn prepare_dependencies(
        &self,
        deps: &[Dependency],
        _target_dir: &Path,
    ) -> anyhow::Result<()> {
        for dep in deps {
            if let Some(sbt_path) = dep.sbt_path() {
                let path = self.resolve_path(sbt_path);
                if !path.exists() {
                    anyhow::bail!("sbt project path does not exist: {}", path.display());
                }
            }
        }
        Ok(())
    }

    fn get_build_args(&self, deps: &[Dependency]) -> Vec<String> {
        let mut args = Vec::new();
        for dep in deps {
            args.push("--dependency".to_string());
            args.push(match dep {
                Dependency::Maven { .. } => dep.coord(),
                Dependency::Sbt { path } => {
                    let resolved = self.resolve_path(path);
                    format!("file://{}", resolved.display())
                }
            });
        }
        args
    }

    fn get_run_args(&self, deps: &[Dependency]) -> Vec<String> {
        self.get_build_args(deps)
    }

    async fn validate_dependency(&self, dep: &Dependency) -> anyhow::Result<()> {
        match dep {
            Dependency::Maven { .. } => {
                let coord = dep.coord();
                let args = vec![
                    "--dependency",
                    &coord,
                    "--quiet",
                    "-e",
                    "println(\"test\")",
                ];
                let output = crate::build::run_scala_cli(&args, None).await?;
                if !output.status.success() {
                    let err = String::from_utf8_lossy(&output.stderr);
                    anyhow::bail!("Dependency {} is not available: {}", dep.coord(), err);
                }
                Ok(())
            }
            Dependency::Sbt { path } => {
                let resolved = self.resolve_path(path);
                if !resolved.exists() {
                    anyhow::bail!("sbt project path does not exist: {}", resolved.display());
                }
                Ok(())
            }
        }
    }

    async fn get_transitive_dependencies(
        &self,
        deps: &[Dependency],
    ) -> anyhow::Result<Vec<Dependency>> {
        let mut all_deps: Vec<Dependency> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        for dep in deps {
            match dep {
                Dependency::Maven { .. } => {
                    // scala-cli 本身不提供传递依赖解析，直接返回原始依赖
                    if seen.insert(dep.coord()) {
                        all_deps.push(dep.clone());
                    }
                }
                Dependency::Sbt { path } => {
                    let sbt_path = self.resolve_path(path);
                    match resolve_sbt_dependencies(&sbt_path).await {
                        Ok(sbt_deps) => {
                            for d in sbt_deps {
                                if seen.insert(d.coord()) {
                                    all_deps.push(d);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: failed to resolve sbt deps for {}: {}",
                                path, e
                            );
                            if seen.insert(dep.coord()) {
                                all_deps.push(dep.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(all_deps)
    }

    fn set_project_dir(&mut self, project_dir: &Path) {
        self.project_dir = Some(project_dir.to_path_buf());
    }
}

impl ScalaCliDependencyManager {
    /// 将相对路径解析为绝对路径（相对于 project_dir）
    fn resolve_path(&self, path: &str) -> PathBuf {
        let p = Path::new(path);
        if p.is_relative() {
            self.project_dir
                .as_deref()
                .map(|dir| dir.join(p))
                .unwrap_or_else(|| p.to_path_buf())
        } else {
            p.to_path_buf()
        }
    }
}

// --- 工厂函数 ---

pub fn default_dependency_manager() -> Box<dyn DependencyManager + Send + Sync> {
    Box::new(ScalaCliDependencyManager::new())
}

// --- SBT 依赖解析 ---

async fn resolve_sbt_dependencies(sbt_project_path: &Path) -> anyhow::Result<Vec<Dependency>> {
    if !sbt_project_path.join("build.sbt").exists() {
        return Ok(vec![]);
    }

    if check_command_available("sbt").await {
        return resolve_sbt_dependencies_via_sbt(sbt_project_path).await;
    }

    eprintln!(
        "Warning: sbt command not found, cannot resolve dependencies for {}",
        sbt_project_path.display()
    );
    Ok(vec![])
}

async fn resolve_sbt_dependencies_via_sbt(
    sbt_project_path: &Path,
) -> anyhow::Result<Vec<Dependency>> {
    let output = Command::new("sbt")
        .arg("dependencyTree")
        .current_dir(sbt_project_path)
        .output()
        .await?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("sbt dependencyTree failed: {}", err));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut deps = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for line in stdout.lines() {
        // 去掉 [info] 前缀
        let line = line
            .trim()
            .trim_start_matches("[info]")
            .trim();

        // 跳过被驱逐的依赖
        if line.contains("(evicted)") {
            continue;
        }

        // 匹配依赖树节点（+- 或缩进后的 +-）
        let dep_line = if let Some(pos) = line.find("+-") {
            line[pos + 2..].trim()
        } else {
            continue;
        };

        // 版本号后可能跟着空格和其他文字，只取第一段
        let dep_line = dep_line
            .split_whitespace()
            .next()
            .unwrap_or(dep_line);

        if let Some(dep) = parse_maven_coord(dep_line) {
            if seen.insert(dep.coord()) {
                deps.push(dep);
            }
        }
    }

    Ok(deps)
}
