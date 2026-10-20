var<workgroup> values: array<atomic<u32>, 4>;
var<workgroup> signedValue: atomic<i32>;

fn main_1() {
    var index: i32 = 0i;
    var expected: u32 = 0u;
    var replacement: u32 = 1u;
    var local: u32;
    var old: u32;
    var local_1: i32;
    var signedOld: i32;

    let _e8 = index;
    index = (_e8 + 1i);
    let _e12 = expected;
    expected = (_e12 + 1u);
    let _e15 = replacement;
    replacement = (_e15 + 1u);
    loop {
        let _e19 = atomicCompareExchangeWeak((&values[_e8]), _e12, _e15);
        local = _e19.old_value;
        if (_e19.exchanged || (_e19.old_value != _e12)) {
            break;
        }
    }
    let _e24 = local;
    old = _e24;
    loop {
        let _e29 = atomicCompareExchangeWeak((&signedValue), 0i, 1i);
        local_1 = _e29.old_value;
        if (_e29.exchanged || (_e29.old_value != 0i)) {
            break;
        }
    }
    let _e34 = local_1;
    signedOld = _e34;
    let _e36 = index;
    index = (_e36 + 1i);
    let _e40 = old;
    let _e41 = signedOld;
    let _e44 = atomicAdd((&values[_e36]), (_e40 + u32(_e41)));
    return;
}

@compute @workgroup_size(1, 1, 1) 
fn main() {
    main_1();
    return;
}
