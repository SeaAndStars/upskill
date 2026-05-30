#version 450

layout(location = 0) in vec2 in_pos;
layout(location = 1) in vec2 in_uv;
layout(location = 2) in float in_r;

layout(location = 0) out vec2 v_uv;
layout(location = 1) out float v_r;

void main() {
    gl_Position = vec4(in_pos, 0.0, 1.0);
    v_uv = in_uv;
    v_r = in_r;
}
