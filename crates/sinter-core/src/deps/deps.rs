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
            // Check for Scala dependency (::) or Java dependency (:)
            let is_scala = key.contains("::");
            let parts: Vec<&str> = if is_scala {
                key.split("::").collect()
            } else if key.contains(":") {
                key.split(":").collect()
            } else {
                vec!["", key]
            };

            let (group, artifact) = if parts.len() >= 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                ("".to_string(), key.to_string())
            };
            Self::Maven {
                group,
                artifact,
                version: version.to_string(),
                is_scala,
            }
        }
    }

    // 生成 Maven 坐标：group:artifact:version 或 group::artifact:version 或 sbt 路径
    pub fn coord(&self) -> String {
        match self {
            Dependency::Maven {
                group,
                artifact,
                version,
                is_scala,
            } => {
                if *is_scala {
                    format!("{}::{}:{}", group, artifact, version)
                } else {
                    format!("{}:{}:{}", group, artifact, version)
                }
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
