use std::env;
use std::fs::{self, Permissions};
use std::path::Path;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn main() {
    // 下载coursier（简化平台检测）
    download_coursier();
}

fn download_coursier() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let bin_dir = Path::new(&manifest_dir).join("bin");
    fs::create_dir_all(&bin_dir).ok();

    let exe_name = if cfg!(target_os = "windows") { "coursier.exe" } else { "coursier" };
    let coursier_path = bin_dir.join(exe_name);

    if coursier_path.exists() && Command::new(&coursier_path).arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
        println!("cargo:warning=coursier already exists");
        return;
    }

    let platform = match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-pc-linux",
        ("linux", "aarch64") => "aarch64-pc-linux",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        _ => return,
    };

    let url = format!("https://github.com/coursier/coursier/releases/latest/download/cs-{}.gz", platform);
    Command::new("sh").args(&["-c", &format!("curl -fL {} | gzip -d > {}", url, coursier_path.display())]).status().ok();

    #[cfg(unix)]
    {
        let perms = Permissions::from_mode(0o755);
        fs::set_permissions(&coursier_path, perms).ok();
    }

    // 拷贝到target/bin
    let binding = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&binding).parent().unwrap().parent().unwrap().parent().unwrap();
    let target_bin_dir = out_dir.join("bin");
    fs::create_dir_all(&target_bin_dir).ok();
    let target_path = target_bin_dir.join(exe_name);
    fs::copy(&coursier_path, &target_path).ok();

    #[cfg(unix)]
    {
        let perms = Permissions::from_mode(0o755);
        fs::set_permissions(&target_path, perms).ok();
    }
}