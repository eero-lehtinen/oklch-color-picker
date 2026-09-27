uniform vec3 values;

vec4 sampl(vec2 uv) {
	float chroma = uv.y * CHROMA_MAX;
	float hue = uv.x;
	float lightness = toe_inv(values.x);
	vec3 lch = vec3(lightness, chroma, hue);
	return oklch_to_linear_antialiased(lch);
}

void main() {
	FragColor = fragOutput(sampl(uv));
}
