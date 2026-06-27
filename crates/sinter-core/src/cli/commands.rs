//! 所有内置命令的执行逻辑 (new / init / build / run / add / test / workspace)

use crate::cli::{Commands, WorkspaceCommands};
use crate::toolkit::file::ProjectCreator;
use crate::toolkit::path::{paths, PathManager};
use crate::toolkit::template::Template;

// ━━━━━━━━━━━━━━━━━━━ Main dispatcher ━━━━━━━━━━━━━━━━━━━

/// 执行内置命令
pub async fn execute_command(command: Commands, cwd: &PathManager) -> anyhow::Result<()> {
    match command {
        Commands::New { name } => cmd_new(cwd, &name).await?,
        Commands::Init => cmd_init(cwd).await?,
        Commands::Workspace { subcommand } => cmd_workspace(cwd, &subcommand).await?,
        Commands::Build => execute_build(cwd).await?,
        Commands::Run { file, lib } => execute_run(cwd, file.map(PathManager::from), lib).await?,
        Commands::Add { deps } => execute_add(cwd, &deps).await?,
        Commands::Test { file } => cmd_test(cwd, file.map(PathManager::from)).await?,
        Commands::Plugin { name, args: _ } => {
            anyhow::bail!("Plugin '{}' was not handled — this is a bug", name);
        }
    }
    Ok(())
}

/// 默认行为 (无命令时)
pub async fn execute_default(cwd: &PathManager) -> anyhow::Result<()> {
    if cwd.join("project.toml").exists_sync() {
        let project = crate::config::load_project(cwd)?;
        let target = project.get_main_file_path();
        if cwd.join(&target).exists_sync() {
            let deps = crate::deps::get_dependencies(&project);
            let output = crate::build::run_single_file_with_deps(cwd, &target, &deps).await?;
            println!("{}", output);
        } else {
            println!("未找到主文件: {}", target.display());
        }
    } else {
        println!("未提供命令。使用 --help 获取用法。");
    }
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━ new ━━━━━━━━━━━━━━━━━━━

pub async fn cmd_new(cwd: &PathManager, name: &str) -> anyhow::Result<()> {
    let proj_dir = cwd.join(name);
    if proj_dir.exists_sync() {
        println!("项目 '{}' 已存在", name);
        return Ok(());
    }

    let creator = ProjectCreator::new(&proj_dir);
    creator.create_dirs(&["src/main/scala"]).await?;

    let template_path = paths::project_template();
    let template_content = template_path.read_sync()?;
    let template = Template::new(&template_content);
    let manifest = template.replace("name", name).into_string();
    creator.write_file("project.toml", &manifest).await?;

    let main_template_path = paths::main_template();
    let code = main_template_path.read_sync()?;
    creator.write_file("src/main/scala/Main.scala", &code).await?;

    if let Some(workspace_root) = crate::config::find_workspace_root(cwd) {
        let manifest_path = workspace_root.join("project.toml");
        let relative_path = proj_dir.strip_prefix(&workspace_root)
            .unwrap_or(&proj_dir).to_string_lossy().to_string();
        match crate::config::add_workspace_member(&manifest_path, &relative_path) {
            Ok(_) => println!("已添加项目 '{}' 到工作空间", name),
            Err(e) => if !e.to_string().contains("already exists") {
                eprintln!("Warning: Failed to add project to workspace: {}", e);
            }
        }
    }

    println!("已创建项目 `{}`", name);
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━ init ━━━━━━━━━━━━━━━━━━━

pub async fn cmd_init(cwd: &PathManager) -> anyhow::Result<()> {
    let manifest_path = cwd.join("project.toml");
    if manifest_path.exists_sync() {
        anyhow::bail!("project.toml 已存在于此目录");
    }
    let template_path = paths::workspace_template();
    let manifest = template_path.read_sync()?;
    manifest_path.write_sync(&manifest)?;
    println!("已初始化空工作空间于 {}", cwd.to_path_buf().display());
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━ build ━━━━━━━━━━━━━━━━━━━

async fn execute_build(cwd: &PathManager) -> anyhow::Result<()> {
    let project = crate::config::load_project(cwd)
        .map_err(|_| anyhow::anyhow!("No project.toml found in {}", cwd.display()))?;

    if project.workspace.is_some() {
        let (root_project, members) = crate::config::load_workspace(cwd)?
            .ok_or_else(|| anyhow::anyhow!("Failed to load workspace configuration"))?;
        let mut all_deps = Vec::new();
        let mut source_dirs = Vec::new();
        let mut backend = None;
        for member in members.iter() {
            let member_dir = cwd.join(member.get_name());
            let transitive_deps = crate::deps::get_transitive_dependencies_with_workspace(
                member, Some(&root_project), &member_dir,
            ).await?;
            all_deps.extend(transitive_deps.clone());
            source_dirs.push((member.get_name().to_string(), member.get_source_dir().to_string()));
            if backend.is_none() { backend = Some(member.get_backend().to_string()); }
            let workspace_target_dir = format!("{}/{}", root_project.get_target_dir(), member.get_name());
            crate::build::build_with_deps(&member_dir, &transitive_deps, member.get_source_dir(),
                &workspace_target_dir, member.get_backend(), Some(cwd), false, true).await?;
            println!("已构建成员: {}", member.get_name());
        }
        if let Some(bk) = backend {
            crate::ide::setup_bsp(cwd, &all_deps, &source_dirs, &bk).await?;
        }
        println!("工作空间构建成功");
    } else if let Some(workspace_root) = crate::config::find_workspace_root(cwd) {
        if let Some((root_project, members)) = crate::config::load_workspace(&workspace_root)? {
            let relative_path = cwd.strip_prefix(&workspace_root)
                .map_err(|_| anyhow::anyhow!("Invalid workspace structure"))?;
            let member_name = relative_path.components().next()
                .and_then(|c| c.as_os_str().to_str())
                .ok_or_else(|| anyhow::anyhow!("Cannot determine member name from path"))?;
            if let Some(member) = members.into_iter().find(|m| m.get_name() == member_name) {
                let transitive_deps = crate::deps::get_transitive_dependencies_with_workspace(
                    &member, Some(&root_project), cwd).await?;
                crate::build::build_with_deps(cwd, &transitive_deps, member.get_source_dir(),
                    member.get_target_dir(), member.get_backend(), Some(&workspace_root), false, false).await?;
                println!("构建成功，包含 {} 个依赖", transitive_deps.len());
            } else {
                anyhow::bail!("Member {} not found in workspace", member_name);
            }
        } else {
            let transitive_deps = crate::deps::get_transitive_dependencies_with_workspace(&project, None, cwd).await?;
            crate::build::build_with_deps(cwd, &transitive_deps, project.get_source_dir(),
                project.get_target_dir(), project.get_backend(), None, true, false).await?;
            println!("构建成功，包含 {} 个依赖", transitive_deps.len());
        }
    } else {
        let transitive_deps = crate::deps::get_transitive_dependencies_with_workspace(&project, None, cwd).await?;
        crate::build::build_with_deps(cwd, &transitive_deps, project.get_source_dir(),
            project.get_target_dir(), project.get_backend(), None, true, false).await?;
        println!("构建成功，包含 {} 个依赖", transitive_deps.len());
    }
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━ run ━━━━━━━━━━━━━━━━━━━

async fn execute_run(cwd: &PathManager, file: Option<PathManager>, lib: bool) -> anyhow::Result<()> {
    let workspace_root = crate::config::find_workspace_root(cwd);
    let workspace_root_ref = workspace_root.as_ref();

    let (project, project_dir) = if let Some(ws_root) = workspace_root_ref {
        if let Some((_ws_proj, members)) = crate::config::load_workspace(ws_root)? {
            let relative_path = cwd.relative_to(&PathManager::from(ws_root.clone()));
            let member_name = relative_path.as_path().components().next()
                .and_then(|c| c.as_os_str().to_str())
                .ok_or_else(|| anyhow::anyhow!("Cannot determine member name from path"))?;
            if let Some(member) = members.into_iter().find(|m| m.package.name == member_name) {
                (member, PathManager::from(ws_root.clone()).join(member_name))
            } else {
                let proj = crate::config::load_project(cwd)?;
                (proj, cwd.clone())
            }
        } else {
            let proj = crate::config::load_project(cwd)?;
            (proj, cwd.clone())
        }
    } else {
        let proj = crate::config::load_project(cwd)?;
        (proj, cwd.clone())
    };

    let deps = if let Some(ws_root) = workspace_root_ref {
        let ws_proj = crate::config::load_project(ws_root)?;
        crate::deps::get_dependencies_with_workspace(&project, Some(&ws_proj))
    } else {
        crate::deps::get_dependencies(&project)
    };

    let (bsp_dir, source_dirs, all_deps, backend) = if let Some(ws_root) = workspace_root_ref {
        let ws_root_path = ws_root.as_path();
        if let Some((_ws_proj, members)) = crate::config::load_workspace(ws_root_path)? {
            let mut ws_source_dirs = Vec::new();
            let mut ws_all_deps = Vec::new();
            let mut ws_backend = None;
            for member in members.iter() {
                let member_dir = PathManager::from(ws_root_path).join(member.get_name());
                let member_deps = crate::deps::get_transitive_dependencies_with_workspace(
                    member, Some(&crate::config::load_project(ws_root_path)?), member_dir.as_path()
                ).await?;
                ws_all_deps.extend(member_deps);
                ws_source_dirs.push((member.get_name().to_string(), member.get_source_dir().to_string()));
                if ws_backend.is_none() { ws_backend = Some(member.get_backend().to_string()); }
            }
            (PathManager::from(ws_root_path), ws_source_dirs, ws_all_deps,
             ws_backend.unwrap_or_else(|| project.get_backend().to_string()))
        } else {
            let bsp_dir = project_dir.clone();
            let source_dirs = vec![("".to_string(), project.get_source_dir().to_string())];
            (bsp_dir, source_dirs, deps.clone(), project.get_backend().to_string())
        }
    } else {
        let bsp_dir = project_dir.clone();
        let source_dirs = vec![("".to_string(), project.get_source_dir().to_string())];
        (bsp_dir, source_dirs, deps.clone(), project.get_backend().to_string())
    };

    crate::ide::setup_bsp(bsp_dir.as_path(), &all_deps, &source_dirs, &backend).await?;

    let target = file.unwrap_or_else(|| PathManager::from(project.get_main_file_path()));
    if !project_dir.join(target.as_path()).exists_sync() {
        anyhow::bail!("File not found: {}", target.to_path_buf().display());
    }

    if lib {
        let _ = crate::build::run_scala_file(&project_dir, &target, true).await?;
        println!("库: {} (仅编译)", target.display());
    } else {
        let output = crate::build::run_single_file_with_deps(&project_dir, &target, &deps).await?;
        println!("{}", output);
    }
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━ add ━━━━━━━━━━━━━━━━━━━

async fn execute_add(cwd: &PathManager, deps: &[String]) -> anyhow::Result<()> {
    let workspace_root = crate::config::find_workspace_root(cwd);
    let project_dir = workspace_root.map(PathManager::from).unwrap_or_else(|| cwd.clone());
    for dep in deps {
        crate::deps::add_dependency(&project_dir.to_path_buf(), dep).await?;
    }
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━ test ━━━━━━━━━━━━━━━━━━━

pub async fn cmd_test(cwd: &PathManager, file: Option<PathManager>) -> anyhow::Result<()> {
    let workspace_root = crate::config::find_workspace_root(cwd);
    let (project, project_dir) = if let Some(ws_root) = workspace_root.as_ref() {
        if let Some((_ws_proj, members)) = crate::config::load_workspace(ws_root)? {
            let relative_path = cwd.relative_to(&PathManager::from(ws_root.clone()));
            if let Some(first_component) = relative_path.as_path().components().next() {
                let member_name = first_component.as_os_str().to_str().unwrap();
                if let Some(member) = members.into_iter().find(|m| m.package.name == member_name) {
                    (member, PathManager::from(ws_root.clone()).join(member_name))
                } else {
                    let proj = crate::config::load_project(cwd)?;
                    (proj, cwd.clone())
                }
            } else {
                let proj = crate::config::load_project(cwd)?;
                (proj, cwd.clone())
            }
        } else {
            let proj = crate::config::load_project(cwd)?;
            (proj, cwd.clone())
        }
    } else {
        let proj = crate::config::load_project(cwd)?;
        (proj, cwd.clone())
    };

    let deps = if let Some(ws_root) = workspace_root {
        let ws_proj = crate::config::load_project(&ws_root)?;
        crate::deps::get_dependencies_with_workspace(&project, Some(&ws_proj))
    } else {
        crate::deps::get_dependencies(&project)
    };

    let test_target = file.unwrap_or_else(|| PathManager::new(&project.package.test_dir));
    let abs_test_target = project_dir.join(test_target.as_path());
    if !abs_test_target.exists_sync() {
        println!("No tests found in {}", test_target.to_path_buf().display());
        return Ok(());
    }

    let mut args: Vec<String> = vec!["test".to_string(), abs_test_target.to_string_lossy().to_string()];
    for dep in deps {
        args.push("--dependency".to_string());
        args.push(dep.coord());
    }
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = crate::build::execute_scala_cli(&args_str, Some(&project_dir)).await?;
    // 清理 scala-cli 在测试目录生成的工件
    if let Some(parent) = abs_test_target.parent() {
        let _ = tokio::fs::remove_dir_all(parent.join(".bsp")).await;
        let _ = tokio::fs::remove_dir_all(parent.join(".scala-build")).await;
    }
    if !output.is_empty() { println!("{}", output); }
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━ workspace ━━━━━━━━━━━━━━━━━━━

pub async fn cmd_workspace(cwd: &PathManager, subcommand: &WorkspaceCommands) -> anyhow::Result<()> {
    match subcommand {
        WorkspaceCommands::Add { paths } => cmd_workspace_add(cwd, paths).await?,
    }
    Ok(())
}

async fn cmd_workspace_add(cwd: &PathManager, member_paths: &[String]) -> anyhow::Result<()> {
    let workspace_root = crate::config::find_workspace_root(cwd)
        .ok_or_else(|| anyhow::anyhow!("不在工作空间中"))?;
    let manifest_path = workspace_root.join("project.toml");
    for member_path in member_paths {
        match crate::config::add_workspace_member(&manifest_path, member_path) {
            Ok(_) => println!("已添加成员 '{}' 到工作空间", member_path),
            Err(_) => println!("成员 '{}' 已存在于工作空间", member_path),
        }
    }
    Ok(())
}
