//! JSP 项目生成插件
//!
//! 实现 CommandHandler trait 即可创建一个新命令。

use async_trait::async_trait;
use clap::{Arg, ArgMatches, Command};
use sinter::CommandHandler;
use std::path::PathBuf;
use tokio::fs;

fn pom_xml_template() -> String {
    include_str!("../templates/pom.xml.template").to_string()
}
fn web_xml_template() -> String {
    include_str!("../templates/web.xml.template").to_string()
}
fn index_jsp_template() -> String {
    include_str!("../templates/index.jsp.template").to_string()
}

pub struct JspPlugin;

#[async_trait]
impl CommandHandler for JspPlugin {
    fn name(&self) -> &'static str { "jsp" }
    fn about(&self) -> &'static str { "Generate a new JSP project" }

    fn configure(&self, cmd: Command) -> Command {
        cmd.about(self.about()).arg(
            Arg::new("name").help("Name of the JSP project").required(true)
        )
    }

    async fn execute(&self, matches: &ArgMatches, cwd: &PathBuf) -> anyhow::Result<()> {
        let name = matches.get_one::<String>("name").expect("name argument is required");
        let proj_dir = cwd.join(name);
        if proj_dir.exists() {
            println!("JSP project '{}' already exists", name);
            return Ok(());
        }

        fs::create_dir_all(proj_dir.join("src/main/webapp/WEB-INF")).await?;
        fs::create_dir_all(proj_dir.join("src/main/java")).await?;
        fs::create_dir_all(proj_dir.join("src/main/resources")).await?;

        fs::write(proj_dir.join("pom.xml"), pom_xml_template().replace("{{name}}", name)).await?;
        fs::write(proj_dir.join("src/main/webapp/WEB-INF/web.xml"), web_xml_template()).await?;
        fs::write(proj_dir.join("src/main/webapp/index.jsp"), index_jsp_template()).await?;

        println!("Created JSP project '{}'", name);
        println!("To build and run:");
        println!("  cd {}", name);
        println!("  mvn clean package");
        Ok(())
    }
}

pub fn jsp_plugin() -> JspPlugin { JspPlugin }
