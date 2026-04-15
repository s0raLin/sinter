use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::fs;
use tokio::process::Command;

static SCALA_CLI_WARNING_PRINTED: AtomicBool = AtomicBool::new(false);

/// 获取打包的scala-cli可执行文件路径
fn get_bundled_scala_cli_path() -> Option<PathBuf> {
    let exe_name = if cfg!(target_os = "windows") {
        "scala-cli.exe"
    } else {
        "scala-cli"
    };

    // 1. 尝试从bin目录（相对于可执行文件）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let bundled_path = exe_dir.join("bin").join(exe_name);
            if bundled_path.exists() {
                return Some(bundled_path);
            }
        }
    }

    // 2. 尝试从CARGO_MANIFEST_DIR/bin目录（开发时）
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let bundled_path = PathBuf::from(manifest_dir).join("bin").join(exe_name);
        if bundled_path.exists() {
            return Some(bundled_path);
        }
    }

    None
}

/// 检查命令是否可用
async fn check_command_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 获取scala-cli可执行文件路径
pub async fn get_scala_cli_path() -> Option<String> {
    eprintln!("DEBUG: Checking for scala-cli path");
    // 首先尝试使用系统命令
    if check_command_available("scala-cli").await {
        eprintln!("DEBUG: Using system scala-cli");
        return Some("scala-cli".to_string());
    }
    eprintln!("DEBUG: System scala-cli not available");

    // 回退到打包的scala-cli
    if let Some(bundled_path) = get_bundled_scala_cli_path() {
        // 确保文件有执行权限
        #[cfg(unix)]
        {
            if let Ok(mut perms) = fs::metadata(&bundled_path).await.map(|m| m.permissions()) {
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&bundled_path, perms).await;
            }
        }

        if let Some(path_str) = bundled_path.to_str() {
            // 验证打包的scala-cli是否可用
            let mut cmd = Command::new(path_str);
            cmd.arg("--version");
            if cmd
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return Some(path_str.to_string());
            }
        }
    }

    None
}

/// 检查scala-cli是否可用，如果不可用则打印安装提示（仅一次）
pub async fn check_scala_cli_available() -> bool {
    let available = get_scala_cli_path().await.is_some();
    if !available {
        // 只打印一次警告
        if !SCALA_CLI_WARNING_PRINTED.swap(true, Ordering::Relaxed) {
            eprintln!("Warning: scala-cli is not available.");
            eprintln!("  - Install scala-cli: curl -fL https://github.com/VirtusLab/scala-cli/releases/latest/download/scala-cli-x86_64-pc-linux.gz | gzip -d > scala-cli && chmod +x scala-cli");
            eprintln!("  - Or visit: https://scala-cli.virtuslab.org/");
            eprintln!("  Attempting to download scala-cli automatically...");
            // 尝试自动下载
            if let Err(e) = download_scala_cli().await {
                eprintln!("  Failed to download scala-cli: {}", e);
            } else {
                eprintln!("  Successfully downloaded scala-cli.");
            }
        }
    }
    available
}

/// 下载scala-cli
pub async fn download_scala_cli() -> anyhow::Result<()> {
    use std::process::Stdio;

    let bin_dir = if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            exe_dir.join("bin")
        } else {
            PathBuf::from("./bin")
        }
    } else {
        PathBuf::from("./bin")
    };

    fs::create_dir_all(&bin_dir).await?;

    let script_path = bin_dir.join("download-scala-cli.sh");
    if script_path.exists() {
        // 运行下载脚本
        let mut cmd = Command::new("bash");
        cmd.arg(&script_path)
            .current_dir(&bin_dir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd.status().await?;
        if !status.success() {
            anyhow::bail!("Failed to download scala-cli");
        }
    } else {
        anyhow::bail!("Download script not found: {}", script_path.display());
    }

    Ok(())
}

/// 执行scala-cli命令
pub async fn run_scala_cli(
    args: &[&str],
    cwd: Option<&std::path::Path>,
) -> anyhow::Result<std::process::Output> {
    let scala_cli_path = get_scala_cli_path()
        .await
        .ok_or_else(|| anyhow::anyhow!("scala-cli is not available"))?;

    eprintln!(
        "DEBUG: Running scala-cli command: {} {:?}",
        scala_cli_path, args
    );
    if let Some(dir) = cwd {
        eprintln!("DEBUG: Working directory for scala-cli: {}", dir.display());
    }

    let mut cmd = Command::new(&scala_cli_path);

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
        eprintln!("DEBUG: Working directory: {}", dir.display());
    }

    // 添加其他参数
    for arg in args {
        cmd.arg(arg);
    }

    cmd.env("HOME", std::env::var("HOME").expect("HOME environment variable not set"));

    // 确保stdin是null，避免等待输入
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    eprintln!("DEBUG: About to execute cmd.output().await");
    let output = cmd.output().await?;
    eprintln!(
        "DEBUG: scala-cli stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    eprintln!(
        "DEBUG: scala-cli stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!(
        "DEBUG: scala-cli command completed with status: {}",
        output.status
    );
    Ok(output)
}

/// 执行scala-cli命令并返回结果
pub async fn execute_scala_cli(
    args: &[&str],
    cwd: Option<&std::path::Path>,
) -> anyhow::Result<String> {
    let output = run_scala_cli(args, cwd).await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        let full_output = if !stderr.is_empty() {
            format!("{}\n{}", stdout, stderr)
        } else {
            stdout.to_string()
        };
        anyhow::bail!("scala-cli failed: {}", full_output);
    }

    Ok(stdout.to_string())
}
