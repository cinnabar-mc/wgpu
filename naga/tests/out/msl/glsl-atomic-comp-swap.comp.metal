// language: metal1.0
#include <metal_stdlib>
#include <simd/simd.h>

using metal::uint;

struct _atomic_compare_exchange_result_Uint_4_ {
    uint old_value;
    bool exchanged;
    char _pad2[3];
};
struct _atomic_compare_exchange_result_Sint_4_ {
    int old_value;
    bool exchanged;
    char _pad2[3];
};
struct type_4 {
    metal::atomic_uint inner[4];
};

template <typename A>
_atomic_compare_exchange_result_Uint_4_ naga_atomic_compare_exchange_weak_explicit(
    device A *atomic_ptr,
    uint cmp,
    uint v
) {
    bool swapped = metal::atomic_compare_exchange_weak_explicit(
        atomic_ptr, &cmp, v,
        metal::memory_order_relaxed, metal::memory_order_relaxed
    );
    return _atomic_compare_exchange_result_Uint_4_{cmp, swapped};
}
template <typename A>
_atomic_compare_exchange_result_Uint_4_ naga_atomic_compare_exchange_weak_explicit(
    threadgroup A *atomic_ptr,
    uint cmp,
    uint v
) {
    bool swapped = metal::atomic_compare_exchange_weak_explicit(
        atomic_ptr, &cmp, v,
        metal::memory_order_relaxed, metal::memory_order_relaxed
    );
    return _atomic_compare_exchange_result_Uint_4_{cmp, swapped};
}

template <typename A>
_atomic_compare_exchange_result_Sint_4_ naga_atomic_compare_exchange_weak_explicit(
    device A *atomic_ptr,
    int cmp,
    int v
) {
    bool swapped = metal::atomic_compare_exchange_weak_explicit(
        atomic_ptr, &cmp, v,
        metal::memory_order_relaxed, metal::memory_order_relaxed
    );
    return _atomic_compare_exchange_result_Sint_4_{cmp, swapped};
}
template <typename A>
_atomic_compare_exchange_result_Sint_4_ naga_atomic_compare_exchange_weak_explicit(
    threadgroup A *atomic_ptr,
    int cmp,
    int v
) {
    bool swapped = metal::atomic_compare_exchange_weak_explicit(
        atomic_ptr, &cmp, v,
        metal::memory_order_relaxed, metal::memory_order_relaxed
    );
    return _atomic_compare_exchange_result_Sint_4_{cmp, swapped};
}

void main_1(
    threadgroup type_4& values,
    threadgroup metal::atomic_int& signedValue
) {
    int index = 0;
    uint expected = 0u;
    uint replacement = 1u;
    uint local = {};
    uint old = {};
    int local_1 = {};
    int signedOld = {};
    int _e8 = index;
    index = as_type<int>(as_type<uint>(_e8) + as_type<uint>(1));
    uint _e12 = expected;
    expected = _e12 + 1u;
    uint _e15 = replacement;
    replacement = _e15 + 1u;
    uint2 loop_bound = uint2(4294967295u);
    while(true) {
        if (metal::all(loop_bound == uint2(0u))) { break; }
        loop_bound -= uint2(loop_bound.y == 0u, 1u);
        _atomic_compare_exchange_result_Uint_4_ _e19 = naga_atomic_compare_exchange_weak_explicit(&values.inner[_e8], _e12, _e15);
        local = _e19.old_value;
        if (_e19.exchanged || (_e19.old_value != _e12)) {
            break;
        }
    }
    uint _e24 = local;
    old = _e24;
    uint2 loop_bound_1 = uint2(4294967295u);
    while(true) {
        if (metal::all(loop_bound_1 == uint2(0u))) { break; }
        loop_bound_1 -= uint2(loop_bound_1.y == 0u, 1u);
        _atomic_compare_exchange_result_Sint_4_ _e29 = naga_atomic_compare_exchange_weak_explicit(&signedValue, 0, 1);
        local_1 = _e29.old_value;
        if (_e29.exchanged || (_e29.old_value != 0)) {
            break;
        }
    }
    int _e34 = local_1;
    signedOld = _e34;
    int _e36 = index;
    index = as_type<int>(as_type<uint>(_e36) + as_type<uint>(1));
    uint _e40 = old;
    int _e41 = signedOld;
    uint _e44 = metal::atomic_fetch_add_explicit(&values.inner[_e36], _e40 + static_cast<uint>(_e41), metal::memory_order_relaxed);
    return;
}

kernel void main_(
  uint __local_invocation_index [[thread_index_in_threadgroup]]
, threadgroup type_4& values
, threadgroup metal::atomic_int& signedValue
) {
    if (__local_invocation_index == 0u) {
        for (int __i0 = 0; __i0 < 4; __i0++) {
            metal::atomic_store_explicit(&values.inner[__i0], 0, metal::memory_order_relaxed);
        }
        metal::atomic_store_explicit(&signedValue, 0, metal::memory_order_relaxed);
    }
    metal::threadgroup_barrier(metal::mem_flags::mem_threadgroup);
    main_1(values, signedValue);
    return;
}
