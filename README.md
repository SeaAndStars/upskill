# upskill — 3D→2D 点线可视化

Rust + **Vulkan（winit + ash）**，将三维点透视投影到二维窗口，Y/Z 旋转动画（60 FPS），**MSAA 4×** 与点 **SDF** 抗锯齿。

## 环境

- [Vulkan 运行时](https://vulkan.lunarg.com/)（显卡驱动含 Vulkan 即可；开发可选装 SDK）
- Windows / Linux；macOS 需 **MoltenVK**

## 构建

```bash
cargo build --release
```

## 运行

```bash
./target/release/upskill
./target/release/upskill -file testpoint.in 1
./target/release/upskill -file testpoint.in -id 2
./target/release/upskill 1 -id 2
./target/release/upskill -help
```

未指定 `-file` 时使用 `testpoint.in`；未指定题目 id 时使用文件中**最后一题**。`Esc` 或关闭窗口退出。

## 输入文件

```xml
<question id=题目id>
points: { { x: ..., y: ..., z: ... }, ... }
connets: { {0, 1, ...}, ... }
width: 500
height: 500
</question>
```

## 渲染与行为

- **投影**：绕 Y、Z 旋转 → `z += 1.5` → `x' = x/z`，`y' = y/z`
- **点 1/4 可见**：NDC 圆盘 9×9 采样，≥25% 在 `[-1,1]²` 内才绘制
- **线**：Liang-Barsky 裁剪后扩为 NDC 四边形；**点**：NDC 四边形 + fragment SDF
- **抗锯齿**：MSAA 4× + 点边缘 smoothstep
- **边**：任一端 `z <= 1e-6` 则跳过该边
- **CLI**：`-id` 优先于位置参数；支持 `-file` / `--file` / `-f`

## 测试

```bash
cargo test
```
