@fragment
fn main(in: VertexOut) -> @location(0) vec4<f32> {
    let x = in.uv.x * in.uv.y;
    return vec4f(vec3f(x), 1.0);
}
