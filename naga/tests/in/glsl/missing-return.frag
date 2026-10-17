#version 450
// Control may reach the end of a function with a return type; its value is undefined.
layout(location = 0) out vec4 o_color;
layout(location = 0) in vec2 v_uv;

float effect(vec2 uv) {
    if (uv.x > 0.5) {
        return uv.y;
    }
}

void main() {
    o_color = vec4(effect(v_uv));
}
