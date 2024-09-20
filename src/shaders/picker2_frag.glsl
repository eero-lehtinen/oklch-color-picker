uniform float lightness;

vec4 sample(vec2 uv) {
	float chroma = uv.y * CHROMA_MAX;
	float hue = uv.x;
	vec3 lch = vec3(lightness, chroma, hue);
	return oklch_to_srgb(lch);
}


void main() {
	vec4 color = vec4(0.);
	
	if (supersample == 1u) {
		vec2 texel_size = 1.0 / size;
		for (int i = 0; i < 4; i++) {
			color += sample(uv + sample_positions[i] * texel_size);
		}
		color /= 4.;
	} else {
		color = sample(uv);
	}

	color.rgb += screen_space_dither(gl_FragCoord.xy);

	FragColor = premultiply(color);
}
