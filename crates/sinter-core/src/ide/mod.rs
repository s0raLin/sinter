//! IDE 集成 — BSP 协议支持和 classpath 生成

use crate::deps::Dependency;
use crate::toolkit::os::{make_dir_all, remove_all, write, PathWrapper};
use std::env;
use std::path::Path;

// ━━━━━━━━━━━━━━━━━━━ BSP Setup ━━━━━━━━━━━━━━━━━━━

async fn get_scala_cli_version(scala_cli_path: &str) -> anyhow::Result<String> {
    use tokio::process::Command;
    let output = Command::new(scala_cli_path).arg("--version").output().await?;
    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Some(v) = version.split_whitespace().last() { return Ok(v.to_string()); }
    }
    Ok("1.11.0".to_string())
}

fn get_env_vars() -> serde_json::Value {
    let mut env_map = serde_json::Map::new();
    for (key, value) in env::vars() {
        env_map.insert(key, serde_json::Value::String(value));
    }
    serde_json::Value::Object(env_map)
}

pub async fn setup_bsp(
    bsp_dir: &Path, deps: &[Dependency],
    source_dirs: &[(String, String)], backend: &str,
) -> anyhow::Result<()> {
    let _ = remove_all(&PathWrapper::new(bsp_dir.join(".bsp"))).await;
    let _ = remove_all(&PathWrapper::new(bsp_dir.join(".scala-build"))).await;

    for (member_name, source_dir) in source_dirs {
        let source_path = if member_name.is_empty() { bsp_dir.join(source_dir) }
            else { bsp_dir.join(member_name).join(source_dir) };
        let _ = remove_all(&PathWrapper::new(source_path.join(".bsp"))).await;
        let _ = remove_all(&PathWrapper::new(source_path.join(".scala-build"))).await;
    }

    match backend {
        "scala-cli" => {
            let scala_cli_path = crate::build::get_scala_cli_path().await
                .ok_or_else(|| anyhow::anyhow!("scala-cli is not available"))?;
            let scala_cli_version = get_scala_cli_version(&scala_cli_path).await
                .unwrap_or_else(|_| "1.11.0".to_string());

            let mut scala_files = Vec::new();
            use walkdir::WalkDir;
            for (member_name, source_dir) in source_dirs {
                let source_path = if member_name.is_empty() { bsp_dir.join(source_dir) }
                    else { bsp_dir.join(member_name).join(source_dir) };
                if source_path.exists() {
                    for entry in WalkDir::new(&source_path).into_iter().filter_map(|e| e.ok()) {
                        if entry.file_type().is_file()
                            && entry.path().extension().map_or(false, |e| e == "scala") {
                            if let Ok(rp) = entry.path().strip_prefix(bsp_dir) {
                                scala_files.push(rp.to_string_lossy().to_string());
                            } else {
                                scala_files.push(entry.path().to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }

            let bsp_path = bsp_dir.join(".bsp");
            make_dir_all(&PathWrapper::new(&bsp_path)).await?;
            let scala_build_path = bsp_dir.join(".scala-build");
            make_dir_all(&PathWrapper::new(&scala_build_path)).await?;
            let scala_build_abs = scala_build_path.canonicalize().unwrap_or_else(|_| scala_build_path.clone());
            let sb = scala_build_abs.to_string_lossy();

            let mut argv = vec![
                scala_cli_path.clone(), "bsp".to_string(),
                "--json-options".to_string(), format!("{}/ide-options-v2.json", sb),
                "--json-launcher-options".to_string(), format!("{}/ide-launcher-options.json", sb),
                "--envs-file".to_string(), format!("{}/ide-envs.json", sb),
            ];
            argv.extend(scala_files);

            let scala_cli_json = serde_json::json!({
                "name": "scala-cli", "argv": argv, "version": scala_cli_version,
                "bspVersion": "2.1.1", "languages": ["scala", "java"]
            });
            let json_content = serde_json::to_string_pretty(&scala_cli_json)?;
            write(&PathWrapper::new(&bsp_path.join("scala-cli.json")), &json_content).await?;
            write(&PathWrapper::new(&scala_build_path.join("ide-launcher-options.json")), "{}").await?;

            let envs_json = get_env_vars();
            write(&PathWrapper::new(&scala_build_path.join("ide-envs.json")), &serde_json::to_string_pretty(&envs_json)?).await?;
            write(&PathWrapper::new(&scala_build_path.join("ide-inputs.json")), "[]").await?;

            let dependencies: Vec<String> = deps.iter().map(|d| d.coord()).collect();
            let scalac_options: Vec<String> = source_dirs.iter().map(|(mn, sd)| {
                let s = if mn.is_empty() { sd.clone() } else { format!("{}/{}", mn, sd) };
                format!("{}/**/*.scala", s)
            }).collect();

            let template_path = crate::toolkit::path::paths::template_file("ide-options-v2.json.template");
            let template = template_path.read_sync()?;
            let json_str = template.replace("{scalac_option}", &scalac_options.join("\",\""));
            let mut options: serde_json::Value = serde_json::from_str(&json_str)?;
            options["dependencies"]["dependency"] = serde_json::Value::Array(
                dependencies.into_iter().map(serde_json::Value::String).collect()
            );
            write(&PathWrapper::new(&scala_build_path.join("ide-options-v2.json")), &serde_json::to_string_pretty(&options)?).await?;
        }
        _ => {}
    }
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━ Classpath Generator ━━━━━━━━━━━━━━━━━━━

use crate::models::Project;

pub fn generate_classpath(_project: &Project, _dependencies: &[Dependency]) -> anyhow::Result<String> {
    // TODO: 实现 classpath 生成逻辑
    Ok(String::new())
}
