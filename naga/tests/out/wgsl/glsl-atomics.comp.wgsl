struct Counts {
    total: atomic<u32>,
    hist: array<atomic<u32>>,
}

@group(0) @binding(0) 
var<storage, read_write> global: Counts;
@group(0) @binding(1) 
var marks: texture_storage_2d<r32uint,atomic>;
var<workgroup> lightCount: atomic<i32>;
var<workgroup> lightHashMap: array<atomic<u32>, 128>;
var<private> gl_LocalInvocationIndex_1: u32;

fn main_1() {
    var index: i32;
    var bit: u32;
    var known: bool;
    var old: u32;

    let _e8 = gl_LocalInvocationIndex_1;
    if (_e8 == 0u) {
        atomicStore((&lightCount), 0i);
    }
    workgroupBarrier();
    workgroupBarrier();
    let _e13 = atomicAdd((&lightCount), 1i);
    index = _e13;
    let _e16 = gl_LocalInvocationIndex_1;
    bit = (1u << (_e16 % 32u));
    let _e21 = bit;
    let _e22 = gl_LocalInvocationIndex_1;
    let _e26 = atomicOr((&lightHashMap[(_e22 / 32u)]), _e21);
    let _e27 = bit;
    known = ((_e26 & _e27) != 0u);
    let _e33 = index;
    let _e35 = atomicMax((&global.hist[_e33]), 3u);
    let _e37 = index;
    let _e39 = atomicCompareExchangeWeak((&global.total), 0u, u32(_e37));
    old = _e39.old_value;
    let _e43 = atomicExchange((&lightCount), -1i);
    let _e44 = index;
    let _e47 = old;
    textureAtomicOr(marks, vec2<i32>(_e44, 0i), _e47);
    textureBarrier();
    storageBarrier();
    workgroupBarrier();
    textureBarrier();
    storageBarrier();
    workgroupBarrier();
    textureBarrier();
    let _e48 = known;
    if _e48 {
        let _e50 = atomicMin((&lightCount), 7i);
        return;
    } else {
        return;
    }
}

@compute @workgroup_size(64, 1, 1) 
fn main(@builtin(local_invocation_index) gl_LocalInvocationIndex: u32) {
    gl_LocalInvocationIndex_1 = gl_LocalInvocationIndex;
    main_1();
    return;
}
