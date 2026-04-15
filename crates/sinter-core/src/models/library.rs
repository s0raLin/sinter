//! 依赖库模型和DTO
//!
//! 映射外部依赖库实体

use super::dependency::DependencySpec;
use std::path::PathBuf;

/// 依赖库实体 - 映射到实际的外部库或本地库
#[derive(Debug, Clone)]
pub struct Library {
    /// 库名称
    pub name: String,
    /// 依赖规范
    pub spec: DependencySpec,
    /// 本地路径（如果已解析）
    pub local_path: Option<PathBuf>,
    /// 是否已下载/可用
    pub available: bool,
    /// 库类型
    pub library_type: LibraryType,
}

/// 库类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryType {
    /// 外部Maven/Ivy仓库库
    External,
    /// 本地文件系统库
    Local,
    /// 工作空间内部库
    Workspace,
}

/// 库DTO - 用于数据传输
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct LibraryDto {
    pub name: String,
    pub spec: super::dependency::DependencyDto,
    pub local_path: Option<PathBuf>,
    pub available: bool,
    pub library_type: LibraryTypeDto,
}

/// 库类型DTO
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub enum LibraryTypeDto {
    External,
    Local,
    Workspace,
}

impl Library {
    /// 从依赖规范创建库实例
    pub fn from_dependency_spec(name: String, spec: DependencySpec) -> Self {
        let library_type = match &spec {
            DependencySpec::Simple(_) => LibraryType::External,
            DependencySpec::Detailed(detail) => {
                if detail.workspace {
                    LibraryType::Workspace
                } else if detail.version.is_some() {
                    LibraryType::External
                } else {
                    LibraryType::Local
                }
            }
        };

        Self {
            name,
            spec,
            local_path: None,
            available: false,
            library_type,
        }
    }

    /// 设置本地路径
    pub fn with_local_path(mut self, path: PathBuf) -> Self {
        self.local_path = Some(path);
        self.available = true;
        self
    }

    /// 标记为可用
    pub fn mark_available(mut self) -> Self {
        self.available = true;
        self
    }

    /// 获取库坐标字符串
    pub fn get_coordinate(&self) -> Option<String> {
        self.spec.to_coordinate()
    }

    /// 检查是否为工作空间库
    pub fn is_workspace_library(&self) -> bool {
        self.library_type == LibraryType::Workspace
    }

    /// 检查是否为外部库
    pub fn is_external_library(&self) -> bool {
        self.library_type == LibraryType::External
    }

    /// 检查是否为本地库
    pub fn is_local_library(&self) -> bool {
        self.library_type == LibraryType::Local
    }

    /// 获取库的显示名称
    pub fn get_display_name(&self) -> String {
        if let Some(coord) = self.get_coordinate() {
            format!("{} ({})", self.name, coord)
        } else {
            self.name.clone()
        }
    }

    /// 获取库的版本（如果有）
    pub fn get_version(&self) -> Option<&str> {
        self.spec.get_version()
    }

    /// 验证库配置
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.name.trim().is_empty() {
            errors.push("库名称不能为空".to_string());
        }

        if let Err(spec_errors) = self.spec.validate() {
            errors.extend(
                spec_errors
                    .into_iter()
                    .map(|e| format!("库 '{}' 依赖错误: {}", self.name, e)),
            );
        }

        // 检查本地路径是否存在（如果指定了）
        if let Some(path) = &self.local_path {
            if !path.exists() {
                errors.push(format!(
                    "库 '{}' 的本地路径不存在: {}",
                    self.name,
                    path.display()
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 转换为DTO
    pub fn to_dto(&self) -> LibraryDto {
        LibraryDto {
            name: self.name.clone(),
            spec: self.spec.to_dto(),
            local_path: self.local_path.clone(),
            available: self.available,
            library_type: self.library_type.to_dto(),
        }
    }
}

impl LibraryType {
    fn to_dto(&self) -> LibraryTypeDto {
        match self {
            LibraryType::External => LibraryTypeDto::External,
            LibraryType::Local => LibraryTypeDto::Local,
            LibraryType::Workspace => LibraryTypeDto::Workspace,
        }
    }
}

impl From<LibraryDto> for Library {
    fn from(dto: LibraryDto) -> Self {
        Self {
            name: dto.name,
            spec: dto.spec.into(),
            local_path: dto.local_path,
            available: dto.available,
            library_type: dto.library_type.into(),
        }
    }
}

impl From<LibraryTypeDto> for LibraryType {
    fn from(dto: LibraryTypeDto) -> Self {
        match dto {
            LibraryTypeDto::External => LibraryType::External,
            LibraryTypeDto::Local => LibraryType::Local,
            LibraryTypeDto::Workspace => LibraryType::Workspace,
        }
    }
}

impl From<(String, DependencySpec)> for Library {
    fn from((name, spec): (String, DependencySpec)) -> Self {
        Self::from_dependency_spec(name, spec)
    }
}
