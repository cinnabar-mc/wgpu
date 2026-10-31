struct Params {
    a: f32,
    lo: array<f32, 6>,
    b: array<vec2<f32>, 2>,
}

struct FragmentOutput {
    @location(0) o: vec4<f32>,
}

var<private> x_1: f32;
var<private> o: vec4<f32>;

fn main_1() {
    var c: Params = Params(1f, array<f32, 6>(1f, 2f, 3f, 4f, 5f, 6f), array<vec2<f32>, 2>(vec2(1f), vec2(2f)));
    var lo: array<f32, 6>;
    var p: Params;

    let _e17 = x_1;
    lo = array<f32, 6>(_e17, 2f, 3f, 4f, 5f, 6f);
    let _e25 = x_1;
    let _e26 = lo;
    let _e27 = x_1;
    p = Params(_e25, _e26, array<vec2<f32>, 2>(vec2(_e27), vec2(2f)));
    let _e36 = c.lo[2];
    let _e39 = p.lo[3];
    let _e43 = p.b[1];
    let _e45 = c.a;
    o = vec4<f32>((_e36 + _e39), _e43.x, _e43.y, _e45);
    return;
}

@fragment 
fn main(@location(0) x: f32) -> FragmentOutput {
    x_1 = x;
    main_1();
    let _e3 = o;
    return FragmentOutput(_e3);
}
