struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) r: f32,
}

@fragment
fn main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.909, 0.909, 0.941, 1.0);
}
