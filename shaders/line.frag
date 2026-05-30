#version 450

layout(location = 0) in vec2 v_uv;
layout(location = 1) in float v_r;

layout(location = 0) out vec4 out_color;

void main() {
    out_color = vec4(0.909, 0.909, 0.941, 1.0);
}
