#version 460
layout(local_size_x = 16, local_size_y = 1, local_size_z = 1) in;

layout(set = 0, binding = 0, r32ui) uniform uimage2D _9;
layout(set = 0, binding = 1, r32i) uniform iimage2D _11;
layout(set = 0, binding = 2, r32ui) uniform uimage2DArray _13;

uint _15 = 0u;

void _19()
{
    uint _32 = 0u;
    int _35 = 0;
    uint _38 = 0u;
    uint _44 = imageAtomicExchange(_9, ivec2(0), _15);
    _32 = _44;
    int _49 = imageAtomicCompSwap(_11, ivec2(0), -1, int(_32));
    _35 = _49;
    uint _52 = imageAtomicCompSwap(_9, ivec2(1, 0), _32, 7u);
    uint _57 = imageAtomicCompSwap(_13, ivec3(ivec2(0), 1), 0u, _32);
    _38 = _57;
    imageStore(_11, ivec2(int(_15), 1), ivec4(_35 + int(_38)));
}

void main()
{
    _15 = gl_LocalInvocationIndex;
    _19();
}

