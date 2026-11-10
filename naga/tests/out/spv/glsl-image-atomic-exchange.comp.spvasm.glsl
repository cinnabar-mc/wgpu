#version 460
layout(local_size_x = 16, local_size_y = 1, local_size_z = 1) in;

layout(set = 0, binding = 0, r32ui) uniform uimage2D _8;
layout(set = 0, binding = 1, r32i) uniform iimage2D _10;

uint _12 = 0u;

void _16()
{
    uint _26 = 0u;
    int _29 = 0;
    uint _37 = imageAtomicExchange(_8, ivec2(0), _12);
    _26 = _37;
    int _42 = imageAtomicCompSwap(_10, ivec2(0), -1, int(_26));
    _29 = _42;
    uint _45 = imageAtomicCompSwap(_8, ivec2(1, 0), _26, 7u);
    imageStore(_10, ivec2(int(_12), 1), ivec4(_29));
}

void main()
{
    _12 = gl_LocalInvocationIndex;
    _16();
}

