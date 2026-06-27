//! 所有内置命令的执行逻辑 (new / init / build / run / add / test / workspace)

use crate::cli::{Commands, WorkspaceCommands};
use crate::toolkit::file::ProjectCreator;
use crate::toolkit::path::{paths, PathManager};
use crate::toolkit::template::Template;
pub async fn execute_command(command: Commands, cwd: &PathManager) -> anyhow::Result<()> {
    match command {
        Commands::New { name, backend } => cmd_new(cwd, &name, &backend).await,
        Commands::Init => cmd_init(cwd).await,
        Commands::Workspace { subcommand } => cmd_workspace(cwd, &subcommand).await,
        Commands::Build => execute_build(cwd).await,
        Commands::Run { file, lib } => execute_run(cwd, file.map(PathManager::from), lib).await,
        Commands::Add { deps } => execute_add(cwd, &deps).await,
        Commands::Test { file } => cmd_test(cwd, file.map(PathManager::from)).await,
        Commands::Plugin { name, .. } => anyhow::bail!("Plugin '{}' was not handled", name),
    }
}

pub async fn execute_default(cwd: &PathManager) -> anyhow::Result<()> {
    if cwd.join("project.toml").exists_sync() {
        let project = crate::config::load_project(cwd)?;
        let target = project.get_main_file_path();
        if cwd.join(&target).exists_sync() {
            let deps = crate::deps::get_dependencies(&project);
            let output = crate::build::run_single_file_with_deps(cwd, &target, &deps).await?;
            println!("{}", output);
        } else { println!("未找到主文件: {}", target.display()); }
    } else { println!("未提供命令。使用 --help 获取用法。"); }
    Ok(())
}

// ━━━━━━━ new (multi-backend) ━━━━━━━

pub async fn cmd_new(cwd: &PathManager, name: &str, backend: &str) -> anyhow::Result<()> {
    let proj_dir = cwd.join(name);
    if proj_dir.exists_sync() { println!("项目 '{}' 已存在", name); return Ok(()); }

    let creator = ProjectCreator::new(&proj_dir);
    creator.create_dirs(&["src/main/scala"]).await?;
    let scala_ver = "2.13";

    // project.toml
    let tpl = paths::project_template().read_sync()?;
    let mut m = Template::new(&tpl).replace("name", name);
    m.replace("scala_version", scala_ver);
    m.replace("backend", backend);
    let manifest = m.into_string();
    creator.write_file("project.toml", &manifest).await?;

    // Main.scala
    let code = paths::main_template().read_sync()?;
    creator.write_file("src/main/scala/Main.scala", &code).await?;

    // backend-specific
    match backend {
        "sbt" => {
            let raw = paths::template_file("build.sbt.template").read_sync().unwrap_or_default();
            let mut t = Template::new(&raw).replace("name", name);
            t.replace("scala_version", scala_ver);
            creator.write_file("build.sbt", &t.into_string()).await?;
            creator.create_dirs(&["project"]).await?;
            creator.write_file("project/plugins.sbt", r#"addSbtPlugin("ch.epfl.scala" % "sbt-scalafix" % "0.11.1")"#).await?;
            println!("Generated build.sbt + project/plugins.sbt");
        }
        "maven" => {
            let raw = paths::template_file("pom.xml.template").read_sync().unwrap_or_default();
            let group = format!("com.{}", name);
            let mut t = Template::new(&raw).replace("{group}", &group);
            t.replace("{name}", name);
            t.replace("{scala_version}", scala_ver);
            creator.write_file("pom.xml", &t.into_string()).await?;
            println!("Generated pom.xml");
        }
        "gradle" => {
            let raw = paths::template_file("build.gradle.template").read_sync().unwrap_or_default();
            let mut t = Template::new(&raw).replace("{scala_version}", scala_ver);
            creator.write_file("build.gradle", &t.into_string()).await?;
            creator.write_file("settings.gradle", &format!("rootProject.name = '{}'", name)).await?;
            println!("Generated build.gradle + settings.gradle");
        }
        _ => println!("Using scala-cli backend"),
    }

    if let Some(root) = crate::config::find_workspace_root(cwd) {
        let mpath = root.join("project.toml");
        let rel = proj_dir.strip_prefix(&root).unwrap_or(&proj_dir).to_string_lossy().to_string();
        match crate::config::add_workspace_member(&mpath, &rel) {
            Ok(_) => println!("已添加项目 '{}' 到工作空间", name),
            Err(e) => if !e.to_string().contains("already exists") { eprintln!("Warning: {}", e); }
        }
    }
    println!("已创建项目 `{}` (backend: {})", name, backend);
    Ok(())
}

// ━━━━━━━ init ━━━━━━━

pub async fn cmd_init(cwd: &PathManager) -> anyhow::Result<()> {
    let mp = cwd.join("project.toml");
    if mp.exists_sync() { anyhow::bail!("project.toml 已存在于此目录"); }
    mp.write_sync(&paths::workspace_template().read_sync()?)?;
    println!("已初始化空工作空间于 {}", cwd.to_path_buf().display());
    Ok(())
}

// ━━━━━━━ build ━━━━━━━

async fn execute_build(cwd: &PathManager) -> anyhow::Result<()> {
    let proj = crate::config::load_project(cwd).map_err(|_| anyhow::anyhow!("No project.toml in {}", cwd.display()))?;
    if proj.workspace.is_some() {
        let (root, members) = crate::config::load_workspace(cwd)?.ok_or_else(|| anyhow::anyhow!("Failed to load workspace"))?;
        let mut ad = Vec::new(); let mut sd = Vec::new(); let mut bk = None;
        for m in &members {
            let md = cwd.join(m.get_name());
            let td = crate::deps::get_transitive_dependencies_with_workspace(m, Some(&root), &md).await?;
            ad.extend(td.clone()); sd.push((m.get_name().to_string(), m.get_source_dir().to_string()));
            if bk.is_none() { bk = Some(m.get_backend().to_string()); }
            crate::build::build_with_deps(&md, &td, m.get_source_dir(), &format!("{}/{}", root.get_target_dir(), m.get_name()),
                m.get_backend(), Some(cwd), false, true).await?;
            println!("已构建成员: {}", m.get_name());
        }
        if let Some(b) = bk { crate::ide::setup_bsp(cwd, &ad, &sd, &b).await?; }
        println!("工作空间构建成功");
    } else if let Some(wr) = crate::config::find_workspace_root(cwd) {
        if let Some((root, members)) = crate::config::load_workspace(&wr)? {
            let rel = cwd.strip_prefix(&wr).map_err(|_| anyhow::anyhow!("Invalid workspace structure"))?;
            let mn = rel.components().next().and_then(|c| c.as_os_str().to_str()).ok_or_else(|| anyhow::anyhow!("Cannot determine member name"))?;
            if let Some(m) = members.into_iter().find(|x| x.get_name() == mn) {
                let td = crate::deps::get_transitive_dependencies_with_workspace(&m, Some(&root), cwd).await?;
                crate::build::build_with_deps(cwd, &td, m.get_source_dir(), m.get_target_dir(), m.get_backend(), Some(&wr), false, false).await?;
                println!("构建成功，包含 {} 个依赖", td.len());
            } else { anyhow::bail!("Member {} not found", mn); }
        } else { build_single(cwd, &proj).await?; }
    } else { build_single(cwd, &proj).await?; }
    Ok(())
}

async fn build_single(cwd: &PathManager, proj: &crate::models::Project) -> anyhow::Result<()> {
    let td = crate::deps::get_transitive_dependencies_with_workspace(proj, None, cwd).await?;
    crate::build::build_with_deps(cwd, &td, proj.get_source_dir(), proj.get_target_dir(), proj.get_backend(), None, true, false).await?;
    println!("构建成功，包含 {} 个依赖", td.len());
    Ok(())
}

// ━━━━━━━ run ━━━━━━━

async fn execute_run(cwd: &PathManager, file: Option<PathManager>, lib: bool) -> anyhow::Result<()> {
    let wr = crate::config::find_workspace_root(cwd);
    let (proj, pdir) = resolve_project(cwd, wr.as_deref()).await?;
    let deps = if let Some(ref w) = wr { let wp = crate::config::load_project(w)?; crate::deps::get_dependencies_with_workspace(&proj, Some(&wp)) }
        else { crate::deps::get_dependencies(&proj) };

    let target = file.unwrap_or_else(|| PathManager::from(proj.get_main_file_path()));
    if !pdir.join(target.as_path()).exists_sync() { anyhow::bail!("File not found: {}", target.to_path_buf().display()); }

    if lib {
        let _ = crate::build::run_scala_file(&pdir, &target, true).await?;
        println!("库: {} (仅编译)", target.display());
    } else if proj.get_backend() != "scala-cli" {
        crate::build::run_with_backend(&pdir, &target, &deps, proj.get_backend()).await?;
    } else {
        let out = crate::build::run_single_file_with_deps(&pdir, &target, &deps).await?;
        println!("{}", out);
    }
    Ok(())
}

// ━━━━━━━ add ━━━━━━━

async fn execute_add(cwd: &PathManager, deps: &[String]) -> anyhow::Result<()> {
    let wr = crate::config::find_workspace_root(cwd);
    let pd = wr.map(PathManager::from).unwrap_or_else(|| cwd.clone());
    for d in deps { crate::deps::add_dependency(&pd.to_path_buf(), d).await?; }
    Ok(())
}

// ━━━━━━━ test ━━━━━━━

pub async fn cmd_test(cwd: &PathManager, file: Option<PathManager>) -> anyhow::Result<()> {
    let wr = crate::config::find_workspace_root(cwd);
    let (proj, pdir) = resolve_project(cwd, wr.as_deref()).await?;
    let deps = if let Some(ref w) = wr { let wp = crate::config::load_project(w)?; crate::deps::get_dependencies_with_workspace(&proj, Some(&wp)) }
        else { crate::deps::get_dependencies(&proj) };
    let tt = file.unwrap_or_else(|| PathManager::new(&proj.package.test_dir));
    let at = pdir.join(tt.as_path());
    if !at.exists_sync() { println!("No tests found in {}", tt.to_path_buf().display()); return Ok(()); }

    match proj.get_backend() {
        "scala-cli" => {
            let mut args: Vec<String> = vec!["test".to_string(), at.to_string_lossy().to_string()];
            for d in &deps { args.push("--dependency".to_string()); args.push(d.coord()); }
            let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let out = crate::build::execute_scala_cli(&args_str, Some(&pdir)).await?;
            if let Some(parent) = at.parent() { let _ = tokio::fs::remove_dir_all(parent.join(".bsp")).await; let _ = tokio::fs::remove_dir_all(parent.join(".scala-build")).await; }
            if !out.is_empty() { println!("{}", out); }
        }
        "sbt" => crate::build::run_build_tool("sbt", &["test"], &pdir).await?,
        "maven" => crate::build::run_build_tool("mvn", &["test"], &pdir).await?,
        "gradle" => crate::build::run_build_tool("gradle", &["test"], &pdir).await?,
        _ => anyhow::bail!("Unsupported backend for testing: {}", proj.get_backend()),
    }
    Ok(())
}

// ━━━━━━━ workspace ━━━━━━━

pub async fn cmd_workspace(cwd: &PathManager, sub: &WorkspaceCommands) -> anyhow::Result<()> {
    match sub { WorkspaceCommands::Add { paths } => cmd_workspace_add(cwd, paths).await? }
    Ok(())
}

async fn cmd_workspace_add(cwd: &PathManager, paths: &[String]) -> anyhow::Result<()> {
    let wr = crate::config::find_workspace_root(cwd).ok_or_else(|| anyhow::anyhow!("不在工作空间中"))?;
    let mp = wr.join("project.toml");
    for p in paths { match crate::config::add_workspace_member(&mp, p) { Ok(_) => println!("已添加成员 '{}'", p), Err(_) => println!("成员 '{}' 已存在", p) } }
    Ok(())
}

// ━━━━━━━ helpers ━━━━━━━

async fn resolve_project(cwd: &PathManager, wr: Option<&std::path::Path>) -> anyhow::Result<(crate::models::Project, PathManager)> {
    if let Some(wr) = wr {
        if let Some((_, members)) = crate::config::load_workspace(wr)? {
            // Determine member name from cwd relative to workspace root
            let mn = cwd.strip_prefix(wr)
                .ok()
                .and_then(|rp| rp.components().next())
                .and_then(|c| c.as_os_str().to_str().map(String::from))
                .ok_or_else(|| anyhow::anyhow!("Cannot determine member name from path"))?;
            if let Some(m) = members.into_iter().find(|x| x.package.name == mn) {
                return Ok((m, PathManager::from(wr.join(&mn))));
            }
        }
    }
    let p = crate::config::load_project(cwd)?;
    Ok((p, cwd.clone()))
}
