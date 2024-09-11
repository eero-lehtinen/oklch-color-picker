out vec4 FragColor;
in vec2 uv;

uniform float hue;
uniform float lightness;


void main() {
	float chroma = uv.x * 0.33;

	vec3 hcl = vec3(hue / 360., chroma, lightness);
	bool valid;
	vec3 rgb = hcl2rgb(hcl, valid);

	vec3 color = valid ? rgb : BG;

    FragColor = vec4(color, 1.);
}
