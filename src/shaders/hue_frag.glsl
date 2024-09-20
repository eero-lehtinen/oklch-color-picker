uniform float chroma;
uniform float lightness;

void main() {
	float chroma = 0.125;
	float lightness = 0.75;
	float hue = uv.x;
	vec3 lch = vec3(lightness, chroma, hue);

	vec4 color = oklch_to_srgb(lch);
	color.rgb += screen_space_dither(gl_FragCoord.xy);

    FragColor = premultiply(color);
}
