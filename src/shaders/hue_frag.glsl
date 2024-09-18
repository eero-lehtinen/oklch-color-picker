uniform float chroma;
uniform float lightness;

void main() {
	float chroma = 0.125;
	float lightness = 0.75;
	
	float hue = uv.x;

	vec3 lch = vec3(lightness, chroma, hue);
	bool valid;
	vec3 rgb = oklch_to_srgb(lch, valid);

	rgb += screen_space_dither(gl_FragCoord.xy);

    FragColor = vec4(rgb, 1.);
}
