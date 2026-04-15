//! 配置加载器
//!
//! 负责从文件系统加载和解析配置文件

use anyhow::Context;
use config::Config;
use std::path::{Path, PathBuf};

use crate::models::*;

/// 加载项目配置
pub fn load_project(dir: &Path) -> anyhow::Result<Project> {
    let manifest_path = dir.join("project.toml");
    let settings = Config::builder()
        .add_source(config::File::from(manifest_path))
        .build()
        .context("Failed to load project configuration")?;
    let proj_dto: ProjectDto = settings
        .try_deserialize()
        .context("Failed to parse project configuration")?;
    let mut proj: Project = proj_dto.into();

    // 设置项目根路径，映射到实际目录
    proj.root_path = dir
        .canonicalize()
        .context("Failed to canonicalize project directory path")?;

    // 验证配置
    if let Err(errors) = proj.validate() {
        return Err(anyhow::anyhow!("项目配置验证失败:\n{}", errors.join("\n")));
    }

    Ok(proj)
}

/// 异步版本的项目配置加载
pub async fn load_project_async(dir: &Path) -> anyhow::Result<Project> {
    // 对于异步操作，如果需要可以扩展
    load_project(dir)
}

/// 查找工作空间根目录
pub fn find_workspace_root(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir;
    loop {
        let manifest = current.join("project.toml");
        if manifest.exists() {
            if let Ok(settings) = Config::builder()
                .add_source(config::File::from(manifest.clone()))
                .build()
            {
                if let Ok(project_dto) = settings.try_deserialize::<ProjectDto>() {
                    let project: Project = project_dto.into();
                    if project.workspace.is_some() {
                        return Some(current.to_path_buf());
                    }
                }
            }
        }
        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            break;
        }
    }
    None
}

/// 加载工作空间配置
pub fn load_workspace(dir: &Path) -> anyhow::Result<Option<(Project, Vec<Project>)>> {
    let manifest_path = dir.join("project.toml");
    if !manifest_path.exists() {
        return Ok(None);
    }

    let settings = Config::builder()
        .add_source(config::File::from(manifest_path))
        .build()
        .context("Failed to load workspace configuration")?;

    let root_project_dto: ProjectDto = settings
        .try_deserialize()
        .context("Failed to parse workspace configuration")?;
    let mut root_project: Project = root_project_dto.into();

    // 设置项目根路径
    root_project.root_path = dir
        .canonicalize()
        .context("Failed to canonicalize workspace directory path")?;

    if let Some(workspace) = &mut root_project.workspace {
        // 设置工作区根路径，映射到实际目录
        workspace.root_path = dir
            .canonicalize()
            .context("Failed to canonicalize workspace directory path")?;

        let mut members = Vec::new();
        for member_path in &workspace.members {
            let member_dir = dir.join(member_path);
            let member_project = load_project(&member_dir)
                .with_context(|| format!("Failed to load workspace member: {}", member_path))?;
            members.push(member_project);
        }
        Ok(Some((root_project, members)))
    } else {
        Ok(None)
    }
}
