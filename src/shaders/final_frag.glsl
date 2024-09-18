uniform vec4 prev_color;
uniform vec4 color;

void main() {
	vec3 cb = checkerboard(0.1, 0.1);

	vec4 color = uv.x < 0.5 ? prev_color : color;

	color = blend(cb, color);

	color.rgb += screen_space_dither(gl_FragCoord.xy);

    FragColor = rounded(color, 2.5, uv2, size);
}
