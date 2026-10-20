struct Mat4x3_ {
    mx: vec4<f32>,
    my: vec4<f32>,
    mz: vec4<f32>,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

var<private> o_color: vec4<f32>;

fn Fma(d: ptr<function, Mat4x3_>, m: Mat4x3_, s: f32) {
    var m_1: Mat4x3_;
    var s_1: f32;

    m_1 = m;
    s_1 = s;
    let _e7 = (*d).mx;
    let _e9 = m_1.mx;
    let _e10 = s_1;
    (*d).mx = (_e7 + (_e9 * _e10));
    let _e15 = (*d).my;
    let _e17 = m_1.my;
    let _e18 = s_1;
    (*d).my = (_e15 + (_e17 * _e18));
    let _e23 = (*d).mz;
    let _e25 = m_1.mz;
    let _e26 = s_1;
    (*d).mz = (_e23 + (_e25 * _e26));
    return;
}

fn main_1() {
    var m1_: Mat4x3_ = Mat4x3_(vec4(0f), vec4(1f), vec4(2f));
    var m2_: Mat4x3_ = Mat4x3_(vec4(0f), vec4(1f), vec4(2f));

    let _e17 = m2_;
    Fma((&m1_), _e17, 2f);
    o_color.x = 1f;
    o_color.y = 1f;
    o_color.z = 1f;
    o_color.w = 1f;
    return;
}

@fragment 
fn main() -> FragmentOutput {
    main_1();
    let _e1 = o_color;
    return FragmentOutput(_e1);
}
