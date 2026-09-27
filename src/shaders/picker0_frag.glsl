uniform vec3 values;

const float BLACK_STRIP_WIDTH = 6.; // pixels
const int BLACK_STRIP_SAMPLES = 6; // per axis

vec3 uv_to_oklch(vec2 uv) {
	return vec3(toe_inv(uv.x), uv.y * CHROMA_MAX, values.z / 360.);
}

vec4 sample_oklch(vec2 uv) {
	vec2 pixel = vec2(dFdx(uv.x), dFdy(uv.y));
	vec4 color = oklch_to_linear_antialiased(uv_to_oklch(uv));
	// Near black the gamut distance is too nonlinear for the derivative based
	// edge estimate, which draws edges that don't exist. Supersample instead.
	if (uv.x < BLACK_STRIP_WIDTH * pixel.x) {
		float n = float(BLACK_STRIP_SAMPLES);
		float inside = 0.;
		for (int i = 0; i < BLACK_STRIP_SAMPLES; i++) {
			for (int j = 0; j < BLACK_STRIP_SAMPLES; j++) {
				vec2 p = uv + ((vec2(i, j) + 0.5) / n - 0.5) * pixel;
				inside += float(gamut_dist(oklab_to_linear(oklch_to_oklab(uv_to_oklch(p)))) <= 0.);
			}
		}
		color.a = inside / (n * n);
	}
	return color;
}

vec4 sample_okhsv(vec2 uv) {
	float saturation = uv.x;
	float value = uv.y;
	float hue = values.x / 360.;
	vec3 hsv = vec3(hue, saturation, value);
	return okhsv_to_linear(hsv);
}

 vec4 sampl(vec2 uv) {
 	if (mode == 0u) {
 		return sample_oklch(uv);
 	} else {
 		return sample_okhsv(uv);
 	}
 }

void main() {
	FragColor = fragOutput(sampl(uv));
}


