use crate::deps::deps::Dependency;
use std::path::Path;
use crate::toolkit::os::{PathWrapper, remove_all, make_dir_all, write};
use serde_json;

/// Copy directory recursively (synchronous version for simplicity)
fn copy_dir_all_sync(src: &Path, dst: &Path) -> anyhow::Result<()> {
    use walkdir::WalkDir;
    
    std::fs::create_dir_all(dst)?;
    
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path.strip_prefix(src)?;
        let dst_path = dst.join(relative_path);
        
        if path.is_dir() {
            std::fs::create_dir_all(&dst_path)?;
        } else {
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(path, &dst_path)?;
        }
    }
    
    Ok(())
}

pub async fn setup_bsp(bsp_dir: &Path, deps: &[Dependency], source_dirs: &[(String, String)], backend: &str) -> anyhow::Result<()> {
    eprintln!("DEBUG: setup_bsp called with bsp_dir: {}", bsp_dir.display());
    eprintln!("DEBUG: source_dirs: {:?}", source_dirs);
    // Remove any existing .bsp and .scala-build in the bsp_dir.
    let _ = remove_all(&PathWrapper::new(bsp_dir.join(".bsp"))).await;
    let _ = remove_all(&PathWrapper::new(bsp_dir.join(".scala-build"))).await;

    // Clean source trees
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
            let mut args: Vec<String> = vec!["setup-ide".to_string()];
            // Force BSP directory to current directory
            args.push("--bsp-dir".to_string());
            args.push(bsp_dir.to_string_lossy().to_string());
            // Pass relative paths to Scala files (relative to bsp_dir) to ensure BSP files are generated in root
            for (member_name, source_dir) in source_dirs {
                let source_path = if member_name.is_empty() {
                    bsp_dir.join(source_dir)
                } else {
                    bsp_dir.join(member_name).join(source_dir)
                };
                // Find all .scala files in the source directory
                if let Ok(entries) = std::fs::read_dir(&source_path) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_file() {
                                if let Some(ext) = entry.path().extension() {
                                    if ext == "scala" {
                                        // Use relative path from bsp_dir to ensure BSP files are generated in root
                                        let file_path = entry.path();
                                        if let Ok(relative_path) = file_path.strip_prefix(bsp_dir) {
                                            args.push(relative_path.to_string_lossy().to_string());
                                        } else {
                                            // Fallback to absolute path if strip_prefix fails
                                            args.push(file_path.to_string_lossy().to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            for dep in deps {
                args.push("--dependency".to_string());
                args.push(dep.coord());
            }
            let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            eprintln!("DEBUG: Running scala-cli setup-ide with args: {:?}", args_str);
            eprintln!("DEBUG: Working directory: {}", bsp_dir.display());
            let output = crate::build::run_scala_cli(&args_str, Some(bsp_dir)).await?;
            if !output.status.success() {
                eprintln!("DEBUG: scala-cli setup-ide failed. stdout: {}", String::from_utf8_lossy(&output.stdout));
                eprintln!("DEBUG: scala-cli setup-ide failed. stderr: {}", String::from_utf8_lossy(&output.stderr));
                anyhow::bail!("BSP setup failed");
            }

            // Immediately after setup-ide, check and move BSP files from source directories to root
            // scala-cli may generate these files in the source directory even with --bsp-dir specified
            for (member_name, source_dir) in source_dirs {
                let source_path = if member_name.is_empty() {
                    bsp_dir.join(source_dir)
                } else {
                    bsp_dir.join(member_name).join(source_dir)
                };
                eprintln!("DEBUG: Checking for BSP files in: {}", source_path.display());
                
                // Check and move .bsp directory
                let source_bsp = source_path.join(".bsp");
                let root_bsp = bsp_dir.join(".bsp");
                
                if source_bsp.exists() {
                    eprintln!("DEBUG: Found .bsp in source directory: {}", source_bsp.display());
                    if !root_bsp.exists() {
                        eprintln!("DEBUG: Moving .bsp from {} to {}", source_bsp.display(), root_bsp.display());
                        match std::fs::rename(&source_bsp, &root_bsp) {
                            Ok(_) => eprintln!("DEBUG: Successfully moved .bsp to root"),
                            Err(e) => {
                                eprintln!("DEBUG: Failed to move .bsp: {}, trying copy instead", e);
                                // Try copy as fallback
                                if let Err(copy_err) = copy_dir_all_sync(&source_bsp, &root_bsp) {
                                    eprintln!("DEBUG: Copy also failed: {}", copy_err);
                                } else {
                                    let _ = remove_all(&PathWrapper::new(source_bsp)).await;
                                    eprintln!("DEBUG: Successfully copied .bsp to root");
                                }
                            }
                        }
                    } else {
                        eprintln!("DEBUG: Root .bsp already exists, removing source .bsp");
                        let _ = remove_all(&PathWrapper::new(source_bsp)).await;
                    }
                }
                
                // Check and move .scala-build directory
                let source_scala_build = source_path.join(".scala-build");
                let root_scala_build = bsp_dir.join(".scala-build");
                
                if source_scala_build.exists() {
                    eprintln!("DEBUG: Found .scala-build in source directory: {}", source_scala_build.display());
                    if !root_scala_build.exists() {
                        eprintln!("DEBUG: Moving .scala-build from {} to {}", source_scala_build.display(), root_scala_build.display());
                        if let Err(e) = copy_dir_all_sync(&source_scala_build, &root_scala_build) {
                            eprintln!("DEBUG: Failed to move .scala-build: {}", e);
                        } else {
                            let _ = remove_all(&PathWrapper::new(source_scala_build)).await;
                            eprintln!("DEBUG: Successfully moved .scala-build to root");
                        }
                    } else {
                        eprintln!("DEBUG: Root .scala-build already exists, merging files");
                        // Merge files from source to root
                        if let Ok(entries) = std::fs::read_dir(&source_scala_build) {
                            for entry in entries.flatten() {
                                let source_file = entry.path();
                                let root_file = root_scala_build.join(entry.file_name());
                                if source_file.is_file() {
                                    if let Err(e) = std::fs::copy(&source_file, &root_file) {
                                        eprintln!("DEBUG: Failed to copy {}: {}", source_file.display(), e);
                                    }
                                }
                            }
                        }
                        let _ = remove_all(&PathWrapper::new(source_scala_build)).await;
                        eprintln!("DEBUG: Merged and removed source .scala-build");
                    }
                }
            }
            
            // Final check: ensure no BSP files remain in source directories
            for (member_name, source_dir) in source_dirs {
                let source_path = if member_name.is_empty() {
                    bsp_dir.join(source_dir)
                } else {
                    bsp_dir.join(member_name).join(source_dir)
                };
                let source_bsp = source_path.join(".bsp");
                let source_scala_build = source_path.join(".scala-build");
                
                if source_bsp.exists() {
                    eprintln!("DEBUG: WARNING: .bsp still exists in source directory after cleanup, removing");
                    let _ = remove_all(&PathWrapper::new(source_bsp)).await;
                }
                if source_scala_build.exists() {
                    eprintln!("DEBUG: WARNING: .scala-build still exists in source directory after cleanup, removing");
                    let _ = remove_all(&PathWrapper::new(source_scala_build)).await;
                }
            }
            
            // Update scala-cli.json paths to point to root .scala-build directory
            let bsp_scala_cli_json = bsp_dir.join(".bsp/scala-cli.json");
            if bsp_scala_cli_json.exists() {
                eprintln!("DEBUG: Updating scala-cli.json paths");
                if let Ok(content) = std::fs::read_to_string(&bsp_scala_cli_json) {
                    let mut json: serde_json::Value = serde_json::from_str(&content)
                        .unwrap_or_else(|_| serde_json::json!({}));
                    
                    let root_scala_build_path = bsp_dir.join(".scala-build");
                    let root_scala_build_str = root_scala_build_path.to_string_lossy().to_string();
                    
                    // Update paths in argv array
                    if let Some(argv) = json.get_mut("argv").and_then(|a| a.as_array_mut()) {
                        for arg in argv.iter_mut() {
                            if let Some(arg_str) = arg.as_str() {
                                let original_path = arg_str.to_string();
                                let mut new_path = original_path.clone();
                                
                                // Replace paths pointing to source_dir/.scala-build with root/.scala-build
                                for (member_name, source_dir) in source_dirs {
                                    let source_path = if member_name.is_empty() {
                                        bsp_dir.join(source_dir)
                                    } else {
                                        bsp_dir.join(member_name).join(source_dir)
                                    };
                                    let source_scala_build_path = source_path.join(".scala-build");
                                    
                                    // Try both absolute and canonical paths
                                    if let Ok(canonical_source) = source_scala_build_path.canonicalize() {
                                        let canonical_source_str = canonical_source.to_string_lossy().to_string();
                                        if new_path.contains(&canonical_source_str) {
                                            new_path = new_path.replace(&canonical_source_str, &root_scala_build_str);
                                        }
                                    }
                                    
                                    // Also try the original path string
                                    let source_scala_build_str = source_scala_build_path.to_string_lossy().to_string();
                                    if new_path.contains(&source_scala_build_str) {
                                        new_path = new_path.replace(&source_scala_build_str, &root_scala_build_str);
                                    }
                                    
                                    // Also try with normalized path separators
                                    let source_scala_build_normalized = source_scala_build_str.replace("\\", "/");
                                    let root_scala_build_normalized = root_scala_build_str.replace("\\", "/");
                                    if new_path.contains(&source_scala_build_normalized) {
                                        new_path = new_path.replace(&source_scala_build_normalized, &root_scala_build_normalized);
                                    }
                                }
                                
                                if new_path != original_path {
                                    *arg = serde_json::Value::String(new_path.clone());
                                    eprintln!("DEBUG: Updated path: {} -> {}", original_path, new_path);
                                }
                            }
                        }
                    }
                    
                    // Write updated json
                    if let Ok(updated_content) = serde_json::to_string_pretty(&json) {
                        if let Err(e) = std::fs::write(&bsp_scala_cli_json, updated_content) {
                            eprintln!("DEBUG: Failed to update scala-cli.json: {}", e);
                        } else {
                            eprintln!("DEBUG: Updated scala-cli.json successfully");
                        }
                    }
                }
            }
        }
        "sbt" | "gradle" | "maven" => {
            // For other backends, BSP setup might be different or not needed
            // For now, skip BSP setup for non-scala-cli backends
            return Ok(());
        }
        _ => {
            anyhow::bail!("Unsupported backend: {}", backend);
        }
    };

    // Manually set ide-options-v2.json
    let options_path = bsp_dir.join(".scala-build/ide-options-v2.json");
    make_dir_all(&PathWrapper::new(options_path.parent().unwrap())).await?;
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
    let content = options.to_string();
    write(&PathWrapper::new(&options_path), &content).await?;
    Ok(())
}