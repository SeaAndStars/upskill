//! CPU 抗锯齿光栅化：alpha 混合、Xiaolin Wu 线段、距离场圆。

/// 圆边抗锯齿过渡宽度（像素）。
const CIRCLE_AA_WIDTH: f32 = 1.25;

/// 从 ARGB 取 R 分量。
fn color_r(c: u32) -> f32 {
    ((c >> 16) & 0xFF) as f32
}

/// 从 ARGB 取 G 分量。
fn color_g(c: u32) -> f32 {
    ((c >> 8) & 0xFF) as f32
}

/// 从 ARGB 取 B 分量。
fn color_b(c: u32) -> f32 {
    (c & 0xFF) as f32
}

/// 合成 ARGB 颜色。
fn pack_rgb(r: f32, g: f32, b: f32) -> u32 {
    let r = r.round().clamp(0.0, 255.0) as u32;
    let g = g.round().clamp(0.0, 255.0) as u32;
    let b = b.round().clamp(0.0, 255.0) as u32;
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// 将前景色以 alpha 混合写入缓冲（alpha 已含线覆盖权重时可再乘）。
pub fn blend_pixel(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    fg: u32,
    alpha: f32,
) {
    if x < 0 || y < 0 {
        return;
    }
    let x = x as usize;
    let y = y as usize;
    if x >= width || y >= height {
        return;
    }
    let a = alpha.clamp(0.0, 1.0);
    if a <= 0.0 {
        return;
    }
    let idx = y * width + x;
    let bg = buffer[idx];
    let r = color_r(bg) * (1.0 - a) + color_r(fg) * a;
    let g = color_g(bg) * (1.0 - a) + color_g(fg) * a;
    let b = color_b(bg) * (1.0 - a) + color_b(fg) * a;
    buffer[idx] = pack_rgb(r, g, b);
}

/// 在 (px, py) 写入 Wu 权重。
fn plot_wu(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    px: f32,
    py: f32,
    color: u32,
    base_alpha: f32,
) {
    let x = px.floor() as i32;
    let y = py.floor() as i32;
    let fx = px - x as f32;
    let fy = py - y as f32;
    blend_pixel(buffer, width, height, x, y, color, base_alpha * (1.0 - fx) * (1.0 - fy));
    blend_pixel(buffer, width, height, x + 1, y, color, base_alpha * fx * (1.0 - fy));
    blend_pixel(buffer, width, height, x, y + 1, color, base_alpha * (1.0 - fx) * fy);
    blend_pixel(
        buffer,
        width,
        height,
        x + 1,
        y + 1,
        color,
        base_alpha * fx * fy,
    );
}

/// Xiaolin Wu 抗锯齿线段（f32 像素坐标）。
pub fn draw_line_wu(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: u32,
) {
    let mut x0 = x0;
    let mut y0 = y0;
    let mut x1 = x1;
    let mut y1 = y1;
    let steep = (y1 - y0).abs() > (x1 - x0).abs();
    if steep {
        std::mem::swap(&mut x0, &mut y0);
        std::mem::swap(&mut x1, &mut y1);
    }
    if x0 > x1 {
        std::mem::swap(&mut x0, &mut x1);
        std::mem::swap(&mut y0, &mut y1);
    }
    let dx = x1 - x0;
    let dy = y1 - y0;
    let gradient = if dx.abs() < 1e-6 { 1.0 } else { dy / dx };

    let xend = x0.round();
    let yend = y0 + gradient * (xend - x0);
    let xgap = 1.0 - (x0 + 0.5).fract();
    if steep {
        plot_wu(buffer, width, height, yend, xend, color, xgap);
        plot_wu(buffer, width, height, yend + gradient, xend, color, 1.0 - xgap);
    } else {
        plot_wu(buffer, width, height, xend, yend, color, xgap);
        plot_wu(buffer, width, height, xend, yend + gradient, color, 1.0 - xgap);
    }
    let mut x = xend;
    let mut y = yend;

    let xend = x1.round();
    let yend = y1 + gradient * (xend - x1);
    let xgap = (x1 + 0.5).fract();

    while x + 1.0 < xend {
        x += 1.0;
        y += gradient;
        if steep {
            plot_wu(buffer, width, height, y, x, color, 1.0);
            plot_wu(buffer, width, height, y + gradient, x, color, 1.0);
        } else {
            plot_wu(buffer, width, height, x, y, color, 1.0);
            plot_wu(buffer, width, height, x, y + gradient, color, 1.0);
        }
    }
    if steep {
        plot_wu(buffer, width, height, yend, xend, color, xgap);
        plot_wu(buffer, width, height, yend + gradient, xend, color, 1.0 - xgap);
    } else {
        plot_wu(buffer, width, height, xend, yend, color, xgap);
        plot_wu(buffer, width, height, xend, yend + gradient, color, 1.0 - xgap);
    }
}

/// smoothstep(edge0, edge1, x)：edge0 外为 0，edge1 内为 1。
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if edge0 >= edge1 {
        return if x <= edge1 { 1.0 } else { 0.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 距离场抗锯齿实心圆。
pub fn draw_circle_aa(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cx: f32,
    cy: f32,
    r_px: f32,
    color: u32,
) {
    if r_px <= 0.0 {
        return;
    }
    let inner = r_px - CIRCLE_AA_WIDTH;
    let r_max = r_px + CIRCLE_AA_WIDTH;
    let xmin = (cx - r_max).floor() as i32;
    let xmax = (cx + r_max).ceil() as i32;
    let ymin = (cy - r_max).floor() as i32;
    let ymax = (cy + r_max).ceil() as i32;
    for y in ymin..=ymax {
        for x in xmin..=xmax {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let alpha = smoothstep(r_px, inner.max(0.0), dist);
            if alpha > 0.0 {
                blend_pixel(buffer, width, height, x, y, color, alpha);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_overwrites_with_full_alpha() {
        let mut buf = vec![0xFF_00_00_00u32; 1];
        blend_pixel(&mut buf, 1, 1, 0, 0, 0xFF_FF_FF_FF, 1.0);
        assert_eq!(buf[0], 0xFF_FF_FF_FF);
    }
}
