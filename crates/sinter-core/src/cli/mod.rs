//! 命令行接口 — 参数解析和命令定义

use clap::{Arg, Command};

pub mod commands;

// ━━━━━━━━━━━━━━━━━━━ Cli / Commands ━━━━━━━━━━━━━━━━━━━

#[derive(Debug)]
pub struct Cli {
    pub command: Option<Commands>,
    pub raw_matches: clap::ArgMatches,
}

impl std::ops::Deref for Cli {
    type Target = Option<Commands>;
    fn deref(&self) -> &Self::Target { &self.command }
}

#[derive(Debug, Clone)]
pub enum Commands {
    New { name: String },
    Init,
    Build,
    Run { file: Option<std::path::PathBuf>, lib: bool },
    Add { deps: Vec<String> },
    Test { file: Option<std::path::PathBuf> },
    Workspace { subcommand: WorkspaceCommands },
    Jsp { name: String },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum WorkspaceCommands {
    Add { paths: Vec<String> },
}

impl Cli {
    pub fn parse() -> Self { Self::parse_with_plugins(&[]) }

    pub fn parse_with_plugins(plugins: &[Box<dyn crate::core::CommandHandler>]) -> Self {
        let mut cmd = Command::new("sinter")
            .about("一个类似 Cargo 的 Scala 构建工具")
            .subcommand(Command::new("new").about("创建一个新的 Scala 项目")
                .arg(Arg::new("name").help("新项目的名称").required(true)))
            .subcommand(Command::new("init").about("初始化一个新的工作空间"))
            .subcommand(Command::new("build").about("构建 Scala 项目"))
            .subcommand(Command::new("run").about("运行 Scala 项目或特定文件")
                .arg(Arg::new("file").help("可选的要运行的 .scala 文件（相对于项目根目录）").value_name("FILE"))
                .arg(Arg::new("lib").long("lib").help("强制库模式（仅编译）").action(clap::ArgAction::SetTrue)))
            .subcommand(Command::new("add").about("向项目添加依赖")
                .arg(Arg::new("dep").help("依赖格式：group::artifact:version[@scala-version]")
                    .value_name("DEP").required(true).num_args(1..)))
            .subcommand(Command::new("test").about("运行测试")
                .arg(Arg::new("file").help("可选的测试文件或目录（相对于项目根目录）").value_name("FILE")))
            .subcommand(Command::new("workspace").about("工作空间管理")
                .subcommand(Command::new("add").about("向工作空间添加成员")
                    .arg(Arg::new("path").help("成员项目的路径").value_name("PATH").required(true).num_args(1..))));

        for handler in plugins {
            cmd = cmd.subcommand(handler.configure(Command::new(handler.name())));
        }

        let matches = cmd.get_matches();
        let command = parse_command_from_matches(&matches);
        Cli { command, raw_matches: matches }
    }
}

// ━━━━━━━━━━━━━━━━━━━ Parser ━━━━━━━━━━━━━━━━━━━

fn parse_command_from_matches(matches: &clap::ArgMatches) -> Option<Commands> {
    match matches.subcommand() {
        Some(("new", sub_m)) => Some(Commands::New {
            name: sub_m.get_one::<String>("name").unwrap().clone(),
        }),
        Some(("init", _)) => Some(Commands::Init),
        Some(("build", _)) => Some(Commands::Build),
        Some(("run", sub_m)) => Some(Commands::Run {
            file: sub_m.get_one::<String>("file").map(|s| std::path::PathBuf::from(s)),
            lib: sub_m.get_flag("lib"),
        }),
        Some(("add", sub_m)) => Some(Commands::Add {
            deps: sub_m.get_many::<String>("dep").unwrap_or_default().map(|s| s.to_string()).collect(),
        }),
        Some(("test", sub_m)) => Some(Commands::Test {
            file: sub_m.get_one::<String>("file").map(|s| std::path::PathBuf::from(s)),
        }),
        Some(("workspace", ws_m)) => match ws_m.subcommand() {
            Some(("add", sub_m)) => Some(Commands::Workspace {
                subcommand: WorkspaceCommands::Add {
                    paths: sub_m.get_many::<String>("path").unwrap_or_default().map(|s| s.to_string()).collect(),
                },
            }),
            _ => None,
        },
        Some(("jsp", sub_m)) => Some(Commands::Jsp {
            name: sub_m.get_one::<String>("name").unwrap().clone(),
        }),
        _ => None,
    }
}
