//! 每帧 NDC 几何：线段四边形与点四边形。

use crate::math::{
    liang_barsky_clip, point_radius_ndc, point_visible, transform_project, transformed_z, EPSILON,
    NdcPoint,
};
use crate::parser::Question;

/// 顶点（与 Vulkan 顶点属性布局一致）。
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// NDC 位置 x,y。
    pub pos: [f32; 2],
    /// 线 quad 局部坐标；点 quad 为 -1..1。
    pub uv: [f32; 2],
    /// 点 NDC 半径；线为 0。
    pub r_ndc: f32,
}

/// 单帧几何数据。
pub struct FrameGeometry {
    /// 线段顶点。
    pub line_verts: Vec<Vertex>,
    /// 线段索引。
    pub line_indices: Vec<u32>,
    /// 点顶点。
    pub point_verts: Vec<Vertex>,
    /// 点索引。
    pub point_indices: Vec<u32>,
}

impl FrameGeometry {
    /// 空几何。
    pub fn empty() -> Self {
        Self {
            line_verts: Vec::new(),
            line_indices: Vec::new(),
            point_verts: Vec::new(),
            point_indices: Vec::new(),
        }
    }
}

/// 根据题目与旋转角构建本帧几何。
pub fn build_frame_geometry(
    question: &Question,
    angle_y: f64,
    angle_z: f64,
) -> FrameGeometry {
    let width = question.width.max(1);
    let r_ndc = point_radius_ndc(width, question.height.max(1)) as f32;
    let half_line = line_half_width_ndc(width);

    let ndc_pts: Vec<Option<NdcPoint>> = question
        .points
        .iter()
        .map(|&p| transform_project(p, angle_y, angle_z))
        .collect();

    let mut geom = FrameGeometry::empty();

    for conn in &question.connets {
        let n = conn.len();
        if n < 2 {
            continue;
        }
        for i in 0..n - 1 {
            add_edge(
                &mut geom,
                question,
                angle_y,
                angle_z,
                &ndc_pts,
                conn[i],
                conn[i + 1],
                half_line,
            );
        }
        if n >= 3 {
            add_edge(
                &mut geom,
                question,
                angle_y,
                angle_z,
                &ndc_pts,
                conn[n - 1],
                conn[0],
                half_line,
            );
        }
    }

    for ndc in ndc_pts.iter().flatten() {
        if point_visible(*ndc, r_ndc as f64) {
            add_point_quad(&mut geom, ndc.x as f32, ndc.y as f32, r_ndc);
        }
    }

    geom
}

/// 线宽一半（NDC），约 1.5 像素。
fn line_half_width_ndc(width: usize) -> f32 {
    let w = width.max(1) as f32;
    1.5 * 2.0 / (w - 1.0).max(1.0)
}

/// 追加一条边（若有效且可裁剪）。
fn add_edge(
    geom: &mut FrameGeometry,
    question: &Question,
    angle_y: f64,
    angle_z: f64,
    ndc_pts: &[Option<NdcPoint>],
    ia: usize,
    ib: usize,
    half_w: f32,
) {
    let Some(&pa) = question.points.get(ia) else {
        return;
    };
    let Some(&pb) = question.points.get(ib) else {
        return;
    };
    if transformed_z(pa, angle_y, angle_z) <= EPSILON
        || transformed_z(pb, angle_y, angle_z) <= EPSILON
    {
        return;
    }
    let Some(a) = ndc_pts.get(ia).and_then(|o| *o) else {
        return;
    };
    let Some(b) = ndc_pts.get(ib).and_then(|o| *o) else {
        return;
    };
    let Some((x0, y0, x1, y1)) = liang_barsky_clip(a.x, a.y, b.x, b.y) else {
        return;
    };
    expand_line_to_quad(
        geom,
        x0 as f32,
        y0 as f32,
        x1 as f32,
        y1 as f32,
        half_w,
    );
}

/// 将线段扩展为 NDC 四边形（2 三角形）。
fn expand_line_to_quad(
    geom: &mut FrameGeometry,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    half_w: f32,
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-8 {
        return;
    }
    let nx = -dy / len * half_w;
    let ny = dx / len * half_w;

    let base = geom.line_verts.len() as u32;
    geom.line_verts.extend_from_slice(&[
        Vertex {
            pos: [x0 + nx, y0 + ny],
            uv: [0.0, 0.0],
            r_ndc: 0.0,
        },
        Vertex {
            pos: [x0 - nx, y0 - ny],
            uv: [1.0, 0.0],
            r_ndc: 0.0,
        },
        Vertex {
            pos: [x1 - nx, y1 - ny],
            uv: [1.0, 1.0],
            r_ndc: 0.0,
        },
        Vertex {
            pos: [x1 + nx, y1 + ny],
            uv: [0.0, 1.0],
            r_ndc: 0.0,
        },
    ]);
    geom.line_indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// 追加以 NDC 中心、半径 r 的点四边形。
fn add_point_quad(geom: &mut FrameGeometry, cx: f32, cy: f32, r: f32) {
    let expand = r * 1.15;
    let base = geom.point_verts.len() as u32;
    geom.point_verts.extend_from_slice(&[
        Vertex {
            pos: [cx - expand, cy - expand],
            uv: [-1.0, -1.0],
            r_ndc: r,
        },
        Vertex {
            pos: [cx + expand, cy - expand],
            uv: [1.0, -1.0],
            r_ndc: r,
        },
        Vertex {
            pos: [cx + expand, cy + expand],
            uv: [1.0, 1.0],
            r_ndc: r,
        },
        Vertex {
            pos: [cx - expand, cy + expand],
            uv: [-1.0, 1.0],
            r_ndc: r,
        },
    ]);
    geom.point_indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_quad_has_four_vertices() {
        let mut g = FrameGeometry::empty();
        expand_line_to_quad(&mut g, -0.5, 0.0, 0.5, 0.0, 0.01);
        assert_eq!(g.line_verts.len(), 4);
        assert_eq!(g.line_indices.len(), 6);
    }
}
