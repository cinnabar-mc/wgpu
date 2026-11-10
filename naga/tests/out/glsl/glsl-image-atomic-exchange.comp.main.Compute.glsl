#version 420 core
#extension GL_ARB_compute_shader : require
#extension GL_OES_shader_image_atomic : require
layout(local_size_x = 16, local_size_y = 1, local_size_z = 1) in;

layout(r32ui) uniform uimage2D _group_0_binding_0_cs;

layout(r32i) uniform iimage2D _group_0_binding_1_cs;

layout(r32ui) uniform uimage2DArray _group_0_binding_2_cs;

uint gen_gl_LocalInvocationIndex_1 = 0u;


void main_1() {
    uint previous = 0u;
    int owner = 0;
    uint layer = 0u;
    uint _e6 = gen_gl_LocalInvocationIndex_1;
    uint _e7 = imageAtomicExchange(_group_0_binding_0_cs, ivec2(0), _e6);
    previous = _e7;
    uint _e12 = previous;
    int _e14 = imageAtomicCompSwap(_group_0_binding_1_cs, ivec2(0), -1, int(_e12));
    owner = _e14;
    uint _e19 = previous;
    uint _e21 = imageAtomicCompSwap(_group_0_binding_0_cs, ivec2(1, 0), _e19, 7u);
    uint _e26 = previous;
    uint _e28 = imageAtomicCompSwap(_group_0_binding_2_cs, ivec2(ivec2(0, 0), 1), 0u, _e26);
    layer = _e28;
    uint _e30 = gen_gl_LocalInvocationIndex_1;
    int _e34 = owner;
    uint _e35 = layer;
    imageStore(_group_0_binding_1_cs, ivec2(int(_e30), 1), ivec4((_e34 + int(_e35))));
    return;
}

void main() {
    uint gen_gl_LocalInvocationIndex = gl_LocalInvocationIndex;
    gen_gl_LocalInvocationIndex_1 = gen_gl_LocalInvocationIndex;
    main_1();
    return;
}

