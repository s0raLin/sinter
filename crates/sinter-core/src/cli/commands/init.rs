use crate::toolkit::path::{paths, PathManager};

pub async fn cmd_init(cwd: &PathManager) -> anyhow::Result<()> {
    // Check if project.toml already exists
    let manifest_path = cwd.join("project.toml");
    if manifest_path.exists_sync() {
        anyhow::bail!("{}", "project.toml 已存在于此目录");
    }

    // Create workspace project.toml
    let template_path = paths::workspace_template();
    let manifest = template_path.read_sync()?;
    manifest_path.write_sync(&manifest)?;

    println!("已初始化空工作空间于 {}", cwd.to_path_buf().display());
    Ok(())
}
