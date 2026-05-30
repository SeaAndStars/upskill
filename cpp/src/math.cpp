#include "math.hpp"

#include <cmath>
#include <tuple>

Point3 rotate_y(Point3 p, double angle) {
    double c = std::cos(angle);
    double s = std::sin(angle);
    return {p.x * c - p.z * s, p.y, p.x * s + p.z * c};
}

Point3 rotate_z(Point3 p, double angle) {
    double c = std::cos(angle);
    double s = std::sin(angle);
    return {p.x * c - p.y * s, p.x * s + p.y * c, p.z};
}

std::optional<NdcPoint> transform_project(Point3 p, double angle_y, double angle_z) {
    p = rotate_y(p, angle_y);
    p = rotate_z(p, angle_z);
    double z = p.z + 1.5;
    if (z <= EPSILON) {
        return std::nullopt;
    }
    return NdcPoint{p.x / z, p.y / z};
}

double transformed_z(Point3 p, double angle_y, double angle_z) {
    p = rotate_y(p, angle_y);
    p = rotate_z(p, angle_z);
    return p.z + 1.5;
}

bool point_visible(NdcPoint ndc, double r_ndc) {
    constexpr int GRID = 9;
    int inside = 0;
    int total = 0;
    int steps = GRID - 1;
    for (int iy = 0; iy < GRID; ++iy) {
        for (int ix = 0; ix < GRID; ++ix) {
            double fu = static_cast<double>(ix) / steps;
            double fv = static_cast<double>(iy) / steps;
            double sx = ndc.x + (fu * 2.0 - 1.0) * r_ndc;
            double sy = ndc.y + (fv * 2.0 - 1.0) * r_ndc;
            double dx = sx - ndc.x;
            double dy = sy - ndc.y;
            if (dx * dx + dy * dy > r_ndc * r_ndc + 1e-12) {
                continue;
            }
            ++total;
            if (sx >= -1.0 && sx <= 1.0 && sy >= -1.0 && sy <= 1.0) {
                ++inside;
            }
        }
    }
    if (total == 0) {
        return false;
    }
    return static_cast<double>(inside) / total >= 0.25;
}

std::optional<std::tuple<double, double, double, double>> liang_barsky_clip(double x0, double y0,
                                                                            double x1, double y1) {
    double dx = x1 - x0;
    double dy = y1 - y0;
    double t0 = 0.0;
    double t1 = 1.0;
    const double edges[4][2] = {{-dx, x0 - (-1.0)}, {dx, 1.0 - x0}, {-dy, y0 - (-1.0)}, {dy, 1.0 - y0}};
    for (const auto& e : edges) {
        double p = e[0];
        double q = e[1];
        if (std::abs(p) < 1e-15) {
            if (q < 0.0) {
                return std::nullopt;
            }
        } else {
            double t = q / p;
            if (p < 0.0) {
                if (t > t1) {
                    return std::nullopt;
                }
                if (t > t0) {
                    t0 = t;
                }
            } else {
                if (t < t0) {
                    return std::nullopt;
                }
                if (t < t1) {
                    t1 = t;
                }
            }
        }
    }
    if (t0 > t1) {
        return std::nullopt;
    }
    return std::make_tuple(x0 + t0 * dx, y0 + t0 * dy, x0 + t1 * dx, y0 + t1 * dy);
}

double point_radius_ndc(std::size_t width, std::size_t) {
    double w = static_cast<double>(width > 0 ? width : 1);
    return std::max(3.0 / w, 0.015);
}
