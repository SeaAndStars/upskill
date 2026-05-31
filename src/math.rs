//! 三维变换、透视投影、点可见性采样与 Liang-Barsky 裁剪。

use crate::parser::Point3;

/// z 近零阈值，小于等于视为无效。
pub const EPSILON: f64 = 1e-6;

/// 透视投影后的 NDC 二维点。
#[derive(Debug, Clone, Copy)]
pub struct NdcPoint {
    /// 归一化设备坐标 x。
    pub x: f64,
    /// 归一化设备坐标 y。
    pub y: f64,
}

/// 绕 Y 轴旋转（作用于 x、z）。
pub fn rotate_y(p: Point3, angle: f64) -> Point3 {
    let c = angle.cos();
    let s = angle.sin();
    Point3 {
        x: p.x * c - p.z * s,
        y: p.y,
        z: p.x * s + p.z * c,
    }
}

/// 绕 Z 轴旋转（作用于 x、y）。
pub fn rotate_z(p: Point3, angle: f64) -> Point3 {
    let c = angle.cos();
    let s = angle.sin();
    Point3 {
        x: p.x * c - p.y * s,
        y: p.x * s + p.y * c,
        z: p.z,
    }
}

/// 对点施加 Y/Z 旋转、z 平移 1.5 后透视投影。
pub fn transform_project(p: Point3, angle_y: f64, angle_z: f64) -> Option<NdcPoint> {
    let p = rotate_y(p, angle_y);
    let p = rotate_z(p, angle_z);
    let z = p.z + 1.5;
    if z <= EPSILON {
        return None;
    }
    Some(NdcPoint {
        x: p.x / z,
        y: p.y / z,
    })
}

/// 从三维点得到投影后的 z（用于边有效性判断）。
pub fn transformed_z(p: Point3, angle_y: f64, angle_z: f64) -> f64 {
    let p = rotate_y(p, angle_y);
    let p = rotate_z(p, angle_z);
    p.z + 1.5
}

/// NDC 坐标映射到像素坐标（使用 width-1 / height-1 防越界，取整）。
pub fn ndc_to_pixel(xn: f64, yn: f64, width: usize, height: usize) -> (i32, i32) {
    let (px, py) = ndc_to_pixel_f32(xn, yn, width, height);
    (
        px.round().clamp(0.0, width as f32 - 1.0) as i32,
        py.round().clamp(0.0, height as f32 - 1.0) as i32,
    )
}

/// NDC 映射到 f32 像素坐标（不取整，供抗锯齿光栅化使用）。
pub fn ndc_to_pixel_f32(xn: f64, yn: f64, width: usize, height: usize) -> (f32, f32) {
    let w = width.max(1) as f64;
    let h = height.max(1) as f64;
    let px = ((xn + 1.0) * 0.5 * (w - 1.0)) as f32;
    let py = ((1.0 - yn) * 0.5 * (h - 1.0)) as f32;
    (
        px.clamp(0.0, (w - 1.0) as f32),
        py.clamp(0.0, (h - 1.0) as f32),
    )
}

/// 将 NDC 半径换算为像素半径（与 ndc_to_pixel_f32 尺度一致）。
pub fn ndc_radius_to_pixels(r_ndc: f64, width: usize) -> f32 {
    let w = width.max(1) as f64;
    (r_ndc * 0.5 * (w - 1.0)).max(1.0) as f32
}

/// 点在 NDC 圆盘内固定网格采样，落入 [-1,1]² 比例 >= 0.25 则可见。
pub fn point_visible(ndc: NdcPoint, r_ndc: f64) -> bool {
    const GRID: i32 = 9;
    let mut inside = 0i32;
    let mut total = 0i32;
    let steps = GRID - 1;
    for iy in 0..GRID {
        for ix in 0..GRID {
            let fu = ix as f64 / steps as f64;
            let fv = iy as f64 / steps as f64;
            let sx = ndc.x + (fu * 2.0 - 1.0) * r_ndc;
            let sy = ndc.y + (fv * 2.0 - 1.0) * r_ndc;
            let dx = sx - ndc.x;
            let dy = sy - ndc.y;
            if dx * dx + dy * dy > r_ndc * r_ndc + 1e-12 {
                continue;
            }
            total += 1;
            if sx >= -1.0 && sx <= 1.0 && sy >= -1.0 && sy <= 1.0 {
                inside += 1;
            }
        }
    }
    if total == 0 {
        return false;
    }
    (inside as f64 / total as f64) >= 0.25
}

/// Liang-Barsky：将 NDC 线段裁剪到 [-1,1]²，成功返回裁剪后端点。
pub fn liang_barsky_clip(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> Option<(f64, f64, f64, f64)> {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let mut t0 = 0.0f64;
    let mut t1 = 1.0f64;

    let edges = [
        (-dx, x0 - (-1.0)),
        (dx, 1.0 - x0),
        (-dy, y0 - (-1.0)),
        (dy, 1.0 - y0),
    ];

    for (p, q) in edges {
        if p.abs() < 1e-15 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                if t > t1 {
                    return None;
                }
                if t > t0 {
                    t0 = t;
                }
            } else if t < t0 {
                return None;
            } else if t < t1 {
                t1 = t;
            }
        }
    }

    if t0 > t1 {
        return None;
    }

    Some((
        x0 + t0 * dx,
        y0 + t0 * dy,
        x0 + t1 * dx,
        y0 + t1 * dy,
    ))
}

/// 根据窗口尺寸估算 NDC 点圆半径。
pub fn point_radius_ndc(width: usize, _height: usize) -> f64 {
    let w = width.max(1) as f64;
    (3.0 / w).max(0.015)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liang_barsky_crosses_viewport() {
        let clipped = liang_barsky_clip(-2.0, 0.0, 2.0, 0.0).unwrap();
        assert!(clipped.0 >= -1.0 - 1e-9 && clipped.2 <= 1.0 + 1e-9);
    }

    #[test]
    fn ndc_to_pixel_matches_f32_round() {
        let (fx, fy) = ndc_to_pixel_f32(0.0, 0.0, 500, 500);
        let (ix, iy) = ndc_to_pixel(0.0, 0.0, 500, 500);
        assert_eq!(ix, fx.round() as i32);
        assert_eq!(iy, fy.round() as i32);
    }

    #[test]
    fn ndc_radius_to_pixels_positive() {
        assert!(ndc_radius_to_pixels(0.02, 500) >= 1.0);
    }
}
