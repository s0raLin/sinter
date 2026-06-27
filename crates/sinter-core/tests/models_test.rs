//! 数据模型单元测试

#[cfg(test)]
mod tests {
    use sinter::models::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    // ━━━━━━━━━━━━━━━━━━━ DependencySpec 测试 ━━━━━━━━━━━━━━━━━━━

    #[test]
    fn dep_spec_simple_valid() {
        let spec = DependencySpec::Simple("com.example:my-lib:1.0.0".into());
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn dep_spec_simple_version_only() {
        let spec = DependencySpec::Simple("2.13.0".into());
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn dep_spec_simple_empty() {
        let spec = DependencySpec::Simple("".into());
        assert!(spec.validate().is_err());
    }

    #[test]
    fn dep_spec_simple_invalid_format() {
        let spec = DependencySpec::Simple("com.example:my-lib".into());
        assert!(spec.validate().is_err());
    }

    #[test]
    fn dep_spec_detailed_workspace_no_version() {
        let spec = DependencySpec::Detailed(DependencyDetail {
            version: None,
            workspace: true,
        });
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn dep_spec_detailed_workspace_with_version_error() {
        let spec = DependencySpec::Detailed(DependencyDetail {
            version: Some("1.0.0".into()),
            workspace: true,
        });
        assert!(spec.validate().is_err());
    }

    #[test]
    fn dep_spec_get_version_simple() {
        let spec = DependencySpec::Simple("com.example:my-lib:2.1.3".into());
        assert_eq!(spec.get_version(), Some("2.1.3"));
    }

    #[test]
    fn dep_spec_get_version_detailed() {
        let spec = DependencySpec::Detailed(DependencyDetail {
            version: Some("3.0.0".into()),
            workspace: false,
        });
        assert_eq!(spec.get_version(), Some("3.0.0"));
    }

    #[test]
    fn dep_spec_get_version_none() {
        let spec = DependencySpec::Detailed(DependencyDetail {
            version: None,
            workspace: true,
        });
        assert_eq!(spec.get_version(), None);
    }

    // ━━━━━━━━━━━━━━━━━━━ Package 验证测试 ━━━━━━━━━━━━━━━━━━━

    fn valid_package() -> Package {
        Package {
            name: "my-project".into(),
            version: "0.1.0".into(),
            main: Some("Main".into()),
            scala_version: "2.13".into(),
            source_dir: "src/main/scala".into(),
            target_dir: "target".into(),
            test_dir: "src/test/scala".into(),
            backend: "scala-cli".into(),
        }
    }

    #[test]
    fn package_valid() {
        let pkg = valid_package();
        assert!(pkg.validate().is_ok());
    }

    #[test]
    fn package_empty_name() {
        let mut pkg = valid_package();
        pkg.name = "".into();
        assert!(pkg.validate().is_err());
    }

    #[test]
    fn package_empty_version() {
        let mut pkg = valid_package();
        pkg.version = "".into();
        assert!(pkg.validate().is_err());
    }

    #[test]
    fn package_invalid_scala_version() {
        let mut pkg = valid_package();
        pkg.scala_version = "1.0".into();
        assert!(pkg.validate().is_err());
    }

    #[test]
    fn package_scala_3_valid() {
        let mut pkg = valid_package();
        pkg.scala_version = "3.3.0".into();
        assert!(pkg.validate().is_ok());
    }

    #[test]
    fn package_empty_source_dir() {
        let mut pkg = valid_package();
        pkg.source_dir = "".into();
        assert!(pkg.validate().is_err());
    }

    #[test]
    fn package_empty_target_dir() {
        let mut pkg = valid_package();
        pkg.target_dir = "".into();
        assert!(pkg.validate().is_err());
    }

    #[test]
    fn package_invalid_backend() {
        let mut pkg = valid_package();
        pkg.backend = "make".into();
        assert!(pkg.validate().is_err());
    }

    #[test]
    fn package_sbt_backend_valid() {
        let mut pkg = valid_package();
        pkg.backend = "sbt".into();
        assert!(pkg.validate().is_ok());
    }

    #[test]
    fn package_gradle_backend_valid() {
        let mut pkg = valid_package();
        pkg.backend = "gradle".into();
        assert!(pkg.validate().is_ok());
    }

    // ━━━━━━━━━━━━━━━━━━━ Workspace 验证测试 ━━━━━━━━━━━━━━━━━━━

    #[test]
    fn workspace_empty_members() {
        let ws = Workspace {
            root_path: PathBuf::from("/tmp/test"),
            members: vec![],
            dependencies: HashMap::new(),
        };
        assert!(ws.validate().is_err());
    }

    #[test]
    fn workspace_valid() {
        let ws = Workspace {
            root_path: PathBuf::from("/tmp/test"),
            members: vec!["member-a".into()],
            dependencies: HashMap::new(),
        };
        assert!(ws.validate().is_ok());
    }

    #[test]
    fn workspace_invalid_dependency() {
        let mut deps = HashMap::new();
        deps.insert(
            "bad-dep".into(),
            DependencySpec::Simple("com.example:lib".into()),
        );
        let ws = Workspace {
            root_path: PathBuf::from("/tmp/test"),
            members: vec!["member-a".into()],
            dependencies: deps,
        };
        assert!(ws.validate().is_err());
    }

    // ━━━━━━━━━━━━━━━━━━━ Project 方法测试 ━━━━━━━━━━━━━━━━━━━

    fn valid_project() -> Project {
        Project {
            root_path: PathBuf::from("/tmp/test"),
            package: valid_package(),
            dependencies: HashMap::new(),
            workspace: None,
        }
    }

    #[test]
    fn project_name() {
        let p = valid_project();
        assert_eq!(p.get_name(), "my-project");
    }

    #[test]
    fn project_backend() {
        let p = valid_project();
        assert_eq!(p.get_backend(), "scala-cli");
    }

    #[test]
    fn project_source_dir() {
        let p = valid_project();
        assert_eq!(p.get_source_dir(), "src/main/scala");
    }

    #[test]
    fn project_target_dir() {
        let p = valid_project();
        assert_eq!(p.get_target_dir(), "target");
    }

    #[test]
    fn project_main_file_path() {
        let p = valid_project();
        let path = p.get_main_file_path();
        assert!(path.to_string_lossy().contains("Main.scala"));
        assert!(path.to_string_lossy().contains("src/main/scala"));
    }

    #[test]
    fn project_main_file_custom() {
        let mut p = valid_project();
        p.package.main = Some("App".into());
        let path = p.get_main_file_path();
        assert!(path.to_string_lossy().contains("App.scala"));
    }

    #[test]
    fn project_is_not_workspace_root() {
        let p = valid_project();
        assert!(!p.is_workspace_root());
    }

    #[test]
    fn project_is_workspace_root() {
        let mut p = valid_project();
        p.workspace = Some(Workspace {
            root_path: PathBuf::from("/tmp/test"),
            members: vec!["foo".into()],
            dependencies: HashMap::new(),
        });
        assert!(p.is_workspace_root());
    }

    #[test]
    fn project_validate_invalid_dep() {
        let mut deps = HashMap::new();
        deps.insert(
            "broken".into(),
            DependencySpec::Simple("bad:format".into()),
        );
        let p = Project {
            root_path: PathBuf::from("/tmp/test"),
            package: valid_package(),
            dependencies: deps,
            workspace: None,
        };
        assert!(p.validate().is_err());
    }

    // ━━━━━━━━━━━━━━━━━━━ DTO 转换测试 ━━━━━━━━━━━━━━━━━━━

    #[test]
    fn project_dto_roundtrip() {
        let dto = ProjectDto {
            package: PackageDto {
                name: "test".into(),
                version: "1.0".into(),
                main: None,
                scala_version: "2.13".into(),
                source_dir: "src".into(),
                target_dir: "out".into(),
                test_dir: "tests".into(),
                backend: "scala-cli".into(),
            },
            dependencies: HashMap::new(),
            workspace: None,
        };
        let project: Project = dto.into();
        assert_eq!(project.get_name(), "test");
        assert_eq!(project.package.version, "1.0");
    }

    #[test]
    fn dep_spec_dto_roundtrip_simple() {
        let dto = DependencyDto::Simple("com.example:lib:1.0".into());
        let spec: DependencySpec = dto.into();
        assert_eq!(spec.get_version(), Some("1.0"));
    }

    #[test]
    fn dep_spec_dto_roundtrip_detailed() {
        let dto = DependencyDto::Detailed(DependencyDetailDto {
            version: Some("2.0".into()),
            workspace: true,
        });
        let spec: DependencySpec = dto.into();
        assert_eq!(spec.get_version(), Some("2.0"));
    }
}
