//! CLI 参数解析单元测试

#[cfg(test)]
mod tests {
    use sinter::Cli;

    #[test]
    fn parse_new_command() {
        let cli = Cli::parse_from_args(&["sinter", "new", "myproject"]);
        assert!(matches!(&*cli, Some(sinter::Commands::New { name, .. }) if name == "myproject"));
    }

    #[test]
    fn parse_init_command() {
        let cli = Cli::parse_from_args(&["sinter", "init"]);
        assert!(matches!(&*cli, Some(sinter::Commands::Init)));
    }

    #[test]
    fn parse_build_command() {
        let cli = Cli::parse_from_args(&["sinter", "build"]);
        assert!(matches!(&*cli, Some(sinter::Commands::Build)));
    }

    #[test]
    fn parse_run_command_no_args() {
        let cli = Cli::parse_from_args(&["sinter", "run"]);
        assert!(matches!(&*cli, Some(sinter::Commands::Run { file: None, lib: false })));
    }

    #[test]
    fn parse_run_command_with_file() {
        let cli = Cli::parse_from_args(&["sinter", "run", "src/Main.scala"]);
        assert!(matches!(&*cli, Some(sinter::Commands::Run { .. })));
    }

    #[test]
    fn parse_run_command_lib_flag() {
        let cli = Cli::parse_from_args(&["sinter", "run", "--lib"]);
        assert!(matches!(&*cli, Some(sinter::Commands::Run { file: None, lib: true })));
    }

    #[test]
    fn parse_add_command() {
        let cli = Cli::parse_from_args(&["sinter", "add", "com.example:lib:1.0"]);
        assert!(
            matches!(&*cli, Some(sinter::Commands::Add { deps }) if deps.contains(&"com.example:lib:1.0".to_string()))
        );
    }

    #[test]
    fn parse_add_multiple_deps() {
        let cli = Cli::parse_from_args(&["sinter", "add", "a:b:1", "c:d:2"]);
        assert!(
            matches!(&*cli, Some(sinter::Commands::Add { deps }) if deps.len() == 2)
        );
    }

    #[test]
    fn parse_test_command_no_args() {
        let cli = Cli::parse_from_args(&["sinter", "test"]);
        assert!(matches!(&*cli, Some(sinter::Commands::Test { file: None })));
    }

    #[test]
    fn parse_test_command_with_file() {
        let cli = Cli::parse_from_args(&["sinter", "test", "src/test/scala"]);
        assert!(matches!(&*cli, Some(sinter::Commands::Test { .. })));
    }

    #[test]
    fn parse_workspace_add_command() {
        let cli = Cli::parse_from_args(&["sinter", "workspace", "add", "member-a"]);
        assert!(matches!(&*cli, Some(sinter::Commands::Workspace { .. })));
    }

    #[test]
    fn no_command_returns_none() {
        let cli = Cli::parse_from_args(&["sinter"]);
        assert!(cli.command.is_none());
    }

}
