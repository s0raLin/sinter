//! 内置命令实现
//!
//! 包含所有内置命令的执行逻辑

use crate::build::{run_scala_file, run_single_file_with_deps};
use crate::cli::{
    commands::{cmd_test, cmd_workspace},
    Commands,
};
use crate::config::loader;
use crate::deps::add_dependency;
use crate::error::{utils, Result};
use crate::ide::setup_bsp;
use crate::toolkit::file::ProjectCreator;
use crate::toolkit::path::paths;
use crate::toolkit::path::PathManager;
use crate::toolkit::template::Template;

/// 执行内置命令
pub async fn execute_command(command: Commands, cwd: &PathManager) -> Result<()> {
    match command {
        Commands::New { name } => {
            create_project(&name, cwd).await?;
        }
        Commands::Init => {
            init_workspace(cwd).await?;
        }
        Commands::Workspace { subcommand } => {
            cmd_workspace(cwd, &subcommand).await?;
        }
        Commands::Build => {
            execute_build(cwd).await?;
        }
        Commands::Run { file, lib } => {
            execute_run(cwd, file.map(PathManager::from), lib).await?;
        }
        Commands::Add { deps } => {
            execute_add(cwd, &deps).await?;
        }
        Commands::Test { file } => {
            cmd_test(cwd, file.map(PathManager::from)).await?;
        }
        Commands::Jsp { name } => {
            // JSP 命令应该由插件系统处理
            return Err(crate::error::utils::single_validation_error(format!(
                "JSP command '{}' requires the JSP plugin to be loaded",
                name
            )));
        }
    }
    Ok(())
}

/// 执行默认行为（无命令时）
pub async fn execute_default(cwd: &PathManager) -> Result<()> {
    if cwd.join("project.toml").exists_sync() {
        let project = loader::load_project(cwd).map_err(crate::error::utils::from_anyhow)?;
        let target = project.get_main_file_path();
        if cwd.join(&target).exists_sync() {
            let deps = crate::dependency::get_dependencies(&project);
            let output = run_single_file_with_deps(cwd, &target, &deps)
                .await
                .map_err(crate::error::utils::from_anyhow)?;
            println!("{}", output);
        } else {
            println!("{}", format!("未找到主文件: {}", target.display()));
        }
    } else {
        println!("{}", "未提供命令。使用 --help 获取用法。");
    }
    Ok(())
}

/// 执行构建命令
async fn execute_build(cwd: &PathManager) -> Result<()> {
    if let Ok(project) = loader::load_project(cwd) {
        if project.workspace.is_some() {
            // Workspace build - build all members
            let (root_project, members) = loader::load_workspace(cwd)?
                .ok_or_else(|| anyhow::anyhow!("Failed to load workspace configuration"))?;
            let mut all_deps = Vec::new();
            let mut source_dirs = Vec::new();
            let mut backend = None;
            for member in members.iter() {
                let member_dir = cwd.join(member.get_name());
                let transitive_deps =
                    crate::dependency::get_transitive_dependencies_with_workspace(
                        &member,
                        Some(&root_project),
                        &member_dir,
                    )
                    .await?;
                all_deps.extend(transitive_deps.clone());
                source_dirs.push((
                    member.get_name().to_string(),
                    member.get_source_dir().to_string(),
                ));
                if backend.is_none() {
                    backend = Some(member.get_backend().to_string());
                }
                // For workspace builds, use target directory relative to workspace root
                let workspace_target_dir =
                    format!("{}/{}", root_project.get_target_dir(), member.get_name());
                crate::build::build_with_deps(
                    &member_dir,
                    &transitive_deps,
                    member.get_source_dir(),
                    &workspace_target_dir,
                    member.get_backend(),
                    Some(cwd),
                    false, // Do not setup BSP for each member
                    true,  // is_workspace_build
                )
                .await?;
                println!("{}", format!("已构建成员: {}", member.get_name()));
            }
            // Setup BSP for the entire workspace
            if let Some(bk) = backend {
                setup_bsp(cwd, &all_deps, &source_dirs, &bk).await?;
            }
            println!("{}", "工作空间构建成功");
        } else {
            // Single project or member in workspace
            if let Some(workspace_root) = crate::config::loader::find_workspace_root(cwd) {
                // Build single member in workspace
                if let Some((root_project, members)) = loader::load_workspace(&workspace_root)? {
                    let relative_path = cwd
                        .strip_prefix(&workspace_root)
                        .map_err(|_| anyhow::anyhow!("Invalid workspace structure"))?;
                    let member_name = relative_path
                        .components()
                        .next()
                        .and_then(|c| c.as_os_str().to_str())
                        .ok_or_else(|| anyhow::anyhow!("Cannot determine member name from path"))?;
                    if let Some(member) = members.into_iter().find(|m| m.get_name() == member_name)
                    {
                        let transitive_deps =
                            crate::dependency::get_transitive_dependencies_with_workspace(
                                &member,
                                Some(&root_project),
                                cwd,
                            )
                            .await?;
                        crate::build::build_with_deps(
                            cwd,
                            &transitive_deps,
                            member.get_source_dir(),
                            member.get_target_dir(),
                            member.get_backend(),
                            Some(&workspace_root),
                            false, // Do not setup BSP for individual member, will setup for workspace
                            false, // not workspace build
                        )
                        .await?;
                        println!(
                            "{}",
                            format!("构建成功，包含 {} 个依赖", transitive_deps.len())
                        );
                    } else {
                        return Err(crate::error::utils::single_validation_error(format!(
                            "Member {} not found in workspace",
                            member_name
                        )));
                    }
                } else {
                    // Not in a workspace, treat as single project
                    let transitive_deps =
                        crate::dependency::get_transitive_dependencies_with_workspace(
                            &project, None, cwd,
                        )
                        .await?;
                    crate::build::build_with_deps(
                        cwd,
                        &transitive_deps,
                        project.get_source_dir(),
                        project.get_target_dir(),
                        project.get_backend(),
                        None,
                        true,  // Setup BSP for IDE support
                        false, // not workspace build
                    )
                    .await?;
                    println!(
                        "{}",
                        format!("构建成功，包含 {} 个依赖", transitive_deps.len())
                    );
                }
            } else {
                // Single project build
                let transitive_deps =
                    crate::dependency::get_transitive_dependencies_with_workspace(
                        &project, None, cwd,
                    )
                    .await?;
                crate::build::build_with_deps(
                    cwd,
                    &transitive_deps,
                    project.get_source_dir(),
                    project.get_target_dir(),
                    project.get_backend(),
                    None,
                    true,  // Setup BSP for IDE support
                    false, // not workspace build
                )
                .await?;
                println!(
                    "{}",
                    format!("构建成功，包含 {} 个依赖", transitive_deps.len())
                );
            }
        }
    } else {
        return Err(crate::error::utils::single_validation_error(format!(
            "No project.toml found in {}",
            cwd.display()
        )));
    }
    Ok(())
}

/// 执行运行命令
async fn execute_run(
    cwd: &PathManager,
    file: Option<PathManager>,
    lib: bool,
) -> anyhow::Result<()> {
    let workspace_root = crate::config::loader::find_workspace_root(cwd);
    let workspace_root_ref = workspace_root.as_ref();

    // 确定项目配置和目录
    let (project, project_dir) = if let Some(ws_root) = workspace_root_ref {
        // 在 workspace 中，查找成员项目
        if let Some((_ws_proj, members)) = crate::config::loader::load_workspace(ws_root)? {
            let relative_path = cwd.relative_to(&PathManager::from(ws_root.clone()));
            let member_name = relative_path
                .as_path()
                .components()
                .next()
                .and_then(|c| c.as_os_str().to_str())
                .ok_or_else(|| anyhow::anyhow!("Cannot determine member name from path"))?;
            if let Some(member) = members.into_iter().find(|m| m.package.name == member_name) {
                (member, PathManager::from(ws_root.clone()).join(member_name))
            } else {
                // 不是成员，作为单个项目处理
                let proj = crate::config::loader::load_project(cwd)?;
                (proj, cwd.clone())
            }
        } else {
            // 实际上不是 workspace，作为单个项目处理
            let proj = crate::config::loader::load_project(cwd)?;
            (proj, cwd.clone())
        }
    } else {
        let proj = crate::config::loader::load_project(cwd)?;
        (proj, cwd.clone())
    };

    // 获取依赖
    let deps = if let Some(ws_root) = workspace_root_ref {
        let ws_proj = crate::config::loader::load_project(ws_root)?;
        crate::dependency::get_dependencies_with_workspace(&project, Some(&ws_proj))
    } else {
        crate::dependency::get_dependencies(&project)
    };

    // 设置 BSP 以支持 IDE
    // For workspace, setup BSP at workspace root with all members
    // For single project, setup BSP at project root
    let (bsp_dir, source_dirs, all_deps, backend) = if let Some(ws_root) = workspace_root_ref {
        // In workspace, setup BSP for entire workspace
        let ws_root_path = ws_root.as_path();
        if let Some((_ws_proj, members)) = crate::config::loader::load_workspace(ws_root_path)? {
            let mut ws_source_dirs = Vec::new();
            let mut ws_all_deps = Vec::new();
            let mut ws_backend = None;

            for member in members.iter() {
                let member_dir = PathManager::from(ws_root_path).join(member.get_name());
                let member_deps = crate::dependency::get_transitive_dependencies_with_workspace(
                    member,
                    Some(&crate::config::loader::load_project(ws_root_path)?),
                    member_dir.as_path(),
                )
                .await?;
                ws_all_deps.extend(member_deps);
                ws_source_dirs.push((
                    member.get_name().to_string(),
                    member.get_source_dir().to_string(),
                ));
                if ws_backend.is_none() {
                    ws_backend = Some(member.get_backend().to_string());
                }
            }

            (
                PathManager::from(ws_root_path),
                ws_source_dirs,
                ws_all_deps,
                ws_backend.unwrap_or_else(|| project.get_backend().to_string()),
            )
        } else {
            // Not actually a workspace, treat as single project
            let bsp_dir = project_dir.clone();
            let source_dirs = vec![("".to_string(), project.get_source_dir().to_string())];
            (
                bsp_dir,
                source_dirs,
                deps.clone(),
                project.get_backend().to_string(),
            )
        }
    } else {
        // Single project
        let bsp_dir = project_dir.clone();
        let source_dirs = vec![("".to_string(), project.get_source_dir().to_string())];
        (
            bsp_dir,
            source_dirs,
            deps.clone(),
            project.get_backend().to_string(),
        )
    };

    setup_bsp(bsp_dir.as_path(), &all_deps, &source_dirs, &backend).await?;

    let target = file.unwrap_or_else(|| PathManager::from(project.get_main_file_path()));

    if !project_dir.join(target.as_path()).exists_sync() {
        anyhow::bail!("File not found: {}", target.to_path_buf().display());
    }

    if lib {
        let _ = run_scala_file(&project_dir, &target, true).await?;
        println!("{}", format!("库: {} (仅编译)", target.display()));
    } else {
        let output = run_single_file_with_deps(&project_dir, &target, &deps).await?;
        println!("{}", output);
    }

    Ok(())
}

/// 执行添加依赖命令
async fn execute_add(cwd: &PathManager, deps: &[String]) -> anyhow::Result<()> {
    let workspace_root = loader::find_workspace_root(cwd);
    let project_dir = workspace_root
        .map(PathManager::from)
        .unwrap_or_else(|| cwd.clone());
    for dep in deps {
        add_dependency(&project_dir.to_path_buf(), dep).await?;
    }
    Ok(())
}

/// 创建新项目
async fn create_project(name: &str, cwd: &PathManager) -> Result<()> {
    let proj_dir = cwd.join(name);
    if proj_dir.exists_sync() {
        println!("{}", format!("项目 '{}' 已存在", name));
        return Ok(());
    }

    let creator = ProjectCreator::new(&proj_dir);
    creator.create_dirs(&["src/main/scala"]).await?;

    // project.toml
    let template_path = paths::project_template();
    let template_content = template_path.read_sync()?;
    let template = Template::new(&template_content);
    let manifest = template.replace("name", name).into_string();
    creator.write_file("project.toml", &manifest).await?;

    // Hello world
    let main_template_path = paths::main_template();
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
                println!("{}", format!("已添加项目 '{}' 到工作空间", name));
            }
            Err(e) => {
                if !e.to_string().contains("already exists") {
                    eprintln!("Warning: Failed to add project to workspace: {}", e);
                }
            }
        }
    }

    println!("{}", format!("已创建项目 `{}`", name));
    Ok(())
}

/// 初始化工作区
async fn init_workspace(cwd: &PathManager) -> Result<()> {
    // Check if project.toml already exists
    let manifest_path = cwd.join("project.toml");
    if manifest_path.exists_sync() {
        return Err(utils::single_validation_error(
            "project.toml 已存在于此目录".to_string(),
        ));
    }

    // Create workspace project.toml
    let template_path = paths::workspace_template();
    let manifest = template_path.read_sync()?;
    manifest_path.write_sync(&manifest)?;

    println!(
        "{}",
        format!("已初始化空工作空间于 {}", cwd.to_path_buf().display())
    );
    Ok(())
}
