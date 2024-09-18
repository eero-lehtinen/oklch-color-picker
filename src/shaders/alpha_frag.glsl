uniform vec3 color;

void main() {
	float alpha = uv.x;

	vec3 cb = checkerboard(0.25, 0.1);

	vec4 color = blend(cb, vec4(color, alpha));

	color.rgb += screen_space_dither(gl_FragCoord.xy);

    FragColor = color;
}
