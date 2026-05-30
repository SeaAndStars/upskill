# upskill_cpp — 3D→2D 点线可视化（Vulkan）

C++17 + GLFW + Vulkan，将三维点透视投影到二维窗口，Y/Z 旋转动画 60 FPS。

## 环境

- [Vulkan SDK](https://vulkan.lunarg.com/)（含 `glslc`）
- CMake 3.20+
- C++17 编译器（MSVC / GCC / Clang）

## 构建

```bash
cd cpp
cmake -B build
cmake --build build --config Release
```

Windows：`build\Release\upskill_cpp.exe`

## 运行

```bash
./build/upskill_cpp
./build/upskill_cpp -file testpoint.in
./build/upskill_cpp -file testpoint.in 1
./build/upskill_cpp -file testpoint.in -id 2
./build/upskill_cpp -help
```

未指定 `-file` 时使用当前目录 `testpoint.in`。未指定题目 id 时使用文件中**最后一题**。`Esc` 关闭窗口。

## 输入格式

```xml
<question id=题目id>
points: { { x: ..., y: ..., z: ... }, ... }
connets: { {0, 1, ...}, ... }
width: 500
height: 500
</question>
```

`connets` 与 `connects` 等价。`id` 可为非数字字符串。

## 行为

- 绕 Y、Z 旋转后 `z += 1.5`，透视 `x' = x/z`，`y' = y/z`
- 点：NDC 圆盘 9×9 采样，≥25% 在 `[-1,1]²` 可见
- 边：Liang-Barsky 裁剪；MSAA 4x + 点 SDF 抗锯齿
- 着色器复用仓库根目录 `shaders/`
