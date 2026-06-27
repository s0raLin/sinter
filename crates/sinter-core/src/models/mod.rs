use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DependencySpec
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum DependencySpec {
    Simple(String),
    Detailed(DependencyDetail),
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct DependencyDetail {
    pub version: Option<String>,
    #[serde(default)]
    pub workspace: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum DependencyDto {
    Simple(String),
    Detailed(DependencyDetailDto),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DependencyDetailDto {
    pub version: Option<String>,
    #[serde(default)]
    pub workspace: bool,
}

impl DependencySpec {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        match self {
            DependencySpec::Simple(dep_str) => {
                if dep_str.trim().is_empty() {
                    errors.push("依赖字符串不能为空".to_string());
                } else if !dep_str.contains(':') {
                    if !is_valid_version(dep_str) { errors.push(format!("依赖版本格式无效: '{}'", dep_str)); }
                } else {
                    let colon_count = dep_str.matches(':').count();
                    if colon_count < 2 {
                        errors.push(format!("依赖格式无效 '{}'，应为 'group:artifact:version'", dep_str));
                    } else if let Some(last_colon_pos) = dep_str.rfind(':') {
                        let version = &dep_str[last_colon_pos + 1..];
                        if dep_str[..last_colon_pos].trim().is_empty() {
                            errors.push("依赖的 group:artifact 部分不能为空".to_string());
                        }
                        if !is_valid_version(version) { errors.push(format!("依赖版本格式无效: '{}'", version)); }
                    }
                }
            }
            DependencySpec::Detailed(detail) => {
                if let Some(version) = &detail.version {
                    if version.trim().is_empty() { errors.push("依赖版本不能为空".to_string()); }
                    else if !is_valid_version(version) { errors.push(format!("依赖版本格式无效: '{}'", version)); }
                }
                if detail.workspace && detail.version.is_some() { errors.push("工作空间依赖不应指定版本".to_string()); }
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }

    pub fn get_version(&self) -> Option<&str> {
        match self {
            DependencySpec::Simple(coord) => coord.split(':').nth(2),
            DependencySpec::Detailed(detail) => detail.version.as_deref(),
        }
    }
}

fn is_valid_version(version: &str) -> bool {
    !version.is_empty() && version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_')
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Project / Package / BuildConfig
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone)]
pub struct Project {
    pub root_path: PathBuf,
    pub package: Package,
    pub dependencies: HashMap<String, DependencySpec>,
    pub workspace: Option<Workspace>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProjectDto {
    pub package: PackageDto,
    #[serde(default)]
    pub dependencies: HashMap<String, DependencyDto>,
    pub workspace: Option<WorkspaceDto>,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub main: Option<String>,
    pub scala_version: String,
    pub source_dir: String,
    pub target_dir: String,
    pub test_dir: String,
    pub backend: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PackageDto {
    pub name: String,
    pub version: String,
    pub main: Option<String>,
    #[serde(default = "default_scala_version")]
    pub scala_version: String,
    #[serde(default = "default_source_dir")]
    pub source_dir: String,
    #[serde(default = "default_target_dir")]
    pub target_dir: String,
    #[serde(default = "default_test_dir")]
    pub test_dir: String,
    #[serde(default = "default_backend")]
    pub backend: String,
}

impl Project {
    pub fn get_name(&self) -> &str { &self.package.name }
    pub fn get_backend(&self) -> &str { &self.package.backend }
    pub fn get_source_dir(&self) -> &str { &self.package.source_dir }
    pub fn get_target_dir(&self) -> &str { &self.package.target_dir }

    pub fn get_main_file_path(&self) -> PathBuf {
        let main_class = self.package.main.as_deref().unwrap_or("Main");
        self.root_path.join(&self.package.source_dir).join(format!("{}.scala", main_class))
    }

    pub fn is_workspace_root(&self) -> bool { self.workspace.is_some() }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if let Err(pkg_errors) = self.package.validate() { errors.extend(pkg_errors); }
        for (name, spec) in &self.dependencies {
            if name.trim().is_empty() { errors.push("依赖名称不能为空".to_string()); }
            if let Err(dep_errors) = spec.validate() {
                for error in dep_errors { errors.push(format!("依赖 '{}' 验证失败: {}", name, error)); }
            }
        }
        if let Some(workspace) = &self.workspace {
            if let Err(ws_errors) = workspace.validate() { errors.extend(ws_errors); }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

impl Package {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() { errors.push("项目名称不能为空".to_string()); }
        if self.version.trim().is_empty() { errors.push("项目版本不能为空".to_string()); }
        if !self.scala_version.starts_with("2.") && !self.scala_version.starts_with("3.") {
            errors.push("Scala版本格式无效，应为 2.x 或 3.x".to_string());
        }
        if self.source_dir.trim().is_empty() { errors.push("源代码目录不能为空".to_string()); }
        if self.target_dir.trim().is_empty() { errors.push("目标目录不能为空".to_string()); }
        let valid_backends = ["scala-cli", "sbt", "gradle", "maven"];
        if !valid_backends.contains(&self.backend.as_str()) {
            errors.push(format!("不支持的后端: {}，支持的后端: {}", self.backend, valid_backends.join(", ")));
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Workspace
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone)]
pub struct Workspace {
    pub root_path: PathBuf,
    pub members: Vec<String>,
    pub dependencies: HashMap<String, DependencySpec>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct WorkspaceDto {
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default)]
    pub dependencies: HashMap<String, DependencyDto>,
}

impl Workspace {
    pub fn with_root_path(mut self, root_path: PathBuf) -> Self { self.root_path = root_path; self }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.members.is_empty() { errors.push("工作空间成员列表不能为空".to_string()); }
        for (name, spec) in &self.dependencies {
            if let Err(dep_errors) = spec.validate() {
                for error in dep_errors { errors.push(format!("工作空间依赖 '{}' 验证失败: {}", name, error)); }
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// From impls
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

impl From<ProjectDto> for Project {
    fn from(dto: ProjectDto) -> Self {
        Self {
            root_path: PathBuf::new(),
            package: dto.package.into(),
            dependencies: dto.dependencies.into_iter().map(|(k, v)| (k, v.into())).collect(),
            workspace: dto.workspace.map(|ws| ws.into()),
        }
    }
}

impl From<PackageDto> for Package {
    fn from(dto: PackageDto) -> Self {
        Self {
            name: dto.name, version: dto.version, main: dto.main,
            scala_version: dto.scala_version, source_dir: dto.source_dir,
            target_dir: dto.target_dir, test_dir: dto.test_dir, backend: dto.backend,
        }
    }
}

impl From<WorkspaceDto> for Workspace {
    fn from(dto: WorkspaceDto) -> Self {
        Self { root_path: PathBuf::new(), members: dto.members, dependencies: dto.dependencies.into_iter().map(|(k, v)| (k, v.into())).collect() }
    }
}

impl From<DependencyDto> for DependencySpec {
    fn from(dto: DependencyDto) -> Self {
        match dto {
            DependencyDto::Simple(s) => DependencySpec::Simple(s),
            DependencyDto::Detailed(d) => DependencySpec::Detailed(DependencyDetail { version: d.version, workspace: d.workspace }),
        }
    }
}

fn default_scala_version() -> String { "2.13".to_string() }
fn default_source_dir() -> String { "src/main/scala".to_string() }
fn default_target_dir() -> String { "target".to_string() }
fn default_test_dir() -> String { "src/test/scala".to_string() }
fn default_backend() -> String { "scala-cli".to_string() }
