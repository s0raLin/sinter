//! 配置写入器
//!
//! 负责修改和写入配置文件

use crate::toolkit::os::{read_sync, write_sync, PathWrapper};
use anyhow::Context;
use std::path::Path;
use toml_edit::{value, DocumentMut, Item, Table};

/// 向项目manifest添加依赖
pub fn add_dependency_to_manifest(
    manifest_path: &Path,
    key: &str,
    version: &str,
) -> anyhow::Result<()> {
    modify_toml_document(manifest_path, |doc| {
        // 确保 dependencies 表存在
        ensure_table_exists(doc, "dependencies");

        let deps_item = doc.get_mut("dependencies").unwrap();
        if let Some(deps_table) = deps_item.as_table_mut() {
            deps_table[key] = value(version.to_string());
            format_table(deps_table);
        }
        Ok(())
    })
}

/// 向工作空间manifest添加依赖
pub fn add_workspace_dependency_to_manifest(
    manifest_path: &Path,
    key: &str,
    version: &str,
) -> anyhow::Result<()> {
    modify_toml_document(manifest_path, |doc| {
        // 确保 workspace 表存在
        ensure_table_exists(doc, "workspace");

        let ws_item = doc.get_mut("workspace").unwrap();
        if let Some(ws_table) = ws_item.as_table_mut() {
            // 确保 workspace.dependencies 表存在
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
        // 确保 workspace 表存在
        ensure_table_exists(doc, "workspace");

        let ws_item = doc.get_mut("workspace").unwrap();
        if let Some(ws_table) = ws_item.as_table_mut() {
            // 确保 members 数组存在
            if !ws_table.contains_key("members") {
                ws_table.insert(
                    "members",
                    Item::Value(toml_edit::Value::Array(Default::default())),
                );
            }

            if let Some(members_item) = ws_table.get_mut("members") {
                if let Some(members_array) = members_item.as_array_mut() {
                    // 检查成员是否已存在
                    let exists = members_array
                        .iter()
                        .any(|v| v.as_str() == Some(member_path));
                    if exists {
                        anyhow::bail!("Member '{}' already exists in workspace", member_path);
                    }
                    members_array.push(member_path);
                }
            }
        }
        Ok(())
    })
}

/// 通用TOML文档修改函数
fn modify_toml_document<F>(manifest_path: &Path, modifier: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut DocumentMut) -> anyhow::Result<()>,
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

/// 确保表存在
fn ensure_table_exists(doc: &mut DocumentMut, key: &str) {
    if !doc.contains_key(key) {
        doc.insert(key, Item::Table(Table::new()));
    }
}

/// 确保子表存在
fn ensure_table_exists_in_table(table: &mut Table, key: &str) {
    if !table.contains_key(key) {
        table.insert(key, Item::Table(Table::new()));
    }
}

/// 格式化表
fn format_table(table: &mut Table) {
    let decor = table.decor_mut();
    decor.set_prefix("\n");
    decor.set_suffix("\n");
}
