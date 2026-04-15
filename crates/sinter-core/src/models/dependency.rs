//! 依赖模型和DTO

use serde::{Deserialize, Serialize};

/// 依赖规范
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum DependencySpec {
    Simple(String),
    Detailed(DependencyDetail),
}

/// 依赖详情
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct DependencyDetail {
    pub version: Option<String>,
    #[serde(default)]
    pub workspace: bool,
}

/// 依赖DTO - 用于数据传输和序列化
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum DependencyDto {
    Simple(String),
    Detailed(DependencyDetailDto),
}

/// 依赖详情DTO - 用于数据传输
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DependencyDetailDto {
    pub version: Option<String>,
    #[serde(default)]
    pub workspace: bool,
}

impl DependencySpec {
    /// 验证依赖规范
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        match self {
            DependencySpec::Simple(dep_str) => {
                if dep_str.trim().is_empty() {
                    errors.push("依赖字符串不能为空".to_string());
                } else {
                    // 如果 value 不包含冒号，说明是简写格式（key = version）
                    // 完整坐标应该从 key 获取，这里先跳过验证
                    if !dep_str.contains(':') {
                        // 简写格式，验证将在解析时通过 coord() 处理
                        // 这里只需要验证版本格式
                        if !is_valid_version(dep_str) {
                            errors.push(format!("依赖版本格式无效: '{}'", dep_str));
                        }
                    } else {
                        // 支持两种格式：
                        // 1. group:artifact:version (Java依赖)
                        // 2. group:artifact_scala_version:version (Scala依赖，artifact可能包含_)
                        let colon_count = dep_str.matches(':').count();
                        if colon_count < 2 {
                            errors.push(format!(
                                "依赖格式无效 '{}'，应为 'group:artifact:version'",
                                dep_str
                            ));
                        } else {
                            // 找到最后一个冒号的位置来分离 version
                            if let Some(last_colon_pos) = dep_str.rfind(':') {
                                let version = &dep_str[last_colon_pos + 1..];
                                let group_artifact = &dep_str[..last_colon_pos];

                                // 检查 group 和 artifact 不为空
                                if group_artifact.trim().is_empty() {
                                    errors.push("依赖的 group:artifact 部分不能为空".to_string());
                                }

                                // 版本格式检查
                                if !is_valid_version(version) {
                                    errors.push(format!("依赖版本格式无效: '{}'", version));
                                }
                            } else {
                                errors.push(format!("依赖格式无效 '{}'", dep_str));
                            }
                        }
                    }
                }
            }
            DependencySpec::Detailed(detail) => {
                if let Some(version) = &detail.version {
                    if version.trim().is_empty() {
                        errors.push("依赖版本不能为空".to_string());
                    } else if !is_valid_version(version) {
                        errors.push(format!("依赖版本格式无效: '{}'", version));
                    }
                }
                // workspace依赖不需要版本
                if detail.workspace && detail.version.is_some() {
                    errors.push("工作空间依赖不应指定版本".to_string());
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 获取依赖坐标字符串
    pub fn to_coordinate(&self) -> Option<String> {
        match self {
            DependencySpec::Simple(coord) => Some(coord.clone()),
            DependencySpec::Detailed(_) => None, // 需要外部提供group和artifact
        }
    }

    /// 检查是否为工作空间依赖
    pub fn is_workspace_dependency(&self) -> bool {
        match self {
            DependencySpec::Simple(_) => false,
            DependencySpec::Detailed(detail) => detail.workspace,
        }
    }

    /// 获取版本
    pub fn get_version(&self) -> Option<&str> {
        match self {
            DependencySpec::Simple(coord) => coord.split(':').nth(2),
            DependencySpec::Detailed(detail) => detail.version.as_deref(),
        }
    }

    /// 转换为DTO
    pub fn to_dto(&self) -> DependencyDto {
        match self {
            DependencySpec::Simple(s) => DependencyDto::Simple(s.clone()),
            DependencySpec::Detailed(d) => DependencyDto::Detailed(DependencyDetailDto {
                version: d.version.clone(),
                workspace: d.workspace,
            }),
        }
    }
}

impl From<DependencyDto> for DependencySpec {
    fn from(dto: DependencyDto) -> Self {
        match dto {
            DependencyDto::Simple(s) => DependencySpec::Simple(s),
            DependencyDto::Detailed(d) => DependencySpec::Detailed(DependencyDetail {
                version: d.version,
                workspace: d.workspace,
            }),
        }
    }
}

impl From<DependencySpec> for DependencyDto {
    fn from(spec: DependencySpec) -> Self {
        spec.to_dto()
    }
}

/// 验证版本字符串格式
fn is_valid_version(version: &str) -> bool {
    // 简单的版本格式检查：允许数字、点、横线、下划线
    !version.is_empty()
        && version
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_')
}
