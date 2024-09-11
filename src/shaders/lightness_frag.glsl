out vec4 FragColor;
in vec2 uv;

uniform float hue;
uniform float chroma;


void main() {
	float lightness = uv.x;

	vec3 hcl = vec3(hue / 360., chroma, lightness);
	bool valid;
	vec3 rgb = hcl2rgb(hcl, valid);

	vec3 color = valid ? rgb : BG;

    FragColor = vec4(color, 1.);
}
