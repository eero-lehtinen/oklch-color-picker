uniform float hue;

vec4 sampl(vec2 uv) {
	float chroma = 0.125;
	float lightness = 0.75;

	if (length(uv) > 1.) {
		return vec4(0,0,0,1);
	}

	float angle = atan(uv.y, uv.x) / (2 * PI);
	float hue = angle;
	vec3 lch = vec3(lightness, chroma, hue);

	return oklch_to_srgb(lch);
}

void main() {
	vec2 uv = uv * 2. - 1.;

	vec4 color = vec4(0.);

	 if (supersample == 1u) {
	 	vec2 texel_size = 1.0 / size;
	 	for (int i = 0; i < 4; i++) {
	 		color += sampl(uv + sample_positions[i] * texel_size);
	 	}
	 	color /= 4.;
	 } else {
	 	color = sampl(uv);
	 }

	color.rgb += screen_space_dither(gl_FragCoord.xy);

    FragColor = premultiply(color);

	// vec4 color = vec4(0.);
	//
	// if (supersample == 1u) {
	// 	vec2 texel_size = 1.0 / size;
	// 	for (int i = 0; i < 4; i++) {
	// 		color += sampl(uv + sample_positions[i] * texel_size);
	// 	}
	// 	color /= 4.;
	// } else {
	// 	color = sampl(uv);
	// }
	//
	// color.rgb += screen_space_dither(gl_FragCoord.xy);
	//
	// FragColor = premultiply(color);
}







