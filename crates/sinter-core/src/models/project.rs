//! 项目配置模型和DTO

use super::dependency::DependencySpec;
use super::directory::Directory;
use super::library::Library;
use super::workspace::Workspace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 项目配置
#[derive(Debug, Clone)]
pub struct Project {
    /// 项目根目录路径 - 映射到文件系统中的实际目录
    pub root_path: PathBuf,
    pub package: Package,
    pub dependencies: HashMap<String, DependencySpec>,
    pub workspace: Option<Workspace>,
}

/// 项目DTO - 用于数据传输
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProjectDto {
    pub package: PackageDto,
    #[serde(default)]
    pub dependencies: HashMap<String, super::dependency::DependencyDto>,
    pub workspace: Option<super::workspace::WorkspaceDto>,
}

/// 包信息 - 领域对象
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

/// 包信息DTO - 用于数据传输
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
    /// 创建带有根路径的项目实例
    pub fn with_root_path(mut self, root_path: PathBuf) -> Self {
        self.root_path = root_path.clone();
        if let Some(ws) = &mut self.workspace {
            *ws = ws.clone().with_root_path(root_path);
        }
        self
    }

    /// 获取项目根目录的绝对路径
    pub fn get_root_path(&self) -> &PathBuf {
        &self.root_path
    }

    /// 获取主文件路径（绝对路径）
    pub fn get_main_file_path(&self) -> PathBuf {
        let main_class = self.package.main.as_deref().unwrap_or("Main");
        self.root_path
            .join(&self.package.source_dir)
            .join(format!("{}.scala", main_class))
    }

    /// 获取源代码目录路径（相对路径）
    pub fn get_source_dir(&self) -> &str {
        &self.package.source_dir
    }

    /// 获取源代码目录绝对路径
    pub fn get_source_dir_abs(&self) -> PathBuf {
        self.root_path.join(&self.package.source_dir)
    }

    /// 获取目标目录路径（相对路径）
    pub fn get_target_dir(&self) -> &str {
        &self.package.target_dir
    }

    /// 获取目标目录绝对路径
    pub fn get_target_dir_abs(&self) -> PathBuf {
        self.root_path.join(&self.package.target_dir)
    }

    /// 获取测试目录路径（相对路径）
    pub fn get_test_dir(&self) -> &str {
        &self.package.test_dir
    }

    /// 获取测试目录绝对路径
    pub fn get_test_dir_abs(&self) -> PathBuf {
        self.root_path.join(&self.package.test_dir)
    }

    /// 获取构建后端
    pub fn get_backend(&self) -> &str {
        &self.package.backend
    }

    /// 获取项目名称
    pub fn get_name(&self) -> &str {
        &self.package.name
    }

    /// 获取项目版本
    pub fn get_version(&self) -> &str {
        &self.package.version
    }

    /// 获取Scala版本
    pub fn get_scala_version(&self) -> &str {
        &self.package.scala_version
    }

    /// 获取所有依赖（包括工作空间级别的）
    pub fn get_all_dependencies(&self) -> HashMap<String, &DependencySpec> {
        let mut deps = HashMap::new();

        // 添加项目级依赖
        for (name, spec) in &self.dependencies {
            deps.insert(name.clone(), spec);
        }

        // 添加工作空间级依赖（如果存在）
        if let Some(workspace) = &self.workspace {
            for (name, spec) in &workspace.dependencies {
                deps.insert(name.clone(), spec);
            }
        }

        deps
    }

    /// 检查是否为工作空间根项目
    pub fn is_workspace_root(&self) -> bool {
        self.workspace.is_some()
    }

    /// 获取工作空间配置（如果存在）
    pub fn get_workspace(&self) -> Option<&Workspace> {
        self.workspace.as_ref()
    }

    /// 获取项目目录实体
    pub fn get_directory(&self) -> Directory {
        Directory::from_path(&self.root_path)
    }

    /// 获取源代码目录实体
    pub fn get_source_directory(&self) -> Directory {
        Directory::from_path(self.get_source_dir_abs())
    }

    /// 获取目标目录实体
    pub fn get_target_directory(&self) -> Directory {
        Directory::from_path(self.get_target_dir_abs())
    }

    /// 获取测试目录实体
    pub fn get_test_directory(&self) -> Directory {
        Directory::from_path(self.get_test_dir_abs())
    }

    /// 获取所有依赖库实体
    pub fn get_libraries(&self) -> Vec<Library> {
        self.get_all_dependencies()
            .into_iter()
            .map(|(name, spec)| Library::from_dependency_spec(name, spec.clone()))
            .collect()
    }

    /// 验证项目配置
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // 验证包信息
        if let Err(pkg_errors) = self.package.validate() {
            errors.extend(pkg_errors);
        }

        // 验证目录路径
        let source_dir = self.get_source_directory();
        if let Err(dir_errors) = source_dir.validate() {
            errors.extend(
                dir_errors
                    .into_iter()
                    .map(|e| format!("源代码目录错误: {}", e)),
            );
        }

        // 验证依赖
        for (name, spec) in &self.dependencies {
            if name.trim().is_empty() {
                errors.push("依赖名称不能为空".to_string());
            }
            if let Err(dep_errors) = spec.validate() {
                for error in dep_errors {
                    errors.push(format!("依赖 '{}' 验证失败: {}", name, error));
                }
            }
        }

        // 验证工作空间
        if let Some(workspace) = &self.workspace {
            if let Err(ws_errors) = workspace.validate() {
                errors.extend(ws_errors);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 转换为DTO
    pub fn to_dto(&self) -> ProjectDto {
        ProjectDto {
            package: self.package.to_dto(),
            dependencies: self
                .dependencies
                .iter()
                .map(|(k, v)| (k.clone(), v.to_dto()))
                .collect(),
            workspace: self.workspace.as_ref().map(|ws| ws.to_dto()),
        }
    }
}

impl Package {
    /// 验证包信息
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.name.trim().is_empty() {
            errors.push("项目名称不能为空".to_string());
        }

        if self.version.trim().is_empty() {
            errors.push("项目版本不能为空".to_string());
        }

        // 验证Scala版本格式
        if !self.scala_version.starts_with("2.") && !self.scala_version.starts_with("3.") {
            errors.push("Scala版本格式无效，应为 2.x 或 3.x".to_string());
        }

        // 验证目录路径
        if self.source_dir.trim().is_empty() {
            errors.push("源代码目录不能为空".to_string());
        }

        if self.target_dir.trim().is_empty() {
            errors.push("目标目录不能为空".to_string());
        }

        // 验证后端
        let valid_backends = ["scala-cli", "sbt", "gradle", "maven"];
        if !valid_backends.contains(&self.backend.as_str()) {
            errors.push(format!(
                "不支持的后端: {}，支持的后端: {}",
                self.backend,
                valid_backends.join(", ")
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 转换为DTO
    pub fn to_dto(&self) -> PackageDto {
        PackageDto {
            name: self.name.clone(),
            version: self.version.clone(),
            main: self.main.clone(),
            scala_version: self.scala_version.clone(),
            source_dir: self.source_dir.clone(),
            target_dir: self.target_dir.clone(),
            test_dir: self.test_dir.clone(),
            backend: self.backend.clone(),
        }
    }
}

impl From<ProjectDto> for Project {
    fn from(dto: ProjectDto) -> Self {
        Self {
            root_path: PathBuf::new(), // 需要外部设置
            package: dto.package.into(),
            dependencies: dto
                .dependencies
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            workspace: dto.workspace.map(|ws| ws.into()),
        }
    }
}

impl From<PackageDto> for Package {
    fn from(dto: PackageDto) -> Self {
        Self {
            name: dto.name,
            version: dto.version,
            main: dto.main,
            scala_version: dto.scala_version,
            source_dir: dto.source_dir,
            target_dir: dto.target_dir,
            test_dir: dto.test_dir,
            backend: dto.backend,
        }
    }
}

fn default_scala_version() -> String {
    "2.13".to_string()
}

fn default_source_dir() -> String {
    "src/main/scala".to_string()
}

fn default_target_dir() -> String {
    "target".to_string()
}

fn default_test_dir() -> String {
    "src/test/scala".to_string()
}

fn default_backend() -> String {
    "scala-cli".to_string()
}
