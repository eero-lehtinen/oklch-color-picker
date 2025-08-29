uniform vec4 prev_color;
uniform vec4 color;

void main() {
	vec4 cb = checkerboard(0.1, 0.1);

	vec4 c = vec4(0);

	if (uv.x < 0.25) {
		c = prev_color;
	} else if (uv.x < 0.5) {
		c = vec4(prev_color.rgb, 1);
	} else if (uv.x < 0.75) {
		c = vec4(color.rgb, 1);
	} else {
		c = color;
	}

	vec4 color = from_srgba(blend(
		cb, 
		to_srgba(c)
	));

    FragColor = output(color);
}
