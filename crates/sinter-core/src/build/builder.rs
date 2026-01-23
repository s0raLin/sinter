use crate::deps::deps::Dependency;
use std::path::Path;
use tokio::fs;

pub async fn build_with_deps(
    proj_dir: &Path,
    deps: &[Dependency],
    source_dir: &str,
    target_dir: &str,
    backend: &str,
    workspace_root: Option<&Path>,
    setup_bsp_flag: bool,
    is_workspace_build: bool,
) -> anyhow::Result<()> {
    eprintln!("DEBUG: build_with_deps called for project at {}", proj_dir.display());
    eprintln!("DEBUG: Starting build with {} dependencies, backend: {}", deps.len(), backend);
    let source_path = proj_dir.join(source_dir);
    let target_path = if let Some(ws_root) = workspace_root {
        ws_root.join(target_dir)
    } else {
        proj_dir.join(target_dir)
    };
    let workspace_dir = workspace_root.unwrap_or(proj_dir);

    // Ensure target directory exists
    fs::create_dir_all(&target_path).await?;

    // Setup BSP for IDE support if requested
    if setup_bsp_flag {
        let bsp_dir = workspace_root.unwrap_or(proj_dir);
        let source_dirs = if let Some(ws_root) = workspace_root {
            let member_name = proj_dir.strip_prefix(ws_root).unwrap().to_str().unwrap();
            vec![(member_name.to_string(), source_dir.to_string())]
        } else {
            vec![("".to_string(), source_dir.to_string())]
        };
        crate::ide::setup_bsp(bsp_dir, deps, &source_dirs, backend).await?;
    }

    match backend {
        "scala-cli" => {
            let mut args: Vec<String> = vec!["compile".to_string()];
            if is_workspace_build {
                args.push("--workspace".to_string());
                args.push(workspace_dir.to_string_lossy().to_string());
            }
            args.push("-d".to_string());
            args.push(target_path.to_string_lossy().to_string());
            args.push(source_path.to_string_lossy().to_string());
            for dep in deps {
                args.push("--dependency".to_string());
                args.push(dep.coord());
            }
            let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let output = crate::build::run_scala_cli(&args_str, Some(proj_dir)).await?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                eprintln!("DEBUG: scala-cli compile failed. stdout: {}", stdout);
                eprintln!("DEBUG: scala-cli compile failed. stderr: {}", stderr);
                anyhow::bail!("Build failed with dependencies: {}", stderr);
            }
        }
        "sbt" => {
            // For sbt, we assume build.sbt exists and contains dependencies
            // 如果有外部依赖（比如从其他项目导入的），可能需要特殊处理
            anyhow::bail!("sbt backend not implemented");
        }
        "gradle" => {
            // For gradle, we assume build.gradle exists and contains dependencies
            anyhow::bail!("gradle backend not implemented");
        }
        "maven" => {
            // For maven, we assume pom.xml exists and contains dependencies
            anyhow::bail!("maven backend not implemented");
        }
        _ => {
            anyhow::bail!("Unsupported backend: {}", backend);
        }
    };

    // Always clean up build artifacts that scala-cli drops inside the source tree.
    // These should be in the project root, not in the source directory
    let _ = fs::remove_dir_all(source_path.join(".bsp")).await;
    let _ = fs::remove_dir_all(source_path.join(".scala-build")).await;
    
    // Clean up scala-cli.json in project root if it exists (it should be in .bsp directory)
    // The correct scala-cli.json should be in .bsp/scala-cli.json, not in the root
    let root_scala_cli_json = proj_dir.join("scala-cli.json");
    if root_scala_cli_json.exists() {
        eprintln!("DEBUG: Removing scala-cli.json from project root (should be in .bsp directory)");
        let _ = fs::remove_file(&root_scala_cli_json).await;
    }

    eprintln!("DEBUG: Build completed successfully");
    Ok(())
}
