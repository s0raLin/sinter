use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

#[derive(Clone, Debug)]
pub struct PathWrapper(PathBuf);

impl PathWrapper {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        PathWrapper(path.as_ref().to_path_buf())
    }

    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathWrapper {
        PathWrapper(self.0.join(path))
    }

    pub fn parent(&self) -> Option<PathWrapper> {
        self.0.parent().map(|p| PathWrapper(p.to_path_buf()))
    }

    pub fn file_name(&self) -> Option<String> {
        self.0.file_name()?.to_str().map(|s| s.to_string())
    }

    pub fn extension(&self) -> Option<String> {
        self.0.extension()?.to_str().map(|s| s.to_string())
    }

    pub fn relative_to(&self, base: &PathWrapper) -> PathWrapper {
        // Simplified relative path calculation
        if let Ok(rel) = self.0.strip_prefix(&base.0) {
            PathWrapper(rel.to_path_buf())
        } else {
            self.clone()
        }
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl std::ops::Deref for PathWrapper {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<PathBuf> for PathWrapper {
    fn from(path: PathBuf) -> Self {
        PathWrapper(path)
    }
}

impl From<&Path> for PathWrapper {
    fn from(path: &Path) -> Self {
        PathWrapper(path.to_path_buf())
    }
}

pub async fn pwd() -> Result<PathWrapper, std::io::Error> {
    Ok(PathWrapper(std::env::current_dir()?))
}

pub async fn read(path: &PathWrapper) -> Result<String, std::io::Error> {
    fs::read_to_string(&path.0).await
}

pub async fn write(path: &PathWrapper, content: &str) -> Result<(), std::io::Error> {
    fs::write(&path.0, content).await
}

pub async fn exists(path: &PathWrapper) -> bool {
    fs::metadata(&path.0).await.is_ok()
}

pub async fn is_file(path: &PathWrapper) -> bool {
    fs::metadata(&path.0)
        .await
        .map(|m| m.is_file())
        .unwrap_or(false)
}

pub async fn is_dir(path: &PathWrapper) -> bool {
    fs::metadata(&path.0)
        .await
        .map(|m| m.is_dir())
        .unwrap_or(false)
}

pub async fn size(path: &PathWrapper) -> Result<u64, std::io::Error> {
    fs::metadata(&path.0).await.map(|m| m.len())
}

pub async fn mtime(path: &PathWrapper) -> Result<std::time::SystemTime, std::io::Error> {
    fs::metadata(&path.0).await.and_then(|m| m.modified())
}

pub async fn list(path: &PathWrapper) -> Result<Vec<PathWrapper>, std::io::Error> {
    let mut entries = fs::read_dir(&path.0).await?;
    let mut paths = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        paths.push(PathWrapper(entry.path()));
    }
    Ok(paths)
}

pub async fn make_dir(path: &PathWrapper) -> Result<(), std::io::Error> {
    fs::create_dir(&path.0).await
}

pub async fn make_dir_all(path: &PathWrapper) -> Result<(), std::io::Error> {
    fs::create_dir_all(&path.0).await
}

pub async fn copy(from: &PathWrapper, to: &PathWrapper) -> Result<u64, std::io::Error> {
    fs::copy(&from.0, &to.0).await
}

pub async fn remove(path: &PathWrapper) -> Result<(), std::io::Error> {
    fs::remove_file(&path.0).await
}

pub async fn remove_all(path: &PathWrapper) -> Result<(), std::io::Error> {
    fs::remove_dir_all(&path.0).await
}

pub fn walk(path: &PathWrapper) -> impl Iterator<Item = PathWrapper> {
    WalkDir::new(&path.0)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| PathWrapper(e.path().to_path_buf()))
}

pub fn walk_files(path: &PathWrapper) -> impl Iterator<Item = PathWrapper> {
    walk(path).filter(|p| p.is_file())
}

pub fn walk_dirs(path: &PathWrapper) -> impl Iterator<Item = PathWrapper> {
    walk(path).filter(|p| p.is_dir())
}

// Synchronous versions for compatibility
pub fn read_sync(path: &PathWrapper) -> Result<String, std::io::Error> {
    std::fs::read_to_string(&path.0)
}

pub fn write_sync(path: &PathWrapper, content: &str) -> Result<(), std::io::Error> {
    std::fs::write(&path.0, content)
}

pub fn exists_sync(path: &PathWrapper) -> bool {
    std::fs::metadata(&path.0).is_ok()
}

pub fn create_dir_all_sync(path: &PathWrapper) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(&path.0)
}

pub fn remove_file_sync(path: &PathWrapper) -> Result<(), std::io::Error> {
    std::fs::remove_file(&path.0)
}

pub fn remove_dir_all_sync(path: &PathWrapper) -> Result<(), std::io::Error> {
    std::fs::remove_dir_all(&path.0)
}
