use crate::toolkit::file::ProjectCreator;
use crate::toolkit::path::PathManager;
use crate::toolkit::template::Template;

pub async fn cmd_new(cwd: &PathManager, name: &str) -> anyhow::Result<()> {
    let proj_dir = cwd.join(name);
    if proj_dir.exists_sync() {
        println!("项目 '{}' 已存在", name);
        return Ok(());
    }

    let creator = ProjectCreator::new(&proj_dir);
    creator.create_dirs(&["src/main/scala"]).await?;

    // project.toml
    let template_path = crate::toolkit::path::paths::project_template();
    let template_content = template_path.read_sync()?;
    let template = Template::new(&template_content);
    let manifest = template.replace("name", name).into_string();
    creator.write_file("project.toml", &manifest).await?;

    // Hello world
    let main_template_path = crate::toolkit::path::paths::main_template();
    let code = main_template_path.read_sync()?;
    creator
        .write_file("src/main/scala/Main.scala", &code)
        .await?;

    // Auto-add to workspace if in one
    if let Some(workspace_root) = crate::config::loader::find_workspace_root(cwd) {
        let manifest_path = workspace_root.join("project.toml");
        let relative_path = proj_dir
            .strip_prefix(&workspace_root)
            .unwrap_or(&proj_dir)
            .to_string_lossy()
            .to_string();
        match crate::config::writer::add_workspace_member(&manifest_path, &relative_path) {
            Ok(_) => {
                println!("已添加项目 '{}' 到工作空间", name);
            }
            Err(e) => {
                if !e.to_string().contains("already exists") {
                    eprintln!("Warning: Failed to add project to workspace: {}", e);
                }
            }
        }
    }

    println!("已创建项目 `{}`", name);
    Ok(())
}
