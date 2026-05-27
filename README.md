# upskill — 3D→2D 点线可视化

Rust + minifb，将三维点透视投影到二维窗口，支持 Y/Z 旋转动画（60 FPS）。

## 构建

```bash
cargo build --release
```

Windows：`target\release\upskill.exe`

## 运行

```bash
./target/release/upskill
./target/release/upskill -file testpoint.in
./target/release/upskill -file testpoint.in 1
./target/release/upskill -file testpoint.in -id 2
./target/release/upskill 1 -id 2    # 以 -id 为准，加载 id=2
./target/release/upskill -help
```

未指定 `-file` 时使用当前目录下的 `testpoint.in`。未指定题目 id 时使用文件中**最后一题**。按 `Esc` 关闭窗口。

## 输入文件格式

```xml
<question id=题目id>
points: { { x: ..., y: ..., z: ... }, ... }
connets: { {0, 1, ...}, ... }
width: 500
height: 500
</question>
```

`id` 可为非数字字符串。`connets` 为索引序列：相邻索引连线；长度 ≥3 时首尾闭合。

## 行为约定

- **CLI**：支持 `-file`、`-id`、`-help` 与 `--file`、`--id`、`--help`、`-f`；**`-id` 优先于位置参数**。
- **坐标**：x、y ∈ [-1, 1]；z 任意。每帧绕 Y、Z 旋转后 **z += 1.5**，再透视 `x' = x/z`，`y' = y/z`。
- **无效点**：`z <= EPSILON`（1e-6）不绘制。
- **无效边**：任一端 `z <= EPSILON` 时**整条边跳过**（无近裁剪）。
- **点可见**：圆盘在 NDC 内 9×9 网格采样，落入 `[-1,1]²` 的比例 ≥ 0.25 才绘制。
- **线可见**：两端 z 有效后，在 NDC 对 `[-1,1]²` 做 **Liang-Barsky** 裁剪再画线。
- **像素**：映射使用 `(width-1)`、`(height-1)`，避免边界越界。

## 测试

```bash
cargo test
```
