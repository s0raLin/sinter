// src/cli/parser.rs
use super::{Commands, WorkspaceCommands};

// 辅助函数：安全提取必需的字符串参数
fn extract_required_string(matches: &clap::ArgMatches, key: &str) -> String {
    matches.get_one::<String>(key).unwrap().clone()
}

// 辅助函数：安全提取可选的字符串参数并转换为PathBuf
fn extract_optional_path(matches: &clap::ArgMatches, key: &str) -> Option<std::path::PathBuf> {
    matches
        .get_one::<String>(key)
        .map(|s| std::path::PathBuf::from(s))
}

pub fn parse_command_from_matches(matches: &clap::ArgMatches) -> Option<Commands> {
    match matches.subcommand() {
        Some(("new", sub_m)) => Some(Commands::New {
            name: extract_required_string(sub_m, "name"),
        }),
        Some(("init", _)) => Some(Commands::Init),
        Some(("build", _)) => Some(Commands::Build),
        Some(("run", sub_m)) => Some(Commands::Run {
            file: extract_optional_path(sub_m, "file"),
            lib: sub_m.get_flag("lib"),
        }),
        Some(("add", sub_m)) => Some(Commands::Add {
            deps: sub_m
                .get_many::<String>("dep")
                .unwrap_or_default()
                .map(|s| s.to_string())
                .collect(),
        }),
        Some(("test", sub_m)) => Some(Commands::Test {
            file: extract_optional_path(sub_m, "file"),
        }),
        Some(("workspace", ws_m)) => match ws_m.subcommand() {
            Some(("add", sub_m)) => Some(Commands::Workspace {
                subcommand: WorkspaceCommands::Add {
                    paths: sub_m
                        .get_many::<String>("path")
                        .unwrap_or_default()
                        .map(|s| s.to_string())
                        .collect(),
                },
            }),
            _ => None,
        },
        Some(("jsp", sub_m)) => Some(Commands::Jsp {
            name: extract_required_string(sub_m, "name"),
        }),
        _ => None,
    }
}
