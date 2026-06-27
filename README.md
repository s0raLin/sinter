# Sinter

A Cargo-like build tool for Scala projects.

📖 **Documentation**: [English](README.md) | [中文](README_CN.md)

## Features

- `sinter new <name>` — create a new Scala project
- `sinter init` — initialize a workspace
- `sinter build` — build the project with dependency resolution
- `sinter run` — run a Scala application (single file or project)
- `sinter add <dep>` — add a dependency to `project.toml`
- `sinter test` — run tests
- `sinter workspace add <path>` — add a member to the workspace
- **Plugin system** — extend sinter with custom commands via `CommandHandler` trait

## Installation

### From Source

```bash
git clone https://github.com/s0raLin/sinter.git
cd sinter
cargo build --release
# Add target/release/sinter to your PATH
```

### Prerequisites

- **Rust** (latest stable)
- **Scala CLI** (`scala-cli`) — for compilation and execution.
  Install: `curl -fL https://github.com/VirtusLab/scala-cli/releases/latest/download/scala-cli-x86_64-pc-linux.gz | gzip -d > scala-cli && chmod +x scala-cli && mv scala-cli ~/.local/bin/`

## Quick Start

```bash
# Create a new project
sinter new hello-scala
cd hello-scala

# Add a dependency (Scala format — group::artifact:version)
sinter add org.typelevel::cats-core:2.10.0

# Build and run
sinter build
sinter run
```

## Commands

### `sinter new <name>`

Creates a new Scala project with the following structure:

```
hello-scala/
├── project.toml
└── src/main/scala/
    └── Main.scala
```

### `sinter init`

Initializes a workspace by creating a `project.toml` with a `[workspace]` section in the current directory.

### `sinter workspace add <path>`

Adds an existing project to the workspace. `<path>` is relative to the workspace root.

### `sinter build`

Compiles all Scala sources. Automatically resolves transitive dependencies and sets up BSP for IDE integration.

- In a workspace: builds **all** members
- In a workspace sub-directory: builds only the current member
- Standalone project: builds the single project

### `sinter run [file] [--lib]`

Runs a Scala application.

- Without arguments: runs the main file specified in `project.toml` (`main` field in `[package]`)
- `sinter run src/main/scala/App.scala` — runs a specific file
- `sinter run --lib` — compile only (library mode)

### `sinter test [file]`

Runs tests in the project's test directory (`test_dir` in `project.toml`, default `src/test/scala`).

### `sinter add <dep>`

Adds a dependency to `project.toml` and validates it.

**Dependency format:**

| Format | Example | Type |
|--------|---------|------|
| `group::artifact:version` | `org.typelevel::cats-core:2.10.0` | Scala (with `::`) |
| `group:artifact:version` | `com.google.guava:guava:31.1-jre` | Java |
| `sbt:path` | `sbt:../my-sbt-project` | SBT sub-project |

- For Scala dependencies you may specify a Scala version: `group::artifact@2.13:version`
- Dependencies added inside a workspace root are added as **workspace dependencies**

## Configuration — `project.toml`

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

| Field | Default | Description |
|-------|---------|-------------|
| `name` | — | Project name (required) |
| `version` | — | Project version (required) |
| `main` | `Main` | Main class name (without `.scala`) |
| `scala_version` | `2.13` | Scala version (`2.13` or `3.x`) |
| `source_dir` | `src/main/scala` | Source directory |
| `target_dir` | `target` | Output directory |
| `test_dir` | `src/test/scala` | Test directory |
| `backend` | `scala-cli` | Build backend (`scala-cli`/`sbt`/`gradle`/`maven`) |

Workspaces define a `[workspace]` section instead of `[package]`:

```toml
[workspace]
members = ["member-a", "member-b"]

[workspace.dependencies]
"org.typelevel::cats-core" = "2.10.0"
```

## Plugin System

Sinter has a lightweight plugin architecture. Plugin commands are registered at startup via the builder pattern:

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

**Built-in plugin**: JSP project generator (`sinter jsp <name>`)

To create your own plugin, implement the `CommandHandler` trait:

```rust
use async_trait::async_trait;
use clap::{Arg, ArgMatches, Command};
use sinter::CommandHandler;
use std::path::PathBuf;

pub struct MyPlugin;

#[async_trait]
impl CommandHandler for MyPlugin {
    fn name(&self) -> &'static str { "mycommand" }
    fn about(&self) -> &'static str { "Description of my command" }

    fn configure(&self, cmd: Command) -> Command {
        cmd.about(self.about())
            .arg(Arg::new("input").help("Input value").required(true))
    }

    async fn execute(&self, matches: &ArgMatches, cwd: &PathBuf) -> anyhow::Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        println!("Processing: {}", input);
        Ok(())
    }
}
```

## Project Structure

```
crates/
├── sinter-core/      # Core library — models, deps, build, CLI, plugin trait
├── sinter-plugins/   # Official plugins (JSP generator)
└── sinter-cli/       # Binary entry point
```

## Testing

```bash
cargo test     # 53 unit tests across models, deps, and CLI parsing
```

## Troubleshooting

- **Scala CLI not found**: Install it (see Prerequisites). Sinter will attempt auto-download once.
- **`sinter build` hangs**: On first run, Scala CLI may download Coursier metadata (~30s timeout). Wait or press Ctrl+C and try again.
- **Dependency format error**: Use `group::artifact:version` (Scala) or `group:artifact:version` (Java).
- **"latest" version rejected**: Sinter requires explicit version numbers for reproducibility.

## License

MIT — see [LICENSE](LICENSE).
