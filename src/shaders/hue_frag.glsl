out vec4 FragColor;
in vec2 uv;

uniform float chroma;
uniform float lightness;


void main() {
	float chroma = 0.125;
	float lightness = 0.75;
	
	float hue = uv.x;

	vec3 hcl = vec3(hue, chroma, lightness);
	bool valid;
	vec3 rgb = hcl2rgb(hcl, valid);

    FragColor = rounded_box(rgb, valid);
}
