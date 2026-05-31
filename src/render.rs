//! 渲染入口：委托 Vulkan 后端。

use crate::gpu;
use crate::parser::Question;

/// 启动可视化主循环。
pub fn run(question: &Question) -> Result<(), String> {
    gpu::run(question)
}
