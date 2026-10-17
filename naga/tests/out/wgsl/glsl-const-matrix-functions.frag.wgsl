struct Inputs {
    xyz: vec3<f32>,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

const TO_XYZ: mat3x3<f32> = mat3x3<f32>(vec3<f32>(1.9393939f, 0.5f, 2.5000002f), vec3<f32>(1f, 1f, 1f), vec3<f32>(0.090909086f, 0.16666666f, 13.166667f));
const WHITE: vec3<f32> = vec3<f32>(0.212639f, 0.7151687f, 0.07219232f);
const FROM_XYZ: mat3x3<f32> = mat3x3<f32>(vec3<f32>(3.2409704f, -1.5373833f, -0.49861085f), vec3<f32>(-0.96924365f, 1.8759675f, 0.04155508f), vec3<f32>(0.055630073f, -0.20397693f, 1.0569714f));
const T: mat2x3<f32> = mat2x3<f32>(vec3<f32>(1f, 3f, 5f), vec3<f32>(2f, 4f, 6f));
const DET: f32 = 5f;
const TINT: vec3<f32> = vec3<f32>(0.5f, 0.25f, 0.125f);
const EDGE: f32 = 0.15625f;

var<private> o_color: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: Inputs;

fn main_1() {
    let _e5 = global.xyz;
    let _e16 = (((FROM_XYZ * _e5) + vec3<f32>(1f, 3f, 5f)) + vec3<f32>(2.5f, 1.25f, 0.625f));
    o_color = vec4<f32>(_e16.x, _e16.y, _e16.z, EDGE);
    return;
}

@fragment 
fn main() -> FragmentOutput {
    main_1();
    let _e1 = o_color;
    return FragmentOutput(_e1);
}
