// src/deps.rs
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
        // Check if it's an sbt path (starts with sbt: or is a relative path)
        if key.starts_with("sbt:")
            || (key.contains("/") && !key.contains("::") && !key.contains(":"))
        {
            let path = if key.starts_with("sbt:") {
                key[4..].to_string()
            } else {
                key.to_string()
            };
            Self::Sbt { path }
        } else {
            // Key can be:
            // - group::artifact (Scala format with ::)
            // - group:artifact (Java format, or Scala with _suffix)
            let is_scala_format = key.contains("::");

            let (group, artifact) = if is_scala_format {
                let parts: Vec<&str> = key.split("::").collect();
                if parts.len() >= 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    ("".to_string(), key.to_string())
                }
            } else if key.contains(":") {
                // Use splitn to only split into 2 parts: group:artifact -> ["group", "artifact"]
                let parts: Vec<&str> = key.splitn(2, ':').collect();
                if parts.len() >= 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    ("".to_string(), key.to_string())
                }
            } else {
                ("".to_string(), key.to_string())
            };

            // For Scala dependencies, the artifact typically ends with _2.13, _2.12, _3, etc.
            // Detect based on artifact suffix
            let is_scala = artifact.contains("_2.") || artifact.contains("_3");

            Self::Maven {
                group,
                artifact,
                version: version.to_string(),
                is_scala,
            }
        }
    }

    // 生成 Maven 坐标：group:artifact_scala_version:version 或 sbt 路径
    pub fn coord(&self) -> String {
        match self {
            Dependency::Maven {
                group,
                artifact,
                version,
                is_scala: _,
            } => {
                // Scala 依赖的 artifact 已经包含 Scala 版本后缀（如 cats-core_2.13）
                format!("{}:{}:{}", group, artifact, version)
            }
            Dependency::Sbt { path } => {
                format!("sbt:{}", path)
            }
        }
    }

    pub fn is_sbt(&self) -> bool {
        matches!(self, Dependency::Sbt { .. })
    }

    pub fn sbt_path(&self) -> Option<&str> {
        match self {
            Dependency::Sbt { path } => Some(path),
            _ => None,
        }
    }
}
