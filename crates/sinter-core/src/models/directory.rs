//! 目录模型
//!
//! 映射文件系统中的目录实体

use std::fs;
use std::path::{Path, PathBuf};

/// 目录实体 - 映射到文件系统中的实际目录
#[derive(Debug, Clone)]
pub struct Directory {
    /// 目录的绝对路径
    pub path: PathBuf,
    /// 目录名称
    pub name: String,
    /// 是否存在
    pub exists: bool,
}

impl Directory {
    /// 从路径创建目录实例
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let path_buf = path.as_ref().to_path_buf();
        let name = path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let exists = path_buf.exists();

        Self {
            path: path_buf,
            name,
            exists,
        }
    }

    /// 创建目录（如果不存在）
    pub fn create(&self) -> std::io::Result<()> {
        if !self.exists {
            fs::create_dir_all(&self.path)?;
        }
        Ok(())
    }

    /// 删除目录
    pub fn remove(&self) -> std::io::Result<()> {
        if self.exists {
            fs::remove_dir_all(&self.path)?;
        }
        Ok(())
    }

    /// 获取子目录
    pub fn get_subdirectory(&self, name: &str) -> Directory {
        let sub_path = self.path.join(name);
        Directory::from_path(sub_path)
    }

    /// 获取父目录
    pub fn get_parent(&self) -> Option<Directory> {
        self.path.parent().map(Directory::from_path)
    }

    /// 列出子目录
    pub fn list_subdirectories(&self) -> std::io::Result<Vec<Directory>> {
        if !self.exists {
            return Ok(Vec::new());
        }

        let mut subdirs = Vec::new();
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                subdirs.push(Directory::from_path(path));
            }
        }
        Ok(subdirs)
    }

    /// 检查是否为空目录
    pub fn is_empty(&self) -> std::io::Result<bool> {
        if !self.exists {
            return Ok(true);
        }

        Ok(self.path.read_dir()?.next().is_none())
    }

    /// 获取目录大小（递归计算）
    pub fn get_size(&self) -> std::io::Result<u64> {
        if !self.exists {
            return Ok(0);
        }

        let mut size = 0u64;
        self.calculate_size_recursive(&mut size)?;
        Ok(size)
    }

    /// 验证目录状态
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if !self.exists {
            errors.push(format!("目录不存在: {}", self.path.display()));
        } else if !self.path.is_dir() {
            errors.push(format!("路径不是目录: {}", self.path.display()));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn calculate_size_recursive(&self, size: &mut u64) -> std::io::Result<()> {
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let subdir = Directory::from_path(path);
                subdir.calculate_size_recursive(size)?;
            } else if path.is_file() {
                *size += entry.metadata()?.len();
            }
        }
        Ok(())
    }
}

impl AsRef<Path> for Directory {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl From<PathBuf> for Directory {
    fn from(path: PathBuf) -> Self {
        Self::from_path(path)
    }
}

impl From<&Path> for Directory {
    fn from(path: &Path) -> Self {
        Self::from_path(path)
    }
}
