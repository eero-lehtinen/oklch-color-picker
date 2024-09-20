uniform vec3 color;

void main() {
	float alpha = uv.x;

	vec4 cb = checkerboard(0.25, 0.1);

	vec4 color = blend_premultiplied(
		premultiply(cb), 
		premultiply(vec4(color, alpha))
	);

    FragColor = color;
}
