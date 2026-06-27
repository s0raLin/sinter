# Sinter

一个类似 Cargo 的 Scala 项目构建工具。

📖 **文档**: [English](README.md) | [中文](README_CN.md)

## 功能特性

- `sinter new <name>` — 创建新的 Scala 项目
- `sinter init` — 初始化工作空间
- `sinter build` — 构建项目，自动解析传递依赖
- `sinter run` — 运行 Scala 应用（单文件或项目）
- `sinter add <dep>` — 添加依赖到 `project.toml`
- `sinter test` — 运行测试
- `sinter workspace add <path>` — 向工作空间添加成员
- **插件系统** — 通过 `CommandHandler` trait 扩展自定义命令

## 安装

### 从源码安装

```bash
git clone https://github.com/s0raLin/sinter.git
cd sinter
cargo build --release
# 将 target/release/sinter 添加到 PATH
```

### 前置要求

- **Rust**（最新稳定版）
- **Scala CLI** (`scala-cli`) — 用于编译和执行。
  安装：`curl -fL https://github.com/VirtusLab/scala-cli/releases/latest/download/scala-cli-x86_64-pc-linux.gz | gzip -d > scala-cli && chmod +x scala-cli && mv scala-cli ~/.local/bin/`

## 快速开始

```bash
# 创建新项目
sinter new hello-scala
cd hello-scala

# 添加依赖（Scala 格式 — group::artifact:version）
sinter add org.typelevel::cats-core:2.10.0

# 构建并运行
sinter build
sinter run
```

## 命令详解

### `sinter new <name>`

创建新的 Scala 项目，目录结构如下：

```
hello-scala/
├── project.toml
└── src/main/scala/
    └── Main.scala
```

### `sinter init`

在当前目录创建工作空间，生成包含 `[workspace]` 段的 `project.toml`。

### `sinter workspace add <path>`

将已有项目添加到工作空间。`<path>` 是相对于工作空间根目录的路径。

### `sinter build`

编译所有 Scala 源文件。自动解析传递依赖并配置 BSP 以支持 IDE 集成。

- 在工作空间根目录中：构建**所有**成员
- 在工作空间子目录中：仅构建当前成员
- 独立项目：构建单项目

### `sinter run [file] [--lib]`

运行 Scala 应用。

- 无参数：运行 `project.toml` 中 `[package]` 段 `main` 字段指定的主文件
- `sinter run src/main/scala/App.scala` — 运行指定文件
- `sinter run --lib` — 仅编译（库模式）

### `sinter test [file]`

运行项目测试目录中的测试（`project.toml` 中 `test_dir` 字段，默认 `src/test/scala`）。

### `sinter add <dep>`

向 `project.toml` 添加依赖并验证依赖是否可用。

**依赖格式：**

| 格式 | 示例 | 类型 |
|------|------|------|
| `group::artifact:version` | `org.typelevel::cats-core:2.10.0` | Scala（双冒号 `::`） |
| `group:artifact:version` | `com.google.guava:guava:31.1-jre` | Java（单冒号） |
| `sbt:path` | `sbt:../my-sbt-project` | SBT 子项目 |

- Scala 依赖可通过 `@` 指定 Scala 版本：`group::artifact@2.13:version`
- 在工作空间根目录中添加的依赖将自动写入 `[workspace.dependencies]`

## 配置 — `project.toml`

```toml
[package]
name = "my-project"
version = "0.1.0"
main = "Main"
scala_version = "2.13"
source_dir = "src/main/scala"
target_dir = "target"
test_dir = "src/test/scala"
backend = "scala-cli"

[dependencies]
"org.typelevel::cats-core" = "2.10.0"
```

| 字段 | 默认值 | 说明 |
|------|-------|------|
| `name` | — | 项目名称（必填） |
| `version` | — | 项目版本（必填） |
| `main` | `Main` | 主类名（不含 `.scala`） |
| `scala_version` | `2.13` | Scala 版本（`2.13` 或 `3.x`） |
| `source_dir` | `src/main/scala` | 源代码目录 |
| `target_dir` | `target` | 输出目录 |
| `test_dir` | `src/test/scala` | 测试目录 |
| `backend` | `scala-cli` | 构建后端（`scala-cli`/`sbt`/`gradle`/`maven`） |

工作空间使用 `[workspace]` 段替代 `[package]`：

```toml
[workspace]
members = ["member-a", "member-b"]

[workspace.dependencies]
"org.typelevel::cats-core" = "2.10.0"
```

## 插件系统

Sinter 采用轻量级插件架构。通过 Builder 模式在启动时注册插件命令：

```rust
use sinter::{Sinter, CommandHandler};
use sinter_plugins::jsp_plugin;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Sinter::new()
        .plugin(jsp_plugin())
        .run()
        .await
}
```

**内置插件**：JSP 项目生成器（`sinter jsp <name>`）

实现 `CommandHandler` trait 即可创建自己的插件：

```rust
use async_trait::async_trait;
use clap::{Arg, ArgMatches, Command};
use sinter::CommandHandler;
use std::path::PathBuf;

pub struct MyPlugin;

#[async_trait]
impl CommandHandler for MyPlugin {
    fn name(&self) -> &'static str { "mycommand" }
    fn about(&self) -> &'static str { "命令描述" }

    fn configure(&self, cmd: Command) -> Command {
        cmd.about(self.about())
            .arg(Arg::new("input").help("输入值").required(true))
    }

    async fn execute(&self, matches: &ArgMatches, cwd: &PathBuf) -> anyhow::Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        println!("处理: {}", input);
        Ok(())
    }
}
```

### 插件命令执行流程

```
用户输入: sinter jsp myapp
    ↓
build_cli_command() — 遍历 plugins, 为每个 handler 注册子命令
    ↓
clap 解析 → raw_matches.subcommand() = ("jsp", matches)
    ↓
Executor::execute():
    1. 遍历 self.plugins, 找到 name() == "jsp" 的 handler
    2. 调用 handler.execute(matches, &cwd)
```

## 项目结构

```
crates/
├── sinter-core/      # 核心库 — 模型、依赖、构建、CLI、插件 trait
├── sinter-plugins/   # 官方插件（JSP 项目生成器）
└── sinter-cli/       # 二进制入口
```

```
sinter-core/src/
├── lib.rs            # 公共 API 入口
├── models/mod.rs     # Project, Dependency, Workspace 领域模型
├── config/mod.rs     # project.toml 加载与写入
├── deps/
│   ├── mod.rs        # Dependency 类型 + 解析/添加依赖
│   └── manager.rs    # DependencyManager trait + ScalaCli 实现
├── build/
│   ├── mod.rs
│   ├── scala_cli.rs  # Scala CLI 发现、下载、执行（含 30s 超时）
│   └── runner.rs     # run_scala_file + build_with_deps
├── cli/
│   ├── mod.rs        # Cli + Commands 枚举 + 参数解析
│   └── commands.rs   # 全部 7 个内置命令的执行逻辑
├── core/
│   ├── mod.rs
│   ├── app.rs        # Sinter Builder + Executor
│   └── handler.rs    # CommandHandler trait
├── ide/mod.rs        # BSP 协议配置
└── toolkit/          # 工具函数（路径、文件、模板、HTTP 等）
```

## 测试

```bash
cargo test     # 53 个单元测试，覆盖 models、deps、CLI 解析
```

## 故障排除

- **找不到 Scala CLI**：安装 Scala CLI（见前置要求）。Sinter 首次运行时会尝试自动下载一次。
- **`sinter build` 卡住**：Scala CLI 首次运行可能会下载 Coursier 元数据（约 30 秒超时）。请等待或按 Ctrl+C 重试。
- **依赖格式错误**：Scala 依赖使用 `group::artifact:version`（双冒号），Java 依赖使用 `group:artifact:version`（单冒号）。
- **版本不允许 `latest`**：Sinter 需要明确的版本号以确保可复现构建。

## 许可证

MIT — 详见 [LICENSE](LICENSE)。
