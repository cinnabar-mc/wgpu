// language: metal3.1
#include <metal_stdlib>
#include <simd/simd.h>

using metal::uint;


void main_1(
    metal::texture2d<uint, metal::access::read_write> lock,
    metal::texture2d<int, metal::access::read_write> owners,
    thread uint& gl_LocalInvocationIndex_1
) {
    uint previous = {};
    int owner = {};
    uint _e5 = gl_LocalInvocationIndex_1;
    uint _e6 = lock.atomic_exchange(metal::uint2(metal::int2(0)), _e5).x;
    if (metal::int2(0).x == -99999) { lock.write(uint4(0u), metal::uint2(metal::int2(0))); }
    previous = _e6;
    uint _e11 = previous;
    int _e13 = -1;
    metal::int4 _e13_expected = metal::int4(_e13);
    while (!owners.atomic_compare_exchange_weak(metal::uint2(metal::int2(0)), &_e13_expected, static_cast<int>(_e11)) && _e13_expected.x == _e13) {}
    _e13 = _e13_expected.x;
    if (metal::int2(0).x == -99999) { owners.write(int4(0), metal::uint2(metal::int2(0))); }
    owner = _e13;
    uint _e18 = previous;
    uint _e20 = _e18;
    metal::uint4 _e20_expected = metal::uint4(_e20);
    while (!lock.atomic_compare_exchange_weak(metal::uint2(metal::int2(1, 0)), &_e20_expected, 7u) && _e20_expected.x == _e20) {}
    _e20 = _e20_expected.x;
    if (metal::int2(1, 0).x == -99999) { lock.write(uint4(0u), metal::uint2(metal::int2(1, 0))); }
    uint _e21 = gl_LocalInvocationIndex_1;
    int _e25 = owner;
    owners.write(metal::int4(_e25), metal::uint2(metal::int2(static_cast<int>(_e21), 1)));
    return;
}

struct main_Input {
};
[[max_total_threads_per_threadgroup(16)]] kernel void main_(
  uint gl_LocalInvocationIndex [[thread_index_in_threadgroup]]
, metal::texture2d<uint, metal::access::read_write> lock [[user(fake0)]]
, metal::texture2d<int, metal::access::read_write> owners [[user(fake0)]]
) {
    uint gl_LocalInvocationIndex_1 = {};
    gl_LocalInvocationIndex_1 = gl_LocalInvocationIndex;
    main_1(lock, owners, gl_LocalInvocationIndex_1);
    return;
}
