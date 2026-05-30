#pragma once

#include <cstdint>
#include <vector>

#include "parser.hpp"

/// 顶点（与 Vulkan / 着色器布局一致）。
struct Vertex {
    float pos[2];   ///< NDC xy。
    float uv[2];    ///< 线 quad 0..1；点 -1..1。
    float r_ndc;    ///< 点半径；线为 0。
};

/// 单帧几何数据。
struct FrameGeometry {
    std::vector<Vertex> line_verts;       ///< 线段顶点。
    std::vector<uint32_t> line_indices;   ///< 线段索引。
    std::vector<Vertex> point_verts;      ///< 点顶点。
    std::vector<uint32_t> point_indices;  ///< 点索引。
};

/// 根据题目与旋转角构建本帧几何。
FrameGeometry build_frame_geometry(const Question& question, double angle_y, double angle_z);
