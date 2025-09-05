var<private> positions: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1, -1),
    vec2<f32>(1, -1),
    vec2<f32>(-1, 1),
    vec2<f32>(-1, 1),
    vec2<f32>(1, -1),
    vec2<f32>(1, 1),
);

var<private> uvs: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0, 0),
    vec2<f32>(1, 0),
    vec2<f32>(0, 1),
    vec2<f32>(0, 1),
    vec2<f32>(1, 0),
    vec2<f32>(1, 1),
);

@vertex
fn main(@builtin(vertex_index) idx: u32) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(positions[idx], 0.0, 1.0);
    out.uv = uvs[idx];
    return out;
}
