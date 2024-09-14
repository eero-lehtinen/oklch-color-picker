out vec4 FragColor;
in vec2 uv;

uniform vec4 color;

void main() {
	float size = 0.1;
	vec2 pos = floor((uv2 + 0.05) / size);
	float mask = mod(pos.x + mod(pos.y, 2.), 2.);

	vec3 cb = vec3(mix(BG, vec3(0.68), mask));

    FragColor = vec4(blend(cb, color), 1.0);
}
