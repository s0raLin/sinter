use crate::deps::deps::Dependency;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashSet;
use anyhow::anyhow;



// --- 核心 Trait 和辅助函数 ---

/// 抽象的依赖管理器trait
#[async_trait::async_trait]
pub trait DependencyManager: Send + Sync {
    /// 准备依赖（下载、构建等）
    async fn prepare_dependencies(&self, deps: &[Dependency], target_dir: &Path) -> anyhow::Result<()>;

    /// 获取构建命令的参数
    fn get_build_args(&self, deps: &[Dependency]) -> Vec<String>;

    /// 获取运行命令的参数
    fn get_run_args(&self, deps: &[Dependency]) -> Vec<String>;

    /// 验证依赖是否可用（用于添加依赖时）
    async fn validate_dependency(&self, dep: &Dependency) -> anyhow::Result<()>;

    /// 获取传递依赖（包括直接依赖和所有传递依赖）
    async fn get_transitive_dependencies(&self, deps: &[Dependency]) -> anyhow::Result<Vec<Dependency>>;

    /// 设置项目目录（用于解析相对路径）
    fn set_project_dir(&mut self, project_dir: &Path);
}

/// 获取打包的coursier可执行文件路径
fn get_bundled_coursier_path() -> Option<PathBuf> {
    let exe_name = if cfg!(target_os = "windows") {
        "coursier.exe"
    } else {
        "coursier"
    };
    
    // 1. 尝试从bin目录（相对于可执行文件）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let bundled_path = exe_dir.join("bin").join(exe_name);
            if bundled_path.exists() {
                return Some(bundled_path);
            }
        }
    }
    
    // 2. 尝试从CARGO_MANIFEST_DIR/bin目录（开发时）
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let bundled_path = PathBuf::from(manifest_dir).join("bin").join(exe_name);
        if bundled_path.exists() {
            return Some(bundled_path);
        }
    }
    
    None
}

/// 获取coursier可执行文件路径
async fn get_coursier_path() -> Option<String> {
    // 首先尝试使用打包的coursier
    if let Some(bundled_path) = get_bundled_coursier_path() {
        // 确保文件有执行权限
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(mut perms) = std::fs::metadata(&bundled_path).map(|m| m.permissions()) {
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&bundled_path, perms);
            }
        }
        
        if let Some(path_str) = bundled_path.to_str() {
            // 验证打包的coursier是否可用
            let mut cmd = Command::new(path_str);
            cmd.arg("--version");
            // 使用 timeout 避免卡死，尽管 tokio::process::Command 默认没有
            if cmd.output().await.map(|o| o.status.success()).unwrap_or(false) {
                return Some(path_str.to_string());
            }
        }
    }
    
    // 回退到系统命令
    if check_command_available("coursier").await {
        Some("coursier".to_string())
    } else {
        None
    }
}

/// 检查命令是否可用
async fn check_command_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

static COURSIER_WARNING_PRINTED: AtomicBool = AtomicBool::new(false);

/// 检查coursier是否可用，如果不可用则打印安装提示（仅一次）
pub async fn check_coursier_available() -> bool {
    let available = get_coursier_path().await.is_some();
    if !available {
        // 只打印一次警告
        if !COURSIER_WARNING_PRINTED.swap(true, Ordering::Relaxed) {
            eprintln!("Warning: coursier is not available. For better dependency management:");
            eprintln!("  - Install coursier: curl -fL https://github.com/coursier/coursier/releases/latest/download/cs-x86_64-pc-linux.gz | gzip -d > cs && chmod +x cs && ./cs install coursier");
            eprintln!("  - Or visit: https://get-coursier.io/");
            eprintln!("  Falling back to scala-cli for dependency management.");
        }
    }
    available
}

// --- Coursier 实现 ---

/// Coursier 依赖管理器
pub struct CoursierDependencyManager {
    project_dir: Option<PathBuf>,
}

impl CoursierDependencyManager {
    pub fn new() -> Self {
        Self { project_dir: None }
    }
}

#[async_trait::async_trait]
impl DependencyManager for CoursierDependencyManager {
    async fn prepare_dependencies(&self, deps: &[Dependency], _target_dir: &Path) -> anyhow::Result<()> {
        eprintln!("DEBUG: Starting dependency preparation for {} dependencies", deps.len());
        let coursier_path = get_coursier_path().await
            .ok_or_else(|| anyhow!("coursier is not available"))?;

        for dep in deps {
            match dep {
                Dependency::Maven { .. } => {
                    eprintln!("DEBUG: Fetching dependency: {}", dep.coord());
                    // 使用coursier fetch下载依赖（这会自动缓存）
                    let mut cmd = Command::new(&coursier_path);
                    cmd.arg("fetch")
                        .arg("--quiet")
                        .arg(dep.coord());

                    let output = cmd.output().await?;
                    if !output.status.success() {
                        let err = String::from_utf8_lossy(&output.stderr);
                        eprintln!("DEBUG: Failed to fetch dependency {}: {}", dep.coord(), err);
                        anyhow::bail!("Failed to fetch dependency {}: {}", dep.coord(), err);
                    } else {
                        eprintln!("DEBUG: Successfully fetched dependency: {}", dep.coord());
                    }
                }
                Dependency::Sbt { path } => {
                    eprintln!("DEBUG: Validating sbt project: {}", path);
                    // 验证sbt项目存在
                    let sbt_project_path = Path::new(path);
                    if !sbt_project_path.exists() {
                        anyhow::bail!("sbt project path does not exist: {}", path);
                    }
                }
            }
        }

        eprintln!("DEBUG: Dependency preparation completed");
        Ok(())
    }

    fn get_build_args(&self, deps: &[Dependency]) -> Vec<String> {
        let mut args = Vec::new();

        for dep in deps {
            match dep {
                Dependency::Maven { .. } => {
                    args.push("--dependency".to_string());
                    args.push(dep.coord());
                }
                Dependency::Sbt { path } => {
                    // 统一使用 file:// 格式，无论是相对路径还是绝对路径
                    let dep_path = if Path::new(path).is_relative() {
                        // 使用 project_dir 来解析相对路径
                        self.project_dir.as_ref()
                            .map(|dir| dir.join(path))
                            .unwrap_or_else(|| PathBuf::from(path))
                            .to_string_lossy()
                            .to_string()
                    } else {
                        path.clone()
                    };
                    
                    args.push("--dependency".to_string());
                    args.push(format!("file://{}", dep_path));
                }
            }
        }

        args
    }

    fn get_run_args(&self, deps: &[Dependency]) -> Vec<String> {
        self.get_build_args(deps)
    }

    async fn validate_dependency(&self, dep: &Dependency) -> anyhow::Result<()> {
        let coursier_path = get_coursier_path().await
            .ok_or_else(|| anyhow!("coursier is not available"))?;

        match dep {
            Dependency::Maven { .. } => {
                // 使用coursier resolve验证依赖是否存在
                let mut cmd = Command::new(&coursier_path);
                cmd.arg("resolve")
                    .arg("--quiet")
                    .arg(dep.coord());

                let output = cmd.output().await?;
                if !output.status.success() {
                    let err = String::from_utf8_lossy(&output.stderr);
                    anyhow::bail!("Dependency {} is not available: {}", dep.coord(), err);
                }
                Ok(())
            }
            Dependency::Sbt { path } => {
                let sbt_project_path = Path::new(path);
                if !sbt_project_path.exists() {
                    anyhow::bail!("sbt project path does not exist: {}", path);
                }
                Ok(())
            }
        }
    }

    async fn get_transitive_dependencies(&self, deps: &[Dependency]) -> anyhow::Result<Vec<Dependency>> {
        eprintln!("DEBUG: CoursierDependencyManager.get_transitive_dependencies called with {} deps", deps.len());
        let coursier_path = get_coursier_path().await
            .ok_or_else(|| anyhow!("coursier is not available"))?;

        let mut all_deps = Vec::new();
        let mut processed_coords: HashSet<String> = HashSet::new();

        for dep in deps {
            eprintln!("DEBUG: Processing dependency: {}", dep.coord());
            match dep {
                Dependency::Maven { group, artifact, version, is_scala } => {
                    let coord = if *is_scala {
                        format!("{}::{}:{}", group, artifact, version)
                    } else {
                        format!("{}:{}:{}", group, artifact, version)
                    };

                    eprintln!("DEBUG: Running coursier resolve for {}", coord);
                    let mut cmd = Command::new(&coursier_path);
                    cmd.arg("resolve")
                        .arg("--quiet")
                        .arg("--print-tree=false")
                        .arg("--intransitive")
                        .arg(&coord);

                    let output = cmd.output().await?;
                    eprintln!("DEBUG: coursier resolve completed for {}", coord);
                    if !output.status.success() {
                        eprintln!("Warning: Failed to resolve transitive dependencies for {}: {}", coord, String::from_utf8_lossy(&output.stderr));
                        if processed_coords.insert(coord.clone()) {
                            all_deps.push(dep.clone());
                        }
                        continue;
                    }

                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }

                        // 解析 Maven 坐标格式 group:artifact:version
                        let parts: Vec<&str> = line.split(':').collect();
                        if parts.len() >= 3 {
                            let group = parts[0];
                            let artifact = parts[1];
                            let version = parts[2];
                            let current_coord = format!("{}:{}:{}", group, artifact, version);

                            if processed_coords.insert(current_coord) {
                                // 继承原始依赖的 is_scala 标志，或者根据 artifact 名称重新判断
                                let is_scala_dep = artifact.contains("_2.13") || artifact.contains("_2.12") || artifact.contains("_3");
                                
                                all_deps.push(Dependency::Maven {
                                    group: group.to_string(),
                                    artifact: artifact.to_string(),
                                    version: version.to_string(),
                                    is_scala: is_scala_dep,
                                });
                            }
                        }
                    }
                }
                Dependency::Sbt { path } => {
                    // 解析 sbt 项目的依赖
                    let sbt_project_path = if let Some(project_dir) = &self.project_dir {
                        // 修正：相对路径相对于 project_dir
                        project_dir.join(path)
                    } else {
                        Path::new(path).to_path_buf()
                    };

                    match resolve_sbt_dependencies(&sbt_project_path).await {
                        Ok(sbt_deps) => {
                            // 仅添加未处理过的 Maven 坐标
                            for sbt_dep in sbt_deps {
                                if let Dependency::Maven { .. } = &sbt_dep {
                                    if processed_coords.insert(sbt_dep.coord()) {
                                        all_deps.push(sbt_dep);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to resolve sbt dependencies for {}: {}", path, e);
                            all_deps.push(dep.clone());
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

// --- ScalaCli 实现 ---

/// Scala CLI 依赖管理器
pub struct ScalaCliDependencyManager;

#[async_trait::async_trait]
impl DependencyManager for ScalaCliDependencyManager {
    async fn prepare_dependencies(&self, deps: &[Dependency], _target_dir: &Path) -> anyhow::Result<()> {
        for dep in deps {
            if let Some(sbt_path) = dep.sbt_path() {
                let sbt_project_path = Path::new(sbt_path);
                if !sbt_project_path.exists() {
                    anyhow::bail!("sbt project path does not exist: {}", sbt_path);
                }
            }
        }
        Ok(())
    }

    fn get_build_args(&self, deps: &[Dependency]) -> Vec<String> {
        let mut args = Vec::new();

        for dep in deps {
            match dep {
                Dependency::Maven { .. } => {
                    args.push("--dependency".to_string());
                    args.push(dep.coord());
                }
                Dependency::Sbt { path } => {
                    // Scala CLI 处理 file:// 路径
                    args.push("--dependency".to_string());
                    args.push(format!("file://{}", path));
                }
            }
        }
        args
    }

    fn get_run_args(&self, deps: &[Dependency]) -> Vec<String> {
        self.get_build_args(deps)
    }

    async fn validate_dependency(&self, dep: &Dependency) -> anyhow::Result<()> {
        match dep {
            Dependency::Maven { .. } => {
                let args: Vec<String> = vec!["--dependency".to_string(), dep.coord(), "--quiet".to_string(), "-e".to_string(), "println(\"test\")".to_string()];
                let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                let output = crate::build::run_scala_cli(&args_str, None).await?;
                if !output.status.success() {
                    let err = String::from_utf8_lossy(&output.stderr);
                    anyhow::bail!("Dependency {} is not available: {}", dep.coord(), err);
                }
                Ok(())
            }
            Dependency::Sbt { path } => {
                let sbt_project_path = Path::new(path);
                if !sbt_project_path.exists() {
                    anyhow::bail!("sbt project path does not exist: {}", path);
                }
                Ok(())
            }
        }
    }

    async fn get_transitive_dependencies(&self, deps: &[Dependency]) -> anyhow::Result<Vec<Dependency>> {
        // Scala CLI 没有原生传递依赖解析功能，尝试回退到 Coursier
        if let Some(coursier_path) = get_coursier_path().await {
            // 使用 CoursierDependencyManager 的逻辑来解析，但不设置 project_dir
            let coursier_manager = CoursierDependencyManager::new();
            coursier_manager.get_transitive_dependencies(deps).await
        } else {
            // 如果coursier不可用，返回直接依赖
            eprintln!("Warning: Coursier is not available. Cannot resolve transitive dependencies using ScalaCliDependencyManager.");
            Ok(deps.to_vec())
        }
    }

    fn set_project_dir(&mut self, _project_dir: &Path) {
        // ScalaCliDependencyManager 不需要项目目录
    }
}

// --- 依赖管理器工厂函数 ---

/// 获取默认的依赖管理器
pub async fn default_dependency_manager() -> Box<dyn DependencyManager + Send + Sync> {
    if check_coursier_available().await {
        Box::new(CoursierDependencyManager::new())
    } else {
        Box::new(ScalaCliDependencyManager)
    }
}

/// 同步版本的默认依赖管理器（用于不需要异步的场景）
pub fn default_dependency_manager_sync() -> Box<dyn DependencyManager + Send + Sync> {
    // 修正：使用 tokio runtime 来安全地调用异步检查，并避免在非异步环境中多次创建 runtime
    // 生产环境中，最好在应用的启动时只创建一次 runtime
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime for sync check");
    
    // 检查 coursier 是否可用
    let is_coursier_available = rt.block_on(check_coursier_available());

    if is_coursier_available {
        Box::new(CoursierDependencyManager::new())
    } else {
        Box::new(ScalaCliDependencyManager)
    }
}

// --- SBT 辅助解析函数 ---

/// 解析 sbt 项目的依赖
async fn resolve_sbt_dependencies(sbt_project_path: &Path) -> anyhow::Result<Vec<Dependency>> {
    let build_sbt_path = sbt_project_path.join("build.sbt");
    if !build_sbt_path.exists() {
        return Ok(vec![]);
    }

    // 优先使用 sbt dependencyTree 命令获取依赖
    if check_command_available("sbt").await {
        return resolve_sbt_dependencies_via_sbt(sbt_project_path).await;
    }

    // 回退：尝试使用 coursier 解析 sbt 项目（仅当 sbt 不可用时）
    if let Some(coursier_path) = get_coursier_path().await {
        let coord = format!("sbt-project:{}", sbt_project_path.display());
        let mut cmd = Command::new(&coursier_path);
        cmd.arg("resolve")
            .arg("--quiet")
            .arg("--print-tree=false")
            .arg("--intransitive")
            .arg(&coord);

        let output = cmd.output().await?;
        if output.status.success() {
            let mut deps = Vec::new();
            let mut processed_coords: HashSet<String> = HashSet::new();

            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                // ... (Maven 坐标解析逻辑与 CoursierDependencyManager 中相同，省略细节)
                if line.is_empty() || line.starts_with('#') { continue; }
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 {
                    let group = parts[0];
                    let artifact = parts[1];
                    let version = parts[2];
                    let coord = format!("{}:{}:{}", group, artifact, version);

                    if processed_coords.insert(coord.clone()) {
                        let is_scala = artifact.contains("_2.13") || artifact.contains("_2.12") || artifact.contains("_3");
                        deps.push(Dependency::Maven {
                            group: group.to_string(),
                            artifact: artifact.to_string(),
                            version: version.to_string(),
                            is_scala,
                        });
                    }
                }
            }
            return Ok(deps);
        }
    }

    eprintln!("Warning: Could not resolve dependencies for sbt project {} (sbt command not found).", sbt_project_path.display());
    Ok(vec![])
}

/// 使用 sbt 命令解析依赖
async fn resolve_sbt_dependencies_via_sbt(sbt_project_path: &Path) -> anyhow::Result<Vec<Dependency>> {
    // 这是一个脆弱的方法，依赖于 sbt dependencyTree 的输出格式
    let mut cmd = Command::new("sbt");
    cmd.arg("dependencyTree")
        .current_dir(sbt_project_path);

    let output = cmd.output().await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to run sbt dependencyTree: {}", err));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut deps = Vec::new();
    let mut processed_coords: HashSet<String> = HashSet::new();

    for line in stdout.lines() {
        let line = line.trim();

        let line = if line.starts_with("[info]") {
            line.trim_start_matches("[info]").trim()
        } else {
            line
        };

        if line.contains(':') && (line.starts_with("+-") || line.contains(" +-")) {
            let dep_line = if line.starts_with("+-") {
                line.trim_start_matches("+-").trim()
            } else if line.contains(" +-") {
                line.trim_start_matches(|c: char| c.is_whitespace() || c == '|')
                    .trim_start_matches("+-").trim()
            } else {
                continue;
            };

            let parts: Vec<&str> = dep_line.split(':').collect();
            // 依赖格式 group:artifact:version
            if parts.len() >= 3 {
                let group = parts[0];
                let artifact = parts[1];
                let version = parts[2].split(|c: char| c.is_whitespace()).next().unwrap_or(parts[2]);
                let coord = format!("{}:{}:{}", group, artifact, version);

                if processed_coords.insert(coord.clone()) {
                    let is_scala = artifact.contains("_2.13") || artifact.contains("_2.12") || artifact.contains("_3");

                    deps.push(Dependency::Maven {
                        group: group.to_string(),
                        artifact: artifact.to_string(),
                        version: version.to_string(),
                        is_scala,
                    });
                }
            }
        }
    }

    Ok(deps)
}