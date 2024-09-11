out vec4 FragColor;
in vec2 uv;

uniform float hue;

void main() {
	float chroma = (uv.y) * 0.33;
	float lightness = uv.x;

	vec3 lch = vec3(lightness, chroma, hue / 360.);
	bool valid;
	vec3 rgb = oklch_to_srgb(lch, valid);

	vec3 color = valid ? rgb : BG;

	FragColor = vec4(color, 1.);
}
