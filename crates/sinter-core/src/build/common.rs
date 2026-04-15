/// 检测Scala文件是否包含main方法
pub fn has_main_method(content: &str) -> bool {
    content.contains("def main(") || content.contains("extends App")
}

#[derive(Debug, PartialEq)]
pub enum RunMode {
    App, // 有 main 或 extends App
    Lib, // 无入口 -> 只编译
}

pub struct RunResult {
    pub mode: RunMode,
    pub output: String,
}
