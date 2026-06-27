//! 依赖解析单元测试

#[cfg(test)]
mod tests {
    use sinter::deps::Dependency;

    // ━━━━━━━━━━━━━━━━━━━ Dependency::from_toml_key ━━━━━━━━━━━━━━━━━━━

    #[test]
    fn from_toml_key_scala_format() {
        // cats-core without _2.x suffix → is_scala = false (heuristic-based detection)
        let dep = Dependency::from_toml_key("org.typelevel::cats-core", "2.9.0");
        assert_eq!(dep.coord(), "org.typelevel:cats-core:2.9.0");
    }

    #[test]
    fn from_toml_key_java_format() {
        let dep = Dependency::from_toml_key("com.google.guava:guava", "31.1-jre");
        assert!(matches!(dep, Dependency::Maven { is_scala: false, .. }));
        assert_eq!(dep.coord(), "com.google.guava:guava:31.1-jre");
    }

    #[test]
    fn from_toml_key_scala_artifact_with_version_suffix() {
        let dep = Dependency::from_toml_key("org.typelevel::cats-core_2.13", "2.9.0");
        assert!(matches!(dep, Dependency::Maven { is_scala: true, .. }));
        assert!(dep.coord().contains("2.9.0"));
    }

    #[test]
    fn from_toml_key_sbt_format() {
        let dep = Dependency::from_toml_key("sbt:../my-sbt-project", "");
        assert!(matches!(dep, Dependency::Sbt { .. }));
        assert_eq!(dep.coord(), "sbt:../my-sbt-project");
        assert!(dep.is_sbt());
        assert_eq!(dep.sbt_path(), Some("../my-sbt-project"));
    }

    #[test]
    fn from_toml_key_sbt_path_with_slash() {
        let dep = Dependency::from_toml_key("../relative/path", "");
        assert!(matches!(dep, Dependency::Sbt { .. }));
        assert!(dep.is_sbt());
    }

    #[test]
    fn from_toml_key_simple_key() {
        let dep = Dependency::from_toml_key("my-lib", "1.0.0");
        assert!(matches!(dep, Dependency::Maven { .. }));
        assert_eq!(dep.coord(), ":my-lib:1.0.0");
    }

    #[test]
    fn maven_dep_is_not_sbt() {
        let dep = Dependency::from_toml_key("com.example:lib", "1.0");
        assert!(!dep.is_sbt());
        assert_eq!(dep.sbt_path(), None);
    }
}
