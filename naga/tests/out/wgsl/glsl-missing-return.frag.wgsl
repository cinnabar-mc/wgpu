struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

var<private> o_color: vec4<f32>;
var<private> v_uv_1: vec2<f32>;

fn effect(uv: vec2<f32>) -> f32 {
    var uv_1: vec2<f32>;

    uv_1 = uv;
    let _e2 = uv_1;
    if (_e2.x > 0.5f) {
        {
            let _e6 = uv_1;
            return _e6.y;
        }
    } else {
        return f32();
    }
}

fn main_1() {
    let _e2 = v_uv_1;
    let _e3 = effect(_e2);
    o_color = vec4(_e3);
    return;
}

@fragment 
fn main(@location(0) v_uv: vec2<f32>) -> FragmentOutput {
    v_uv_1 = v_uv;
    main_1();
    let _e3 = o_color;
    return FragmentOutput(_e3);
}
