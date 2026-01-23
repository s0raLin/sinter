// src/run.rs
use std::path::Path;

use crate::build::common::{has_main_method, RunMode, RunResult};

pub async fn run_scala_file(
    proj_dir: &Path,
    file_path: &Path,
    force_lib: bool,
) -> anyhow::Result<RunResult> {
    let abs_file = proj_dir.join(file_path);

    // 1. 读取文件内容，检测是否有 main
    let content = tokio::fs::read_to_string(&abs_file).await?;
    let has_main = has_main_method(&content);
    let mode = if force_lib || !has_main {
        RunMode::Lib
    } else {
        RunMode::App
    };

    // 2. 调用 scala-cli
    let args: Vec<String> = if mode == RunMode::Lib {
        vec!["compile".to_string(), abs_file.to_string_lossy().to_string()]
    } else {
        vec!["run".to_string(), abs_file.to_string_lossy().to_string()]
    };

    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = crate::build::run_scala_cli(&args_str, Some(proj_dir)).await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let full_output = if !stderr.is_empty() {
        format!("{stdout}\n{stderr}")
    } else {
        stdout.to_string()
    };

    if !output.status.success() {
        anyhow::bail!("scala-cli failed: {}", full_output);
    }

    Ok(RunResult {
        mode,
        output: full_output.trim().to_string(),
    })
}


// src/run.rs (新增函数)
use crate::deps::deps::Dependency;

pub async fn run_single_file_with_deps(
    proj_dir: &Path,
    file_path: &Path,
    deps: &[Dependency],
) -> anyhow::Result<String> {
    eprintln!("DEBUG: run_single_file_with_deps called for project at {}", proj_dir.display());
    eprintln!("DEBUG: Starting run_single_file_with_deps for file: {}", file_path.display());
    let abs_file = proj_dir.join(file_path);
    let content = tokio::fs::read_to_string(&abs_file).await?;
    let has_main = has_main_method(&content);

    // 使用抽象的依赖管理器
    eprintln!("DEBUG: Preparing dependencies");
    let dep_manager = crate::deps::default_dependency_manager().await;
    dep_manager.prepare_dependencies(deps, &proj_dir.join("target")).await?;
    eprintln!("DEBUG: Dependencies prepared");

    let mut args: Vec<String> = if has_main {
        vec!["run".to_string(), abs_file.to_string_lossy().to_string()]
    } else {
        vec!["compile".to_string(), abs_file.to_string_lossy().to_string()]
    };

    // 添加依赖参数
    let dep_args = dep_manager.get_run_args(deps);
    args.extend(dep_args);

    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = crate::build::run_scala_cli(&args_str, Some(proj_dir)).await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        eprintln!("DEBUG: run_single_file_with_deps failed: {}", err);
        anyhow::bail!("Failed: {}", err);
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    eprintln!("DEBUG: run_single_file_with_deps completed successfully");
    Ok(stdout)
}


