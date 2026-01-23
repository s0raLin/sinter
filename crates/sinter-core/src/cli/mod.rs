// src/cli/mod.rs
use clap::{Arg, Command};

pub mod commands;
pub mod builtin;
pub mod parser;



#[derive(Debug)]
pub struct Cli {
    pub command: Option<Commands>,
    pub raw_matches: clap::ArgMatches,
}

impl std::ops::Deref for Cli {
    type Target = Option<Commands>;

    fn deref(&self) -> &Self::Target {
        &self.command
    }
}


// 命令枚举定义
#[derive(Debug, Clone)]
pub enum Commands {
    New {name: String},
    Init,
    Build,
    Run {file: Option<std::path::PathBuf>,lib: bool,},
    Add {deps: Vec<String>,},
    Test {file: Option<std::path::PathBuf>,},
    Workspace {subcommand: WorkspaceCommands,},
    Jsp {name: String,},
}


#[derive(clap::Subcommand, Debug, Clone)]
pub enum WorkspaceCommands {
    Add {paths: Vec<String>,},
}

impl Cli {
    pub fn parse() -> Self {
        Self::parse_with_plugins(&[])
    }

    pub fn parse_with_plugins(plugins: &[Box<dyn crate::core::CommandHandler>]) -> Self {
        let mut cmd = Command::new("sinter")
            .about("一个类似 Cargo 的 Scala 构建工具")
            .subcommand(
                Command::new("new")
                    .about("创建一个新的 Scala 项目")
                    .arg(
                        Arg::new("name")
                            .help("新项目的名称")
                            .required(true)
                    )
            )
            .subcommand(
                Command::new("init")
                    .about("初始化一个新的工作空间")
            )
            .subcommand(
                Command::new("build")
                    .about("构建 Scala 项目")
            )
            .subcommand(
                Command::new("run")
                    .about("运行 Scala 项目或特定文件")
                    .arg(
                        Arg::new("file")
                            .help("可选的要运行的 .scala 文件（相对于项目根目录）")
                            .value_name("FILE")
                    )
                    .arg(
                        Arg::new("lib")
                            .long("lib")
                            .help("强制库模式（仅编译）")
                            .action(clap::ArgAction::SetTrue)
                    )
            )
            .subcommand(
                Command::new("add")
                    .about("向项目添加依赖")
                    .arg(
                        Arg::new("dep")
                            .help("依赖格式：group::artifact:version[@scala-version]")
                            .value_name("DEP")
                            .required(true)
                            .num_args(1..)
                    )
            )
            .subcommand(
                Command::new("test")
                    .about("运行测试")
                    .arg(
                        Arg::new("file")
                            .help("可选的测试文件或目录（相对于项目根目录）")
                            .value_name("FILE")
                    )
            )
            .subcommand(
                Command::new("workspace")
                    .about("工作空间管理")
                    .subcommand(
                        Command::new("add")
                            .about("向工作空间添加成员")
                            .arg(
                                Arg::new("path")
                                    .help("成员项目的路径")
                                    .value_name("PATH")
                                    .required(true)
                                    .num_args(1..)
                            )
                    )
            );

        // 自动添加所有插件命令
        for handler in plugins {
            cmd = cmd.subcommand(handler.configure(Command::new(handler.name())));
        }

        let matches = cmd.get_matches();

        let command = parser::parse_command_from_matches(&matches);

        Cli { command, raw_matches: matches }
    }
}