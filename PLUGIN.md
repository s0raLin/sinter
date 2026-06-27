# Sinter 插件系统

## 概述

Sinter 采用极致简化的插件架构，允许开发者通过实现简单的 trait 来添加新命令，整个过程只需几行代码，无需修改核心代码。

## 核心设计理念

- **函数式Builder模式**：使用链式调用注册插件，如 `Sinter::new().plugin(A).plugin(B).run()`
- **零配置注册**：插件自动注册，无需手动配置
- **类型安全**：编译时检查插件接口
- **运行时灵活**：支持条件加载和动态扩展
- **极致简单**：添加新命令只需实现 trait

## 架构组件

### CommandHandler Trait

所有插件命令都实现 `CommandHandler` trait：

```rust
#[async_trait]
pub trait CommandHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn about(&self) -> &'static str;
    fn configure(&self, cmd: Command) -> Command;
    async fn execute(&self, matches: &ArgMatches, cwd: &PathBuf) -> anyhow::Result<()>;
}
```

### Sinter Builder

使用函数式Builder模式构建应用，支持链式插件注册：

```rust
pub struct Sinter {
    plugins: Vec<Box<dyn CommandHandler>>,
}

impl Sinter {
    pub fn new() -> Self;
    pub fn plugin<H: CommandHandler + 'static>(self, handler: H) -> Self;
    pub fn plugins<H: CommandHandler + 'static, I: IntoIterator<Item = H>>(self, handlers: I) -> Self;
    pub async fn run(self) -> anyhow::Result<()>;
}
```

## 添加新插件的步骤

### 方式一：使用超级简单的宏（推荐）

最简单的方式，无需了解内部细节：

```rust
// src/cmd/my_plugin.rs
use crate::simple_plugin;

simple_plugin!(
    "mycommand",           // 命令名
    "My awesome command",  // 命令描述
    |cmd: clap::Command| cmd.arg(  // 参数配置
        clap::Arg::new("input")
            .help("Input file")
            .required(true)
    ),
    |matches: clap::ArgMatches, cwd: std::path::PathBuf| async move {  // 执行逻辑
        let input = matches.get_one::<String>("input").unwrap();
        println!("Processing: {}", input);
        Ok(())
    }
);
```

然后在 `main.rs` 中注册：

```rust
use sinter::{Sinter, cmd::plugin_mycommand};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Sinter::new()
        .plugin(plugin_mycommand)
        .run()
        .await
}
```

### 方式二：传统方式（了解内部细节）

如果你需要更多控制，可以使用传统方式：

```rust
pub struct MyPlugin;

#[async_trait]
impl CommandHandler for MyPlugin {
    fn name(&self) -> &'static str {
        "mycommand"
    }

    fn about(&self) -> &'static str {
        "My awesome command"
    }

    fn configure(&self, cmd: Command) -> Command {
        cmd.arg(
            Arg::new("input")
                .help("Input file")
                .required(true)
        )
    }

    async fn execute(&self, matches: &ArgMatches, cwd: &PathBuf) -> anyhow::Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        println!("Processing: {}", input);
        Ok(())
    }
}
```

## 完整示例

### 使用简单宏的完整示例

```rust
// src/cmd/hello_plugin.rs
use crate::simple_plugin;

simple_plugin!(
    "hello",
    "Say hello to someone",
    |cmd: clap::Command| cmd.arg(
        clap::Arg::new("name")
            .help("Name to greet")
            .required(true)
    ),
    |matches: clap::ArgMatches, _cwd: std::path::PathBuf| async move {
        let name = matches.get_one::<String>("name").unwrap();
        println!("Hello, {}!", name);
        Ok(())
    }
);
```

```rust
// src/cmd/mod.rs
pub mod hello_plugin;
pub use hello_plugin::plugin_hello;
```

```rust
// src/main.rs - 函数式Builder模式
use sinter::{Sinter, cmd::plugin_hello};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 链式注册插件，极致简洁
    Sinter::new()
        .plugin(plugin_hello)
        .run()
        .await
}
```

### 传统方式的完整示例

```rust
// src/cmd/hello_plugin.rs
use crate::CommandHandler;
use async_trait::async_trait;
use clap::{Arg, ArgMatches, Command};
use std::path::PathBuf;

pub struct HelloPlugin;

#[async_trait]
impl CommandHandler for HelloPlugin {
    fn name(&self) -> &'static str {
        "hello"
    }

    fn about(&self) -> &'static str {
        "Say hello to someone"
    }

    fn configure(&self, cmd: Command) -> Command {
        cmd.arg(
            Arg::new("name")
                .help("Name to greet")
                .required(true)
        )
    }

    async fn execute(&self, matches: &ArgMatches, _cwd: &PathBuf) -> anyhow::Result<()> {
        let name = matches.get_one::<String>("name").unwrap();
        println!("Hello, {}!", name);
        Ok(())
    }
}
```

```rust
// src/cmd/mod.rs
pub mod hello_plugin;
pub use hello_plugin::HelloPlugin;
```

```rust
// src/main.rs - 函数式Builder模式
use sinter::{Sinter, cmd::HelloPlugin};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 链式注册插件，极致简洁
    Sinter::new()
        .plugin(HelloPlugin)
        .run()
        .await
}
```

## 函数式Builder模式使用

### 基本用法

```rust
use sinter::{Sinter, cmd::*};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Sinter::new()
        .plugin(JspPlugin)
        .run()
        .await
}
```

### 高级用法

```rust
// 批量注册插件
let plugins = vec![JspPlugin, HelloPlugin, BuildPlugin];
Sinter::new()
    .plugins(plugins)
    .run()
    .await?;

// 条件注册
Sinter::new()
    .plugin(BasicPlugin)
    .plugin(AdvancedPlugin)
    .run()
    .await?;
```

### 链式调用的美学

```rust
// 像搭积木一样构建应用
Sinter::new()
    .plugin(WebPlugin)      // Web开发插件
    .plugin(DatabasePlugin) // 数据库插件
    .plugin(TestPlugin)     // 测试插件
    .plugin(DocsPlugin)     // 文档插件
    .run()
    .await?;
```

## 使用插件

```bash
# 插件会自动出现在帮助中
sinter --help

# 使用插件命令
sinter jsp myapp
sinter hello world
# 输出: Hello, world!
```

## 插件生命周期

1. **编译时注册**：插件在编译时通过 `register_plugin!` 宏注册到全局注册表
2. **运行时初始化**：在 `main` 函数中调用插件初始化函数
3. **命令执行**：CLI 解析时自动发现插件命令，执行时调用对应处理器

## 设计优势

### 1. 极致简单
- 添加新命令只需实现 trait
- 无需修改核心代码
- 自动命令发现和注册

### 2. 类型安全
- 编译时检查所有接口
- 避免运行时类型错误
- 智能提示和重构支持

### 3. 高性能
- 零成本抽象
- 静态分发
- 最小运行时开销

### 4. 可扩展性
- 支持任意数量的插件
- 运行时动态加载
- 条件编译支持

### 5. 易维护
- 插件完全独立
- 移除插件只需删除文件
- 无残留代码

## 最佳实践

### 插件命名
- 使用小写字母和下划线
- 避免与内置命令冲突
- 保持简洁明了

### 错误处理
- 使用 `anyhow::Result` 进行错误处理
- 提供有意义的错误信息
- 正确处理异步操作

### 参数设计
- 使用 clap 的类型安全参数
- 提供清晰的帮助信息
- 支持可选参数

### 测试
- 为每个插件编写单元测试
- 测试参数解析和执行逻辑
- 验证错误处理

## 高级用法

### 条件插件
```rust
#[cfg(feature = "advanced")]
pub fn init_advanced_plugin() {
    crate::register_plugin(AdvancedPlugin);
}
```

### 插件组
```rust
pub fn init_all_plugins() {
    init_basic_plugin();
    init_advanced_plugin();
    init_experimental_plugin();
}
```

### 动态配置
```rust
impl CommandHandler for ConfigurablePlugin {
    fn configure(&self, cmd: Command) -> Command {
        // 从配置文件读取参数定义
        let config = load_config();
        cmd.args(&config.args)
    }
}
```

## 迁移指南

### 从旧架构迁移

旧的命令系统需要手动修改多个文件：

```rust
// 旧方式 - 需要修改4个文件
// 1. cli.rs - 添加枚举变体和解析
// 2. lib.rs - 添加match分支
// 3. cmd/mod.rs - 添加模块
// 4. main.rs - 可能需要注册
```

新的插件系统只需：

```rust
// 新方式 - 只需一个文件
pub struct MyCommand;
impl CommandHandler for MyCommand { /* ... */ }
crate::register_plugin(MyCommand);
```

## 故障排除

### 常见问题

1. **插件未显示在帮助中**
   - 检查是否调用了初始化函数
   - 验证插件名称不与内置命令冲突

2. **编译错误**
   - 确保实现了所有 trait 方法
   - 检查异步函数签名

3. **运行时错误**
   - 验证参数解析逻辑
   - 检查文件路径处理

### 调试技巧

- 使用 `cargo build --verbose` 查看编译详情
- 添加日志输出调试插件执行
- 使用 `println!` 验证初始化顺序

## 未来扩展

- **插件市场**：支持从远程仓库加载插件
- **热重载**：运行时重新加载插件
- **依赖管理**：插件间的依赖关系
- **配置界面**：图形化插件配置

---

这个插件系统将复杂性降到最低，同时保持了最大的灵活性和扩展性。开发者可以专注于业务逻辑，而不必关心框架细节。