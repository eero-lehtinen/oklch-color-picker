out vec4 FragColor;
in vec2 uv;

uniform float chroma;
uniform float lightness;


void main() {
	float chroma = 0.125;
	float lightness = 0.75;
	
	float hue = uv.x;

	vec3 lch = vec3(lightness, chroma, hue);
	bool valid;
	vec3 rgb = oklch_to_srgb(lch, valid);

    FragColor = vec4(rgb, 1.);
}
