#version 450
// inverse, transpose, determinant, mix and smoothstep in constant expressions.
const mat3 TO_XYZ = mat3(
    0.64 / 0.33, 0.3 / 0.6, 0.15 / 0.06,
    1.0, 1.0, 1.0,
    0.03 / 0.33, 0.1 / 0.6, 0.79 / 0.06
);
const vec3 WHITE = vec3(0.3127 / 0.329, 1.0, 0.3583 / 0.329) * inverse(TO_XYZ);
const mat3 FROM_XYZ = inverse(mat3(WHITE * TO_XYZ[0], WHITE, WHITE * TO_XYZ[2]));
const mat2x3 T = transpose(mat3x2(1.0, 2.0, 3.0, 4.0, 5.0, 6.0));
const float DET = determinant(mat2(2.0, 1.0, 1.0, 3.0));
const vec3 TINT = mix(vec3(0.0), vec3(1.0, 0.5, 0.25), 0.5);
const float EDGE = smoothstep(0.0, 2.0, 0.5);

layout(location = 0) out vec4 o_color;
layout(binding = 0) uniform Inputs { vec3 xyz; };

void main() {
    o_color = vec4(FROM_XYZ * xyz + T[0] + TINT * DET, EDGE);
}
