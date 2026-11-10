struct NagaConstants {
    int first_vertex;
    int first_instance;
    uint other;
};
ConstantBuffer<NagaConstants> _NagaConstants: register(b0, space1);

RWTexture2D<uint> lock : register(u0);
RWTexture2D<int> owners : register(u1);
RWTexture2DArray<uint> layers : register(u2);
static uint gl_LocalInvocationIndex_1 = (uint)0;

void main_1()
{
    uint previous = (uint)0;
    int owner = (int)0;
    uint layer = (uint)0;

    uint _e6 = gl_LocalInvocationIndex_1;
    uint _e7; InterlockedExchange(lock[(int(0)).xx],_e6, _e7);
    previous = _e7;
    uint _e12 = previous;
    int _e14; InterlockedCompareExchange(owners[(int(0)).xx],int(-1), int(_e12), _e14);
    owner = _e14;
    uint _e19 = previous;
    uint _e21; InterlockedCompareExchange(lock[int2(int(1), int(0))],_e19, 7u, _e21);
    uint _e26 = previous;
    uint _e28; InterlockedCompareExchange(layers[int3(int2(int(0), int(0)), int(1))],0u, _e26, _e28);
    layer = _e28;
    uint _e30 = gl_LocalInvocationIndex_1;
    int _e34 = owner;
    uint _e35 = layer;
    owners[int2(int(_e30), int(1))] = (asint(asuint(_e34) + asuint(int(_e35)))).xxxx;
    return;
}

[numthreads(16, 1, 1)]
void main(uint gl_LocalInvocationIndex : SV_GroupIndex)
{
    gl_LocalInvocationIndex_1 = gl_LocalInvocationIndex;
    main_1();
    return;
}
