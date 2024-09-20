uniform vec4 prev_color;
uniform vec4 color;

void main() {
	vec4 cb = checkerboard(0.1, 0.1);

	vec4 color = uv.x < 0.5 ? prev_color : color;

	color = blend_premultiplied(
		premultiply(cb), 
		premultiply(color)
	);

    FragColor = color;
}
