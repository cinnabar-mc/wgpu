//! Source access contracts for physically native image arguments.

use super::{Configuration, Frontend, OpaqueType, Options, ParseErrors};
use crate::{
    ImageClass, ImageDimension, Module, ShaderStage, StorageAccess, StorageFormat, TypeInner,
};
use alloc::string::String;

/// Parses a fragment whose image alias retains a read-write native representation.
fn parse(source: &str) -> Result<Module, ParseErrors> {
    Frontend::default().parse_with_configuration(
        &Options::from(ShaderStage::Fragment),
        source,
        &Configuration {
            opaque_types: alloc::vec![OpaqueType {
                name: "Image".into(),
                logical_name: "sourceImage".into(),
                inner: TypeInner::Image {
                    dim: ImageDimension::D2,
                    arrayed: false,
                    class: ImageClass::Storage {
                        format: StorageFormat::Rgba8Unorm,
                        access: StorageAccess::LOAD | StorageAccess::STORE,
                    },
                },
            }],
            ..Configuration::default()
        },
    )
}

#[test]
fn source_image_access_preserves_native_ir_and_prototypes() {
    let source = r#"#version 450
        layout(binding=0,rgba8) uniform Image source;
        layout(location=0) out vec4 color;
        vec4 read(readonly Image s);
        void write(writeonly Image s);
        vec4 read(readonly Image s) { return imageLoad(s,ivec2(0)); }
        void write(writeonly Image s) { imageStore(s,ivec2(0),vec4(1)); }
        vec4 nestedRead(const in readonly Image s) { return read(s); }
        void nestedWrite(in writeonly Image s) { write(s); }
        vec4 imageStore(readonly Image s,float x) { return read(s); }
        void main() { nestedWrite(source); color=nestedRead(source)+imageStore(source,0.5); }
    "#;
    let module = parse(source).unwrap_or_else(|error| panic!("{}", error.emit_to_string(source)));
    crate::valid::Validator::new(
        crate::valid::ValidationFlags::all(),
        crate::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
    for (_, function) in module.functions.iter() {
        for argument in &function.arguments {
            if let TypeInner::Image {
                class: ImageClass::Storage { format, access },
                ..
            } = module.types[argument.ty].inner
            {
                assert_eq!(format, StorageFormat::Rgba8Unorm);
                assert_eq!(access, StorageAccess::LOAD | StorageAccess::STORE);
            }
        }
    }
}

#[test]
fn source_image_access_rejects_direct_and_nested_widening() {
    for (qualifier, operation) in [
        ("readonly", "imageStore(s,ivec2(0),vec4(1));"),
        ("writeonly", "color=imageLoad(s,ivec2(0));"),
    ] {
        for nested in [false, true] {
            let (leaf, body): (String, String) = if nested {
                (
                    alloc::format!("void leaf(Image s) {{ {operation} }}"),
                    "leaf(s);".into(),
                )
            } else {
                (String::new(), operation.into())
            };
            let source = alloc::format!(
                "#version 450\nlayout(binding=0,rgba8) uniform Image source;\nlayout(location=0) out vec4 color;\n{leaf}\nvoid restricted({qualifier} Image s) {{ {body} }}\nvoid main() {{ restricted(source); }}"
            );
            let error = parse(&source).unwrap_err().emit_to_string(&source);
            assert!(error.contains("image needs"), "{error}");
        }
    }
}

#[test]
fn source_image_access_prototypes_reject_changed_qualifiers() {
    for (declaration, definition) in [
        ("readonly", "writeonly"),
        ("writeonly", "readonly"),
        ("restrict readonly", "readonly"),
        ("const readonly", "readonly"),
    ] {
        let source = alloc::format!(
            "#version 450\nvoid restricted({declaration} Image);\nvoid restricted({definition} Image s) {{}}\nvoid main() {{}}"
        );
        let error = parse(&source).unwrap_err().emit_to_string(&source);
        assert!(error.contains("qualifiers must match"), "{error}");
    }
}

#[test]
fn source_image_parameter_qualifiers_reject_repetition() {
    for qualifiers in [
        "restrict restrict",
        "readonly readonly",
        "writeonly writeonly",
        "const const",
    ] {
        let source = alloc::format!(
            "#version 450\nvoid helper({qualifiers} Image s) {{}}\nvoid main() {{}}"
        );
        let errors = parse(&source).unwrap_err();
        let keyword = qualifiers.split_whitespace().last().unwrap();
        let start = source.rfind(keyword).unwrap();
        assert!(
            errors
                .errors
                .iter()
                .any(|error| error.meta.to_range() == Some(start..start + keyword.len())),
            "{errors}"
        );
    }
}

#[test]
fn source_image_parameter_qualifiers_allow_distinct_orders_and_query_only_access() {
    for qualifiers in [
        "restrict readonly writeonly const in",
        "in const writeonly readonly restrict",
    ] {
        let source = alloc::format!(
            "#version 450\nlayout(binding=0,rgba8) uniform Image source;\nvoid query({qualifiers} Image s) {{ ivec2 size=imageSize(s); }}\nvoid main() {{ query(source); }}"
        );
        let module =
            parse(&source).unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
        crate::valid::Validator::new(
            crate::valid::ValidationFlags::all(),
            crate::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
}
