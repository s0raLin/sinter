//! 构建运行器 — 单文件运行、带依赖构建、多后端命令转发

use crate::deps::Dependency;
use std::path::Path;

// ━━━━━━━━━━━━━━━━━━━ Common helpers ━━━━━━━━━━━━━━━━━━━

pub fn has_main_method(content: &str) -> bool {
    content.contains("def main(") || content.contains("extends App") || content.contains("@main")
}

#[derive(Debug, PartialEq)]
pub enum RunMode { App, Lib }

pub struct RunResult { pub mode: RunMode, pub output: String }

async fn clean_artifacts(proj_dir: &Path, source_path: &Path) {
    let _ = tokio::fs::remove_dir_all(source_path.join(".bsp")).await;
    let _ = tokio::fs::remove_dir_all(source_path.join(".scala-build")).await;
    let root_json = proj_dir.join("scala-cli.json");
    if root_json.exists() { let _ = tokio::fs::remove_file(&root_json).await; }
}

// ━━━━━━━━━━━━━━━━━━━ scala-cli runners ━━━━━━━━━━━━━━━━━━━

pub async fn run_scala_file(proj_dir: &Path, file_path: &Path, force_lib: bool) -> anyhow::Result<RunResult> {
    let abs_file = proj_dir.join(file_path);
    let content = tokio::fs::read_to_string(&abs_file).await?;
    let has_main = has_main_method(&content);
    let mode = if force_lib || !has_main { RunMode::Lib } else { RunMode::App };

    let args: Vec<String> = if mode == RunMode::Lib {
        vec!["compile".to_string(), abs_file.to_string_lossy().to_string()]
    } else {
        vec!["run".to_string(), abs_file.to_string_lossy().to_string()]
    };
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = crate::build::run_scala_cli(&args_str, Some(proj_dir)).await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let full = if !stderr.is_empty() { format!("{stdout}\n{stderr}") } else { stdout.to_string() };
    if !output.status.success() { anyhow::bail!("scala-cli failed: {}", full); }
    if let Some(parent) = file_path.parent() { clean_artifacts(proj_dir, &proj_dir.join(parent)).await; }
    Ok(RunResult { mode, output: full.trim().to_string() })
}

pub async fn run_single_file_with_deps(proj_dir: &Path, file_path: &Path, deps: &[Dependency]) -> anyhow::Result<String> {
    let abs_file = proj_dir.join(file_path);
    let content = tokio::fs::read_to_string(&abs_file).await?;
    let has_main = has_main_method(&content);
    let dep_manager = crate::deps::default_dependency_manager();
    dep_manager.prepare_dependencies(deps, &proj_dir.join("target")).await?;
    let mut args: Vec<String> = if has_main { vec!["run".to_string(), abs_file.to_string_lossy().to_string()] }
        else { vec!["compile".to_string(), abs_file.to_string_lossy().to_string()] };
    args.extend(dep_manager.get_run_args(deps));
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = crate::build::run_scala_cli(&args_str, Some(proj_dir)).await?;
    if !output.status.success() { let err = String::from_utf8_lossy(&output.stderr); anyhow::bail!("Failed: {}", err); }
    if let Some(parent) = file_path.parent() { clean_artifacts(proj_dir, &proj_dir.join(parent)).await; }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ━━━━━━━━━━━━━━━━━━━ build_with_deps ━━━━━━━━━━━━━━━━━━━

use tokio::fs;

pub async fn build_with_deps(
    proj_dir: &Path, deps: &[Dependency], source_dir: &str, target_dir: &str,
    backend: &str, workspace_root: Option<&Path>, setup_bsp_flag: bool, is_workspace_build: bool,
) -> anyhow::Result<()> {
    let source_path = proj_dir.join(source_dir);
    let target_path = if let Some(ws_root) = workspace_root { ws_root.join(target_dir) } else { proj_dir.join(target_dir) };
    let workspace_dir = workspace_root.unwrap_or(proj_dir);
    fs::create_dir_all(&target_path).await?;

    if setup_bsp_flag {
        let bsp_dir = workspace_root.unwrap_or(proj_dir);
        let source_dirs = if let Some(ws_root) = workspace_root {
            let member_name = proj_dir.strip_prefix(ws_root).unwrap().to_str().unwrap();
            vec![(member_name.to_string(), source_dir.to_string())]
        } else { vec![("".to_string(), source_dir.to_string())] };
        crate::ide::setup_bsp(bsp_dir, deps, &source_dirs, backend).await?;
    }

    match backend {
        "scala-cli" => {
            let mut args: Vec<String> = vec!["compile".to_string()];
            if is_workspace_build { args.push("--workspace".to_string()); args.push(workspace_dir.to_string_lossy().to_string()); }
            args.push("-d".to_string()); args.push(target_path.to_string_lossy().to_string());
            args.push(source_path.to_string_lossy().to_string());
            for dep in deps { args.push("--dependency".to_string()); args.push(dep.coord()); }
            let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let output = crate::build::run_scala_cli(&args_str, Some(proj_dir)).await?;
            if !output.status.success() { let stderr = String::from_utf8_lossy(&output.stderr); anyhow::bail!("Build failed: {}", stderr); }
        }
        "sbt" => { run_build_tool("sbt", &["compile"], proj_dir).await?; }
        "maven" => { run_build_tool("mvn", &["compile"], proj_dir).await?; }
        "gradle" => { run_build_tool("gradle", &["build"], proj_dir).await?; }
        _ => anyhow::bail!("Unsupported backend: {}", backend),
    };

    clean_artifacts(proj_dir, &source_path).await;
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━ Multi-backend helpers ━━━━━━━━━━━━━━━━━━━

/// 使用非 scala-cli 后端运行 Main.scala
pub async fn run_with_backend(proj_dir: &Path, file_path: &Path, deps: &[Dependency], backend: &str) -> anyhow::Result<()> {
    match backend {
        "sbt" => run_build_tool("sbt", &["run"], proj_dir).await,
        "maven" => run_build_tool("mvn", &["exec:java"], proj_dir).await,
        "gradle" => run_build_tool("gradle", &["run"], proj_dir).await,
        _ => anyhow::bail!("Backend '{}' not supported for running", backend),
    }
}

/// 运行构建工具命令（sbt / mvn / gradle）
pub async fn run_build_tool(tool: &str, args: &[&str], cwd: &Path) -> anyhow::Result<()> {
    use tokio::process::Command;
    eprintln!("  Running {} {}...", tool, args.join(" "));
    let mut cmd = Command::new(tool);
    cmd.args(args).current_dir(cwd).stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::inherit()).stderr(std::process::Stdio::inherit());
    let status = cmd.status().await?;
    if !status.success() { anyhow::bail!("{} exited with status {}", tool, status); }
    Ok(())
}
