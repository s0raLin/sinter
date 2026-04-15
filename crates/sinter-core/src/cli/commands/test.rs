use crate::toolkit::path::PathManager;

pub async fn cmd_test(cwd: &PathManager, file: Option<PathManager>) -> anyhow::Result<()> {
    let workspace_root = crate::config::loader::find_workspace_root(cwd);
    let (project, project_dir) = if let Some(ws_root) = workspace_root.as_ref() {
        // In workspace, check if this is a member project
        if let Some((_ws_proj, members)) = crate::config::loader::load_workspace(ws_root)? {
            let relative_path = cwd.relative_to(&PathManager::from(ws_root.clone()));
            if let Some(first_component) = relative_path.as_path().components().next() {
                let member_name = first_component.as_os_str().to_str().unwrap();
                if let Some(member) = members.into_iter().find(|m| m.package.name == member_name) {
                    (member, PathManager::from(ws_root.clone()).join(member_name))
                } else {
                    // Not a workspace member, treat as standalone project
                    let proj = crate::config::loader::load_project(cwd)?;
                    (proj, cwd.clone())
                }
            } else {
                // cwd == ws_root, treat as standalone project
                let proj = crate::config::loader::load_project(cwd)?;
                (proj, cwd.clone())
            }
        } else {
            // No workspace config, treat as standalone project
            let proj = crate::config::loader::load_project(cwd)?;
            (proj, cwd.clone())
        }
    } else {
        let proj = crate::config::loader::load_project(cwd)?;
        (proj, cwd.clone())
    };

    let deps = if let Some(ws_root) = workspace_root {
        let ws_proj = crate::config::loader::load_project(&ws_root)?;
        crate::deps::get_dependencies_with_workspace(&project, Some(&ws_proj))
    } else {
        crate::deps::get_dependencies(&project)
    };

    let test_target = if let Some(f) = file {
        f
    } else {
        PathManager::new(&project.package.test_dir)
    };

    let abs_test_target = project_dir.join(test_target.as_path());

    if !abs_test_target.exists_sync() {
        println!("No tests found in {}", test_target.to_path_buf().display());
        return Ok(());
    }

    // Use scala-cli test
    let mut args: Vec<String> = vec![
        "test".to_string(),
        abs_test_target.to_string_lossy().to_string(),
    ];

    for dep in deps {
        args.push("--dependency".to_string());
        args.push(dep.coord());
    }

    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = crate::build::execute_scala_cli(&args_str, Some(&project_dir)).await?;

    if !output.is_empty() {
        println!("{}", output);
    }

    Ok(())
}
