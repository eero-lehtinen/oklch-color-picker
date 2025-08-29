uniform vec3 values;

void main() {
	vec4 color = vec4(0);
	if (mode == 0u) {
		float chroma = 0.125;
		float lightness = 0.75;
		float hue = uv.x;
		color = oklch_to_linear_clamped(vec3(lightness, chroma, hue));
	} else {
		float hue = values.x / 360.;
		float saturation = values.y;
		float value = uv.x;
		color = okhsv_to_linear(vec3(hue, saturation, value));
	}

    FragColor = output(color);
}
