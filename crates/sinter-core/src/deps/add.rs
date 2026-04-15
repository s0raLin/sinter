use std::path::Path;

pub async fn add_dependency(project_dir: &Path, dep_spec: &str) -> anyhow::Result<()> {
    let project = crate::config::loader::load_project(project_dir)?;
    let manifest_path = project_dir.join("project.toml");

    // 检测是否在工作空间根目录
    let is_workspace_root = project.workspace.is_some();

    // Check if it's an sbt path
    if dep_spec.starts_with("sbt:") || (dep_spec.contains("/") && !dep_spec.contains("::")) {
        let sbt_path = if dep_spec.starts_with("sbt:") {
            dep_spec[4..].to_string()
        } else {
            dep_spec.to_string()
        };

        // Validate that the sbt project exists
        let sbt_project_path = project_dir.join(&sbt_path);
        if !sbt_project_path.exists() {
            anyhow::bail!("sbt project path does not exist: {}", sbt_path);
        }

        let key = format!("sbt:{}", sbt_path);
        // 如果在工作空间根目录，添加到 workspace.dependencies
        if is_workspace_root {
            crate::config::writer::add_workspace_dependency_to_manifest(&manifest_path, &key, "")?;
            println!("已添加依赖: {} = {}", key, "sbt project (workspace)");
        } else {
            crate::config::writer::add_dependency_to_manifest(&manifest_path, &key, "")?;
            println!("已添加依赖: {} = {}", key, "sbt project");
        }
        return Ok(());
    }

    let (artifact, scala_ver, version) =
        parse_dep_spec(dep_spec, &project.package.scala_version).await?;

    let key = artifact.clone();

    // 对于Java依赖（artifact包含单冒号），不添加Scala版本后缀
    let full_key = if artifact.contains(':') && !artifact.contains("::") {
        // Java依赖格式：group:artifact
        key
    } else if !scala_ver.is_empty() && scala_ver != "latest" {
        format!("{}_{}", key, scala_ver)
    } else {
        key
    };

    // 使用依赖管理器验证依赖是否可用
    let dep_manager = crate::deps::default_dependency_manager();
    let dep = crate::deps::deps::Dependency::from_toml_key(&full_key, &version);

    // 验证依赖是否可用
    if let Err(e) = dep_manager.validate_dependency(&dep).await {
        anyhow::bail!("Failed to validate dependency {}: {}\nPlease check that the dependency coordinates are correct and the version exists.", full_key, e);
    }

    // 如果验证通过，下载依赖（使用coursier时会预先下载并缓存）
    if let Err(e) = dep_manager
        .prepare_dependencies(&[dep.clone()], &project_dir.join("target"))
        .await
    {
        anyhow::bail!("Failed to download dependency {}: {}\nPlease check your network connection and try again.", full_key, e);
    }

    // 如果在工作空间根目录，添加到 workspace.dependencies
    if is_workspace_root {
        crate::config::writer::add_workspace_dependency_to_manifest(
            &manifest_path,
            &full_key,
            &version,
        )?;
        println!(
            "已添加依赖: {} = {}",
            full_key,
            format!("{} (workspace)", version)
        );
    } else {
        crate::config::writer::add_dependency_to_manifest(&manifest_path, &full_key, &version)?;
        println!("已添加依赖: {} = {}", full_key, version);
    }
    Ok(())
}

async fn parse_dep_spec(
    spec: &str,
    default_scala_version: &str,
) -> anyhow::Result<(String, String, String)> {
    // 检查是否是Scala依赖（使用::）还是Java依赖（使用:）
    let is_scala_format = spec.contains("::");

    let (group, artifact_version, is_scala) = if is_scala_format {
        // Scala格式：group::artifact:version
        let parts: Vec<&str> = spec.split("::").collect();
        if parts.len() != 2 {
            anyhow::bail!("{}", "依赖格式无效，请使用完整格式：group::artifact:version，例如 org.typelevel::cats-core:2.9.0");
        }
        let group = parts[0];
        let artifact_version = parts[1];
        (group, artifact_version, true)
    } else {
        // Java格式：group:artifact:version
        let parts: Vec<&str> = spec.split(':').collect();
        if parts.len() != 3 {
            anyhow::bail!("{}", "依赖格式无效，请使用完整格式：group::artifact:version，例如 org.typelevel::cats-core:2.9.0");
        }
        let group = parts[0];
        let artifact = parts[1];
        let version = parts[2];
        // 对于Java格式，直接返回，不需要进一步解析
        let full_artifact = format!("{}:{}", group, artifact);
        return Ok((full_artifact, "".to_string(), version.to_string()));
    };

    let av_parts: Vec<&str> = artifact_version.split(':').collect();
    if av_parts.len() != 2 {
        anyhow::bail!("{}", "依赖格式无效，请使用完整格式：group::artifact:version，例如 org.typelevel::cats-core:2.9.0");
    }

    let artifact_with_scala = av_parts[0];
    let version = av_parts[1];

    // 检查artifact是否包含::，如果是，则报错，因为artifact不应该有::
    if artifact_with_scala.contains("::") {
        anyhow::bail!(
            "{}",
            "依赖格式无效，artifact 不应包含 '::', 请使用 group:artifact:version 格式"
        );
    }

    let artifact_parts: Vec<&str> = artifact_with_scala.split('@').collect();
    let (artifact, scala_ver) = if artifact_parts.len() == 2 {
        (artifact_parts[0].to_string(), artifact_parts[1])
    } else {
        (artifact_with_scala.to_string(), default_scala_version)
    };

    let full_artifact = format!("{}::{}", group, artifact);

    if version.is_empty() || version == "latest" {
        anyhow::bail!("{}", "必须明确指定版本，不允许使用 'latest'");
    }

    Ok((full_artifact, scala_ver.to_string(), version.to_string()))
}
