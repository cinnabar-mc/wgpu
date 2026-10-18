//! Writer coverage for validated resource arguments and their access qualifiers.

use alloc::{format, string::String};

#[test]
fn resource_parameters_emit_with_native_access() {
    let module = crate::front::wgsl::parse_str(
        r#"
        @group(0) @binding(0) var source: texture_storage_2d<rgba8unorm, read>;
        @group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;
        @group(0) @binding(2) var mixed: texture_storage_2d<rgba8unorm, read_write>;
        @group(0) @binding(3) var unused: texture_storage_2d<rgba8unorm, write>;
        @group(0) @binding(4) var plane: texture_2d<f32>;
        @group(0) @binding(5) var filtering: sampler;
        @group(0) @binding(6) var unused_plane: texture_2d<f32>;
        @group(0) @binding(7) var unused_sampler: sampler;
        fn copy(src: texture_storage_2d<rgba8unorm, read>, dst: texture_storage_2d<rgba8unorm, write>) {
            textureStore(dst, vec2i(0), textureLoad(src, vec2i(0)));
        }
        fn update(image: texture_storage_2d<rgba8unorm, read_write>) {
            textureStore(image, vec2i(0), textureLoad(image, vec2i(0)));
        }
        fn nested(src: texture_storage_2d<rgba8unorm, read>, dst: texture_storage_2d<rgba8unorm, write>) {
            copy(src, dst);
        }
        fn read_sample(image: texture_2d<f32>, filtering: sampler) -> vec4f {
            return textureSampleLevel(image, filtering, vec2f(0.5), 0.0);
        }
        fn nested_sample(image: texture_2d<f32>, filtering: sampler) -> vec4f {
            return read_sample(image, filtering);
        }
        fn noop(image: texture_storage_2d<rgba8unorm, write>, plane: texture_2d<f32>, filtering: sampler) {}
        @compute @workgroup_size(1) fn main() {
            nested(source, output); update(mixed);
            textureStore(output, vec2i(0), nested_sample(plane, filtering));
            noop(unused, unused_plane, unused_sampler);
        }
    "#,
    )
    .unwrap();
    let info = crate::valid::Validator::new(
        crate::valid::ValidationFlags::all(),
        crate::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();

    #[cfg(msl_out)]
    {
        let (text, translation) = super::msl::write_string(
            &module,
            &info,
            &super::msl::Options {
                lang_version: (2, 0),
                ..super::msl::Options::default()
            },
            &super::msl::PipelineOptions::default(),
        )
        .unwrap();
        assert!(translation.entry_point_names.iter().all(Result::is_ok));
        for access in ["read", "write", "read_write"] {
            assert!(text.contains(&format!("metal::texture2d<float, metal::access::{access}>")));
        }
    }
    #[cfg(glsl_out)]
    {
        let mut text = String::new();
        let options = super::glsl::Options {
            version: super::glsl::Version::Desktop(450),
            ..super::glsl::Options::default()
        };
        super::glsl::Writer::new(
            &mut text,
            &module,
            &info,
            &options,
            &super::glsl::PipelineOptions {
                shader_stage: crate::ShaderStage::Compute,
                entry_point: "main".into(),
                multiview: None,
            },
            crate::proc::BoundsCheckPolicies::default(),
        )
        .unwrap()
        .write()
        .unwrap();
        for (name, parameters) in [
            ("copy", &["readonly image2D", "writeonly image2D"][..]),
            ("nested", &["readonly image2D", "writeonly image2D"][..]),
            ("update", &["image2D"][..]),
            ("noop", &["writeonly image2D", "sampler2D"][..]),
        ] {
            let prefix = format!("void {name}(");
            let signature = text.lines().find(|line| line.starts_with(&prefix)).unwrap();
            let parameter_types: alloc::vec::Vec<_> = signature[prefix.len()..]
                .split(')')
                .next()
                .unwrap()
                .split(", ")
                .map(|parameter| parameter.rsplit_once(' ').unwrap().0)
                .collect();
            assert_eq!(parameter_types, parameters, "{signature}");
        }
    }
    #[cfg(hlsl_out)]
    {
        let mut text = String::new();
        let translation = super::hlsl::Writer::new(
            &mut text,
            &super::hlsl::Options::default(),
            &super::hlsl::PipelineOptions::default(),
        )
        .write(&module, &info, None)
        .unwrap();
        assert!(translation.entry_point_names.iter().all(Result::is_ok));
    }
    #[cfg(spv_out)]
    {
        let words =
            super::spv::write_vec(&module, &info, &super::spv::Options::default(), None).unwrap();
        rspirv::dr::load_words(words).unwrap();
    }
    #[cfg(wgsl_out)]
    {
        let text =
            super::wgsl::write_string(&module, &info, super::wgsl::WriterFlags::empty()).unwrap();
        let reparsed = crate::front::wgsl::parse_str(&text).unwrap();
        crate::valid::Validator::new(
            crate::valid::ValidationFlags::all(),
            crate::valid::Capabilities::all(),
        )
        .validate(&reparsed)
        .unwrap();
    }
}
