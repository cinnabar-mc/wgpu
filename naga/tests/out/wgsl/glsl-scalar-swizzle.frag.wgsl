struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

var<private> o_color: vec4<f32>;
var<private> v_depth_1: f32;

fn main_1() {
    var d: f32;

    let _e2 = v_depth_1;
    d = _e2;
    let _e4 = d;
    d = (_e4 * 2f);
    let _e7 = d;
    let _e8 = vec2(_e7);
    let _e9 = d;
    let _e10 = d;
    o_color = vec4<f32>(_e8.x, _e8.y, _e9, _e10);
    let _e14 = v_depth_1;
    let _e15 = vec3(_e14);
    o_color.y = _e15.x;
    o_color.z = _e15.y;
    o_color.w = _e15.z;
    return;
}

@fragment 
fn main(@location(0) v_depth: f32) -> FragmentOutput {
    v_depth_1 = v_depth;
    main_1();
    let _e3 = o_color;
    return FragmentOutput(_e3);
}
