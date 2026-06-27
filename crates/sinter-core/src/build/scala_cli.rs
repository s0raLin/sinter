//! Scala CLI 后端 — 发现、下载和执行 scala-cli

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::fs;
use tokio::process::Command;
use tokio::sync::OnceCell;

static SCALA_CLI_WARNING_PRINTED: AtomicBool = AtomicBool::new(false);
static SCALA_CLI_PATH: OnceCell<Option<String>> = OnceCell::const_new();

/// 获取打包的 scala-cli 可执行文件路径
fn get_bundled_scala_cli_path() -> Option<PathBuf> {
    let exe_name = if cfg!(target_os = "windows") { "scala-cli.exe" } else { "scala-cli" };
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let bundled_path = exe_dir.join("bin").join(exe_name);
            if bundled_path.exists() { return Some(bundled_path); }
        }
    }
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let bundled_path = PathBuf::from(manifest_dir).join("bin").join(exe_name);
        if bundled_path.exists() { return Some(bundled_path); }
    }
    None
}

/// 用 `which` 检查命令是否存在（极快，不触发 JVM 启动）
async fn which(cmd: &str) -> bool {
    match Command::new("which").arg(cmd).output().await {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

async fn discover_scala_cli_path() -> Option<String> {
    eprintln!("  Checking for scala-cli (via which)...");
    if which("scala-cli").await {
        eprintln!("  Found system scala-cli");
        return Some("scala-cli".to_string());
    }
    if let Some(bundled_path) = get_bundled_scala_cli_path() {
        eprintln!("  Found bundled scala-cli at {}", bundled_path.display());
        return bundled_path.to_str().map(|s| s.to_string());
    }
    eprintln!("  scala-cli not found");
    None
}

/// 获取 scala-cli 可执行文件路径（结果缓存，仅探测一次）
pub async fn get_scala_cli_path() -> Option<String> {
    SCALA_CLI_PATH
        .get_or_init(|| async { discover_scala_cli_path().await })
        .await
        .clone()
}

/// 检查 scala-cli 是否可用（仅打印一次警告）
pub async fn check_scala_cli_available() -> bool {
    let available = get_scala_cli_path().await.is_some();
    if !available && !SCALA_CLI_WARNING_PRINTED.swap(true, Ordering::Relaxed) {
        eprintln!("Warning: scala-cli is not available.");
        eprintln!("  Install: curl -fL https://github.com/VirtusLab/scala-cli/releases/latest/download/scala-cli-x86_64-pc-linux.gz | gzip -d > scala-cli && chmod +x scala-cli");
        if let Err(e) = download_scala_cli().await {
            eprintln!("  Failed to download scala-cli: {}", e);
        } else {
            eprintln!("  Successfully downloaded scala-cli.");
        }
    }
    available
}

/// 下载 scala-cli
pub async fn download_scala_cli() -> anyhow::Result<()> {
    use std::process::Stdio;
    let bin_dir = if let Ok(exe) = std::env::current_exe() {
        exe.parent().map(|p| p.join("bin")).unwrap_or_else(|| PathBuf::from("./bin"))
    } else {
        PathBuf::from("./bin")
    };
    fs::create_dir_all(&bin_dir).await?;
    let script_path = bin_dir.join("download-scala-cli.sh");
    if script_path.exists() {
        let status = Command::new("bash")
            .arg(&script_path).current_dir(&bin_dir)
            .stdout(Stdio::inherit()).stderr(Stdio::inherit())
            .status().await?;
        if !status.success() { anyhow::bail!("Failed to download scala-cli"); }
    } else {
        anyhow::bail!("Download script not found: {}", script_path.display());
    }
    Ok(())
}

/// 执行 scala-cli 命令 (无超时 — 用于用户的构建/运行命令)
pub async fn run_scala_cli(
    args: &[&str], cwd: Option<&std::path::Path>,
) -> anyhow::Result<std::process::Output> {
    let scala_cli_path = get_scala_cli_path().await
        .ok_or_else(|| anyhow::anyhow!("scala-cli is not available"))?;

    eprintln!("  Running scala-cli {}...", args.join(" "));
    let mut cmd = Command::new(&scala_cli_path);
    if let Some(dir) = cwd { cmd.current_dir(dir); }
    for arg in args { cmd.arg(arg); }
    cmd.env("HOME", std::env::var("HOME").expect("HOME environment variable not set"));
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.output().await.map_err(Into::into)
}

/// 执行 scala-cli 并返回 stdout 字符串
pub async fn execute_scala_cli(
    args: &[&str], cwd: Option<&std::path::Path>,
) -> anyhow::Result<String> {
    let output = run_scala_cli(args, cwd).await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let full = if !stderr.is_empty() { format!("{}\n{}", stdout, stderr) } else { stdout.to_string() };
        anyhow::bail!("scala-cli failed: {}", full);
    }
    Ok(stdout.to_string())
}
