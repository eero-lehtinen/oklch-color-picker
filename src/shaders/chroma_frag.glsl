uniform float hue;
uniform float lightness;

void main() {
	float chroma = uv.x * CHROMA_MAX;
	vec3 lch = vec3(lightness, chroma, hue / 360.);

	vec4 color = oklch_to_srgb(lch);
	color.rgb += screen_space_dither(gl_FragCoord.xy);

    FragColor = premultiply(color);
}
