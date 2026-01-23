use crate::deps::deps::Dependency;
use std::path::Path;
use crate::toolkit::os::{PathWrapper, remove_all, make_dir_all, write};
use std::env;

/// Get scala-cli version
async fn get_scala_cli_version(scala_cli_path: &str) -> anyhow::Result<String> {
    use tokio::process::Command;
    let output = Command::new(scala_cli_path)
        .arg("--version")
        .output()
        .await?;
    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        // Extract version number (e.g., "Scala CLI version 1.11.0" -> "1.11.0")
        let version = version.trim();
        if let Some(v) = version.split_whitespace().last() {
            return Ok(v.to_string());
        }
    }
    Ok("1.11.0".to_string()) // Default version
}

/// Get environment variables as JSON
fn get_env_vars() -> serde_json::Value {
    let mut env_map = serde_json::Map::new();
    for (key, value) in env::vars() {
        env_map.insert(key, serde_json::Value::String(value));
    }
    serde_json::Value::Object(env_map)
}

pub async fn setup_bsp(bsp_dir: &Path, deps: &[Dependency], source_dirs: &[(String, String)], backend: &str) -> anyhow::Result<()> {
    eprintln!("DEBUG: setup_bsp called with bsp_dir: {}", bsp_dir.display());
    eprintln!("DEBUG: source_dirs: {:?}", source_dirs);
    
    // Remove any existing .bsp and .scala-build in the bsp_dir.
    let _ = remove_all(&PathWrapper::new(bsp_dir.join(".bsp"))).await;
    let _ = remove_all(&PathWrapper::new(bsp_dir.join(".scala-build"))).await;

    // Clean source trees - remove any .bsp and .scala-build from source directories
    for (member_name, source_dir) in source_dirs {
        let source_path = if member_name.is_empty() {
            bsp_dir.join(source_dir)
        } else {
            bsp_dir.join(member_name).join(source_dir)
        };
        let _ = remove_all(&PathWrapper::new(source_path.join(".bsp"))).await;
        let _ = remove_all(&PathWrapper::new(source_path.join(".scala-build"))).await;
    }

    match backend {
        "scala-cli" => {
            // Get scala-cli path
            let scala_cli_path = crate::build::get_scala_cli_path().await
                .ok_or_else(|| anyhow::anyhow!("scala-cli is not available"))?;
            
            // Get scala-cli version
            let scala_cli_version = get_scala_cli_version(&scala_cli_path).await.unwrap_or_else(|_| "1.11.0".to_string());
            
            // Collect all Scala files recursively
            let mut scala_files = Vec::new();
            use walkdir::WalkDir;
            
            for (member_name, source_dir) in source_dirs {
                let source_path = if member_name.is_empty() {
                    bsp_dir.join(source_dir)
                } else {
                    bsp_dir.join(member_name).join(source_dir)
                };
                
                // Recursively find all .scala files
                if source_path.exists() {
                    for entry in WalkDir::new(&source_path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        if entry.file_type().is_file() {
                            if let Some(ext) = entry.path().extension() {
                                if ext == "scala" {
                                    let file_path = entry.path();
                                    if let Ok(relative_path) = file_path.strip_prefix(bsp_dir) {
                                        scala_files.push(relative_path.to_string_lossy().to_string());
                                    } else {
                                        scala_files.push(file_path.to_string_lossy().to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Ensure .bsp directory exists
            let bsp_path = bsp_dir.join(".bsp");
            make_dir_all(&PathWrapper::new(&bsp_path)).await?;
            
            // Ensure .scala-build directory exists
            let scala_build_path = bsp_dir.join(".scala-build");
            make_dir_all(&PathWrapper::new(&scala_build_path)).await?;
            
            // Get absolute paths
            let scala_build_path_abs = scala_build_path.canonicalize()
                .unwrap_or_else(|_| scala_build_path.clone());
            let scala_build_path_str = scala_build_path_abs.to_string_lossy().to_string();
            
            // Build argv array for scala-cli.json
            let mut argv = vec![
                scala_cli_path.clone(),
                "bsp".to_string(),
                "--json-options".to_string(),
                format!("{}/ide-options-v2.json", scala_build_path_str),
                "--json-launcher-options".to_string(),
                format!("{}/ide-launcher-options.json", scala_build_path_str),
                "--envs-file".to_string(),
                format!("{}/ide-envs.json", scala_build_path_str),
            ];
            // Add all Scala files
            argv.extend(scala_files);
            
            // Create scala-cli.json
            let scala_cli_json = serde_json::json!({
                "name": "scala-cli",
                "argv": argv,
                "version": scala_cli_version,
                "bspVersion": "2.1.1",
                "languages": ["scala", "java"]
            });
            
            let scala_cli_json_path = bsp_path.join("scala-cli.json");
            let scala_cli_json_content = serde_json::to_string_pretty(&scala_cli_json)?;
            write(&PathWrapper::new(&scala_cli_json_path), &scala_cli_json_content).await?;
            
            // Create ide-launcher-options.json (empty object)
            let launcher_options_path = scala_build_path.join("ide-launcher-options.json");
            write(&PathWrapper::new(&launcher_options_path), "{}").await?;
            
            // Create ide-envs.json (environment variables)
            let envs_json = get_env_vars();
            let envs_path = scala_build_path.join("ide-envs.json");
            let envs_content = serde_json::to_string_pretty(&envs_json)?;
            write(&PathWrapper::new(&envs_path), &envs_content).await?;
            
            // Create ide-inputs.json (empty array for now)
            let inputs_path = scala_build_path.join("ide-inputs.json");
            write(&PathWrapper::new(&inputs_path), "[]").await?;
            
            // Create ide-options-v2.json
            let dependencies: Vec<String> = deps.iter().map(|d| d.coord()).collect();
            let scalac_options: Vec<String> = source_dirs.iter().map(|(member_name, source_dir)| {
                let source_dir_rel = if member_name.is_empty() {
                    source_dir.clone()
                } else {
                    format!("{}/{}", member_name, source_dir)
                };
                format!("{}/**/*.scala", source_dir_rel)
            }).collect();
            let template_path = crate::toolkit::path::paths::template_file("ide-options-v2.json.template");
            let template = template_path.read_sync()?;
            let json_str = template.replace("{scalac_option}", &scalac_options.join("\",\""));
            let mut options: serde_json::Value = serde_json::from_str(&json_str)?;
            options["dependencies"]["dependency"] = serde_json::Value::Array(dependencies.into_iter().map(serde_json::Value::String).collect());
            let content = serde_json::to_string_pretty(&options)?;
            let options_path = scala_build_path.join("ide-options-v2.json");
            write(&PathWrapper::new(&options_path), &content).await?;
            
            eprintln!("DEBUG: Successfully created .bsp and .scala-build directories in {}", bsp_dir.display());
        }
        "sbt" | "gradle" | "maven" => {
            // For other backends, BSP setup might be different or not needed
            // For now, skip BSP setup for non-scala-cli backends
            return Ok(());
        }
        _ => {
            anyhow::bail!("Unsupported backend: {}", backend);
        }
    }

    Ok(())
}