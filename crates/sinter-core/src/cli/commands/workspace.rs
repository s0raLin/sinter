use crate::cli::WorkspaceCommands;
use crate::toolkit::path::PathManager;

pub async fn cmd_workspace(
    cwd: &PathManager,
    subcommand: &WorkspaceCommands,
) -> anyhow::Result<()> {
    match subcommand {
        WorkspaceCommands::Add { paths } => {
            cmd_workspace_add(cwd, paths).await?;
        }
    }
    Ok(())
}

async fn cmd_workspace_add(cwd: &PathManager, member_paths: &[String]) -> anyhow::Result<()> {
    // Find workspace root
    let workspace_root = crate::config::loader::find_workspace_root(cwd)
        .ok_or_else(|| anyhow::anyhow!("{}", "不在工作空间中"))?;

    let manifest_path = workspace_root.join("project.toml");

    for member_path in member_paths {
        // Check if member already exists by trying to add it
        match crate::config::writer::add_workspace_member(&manifest_path, member_path) {
            Ok(_) => {
                println!("已添加成员 '{}' 到工作空间", member_path);
            }
            Err(_) => {
                println!("成员 '{}' 已存在于工作空间", member_path);
            }
        }
    }
    Ok(())
}
