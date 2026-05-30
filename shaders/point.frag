#version 450

layout(location = 0) in vec2 v_uv;
layout(location = 1) in float v_r;

layout(location = 0) out vec4 out_color;

void main() {
    float d = length(v_uv);
    float alpha = 1.0 - smoothstep(0.82, 1.0, d);
    if (alpha < 0.004) {
        discard;
    }
    out_color = vec4(0.909, 0.909, 0.941, alpha);
}
