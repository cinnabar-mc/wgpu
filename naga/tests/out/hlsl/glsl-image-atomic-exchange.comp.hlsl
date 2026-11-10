struct NagaConstants {
    int first_vertex;
    int first_instance;
    uint other;
};
ConstantBuffer<NagaConstants> _NagaConstants: register(b0, space1);

RWTexture2D<uint> lock : register(u0);
RWTexture2D<int> owners : register(u1);
static uint gl_LocalInvocationIndex_1 = (uint)0;

void main_1()
{
    uint previous = (uint)0;
    int owner = (int)0;

    uint _e5 = gl_LocalInvocationIndex_1;
    uint _e6; InterlockedExchange(lock[(int(0)).xx],_e5, _e6);
    previous = _e6;
    uint _e11 = previous;
    int _e13; InterlockedCompareExchange(owners[(int(0)).xx],int(-1), int(_e11), _e13);
    owner = _e13;
    uint _e18 = previous;
    uint _e20; InterlockedCompareExchange(lock[int2(int(1), int(0))],_e18, 7u, _e20);
    uint _e21 = gl_LocalInvocationIndex_1;
    int _e25 = owner;
    owners[int2(int(_e21), int(1))] = (_e25).xxxx;
    return;
}

[numthreads(16, 1, 1)]
void main(uint gl_LocalInvocationIndex : SV_GroupIndex)
{
    gl_LocalInvocationIndex_1 = gl_LocalInvocationIndex;
    main_1();
    return;
}
