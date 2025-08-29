uniform vec3 values;

void main() {
	vec4 color = vec4(0);
	if (mode == 0u) {
		float lightness = toe_inv(values[0]);
		float chroma = uv.x * CHROMA_MAX;
		float hue = values.z / 360.;
		color = oklch_to_linear_clamped(vec3(lightness, chroma, hue));
	} else {
		float hue = values.x / 360.;
		float saturation = uv.x;
		float value = values.z;
		color = okhsv_to_linear(vec3(hue, saturation, value));
	}

    FragColor = fragOutput(color);
}
