//! minifb 窗口、60fps 主循环与绘制。

use std::time::{Duration, Instant};

use minifb::{Key, Window, WindowOptions};

use crate::math::{
    liang_barsky_clip, ndc_radius_to_pixels, ndc_to_pixel_f32, point_radius_ndc, point_visible,
    transform_project, transformed_z, EPSILON, NdcPoint,
};
use crate::parser::Question;
use crate::raster::{draw_circle_aa, draw_line_wu};

/// 帧间隔（60 FPS）。
const FRAME_MS: u64 = 16;

/// 清屏色（ARGB）。
const COLOR_BG: u32 = 0xFF_10_10_18;

/// 点线颜色（ARGB）。
const COLOR_FG: u32 = 0xFF_E8_E8_F0;

/// Y 轴角速度（弧度/秒）。
const OMEGA_Y: f64 = 0.7;

/// Z 轴角速度（弧度/秒）。
const OMEGA_Z: f64 = 0.5;

/// 运行渲染主循环。
pub fn run(question: &Question) -> Result<(), String> {
    let w = question.width.max(1);
    let h = question.height.max(1);
    let mut window = Window::new(
        &format!("upskill - id {}", question.id),
        w,
        h,
        WindowOptions::default(),
    )
    .map_err(|e| format!("无法创建窗口: {e}"))?;

    let mut buffer = vec![COLOR_BG; w * h];
    let r_ndc = point_radius_ndc(w, h);
    let mut angle_y = 0.0f64;
    let mut angle_z = 0.0f64;
    let mut last = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame_start = Instant::now();
        let dt = frame_start.duration_since(last).as_secs_f64();
        last = frame_start;
        angle_y += OMEGA_Y * dt;
        angle_z += OMEGA_Z * dt;

        buffer.fill(COLOR_BG);
        draw_frame(
            &mut buffer,
            w,
            h,
            question,
            angle_y,
            angle_z,
            r_ndc,
        );

        window
            .update_with_buffer(&buffer, w, h)
            .map_err(|e| format!("更新缓冲失败: {e}"))?;

        let elapsed = frame_start.elapsed();
        if elapsed < Duration::from_millis(FRAME_MS) {
            std::thread::sleep(Duration::from_millis(FRAME_MS) - elapsed);
        }
    }
    Ok(())
}

/// 绘制一帧：连线后画点。
fn draw_frame(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    question: &Question,
    angle_y: f64,
    angle_z: f64,
    r_ndc: f64,
) {
    let ndc_pts: Vec<Option<NdcPoint>> = question
        .points
        .iter()
        .map(|&p| transform_project(p, angle_y, angle_z))
        .collect();

    let r_px = ndc_radius_to_pixels(r_ndc, width);

    for conn in &question.connets {
        let n = conn.len();
        if n < 2 {
            continue;
        }
        for i in 0..n - 1 {
            draw_edge(
                buffer,
                width,
                height,
                question,
                angle_y,
                angle_z,
                &ndc_pts,
                conn[i],
                conn[i + 1],
            );
        }
        if n >= 3 {
            draw_edge(
                buffer,
                width,
                height,
                question,
                angle_y,
                angle_z,
                &ndc_pts,
                conn[n - 1],
                conn[0],
            );
        }
    }

    for ndc in ndc_pts.iter().flatten() {
        if point_visible(*ndc, r_ndc) {
            let (px, py) = ndc_to_pixel_f32(ndc.x, ndc.y, width, height);
            draw_circle_aa(buffer, width, height, px, py, r_px, COLOR_FG);
        }
    }
}

/// 绘制一条边（索引 ia、ib）。
fn draw_edge(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    question: &Question,
    angle_y: f64,
    angle_z: f64,
    ndc_pts: &[Option<NdcPoint>],
    ia: usize,
    ib: usize,
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
    let (px0, py0) = ndc_to_pixel_f32(x0, y0, width, height);
    let (px1, py1) = ndc_to_pixel_f32(x1, y1, width, height);
    draw_line_wu(buffer, width, height, px0, py0, px1, py1, COLOR_FG);
}
