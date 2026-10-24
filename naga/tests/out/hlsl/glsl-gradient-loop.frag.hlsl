struct Params {
    int steps;
};

struct FragmentOutput {
    float4 color : SV_Target0;
};

Texture2D<float4> tex : register(t0);
SamplerState nagaSamplerHeap[2048]: register(s0, space0);
SamplerComparisonState nagaComparisonSamplerHeap[2048]: register(s0, space1);
StructuredBuffer<uint> nagaGroup0SamplerIndexArray : register(t0, space255);
static const SamplerState samp = nagaSamplerHeap[nagaGroup0SamplerIndexArray[1]];
cbuffer global : register(b2) { Params global; }
static float2 uv_1 = (float2)0;
static float4 color = (float4)0;

struct FragmentInput_main {
    float2 uv_2 : LOC0;
};

float density(float2 p)
{
    float2 p_1 = (float2)0;

    p_1 = p;
    float2 _e4 = p_1;
    float4 _e5 = tex.Sample(samp, _e4);
    return _e5.x;
}

void main_1()
{
    float4 sum = (0.0).xxxx;
    int i = int(0);
    int i_1 = int(0);
    int i_2 = int(0);

    uint2 loop_bound = uint2(4294967295u, 4294967295u);
    bool loop_init = true;
    [loop] while(true) {
        if (all(loop_bound == uint2(0u, 0u))) { break; }
        loop_bound -= uint2(loop_bound.y == 0u, 1u);
        if (!loop_init) {
            int _e15 = i;
            i = asint(asuint(_e15) + asuint(int(1)));
        }
        loop_init = false;
        int _e11 = i;
        int _e12 = global.steps;
        if (!((_e11 < _e12))) {
            break;
        }
        {
            float4 _e18 = sum;
            float2 _e19 = uv_1;
            int _e20 = i;
            float4 _e24 = tex.Sample(samp, (_e19 + (float(_e20)).xx));
            sum = (_e18 + _e24);
            float4 _e26 = sum;
            if ((_e26.w > 0.5)) {
                {
                    break;
                }
            }
        }
    }
    uint2 loop_bound_1 = uint2(4294967295u, 4294967295u);
    bool loop_init_1 = true;
    [loop] while(true) {
        if (all(loop_bound_1 == uint2(0u, 0u))) { break; }
        loop_bound_1 -= uint2(loop_bound_1.y == 0u, 1u);
        if (!loop_init_1) {
            int _e36 = i_1;
            i_1 = asint(asuint(_e36) + asuint(int(1)));
        }
        loop_init_1 = false;
        int _e32 = i_1;
        int _e33 = global.steps;
        if (!((_e32 < _e33))) {
            break;
        }
        {
            float4 _e40 = sum;
            float2 _e42 = uv_1;
            int _e44 = i_1;
            float _e47 = ddx((_e42.x * float(_e44)));
            float2 _e48 = uv_1;
            int _e49 = i_1;
            const float _e53 = density((_e48 + (float(_e49)).xx));
            sum.x = (_e40.x + (_e47 + _e53));
            float4 _e56 = sum;
            if ((_e56.x > 0.5)) {
                {
                    break;
                }
            }
        }
    }
    uint2 loop_bound_2 = uint2(4294967295u, 4294967295u);
    bool loop_init_2 = true;
    while(true) {
        if (all(loop_bound_2 == uint2(0u, 0u))) { break; }
        loop_bound_2 -= uint2(loop_bound_2.y == 0u, 1u);
        if (!loop_init_2) {
            int _e66 = i_2;
            i_2 = asint(asuint(_e66) + asuint(int(1)));
        }
        loop_init_2 = false;
        int _e62 = i_2;
        int _e63 = global.steps;
        if (!((_e62 < _e63))) {
            break;
        }
        {
            float4 _e69 = sum;
            float2 _e70 = uv_1;
            int _e71 = i_2;
            float4 _e76 = tex.SampleLevel(samp, (_e70 + (float(_e71)).xx), 0.0);
            sum = (_e69 + _e76);
            float4 _e78 = sum;
            if ((_e78.y > 0.5)) {
                {
                    break;
                }
            }
        }
    }
    float4 _e82 = sum;
    color = _e82;
    return;
}

FragmentOutput ConstructFragmentOutput(float4 arg0) {
    FragmentOutput ret = (FragmentOutput)0;
    ret.color = arg0;
    return ret;
}

FragmentOutput main(FragmentInput_main fragmentinput_main)
{
    float2 uv = fragmentinput_main.uv_2;
    uv_1 = uv;
    main_1();
    float4 _e3 = color;
    const FragmentOutput fragmentoutput = ConstructFragmentOutput(_e3);
    return fragmentoutput;
}
