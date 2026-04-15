use crate::toolkit::os::{make_dir_all, write, PathWrapper};
use std::path::Path;

/// Create a directory structure and write files
pub struct ProjectCreator {
    base_path: PathWrapper,
}

impl ProjectCreator {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        ProjectCreator {
            base_path: PathWrapper::new(base_path),
        }
    }

    pub async fn create_dirs(&self, dirs: &[&str]) -> Result<(), std::io::Error> {
        for dir in dirs {
            let path = self.base_path.join(dir);
            make_dir_all(&path).await?;
        }
        Ok(())
    }

    pub async fn write_file(
        &self,
        relative_path: &str,
        content: &str,
    ) -> Result<(), std::io::Error> {
        let path = self.base_path.join(relative_path);
        write(&path, content).await
    }

    pub async fn write_template_file(
        &self,
        relative_path: &str,
        template: &str,
        replacements: &std::collections::HashMap<&str, &str>,
    ) -> Result<(), std::io::Error> {
        let mut content = template.to_string();
        for (key, value) in replacements {
            content = content.replace(&format!("{{{}}}", key), value);
        }
        self.write_file(relative_path, &content).await
    }
}

/// Utility for downloading files
pub async fn download_file(
    url: &str,
    dest_path: &PathWrapper,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    write(dest_path, std::str::from_utf8(&bytes)?).await?;
    Ok(())
}

/// Set executable permissions (Unix only)
#[cfg(unix)]
pub fn set_executable(path: &PathWrapper) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&path.as_path())?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path.as_path(), perms)
}

#[cfg(not(unix))]
pub fn set_executable(_path: &PathWrapper) -> Result<(), std::io::Error> {
    Ok(())
}
