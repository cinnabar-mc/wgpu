#version 450
// GLSL 4.20 allows swizzling scalars.
layout(location = 0) out vec4 o_color;
layout(location = 0) in float v_depth;

void main() {
    float d = v_depth;
    d.x = d.x * 2.0;
    o_color = vec4(d.xx, d.r, d.s);
    o_color.yzw = v_depth.sss;
}
