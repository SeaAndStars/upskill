struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) r: f32,
}

@fragment
fn main(input: VertexOutput) -> @location(0) vec4<f32> {
    let d = length(input.uv);
    let alpha = 1.0 - smoothstep(0.82, 1.0, d);
    if alpha < 0.004 {
        discard;
    }
    return vec4<f32>(0.909, 0.909, 0.941, alpha);
}
