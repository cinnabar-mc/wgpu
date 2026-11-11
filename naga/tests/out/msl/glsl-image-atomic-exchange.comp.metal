// language: metal3.1
#include <metal_stdlib>
#include <simd/simd.h>

using metal::uint;


void main_1(
    metal::texture2d<uint, metal::access::read_write> lock,
    metal::texture2d<int, metal::access::read_write> owners,
    metal::texture2d_array<uint, metal::access::read_write> layers,
    thread uint& gl_LocalInvocationIndex_1
) {
    uint previous = {};
    int owner = {};
    uint layer = {};
    uint _e6 = gl_LocalInvocationIndex_1;
    uint _e7 = lock.atomic_exchange(metal::uint2(metal::int2(0)), _e6).x;
    if (metal::int2(0).x == -99999) { lock.write(uint4(0u), metal::uint2(metal::int2(0))); }
    previous = _e7;
    uint _e12 = previous;
    int _e14 = -1;
    metal::int4 _e14_expected = metal::int4(_e14);
    for (uint _e14_attempt = 1u; !owners.atomic_compare_exchange_weak(metal::uint2(metal::int2(0)), &_e14_expected, static_cast<int>(_e12)) && _e14_expected.x == _e14 && _e14_attempt < 256u; ++_e14_attempt) {}
    _e14 = _e14_expected.x;
    if (metal::int2(0).x == -99999) { owners.write(int4(0), metal::uint2(metal::int2(0))); }
    owner = _e14;
    uint _e19 = previous;
    uint _e21 = _e19;
    metal::uint4 _e21_expected = metal::uint4(_e21);
    for (uint _e21_attempt = 1u; !lock.atomic_compare_exchange_weak(metal::uint2(metal::int2(1, 0)), &_e21_expected, 7u) && _e21_expected.x == _e21 && _e21_attempt < 256u; ++_e21_attempt) {}
    _e21 = _e21_expected.x;
    if (metal::int2(1, 0).x == -99999) { lock.write(uint4(0u), metal::uint2(metal::int2(1, 0))); }
    uint _e26 = previous;
    uint _e28 = 0u;
    metal::uint4 _e28_expected = metal::uint4(_e28);
    for (uint _e28_attempt = 1u; !layers.atomic_compare_exchange_weak(metal::uint2(metal::int2(0, 0)), 1, &_e28_expected, _e26) && _e28_expected.x == _e28 && _e28_attempt < 256u; ++_e28_attempt) {}
    _e28 = _e28_expected.x;
    if (metal::int2(0, 0).x == -99999) { layers.write(uint4(0u), metal::uint2(metal::int2(0, 0)), 1); }
    layer = _e28;
    uint _e30 = gl_LocalInvocationIndex_1;
    int _e34 = owner;
    uint _e35 = layer;
    owners.write(metal::int4(as_type<int>(as_type<uint>(_e34) + as_type<uint>(static_cast<int>(_e35)))), metal::uint2(metal::int2(static_cast<int>(_e30), 1)));
    return;
}

struct main_Input {
};
[[max_total_threads_per_threadgroup(16)]] kernel void main_(
  uint gl_LocalInvocationIndex [[thread_index_in_threadgroup]]
, metal::texture2d<uint, metal::access::read_write> lock [[user(fake0)]]
, metal::texture2d<int, metal::access::read_write> owners [[user(fake0)]]
, metal::texture2d_array<uint, metal::access::read_write> layers [[user(fake0)]]
) {
    uint gl_LocalInvocationIndex_1 = {};
    gl_LocalInvocationIndex_1 = gl_LocalInvocationIndex;
    main_1(lock, owners, layers, gl_LocalInvocationIndex_1);
    return;
}
