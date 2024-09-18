uniform float hue;
uniform float lightness;

void main() {
	float chroma = uv.x * 0.33;

	vec3 lch = vec3(lightness, chroma, hue / 360.);
	bool valid;
	vec3 rgb = oklch_to_srgb(lch, valid);

	vec3 color = valid ? rgb : BG;

	color += screen_space_dither(gl_FragCoord.xy);

    FragColor = rounded(vec4(color, 1.), 1.5, uv2, size);
}
