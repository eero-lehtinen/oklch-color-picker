out vec4 FragColor;
in vec2 uv;

uniform float lightness;

void main() {
	float chroma = (uv.y) * 0.33;
	float hue = uv.x;

	vec3 hcl = vec3(hue, chroma, lightness);
	bool valid;
	vec3 rgb = hcl2rgb(hcl, valid);

	vec3 color = valid ? rgb : BG;

	FragColor = vec4(color, 1.);
}
