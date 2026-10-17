#version 450
// Constant expressions may use the conditional operator and index other constants.
const vec3 GRADE = vec3(1.0, 0.5, 0.25);
const vec3 TARGET = length(GRADE) < 0.001 ? vec3(0.0) : normalize(GRADE);
const float PICK = GRADE.x > 2.0 ? 1 : 2.0;
const mat2 M = mat2(1.0, 2.0, 3.0, 4.0);
const vec2 COLUMN = M[1] * GRADE[0];

layout(location = 0) out vec4 o_color;

void main() {
    o_color = vec4(TARGET * PICK, COLUMN.y);
}
