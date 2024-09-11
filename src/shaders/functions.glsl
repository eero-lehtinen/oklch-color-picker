#version 450

const float PI = 3.14159265358979323846;
const vec3 BG = vec3(0.17, 0.17, 0.18);

float gamma_function_inverse(float v) {
	if (v <= 0.0) {
		return v;
	}
	if (v <= 0.0031308) {
		return 12.92 * v;
	}
	return 1.055 * pow(v, 1.0 / 2.4) - 0.055;
}

vec3 gamma_function_inverse(vec3 v) {
    return vec3(gamma_function_inverse(v.r), gamma_function_inverse(v.g), gamma_function_inverse(v.b));
}

vec3 hcl2rgb(vec3 hcl, out bool valid) {
    vec3 lab = vec3(
        hcl.z,
        hcl.y * cos(hcl.x * PI*2.0),
        hcl.y * sin(hcl.x * PI*2.0)
    );
    
    vec3 lms = vec3(
        lab.x + 0.3963377774f * lab.y + 0.2158037573f * lab.z,
        lab.x - 0.1055613458f * lab.y - 0.0638541728f * lab.z,
        lab.x - 0.0894841775f * lab.y - 1.2914855480f * lab.z
    );
    
    lms = pow(max(lms, vec3(0.0)), vec3(3.0));
    
    vec3 rgb = vec3(
        +4.0767416621f * lms.x - 3.3077115913f * lms.y + 0.2309699292f * lms.z,
        -1.2684380046f * lms.x + 2.6097574011f * lms.y - 0.3413193965f * lms.z,
        -0.0041960863f * lms.x - 0.7034186147f * lms.y + 1.7076147010f * lms.z
    );
     
    valid = !(any(lessThan(rgb, vec3(0.0))) || any(greaterThan(rgb, vec3(1.0))));

    rgb = gamma_function_inverse(rgb);
    

    return rgb;
}


uniform float width;
in vec2 uv2;

// b.x = width
// b.y = height
// r.x = roundness top-right  
// r.y = roundness boottom-right
// r.z = roundness top-left
// r.w = roundness bottom-left
float sdf_rounded_box(vec2 p, vec2 b, vec4 r) 
{
    vec2 q = abs(p)-b+r.x;
	vec2 q2 = vec2(q.x * width, q.y);
    return min(max(q.x,q.y),0.0) + length(max(q,0.0)) - r.x;
}


float sdf_rounded_box(vec2 uv, float radius) {
	vec2 b = vec2(width, 1.);
	float r = radius;
	vec2 q = abs(uv2) - b + r;
	return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
}

float radius = 0.5;

vec4 rounded_box(vec3 color, bool valid) {
    vec2 size = width >= 1. ? vec2(width, 1.) : vec2(1., 1. / width);

	vec2 b = size;
	float r = radius;
	vec2 q = abs(uv2) - b + r;
	float d = min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;

	if (valid) {
		return vec4(color.rgb, smoothstep(0.0, -0.01, d));
	} else {
		float thickness = 0.04;
		return vec4(vec3(0.6), smoothstep(0.0, -0.01, d) * smoothstep(-0.01 - thickness, -0.01 - thickness + 0.01, d) * 0.5);
	}
}

float xor(float a, float b) {
	if (a == b) {
		return 0.;
	}
	return 1.;
}

vec4 blend(vec4 below, vec4 above) {
	float a_result = above.a + below.a * (1. - above.a);

	return vec4((above.a * above.rgb + below.a * below.rgb * (1. - above.a)) / a_result, a_result);
}
	

// float sdf_rounded_box(vec2 uv, float radius) {
// 	float w = 4.;
// 	vec2 q = abs(uv * 0.4 - 0.2) - vec2(radius, radius * w);
// 	return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - radius;
// }

