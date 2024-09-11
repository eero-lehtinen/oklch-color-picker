out vec4 FragColor;
in vec2 uv;

uniform vec4 color;

void main() {
	float checkerboard = xor(floor(mod((uv.x + 0.08) * 100., 2.)), floor(mod((uv.y + 0.11) * 9., 2.)));

	vec4 cb = vec4(vec3(mix(BG, vec3(0.3), checkerboard)), 1.);

    FragColor = blend(cb, color);
}
