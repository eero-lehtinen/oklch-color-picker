uniform vec3 values;

void main() {
	vec4 color = vec4(0); 
	if (mode == 0u) {
		float lightness = toe_inv(uv.x);
		float chroma = values.y;
		float hue = values.z / 360.;
		color = oklch_to_srgb(vec3(lightness, chroma, hue));
	} else {
		float hue = uv.x;
		float saturation = 0.68;
		float value = 0.84;
		color = okhsv_to_srgb(vec3(hue, saturation, value));
	}

	color.rgb += screen_space_dither(gl_FragCoord.xy);

    FragColor = premultiply(color);
}
