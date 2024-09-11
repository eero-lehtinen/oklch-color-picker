#version 450

// varying lowp vec2 uv;


// uniform float width;
// varying out vec2 uv2;
//
out vec2 uv;
out vec2 uv2;

const vec2 verts[4] = vec2[4](
    vec2(-1., -1.),
    vec2(1., -1.),
    vec2(-1., 1.),
    vec2(1., 1.)
);


void main() {
    gl_Position = vec4(verts[gl_VertexID], 0., 1.);
    uv = verts[gl_VertexID] * 0.5 + 0.5;
    // uv = texcoord;
    //
    // vec2 size = width >= 1. ? vec2(width, 1.) : vec2(1., 1. / width);
    //
    // uv2 = (texcoord - vec2(0.5)) * 2. * size;
}
