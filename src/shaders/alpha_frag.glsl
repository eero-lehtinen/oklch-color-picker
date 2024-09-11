out vec4 FragColor;
in vec2 uv;

uniform vec3 color;

void main() {
	float alpha = uv.x;

	float size = 0.2;
	vec2 pos = floor((uv2 + 0.1) / size);
	float mask = mod(pos.x + mod(pos.y, 2.), 2.);

	vec3 cb = vec3(mix(vec3(0.11), vec3(0.3), mask));

	vec4 color = vec4(color, alpha);

    FragColor = vec4(blend(cb, color), 1.0);
}
