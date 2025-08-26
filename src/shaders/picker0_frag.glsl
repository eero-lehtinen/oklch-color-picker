uniform vec3 values;

vec4 sample_oklch(vec2 uv) {
	float chroma = uv.y * CHROMA_MAX;
	float lightness = toe_inv(uv.x);
	float hue = values.z / 360;
	vec3 lch = vec3(lightness, chroma, hue);
	return oklch_to_srgb(lch);
}

vec4 sample_okhsv(vec2 uv) {
	float saturation = uv.x;
	float value = uv.y;
	float hue = values.x / 360.;
	vec3 hsv = vec3(hue, saturation, value);
	return okhsv_to_srgb(hsv);
}

 vec4 sampl(vec2 uv) {
 	if (mode == 0u) {
 		return sample_oklch(uv);
 	} else {
 		return sample_okhsv(uv);
 	}
 }

void main() {
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
}


