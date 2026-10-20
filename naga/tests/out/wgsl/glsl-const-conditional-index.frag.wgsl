struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

const GRADE: vec3<f32> = vec3<f32>(1f, 0.5f, 0.25f);
const TARGET: vec3<f32> = vec3<f32>(0.8728715f, 0.43643576f, 0.21821788f);
const PICK: f32 = 2f;
const M: mat2x2<f32> = mat2x2<f32>(vec2<f32>(1f, 2f), vec2<f32>(3f, 4f));
const COLUMN: vec2<f32> = vec2<f32>(3f, 4f);

var<private> o_color: vec4<f32>;

fn main_1() {
    o_color = vec4<f32>(1.745743f, 0.8728715f, 0.43643576f, 4f);
    return;
}

@fragment 
fn main() -> FragmentOutput {
    main_1();
    let _e1 = o_color;
    return FragmentOutput(_e1);
}
