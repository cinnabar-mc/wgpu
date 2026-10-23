#version 450

layout(set = 0, binding = 0) uniform texture2D tex;
layout(set = 0, binding = 1) uniform sampler samp;
layout(set = 0, binding = 2) uniform Params {
    int steps;
};

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 color;

float density(vec2 p) {
    return texture(sampler2D(tex, samp), p).r;
}

void main() {
    vec4 sum = vec4(0.0);
    // FXC cannot unroll these loops, which have uniform bounds and data-dependent exits.
    for (int i = 0; i < steps; i++) {
        sum += texture(sampler2D(tex, samp), uv + float(i));
        if (sum.a > 0.5) {
            break;
        }
    }
    for (int i = 0; i < steps; i++) {
        sum.r += dFdx(uv.x * float(i)) + density(uv + float(i));
        if (sum.r > 0.5) {
            break;
        }
    }
    // Explicit-level sampling needs no derivatives, so the loop keeps no attribute.
    for (int i = 0; i < steps; i++) {
        sum += textureLod(sampler2D(tex, samp), uv + float(i), 0.0);
        if (sum.g > 0.5) {
            break;
        }
    }
    color = sum;
}
