out vec4 FragColor;
in vec2 uv;

uniform vec3 color;

void main() {
	float alpha = uv.x;

	float size = 0.2;
	vec2 pos = floor((uv2 + 0.1) / size);
	float mask = mod(pos.x + mod(pos.y, 2.), 2.);

	vec3 cb = vec3(mix(BG, vec3(0.68), mask));

	vec4 color = blend(cb, vec4(color, alpha));

	color.rgb += screen_space_dither(gl_FragCoord.xy);

    FragColor = color;
}
