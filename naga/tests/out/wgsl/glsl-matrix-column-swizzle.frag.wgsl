struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

var<private> o_color: vec4<f32>;
var<private> v_pos_1: vec3<f32>;

fn main_1() {
    var m: mat4x4<f32> = mat4x4<f32>(vec4<f32>(1f, 0f, 0f, 0f), vec4<f32>(0f, 1f, 0f, 0f), vec4<f32>(0f, 0f, 1f, 0f), vec4<f32>(0f, 0f, 0f, 1f));
    var c: vec2<f32> = vec2(1f);

    let _e12 = v_pos_1;
    m[0i][0] = _e12.x;
    m[0i][1] = _e12.y;
    m[0i][2] = _e12.z;
    m[3i][3] = 2f;
    let _e26 = m[0];
    let _e27 = _e26.xy;
    m[1i][2] = _e27.x;
    m[1i][0] = _e27.y;
    let _e38 = c.y;
    c[0i] = (_e38 * 3f);
    let _e42 = m[0];
    let _e45 = m[1];
    let _e47 = (_e42.xyz + _e45.zyx);
    let _e49 = m[3];
    let _e52 = c.x;
    o_color = vec4<f32>(_e47.x, _e47.y, _e47.z, (_e49.w + _e52));
    return;
}

@fragment 
fn main(@location(0) v_pos: vec3<f32>) -> FragmentOutput {
    v_pos_1 = v_pos;
    main_1();
    let _e3 = o_color;
    return FragmentOutput(_e3);
}
