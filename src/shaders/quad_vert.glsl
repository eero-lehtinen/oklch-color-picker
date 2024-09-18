#version 330

uniform vec2 size;

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
	uv2 = uv * size;
}
