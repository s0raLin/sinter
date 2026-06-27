//! 配置管理 — 加载、解析和写入 project.toml

use anyhow::Context;
use config::Config;
use std::path::{Path, PathBuf};

use crate::models::*;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 加载 ━━━━━━━━━━━━━━━━━━━━━━━━━

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
    proj.root_path = dir
        .canonicalize()
        .context("Failed to canonicalize project directory path")?;
    if let Err(errors) = proj.validate() {
        return Err(anyhow::anyhow!("项目配置验证失败:\n{}", errors.join("\n")));
    }
    Ok(proj)
}

/// 向上查找工作空间根目录
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
        current = current.parent()?;
    }
}

/// 加载工作空间配置 (root project + members)
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
    root_project.root_path = dir
        .canonicalize()
        .context("Failed to canonicalize workspace directory path")?;

    if let Some(workspace) = &mut root_project.workspace {
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 写入 ━━━━━━━━━━━━━━━━━━━━━━━━━

use crate::toolkit::os::{read_sync, write_sync, PathWrapper};
use toml_edit::{value, DocumentMut, Item, Table};

/// 向项目 manifest 添加依赖
pub fn add_dependency_to_manifest(
    manifest_path: &Path, key: &str, version: &str,
) -> anyhow::Result<()> {
    modify_toml_document(manifest_path, |doc| {
        ensure_table_exists(doc, "dependencies");
        if let Some(deps_table) = doc.get_mut("dependencies").and_then(|d| d.as_table_mut()) {
            deps_table[key] = value(version.to_string());
            format_table(deps_table);
        }
        Ok(())
    })
}

/// 向工作空间 manifest 添加依赖
pub fn add_workspace_dependency_to_manifest(
    manifest_path: &Path, key: &str, version: &str,
) -> anyhow::Result<()> {
    modify_toml_document(manifest_path, |doc| {
        ensure_table_exists(doc, "workspace");
        if let Some(ws_table) = doc.get_mut("workspace").and_then(|w| w.as_table_mut()) {
            ensure_table_exists_in_table(ws_table, "dependencies");
            if let Some(deps_item) = ws_table.get_mut("dependencies") {
                if let Some(deps_table) = deps_item.as_table_mut() {
                    deps_table[key] = value(version.to_string());
                    format_table(deps_table);
                }
            }
        }
        Ok(())
    })
}

/// 添加工作空间成员
pub fn add_workspace_member(manifest_path: &Path, member_path: &str) -> anyhow::Result<()> {
    modify_toml_document(manifest_path, |doc| {
        ensure_table_exists(doc, "workspace");
        if let Some(ws_table) = doc.get_mut("workspace").and_then(|w| w.as_table_mut()) {
            if !ws_table.contains_key("members") {
                ws_table.insert("members", Item::Value(toml_edit::Value::Array(Default::default())));
            }
            if let Some(members_array) = ws_table.get_mut("members").and_then(|m| m.as_array_mut()) {
                let exists = members_array.iter().any(|v| v.as_str() == Some(member_path));
                if exists {
                    anyhow::bail!("Member '{}' already exists in workspace", member_path);
                }
                members_array.push(member_path);
            }
        }
        Ok(())
    })
}

fn modify_toml_document<F>(manifest_path: &Path, modifier: F) -> anyhow::Result<()>
where F: FnOnce(&mut DocumentMut) -> anyhow::Result<()>
{
    let path_wrapper = PathWrapper::new(manifest_path);
    let content = read_sync(&path_wrapper)
        .with_context(|| format!("Failed to read file: {}", manifest_path.display()))?;
    let mut doc: DocumentMut = content.parse().context("Failed to parse TOML document")?;
    modifier(&mut doc)?;
    write_sync(&path_wrapper, &doc.to_string())
        .with_context(|| format!("Failed to write file: {}", manifest_path.display()))?;
    Ok(())
}

fn ensure_table_exists(doc: &mut DocumentMut, key: &str) {
    if !doc.contains_key(key) {
        doc.insert(key, Item::Table(Table::new()));
    }
}

fn ensure_table_exists_in_table(table: &mut Table, key: &str) {
    if !table.contains_key(key) {
        table.insert(key, Item::Table(Table::new()));
    }
}

fn format_table(table: &mut Table) {
    let decor = table.decor_mut();
    decor.set_prefix("\n");
    decor.set_suffix("\n");
}
