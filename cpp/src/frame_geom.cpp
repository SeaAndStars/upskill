#include "frame_geom.hpp"

#include <cmath>

#include "math.hpp"

namespace {

float line_half_width_ndc(std::size_t width) {
    float w = static_cast<float>(width > 0 ? width : 1);
    return 1.5f * 2.0f / std::max(w - 1.0f, 1.0f);
}

void expand_line_to_quad(FrameGeometry& geom, float x0, float y0, float x1, float y1, float half_w) {
    float dx = x1 - x0;
    float dy = y1 - y0;
    float len = std::sqrt(dx * dx + dy * dy);
    if (len < 1e-8f) {
        return;
    }
    float nx = -dy / len * half_w;
    float ny = dx / len * half_w;
    uint32_t base = static_cast<uint32_t>(geom.line_verts.size());
    geom.line_verts.push_back(Vertex{{x0 + nx, y0 + ny}, {0, 0}, 0});
    geom.line_verts.push_back(Vertex{{x0 - nx, y0 - ny}, {1, 0}, 0});
    geom.line_verts.push_back(Vertex{{x1 - nx, y1 - ny}, {1, 1}, 0});
    geom.line_verts.push_back(Vertex{{x1 + nx, y1 + ny}, {0, 1}, 0});
    geom.line_indices.insert(geom.line_indices.end(),
                            {base, base + 1, base + 2, base, base + 2, base + 3});
}

void add_point_quad(FrameGeometry& geom, float cx, float cy, float r) {
    float expand = r * 1.15f;
    uint32_t base = static_cast<uint32_t>(geom.point_verts.size());
    geom.point_verts.push_back(Vertex{{cx - expand, cy - expand}, {-1, -1}, r});
    geom.point_verts.push_back(Vertex{{cx + expand, cy - expand}, {1, -1}, r});
    geom.point_verts.push_back(Vertex{{cx + expand, cy + expand}, {1, 1}, r});
    geom.point_verts.push_back(Vertex{{cx - expand, cy + expand}, {-1, 1}, r});
    geom.point_indices.insert(geom.point_indices.end(),
                              {base, base + 1, base + 2, base, base + 2, base + 3});
}

void add_edge(FrameGeometry& geom, const Question& question, double angle_y, double angle_z,
              const std::vector<std::optional<NdcPoint>>& ndc_pts, std::size_t ia, std::size_t ib,
              float half_w) {
    if (ia >= question.points.size() || ib >= question.points.size()) {
        return;
    }
    const Point3& pa = question.points[ia];
    const Point3& pb = question.points[ib];
    if (transformed_z(pa, angle_y, angle_z) <= EPSILON ||
        transformed_z(pb, angle_y, angle_z) <= EPSILON) {
        return;
    }
    if (ia >= ndc_pts.size() || ib >= ndc_pts.size() || !ndc_pts[ia] || !ndc_pts[ib]) {
        return;
    }
    auto clipped = liang_barsky_clip(ndc_pts[ia]->x, ndc_pts[ia]->y, ndc_pts[ib]->x, ndc_pts[ib]->y);
    if (!clipped) {
        return;
    }
    auto [x0, y0, x1, y1] = *clipped;
    expand_line_to_quad(geom, static_cast<float>(x0), static_cast<float>(y0),
                        static_cast<float>(x1), static_cast<float>(y1), half_w);
}

}  // namespace

FrameGeometry build_frame_geometry(const Question& question, double angle_y, double angle_z) {
    FrameGeometry geom;
    std::size_t width = question.width > 0 ? question.width : 1;
    float r_ndc = static_cast<float>(point_radius_ndc(width, question.height));
    float half_line = line_half_width_ndc(width);

    std::vector<std::optional<NdcPoint>> ndc_pts;
    ndc_pts.reserve(question.points.size());
    for (const auto& p : question.points) {
        ndc_pts.push_back(transform_project(p, angle_y, angle_z));
    }

    for (const auto& conn : question.connets) {
        if (conn.size() < 2) {
            continue;
        }
        for (std::size_t i = 0; i + 1 < conn.size(); ++i) {
            add_edge(geom, question, angle_y, angle_z, ndc_pts, conn[i], conn[i + 1], half_line);
        }
        if (conn.size() >= 3) {
            add_edge(geom, question, angle_y, angle_z, ndc_pts, conn.back(), conn.front(),
                     half_line);
        }
    }

    for (const auto& ndc : ndc_pts) {
        if (ndc && point_visible(*ndc, r_ndc)) {
            add_point_quad(geom, static_cast<float>(ndc->x), static_cast<float>(ndc->y), r_ndc);
        }
    }
    return geom;
}
