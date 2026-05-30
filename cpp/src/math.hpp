#pragma once

#include <optional>

#include "parser.hpp"

/// z 近零阈值。
constexpr double EPSILON = 1e-6;

/// 透视投影后的 NDC 二维点。
struct NdcPoint {
    double x = 0;  ///< 归一化设备坐标 x。
    double y = 0;  ///< 归一化设备坐标 y。
};

/// 绕 Y 轴旋转（作用于 x、z）。
Point3 rotate_y(Point3 p, double angle);

/// 绕 Z 轴旋转（作用于 x、y）。
Point3 rotate_z(Point3 p, double angle);

/// 旋转、z+1.5 后透视投影。
std::optional<NdcPoint> transform_project(Point3 p, double angle_y, double angle_z);

/// 变换后的 z（用于边有效性）。
double transformed_z(Point3 p, double angle_y, double angle_z);

/// 点可见性：9×9 采样，≥25% 在 [-1,1]²。
bool point_visible(NdcPoint ndc, double r_ndc);

/// Liang-Barsky 裁剪到 [-1,1]²。
std::optional<std::tuple<double, double, double, double>> liang_barsky_clip(double x0, double y0,
                                                                            double x1, double y1);

/// NDC 点圆半径。
double point_radius_ndc(std::size_t width, std::size_t height);
