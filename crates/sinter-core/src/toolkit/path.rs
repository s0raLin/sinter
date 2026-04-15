use crate::toolkit::os::PathWrapper;
use std::path::{Component, Path, PathBuf};

/// 统一的路径管理工具
/// 提供一致的路径操作接口，避免直接使用 std::path::PathBuf 和 PathWrapper 的混乱
#[derive(Clone, Debug)]
pub struct PathManager(PathWrapper);

impl PathManager {
    /// 创建新的路径管理器
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        PathManager(PathWrapper::new(path))
    }

    /// 获取当前工作目录
    pub async fn current_dir() -> Result<Self, std::io::Error> {
        Ok(PathManager(crate::toolkit::os::pwd().await?))
    }

    /// 连接路径
    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathManager {
        PathManager(self.0.join(path))
    }

    /// 获取父目录
    pub fn parent(&self) -> Option<PathManager> {
        self.0.parent().map(PathManager)
    }

    /// 获取文件名
    pub fn file_name(&self) -> Option<String> {
        self.0.file_name()
    }

    /// 获取扩展名
    pub fn extension(&self) -> Option<String> {
        self.0.extension()
    }

    /// 计算相对路径
    pub fn relative_to(&self, base: &PathManager) -> PathManager {
        PathManager(self.0.relative_to(&base.0))
    }

    /// 检查路径是否存在
    pub async fn exists(&self) -> bool {
        crate::toolkit::os::exists(&self.0).await
    }

    /// 检查是否是文件
    pub async fn is_file(&self) -> bool {
        crate::toolkit::os::is_file(&self.0).await
    }

    /// 检查是否是目录
    pub async fn is_dir(&self) -> bool {
        crate::toolkit::os::is_dir(&self.0).await
    }

    /// 读取文件内容
    pub async fn read(&self) -> Result<String, std::io::Error> {
        crate::toolkit::os::read(&self.0).await
    }

    /// 写入文件内容
    pub async fn write(&self, content: &str) -> Result<(), std::io::Error> {
        crate::toolkit::os::write(&self.0, content).await
    }

    /// 创建目录
    pub async fn create_dir(&self) -> Result<(), std::io::Error> {
        crate::toolkit::os::make_dir(&self.0).await
    }

    /// 创建目录（递归）
    pub async fn create_dir_all(&self) -> Result<(), std::io::Error> {
        crate::toolkit::os::make_dir_all(&self.0).await
    }

    /// 删除文件
    pub async fn remove_file(&self) -> Result<(), std::io::Error> {
        crate::toolkit::os::remove(&self.0).await
    }

    /// 删除目录（递归）
    pub async fn remove_dir_all(&self) -> Result<(), std::io::Error> {
        crate::toolkit::os::remove_all(&self.0).await
    }

    /// 复制文件
    pub async fn copy_to(&self, dest: &PathManager) -> Result<u64, std::io::Error> {
        crate::toolkit::os::copy(&self.0, &dest.0).await
    }

    /// 获取文件大小
    pub async fn size(&self) -> Result<u64, std::io::Error> {
        crate::toolkit::os::size(&self.0).await
    }

    /// 获取修改时间
    pub async fn mtime(&self) -> Result<std::time::SystemTime, std::io::Error> {
        crate::toolkit::os::mtime(&self.0).await
    }

    /// 列出目录内容
    pub async fn list(&self) -> Result<Vec<PathManager>, std::io::Error> {
        let paths = crate::toolkit::os::list(&self.0).await?;
        Ok(paths.into_iter().map(PathManager).collect())
    }

    /// 遍历目录
    pub fn walk(&self) -> impl Iterator<Item = PathManager> {
        crate::toolkit::os::walk(&self.0).map(PathManager)
    }

    /// 遍历文件
    pub fn walk_files(&self) -> impl Iterator<Item = PathManager> {
        crate::toolkit::os::walk_files(&self.0).map(PathManager)
    }

    /// 遍历目录
    pub fn walk_dirs(&self) -> impl Iterator<Item = PathManager> {
        crate::toolkit::os::walk_dirs(&self.0).map(PathManager)
    }

    /// 转换为 Path
    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    /// 转换为 PathBuf
    pub fn to_path_buf(&self) -> PathBuf {
        self.0.as_path().to_path_buf()
    }

    /// 获取内部 PathWrapper（用于兼容性）
    pub fn inner(&self) -> &PathWrapper {
        &self.0
    }
}

impl std::ops::Deref for PathManager {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.0.as_path()
    }
}

impl From<PathBuf> for PathManager {
    fn from(path: PathBuf) -> Self {
        PathManager(PathWrapper::from(path))
    }
}

impl From<&Path> for PathManager {
    fn from(path: &Path) -> Self {
        PathManager(PathWrapper::from(path))
    }
}

impl From<PathWrapper> for PathManager {
    fn from(wrapper: PathWrapper) -> Self {
        PathManager(wrapper)
    }
}

impl AsRef<Path> for PathManager {
    fn as_ref(&self) -> &Path {
        self.0.as_path()
    }
}

/// 同步版本的路径操作（用于兼容性）
impl PathManager {
    /// 同步读取文件
    pub fn read_sync(&self) -> Result<String, std::io::Error> {
        crate::toolkit::os::read_sync(&self.0)
    }

    /// 同步写入文件
    pub fn write_sync(&self, content: &str) -> Result<(), std::io::Error> {
        crate::toolkit::os::write_sync(&self.0, content)
    }

    /// 同步检查存在
    pub fn exists_sync(&self) -> bool {
        crate::toolkit::os::exists_sync(&self.0)
    }

    /// 同步创建目录（递归）
    pub fn create_dir_all_sync(&self) -> Result<(), std::io::Error> {
        crate::toolkit::os::create_dir_all_sync(&self.0)
    }

    /// 同步删除文件
    pub fn remove_file_sync(&self) -> Result<(), std::io::Error> {
        crate::toolkit::os::remove_file_sync(&self.0)
    }

    /// 同步删除目录（递归）
    pub fn remove_dir_all_sync(&self) -> Result<(), std::io::Error> {
        crate::toolkit::os::remove_dir_all_sync(&self.0)
    }
}

/// 路径常量和工具函数
pub mod paths {
    use super::PathManager;
    use std::path::{Path, PathBuf};

    /// 获取模板目录路径
    pub fn templates_dir() -> PathManager {
        // 在运行时，模板文件相对于可执行文件的路径
        // 通常在 target/debug 或 target/release 目录中
        let exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        let exe_dir = exe_path.parent().unwrap_or_else(|| Path::new("."));
        // 向上查找 crates/sinter-core 目录
        let mut current = PathManager::from(exe_dir);
        loop {
            let templates_dir = current.join("../../crates/sinter-core/templates");
            if templates_dir.exists_sync() {
                return templates_dir;
            }
            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                break;
            }
        }
        // 回退到相对路径
        PathManager::new("templates")
    }

    /// 获取模板文件路径
    pub fn template_file(name: &str) -> PathManager {
        templates_dir().join(name)
    }

    /// 获取项目模板路径
    pub fn project_template() -> PathManager {
        template_file("project.toml.template")
    }

    /// 获取主文件模板路径
    pub fn main_template() -> PathManager {
        template_file("main.scala.template")
    }

    /// 获取工作空间模板路径
    pub fn workspace_template() -> PathManager {
        template_file("workspace.project.toml.template")
    }

    /// 获取 BSP 相关模板路径
    pub fn bsp_templates() -> Vec<PathManager> {
        vec![
            template_file(".classpath.template"),
            template_file(".scala-build.template"),
            template_file("ide-options-v2.json.template"),
        ]
    }

    /// 获取插件模板目录
    pub fn plugin_templates_dir() -> PathManager {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        PathManager::new(manifest_dir).join("../sinter-plugins/templates")
    }

    /// 获取插件模板文件路径
    pub fn plugin_template_file(name: &str) -> PathManager {
        plugin_templates_dir().join(name)
    }
}

/// 路径验证和规范化工具
pub mod validation {
    use super::PathManager;
    use std::path::Path;

    /// 验证路径是否安全（不包含 .. 或绝对路径）
    pub fn is_safe_path(path: &Path) -> bool {
        !path.to_string_lossy().contains("..") && !path.is_absolute()
    }

    /// 规范化路径，移除 .. 和 .
    pub fn normalize_path(path: &Path) -> PathManager {
        // 简单的规范化实现
        let components: Vec<_> = path.components().collect();
        let mut normalized = Vec::new();

        for component in components {
            match component {
                std::path::Component::Normal(name) => normalized.push(name),
                std::path::Component::ParentDir => {
                    normalized.pop(); // 移除上一级
                }
                std::path::Component::CurDir => {
                    // 忽略当前目录 .
                }
                _ => {} // 忽略其他组件
            }
        }

        let normalized_path = normalized
            .iter()
            .fold(std::path::PathBuf::new(), |mut acc, comp| {
                acc.push(comp);
                acc
            });

        PathManager::new(normalized_path)
    }

    /// 检查路径是否在指定目录内
    pub fn is_within_dir(path: &PathManager, dir: &PathManager) -> bool {
        path.as_path().starts_with(dir.as_path())
    }
}
