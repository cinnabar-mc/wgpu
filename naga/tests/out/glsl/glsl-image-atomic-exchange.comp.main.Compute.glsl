#version 420 core
#extension GL_ARB_compute_shader : require
#extension GL_OES_shader_image_atomic : require
layout(local_size_x = 16, local_size_y = 1, local_size_z = 1) in;

layout(r32ui) uniform uimage2D _group_0_binding_0_cs;

layout(r32i) uniform iimage2D _group_0_binding_1_cs;

uint gen_gl_LocalInvocationIndex_1 = 0u;


void main_1() {
    uint previous = 0u;
    int owner = 0;
    uint _e5 = gen_gl_LocalInvocationIndex_1;
    uint _e6 = imageAtomicExchange(_group_0_binding_0_cs, ivec2(0), _e5);
    previous = _e6;
    uint _e11 = previous;
    int _e13 = imageAtomicCompSwap(_group_0_binding_1_cs, ivec2(0), -1, int(_e11));
    owner = _e13;
    uint _e18 = previous;
    uint _e20 = imageAtomicCompSwap(_group_0_binding_0_cs, ivec2(1, 0), _e18, 7u);
    uint _e21 = gen_gl_LocalInvocationIndex_1;
    int _e25 = owner;
    imageStore(_group_0_binding_1_cs, ivec2(int(_e21), 1), ivec4(_e25));
    return;
}

void main() {
    uint gen_gl_LocalInvocationIndex = gl_LocalInvocationIndex;
    gen_gl_LocalInvocationIndex_1 = gen_gl_LocalInvocationIndex;
    main_1();
    return;
}

