
const KIND_PICKER_1 = 0;
const KIND_PICKER_2 = 1;
const KIND_SLIDER_1 = 2;
const KIND_SLIDER_2 = 3;
const KIND_SLIDER_3 = 4;
const KIND_SLIDER_4 = 5;
const KIND_FINAL = 6;

const MODE_OKLCH = 0;
const MODE_OKHSV = 1;

fn picker1(uv: vec2<f32>) -> vec4<f32> {
    if u_mode == MODE_OKLCH {
        let chroma = uv.y * CHROMA_MAX;
        let lightness = toe_inv(uv.x);
        let hue = u_values.x / 360.;
        let lch = vec3f(lightness, chroma, hue);
        return oklch_to_linear_clamped(lch);
    } else {
        let saturation = uv.x;
        let value = uv.y;
        let hue = u_values.x / 360.;
        let hsv = vec3f(hue, saturation, value);
        return okhsv_to_linear(hsv);
    }
}

fn picker2(uv: vec2<f32>) -> vec4<f32> {
    let chroma = uv.y * CHROMA_MAX;
    let hue = uv.x;
    let lightness = toe_inv(u_values.x);
    let lch = vec3f(lightness, chroma, hue);
    return oklch_to_linear_clamped(lch);
}

fn slider1(uv: vec2<f32>) -> vec4<f32> {
    if u_mode == MODE_OKLCH {
        let lightness = toe_inv(uv.x);
        let chroma = u_values.y;
        let hue = u_values.z / 360.;
        let lch = vec3f(lightness, chroma, hue);
        return oklch_to_linear_clamped(lch);
    } else {
        let hue = uv.x;
        let saturation = 0.68;
        let value = 0.84;
        let hsv = vec3f(hue, saturation, value);
        return okhsv_to_linear(hsv);

    }
}

// void main() {
// 	vec4 color = vec4(0);
// 	if (mode == 0u) {
// 		float lightness = toe_inv(values[0]);
// 		float chroma = uv.x * CHROMA_MAX;
// 		float hue = values.z / 360.;
// 		color = oklch_to_linear_clamped(vec3(lightness, chroma, hue));
// 	} else {
// 		float hue = values.x / 360.;
// 		float saturation = uv.x;
// 		float value = values.z;
// 		color = okhsv_to_linear(vec3(hue, saturation, value));
// 	}
//
//     FragColor = fragOutput(color);
// }

fn slider2(uv: vec2<f32>) -> vec4<f32> {
    if u_mode == MODE_OKLCH {
        let lightness = toe_inv(u_values.x);
        let chroma = uv.x * CHROMA_MAX;
        let hue = u_values.z / 360.;
        let lch = vec3f(lightness, chroma, hue);
        return oklch_to_linear_clamped(lch);
    } else {
        let hue = u_values.x / 360.;
        let saturation = uv.x;
        let value = u_values.z;
        let hsv = vec3f(hue, saturation, value);
        return okhsv_to_linear(hsv);
    }
}

// void main() {
// 	vec4 color = vec4(0);
// 	if (mode == 0u) {
// 		float chroma = 0.125;
// 		float lightness = 0.75;
// 		float hue = uv.x;
// 		color = oklch_to_linear_clamped(vec3(lightness, chroma, hue));
// 	} else {
// 		float hue = values.x / 360.;
// 		float saturation = values.y;
// 		float value = uv.x;
// 		color = okhsv_to_linear(vec3(hue, saturation, value));
// 	}
//
//     FragColor = fragOutput(color);
// }
//
fn slider3(uv: vec2<f32>) -> vec4<f32> {
    if u_mode == MODE_OKLCH {
        let chroma = 0.125;
        let lightness = 0.75;
        let hue = uv.x;
        let lch = vec3f(lightness, chroma, hue);
        return oklch_to_linear_clamped(lch);
    } else {
        let hue = u_values.x / 360.;
        let saturation = u_values.y;
        let value = uv.x;
        let hsv = vec3f(hue, saturation, value);
        return okhsv_to_linear(hsv);
    }
}

fn sample(uv: vec2<f32>) -> vec4<f32> {
    switch (u_kind) {
        case KIND_PICKER_1: {
            return picker1(uv);
        }
        case KIND_PICKER_2: {
            return picker2(uv);
        }
        case KIND_SLIDER_1: {
            return slider1(uv);
        }
        case KIND_SLIDER_2: {
            return slider2(uv);
        }
        case KIND_SLIDER_3: {
            return slider3(uv);
        }
        case KIND_SLIDER_4: {
            return vec4f(0,0,0,1);
        }
        case KIND_FINAL: {
            return vec4f(0,0,0,1);
        }
        default: {
            // Invalid kind
            return vec4f(1.0, 0.0, 1.0, 1.0);
        }
    }
}


@fragment
fn main(in: VertexOut) -> @location(0) vec4<f32> {
    var color = vec4f(0,0,0,0);
    if (u_supersample == 1u) {
        let texel_size = 1.0 / u_size;
        for (var i = 0u; i < 4u; i = i + 1u) {
            color += sample(in.uv + sample_positions[i] * texel_size);
        }
        color /= 4.0;
    } else {
        color = sample(in.uv);
    }

    return color;
}
