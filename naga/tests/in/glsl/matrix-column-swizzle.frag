#version 450
// Swizzles of a matrix column reached through a variable, read and assigned.
layout(location = 0) out vec4 o_color;
layout(location = 0) in vec3 v_pos;

void main() {
    mat4 m = mat4(1.0);
    m[0].xyz = v_pos;
    m[3].w = 2.0;
    m[1].zx = m[0].xy;
    vec2 c = vec2(1.0);
    c[0].x = c[1].r * 3.0;
    o_color = vec4(m[0].xyz + m[1].zyx, m[3].w + c[0].x);
}
