// src/cli/commands/mod.rs
pub mod init;
pub mod new;
pub mod test;
pub mod workspace;

// 导出命令函数
pub use init::cmd_init;
pub use new::cmd_new;
pub use test::cmd_test;
pub use workspace::cmd_workspace;

/// 极致简化的命令定义宏
#[macro_export]
macro_rules! plugin_command {
    ($name:ident, $about:expr, $body:block) => {
        pub struct $name;

        #[async_trait::async_trait]
        impl crate::core::CommandHandler for $name {
            fn name(&self) -> &'static str {
                stringify!($name).to_lowercase().trim_end_matches("command")
            }

            fn about(&self) -> &'static str {
                $about
            }

            async fn execute(
                &self,
                matches: &clap::ArgMatches,
                cwd: &std::path::PathBuf,
            ) -> anyhow::Result<()> {
                $body
            }
        }

        inventory::submit! {
            Box::new($name) as Box<dyn crate::core::CommandHandler>
        }
    };
}

/// 超级简单的插件定义宏 - 无需了解内部细节
#[macro_export]
macro_rules! simple_plugin {
    ($cmd_name:literal, $description:literal, $config:expr, $handler:expr) => {
        paste::paste! {
            pub struct [<Simple $cmd_name:camel Plugin>];

            #[async_trait::async_trait]
            impl crate::core::CommandHandler for [<Simple $cmd_name:camel Plugin>] {
                fn name(&self) -> &'static str {
                    $cmd_name
                }

                fn about(&self) -> &'static str {
                    $description
                }

                fn configure(&self, cmd: clap::Command) -> clap::Command {
                    let config_fn = $config;
                    config_fn(cmd.about(self.about()))
                }

                async fn execute(&self, matches: &clap::ArgMatches, cwd: &std::path::PathBuf) -> anyhow::Result<()> {
                    let handler = $handler;
                    handler(matches.clone(), cwd.clone()).await
                }
            }

            // 导出插件结构体供注册使用
            pub use [<Simple $cmd_name:camel Plugin>] as [<plugin_ $cmd_name>];
        }
    };
}
