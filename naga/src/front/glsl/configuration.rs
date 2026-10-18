//! Typed declarations supplied by source-language adapters.

use super::{Error, ErrorKind, Frontend, Result};
use crate::{Handle, Module, Span, Type, TypeInner, VectorSize};
use alloc::{string::String, vec::Vec};

/// Additional declarations whose physical types are already known by the caller.
#[derive(Clone, Debug, Default)]
pub struct Configuration {
    /// Native image and sampler types exposed under source-language identifiers.
    pub opaque_types: Vec<OpaqueType>,
    /// Source families that retain the selected native builtin's overloads.
    pub functions: Vec<FunctionAlias>,
}

/// A source opaque type with an explicit physical representation and logical identity.
#[derive(Clone, Debug)]
pub struct OpaqueType {
    /// Identifier recognized as a type name in this GLSL translation.
    pub name: String,
    /// Source identity used in addition to the complete native type during matching.
    pub logical_name: String,
    /// Only image and sampler types are accepted; these contain no arena handles.
    pub inner: TypeInner,
}

/// An independent function family that also receives a native builtin's candidates.
#[derive(Clone, Debug)]
pub struct FunctionAlias {
    /// Source family name used by declarations, prototypes and calls.
    pub name: String,
    /// Native builtin family that supplies additional candidates.
    pub builtin: String,
    /// Splat only a selected builtin result, preserving user overload return types.
    pub result_splat: Option<VectorSize>,
}

impl Frontend {
    /// Registers handle-free opaque aliases in this parse's type arena.
    pub(super) fn register_types(&mut self, module: &mut Module) -> Result<()> {
        for (index, alias) in self.configuration.functions.iter().enumerate() {
            if alias.name.is_empty()
                || alias.builtin.is_empty()
                || self
                    .function_aliases
                    .insert(alias.name.clone(), index)
                    .is_some()
            {
                return Err(Error {
                    kind: ErrorKind::SemanticError("invalid builtin family registration".into()),
                    meta: Span::default(),
                });
            }
        }
        for alias in &self.configuration.opaque_types {
            if alias.name.is_empty()
                || alias.logical_name.is_empty()
                || super::types::parse_type(&alias.name).is_some()
                || self.lookup_type.contains_key(&alias.name)
                || !matches!(
                    alias.inner,
                    TypeInner::Image { .. } | TypeInner::Sampler { .. }
                )
            {
                return Err(Error {
                    kind: ErrorKind::SemanticError("invalid opaque type registration".into()),
                    meta: Span::default(),
                });
            }
            let handle = module.types.insert(
                Type {
                    name: Some(alias.name.clone()),
                    inner: alias.inner.clone(),
                },
                Span::default(),
            );
            self.lookup_type.insert(alias.name.clone(), handle);
            self.logical_types
                .insert(alias.name.clone(), alias.logical_name.clone());
        }
        Ok(())
    }

    /// Looks up a validated builtin family without scanning unrelated source declarations.
    pub(super) fn builtin_alias(&self, name: &str) -> Option<&FunctionAlias> {
        self.function_aliases
            .get(name)
            .map(|&index| &self.configuration.functions[index])
    }

    /// Keeps distinct registered source identities out of one user overload signature.
    pub(super) fn same_logical_type(
        &self,
        module: &Module,
        a: Handle<Type>,
        b: Handle<Type>,
    ) -> bool {
        same_logical_type(&self.logical_types, module, a, b)
    }
}

/// Returns a registered source identity through native array and pointer shells.
pub(super) fn logical_type<'a>(
    types: &'a crate::FastHashMap<String, String>,
    module: &Module,
    mut ty: Handle<Type>,
) -> Option<&'a str> {
    loop {
        if let Some(name) = module.types[ty].name.as_ref() {
            if let Some(logical) = types.get(name) {
                return Some(logical);
            }
        }
        match module.types[ty].inner {
            TypeInner::Array { base, .. } | TypeInner::Pointer { base, .. } => ty = base,
            _ => return None,
        }
    }
}

/// Compares source identities before the caller checks physical type compatibility.
pub(super) fn same_logical_type(
    types: &crate::FastHashMap<String, String>,
    module: &Module,
    a: Handle<Type>,
    b: Handle<Type>,
) -> bool {
    logical_type(types, module, a) == logical_type(types, module, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ImageClass, ImageDimension, ScalarKind, ShaderStage, StorageAccess, StorageFormat,
    };

    /// Parses and validates a fragment shader with explicit source-language declarations.
    fn parse(source: &str, configuration: &Configuration) -> Module {
        let module = Frontend::default()
            .parse_with_configuration(
                &super::super::Options::from(ShaderStage::Fragment),
                source,
                configuration,
            )
            .unwrap_or_else(|error| panic!("{}", error.emit_to_string(source)));
        crate::valid::Validator::new(
            crate::valid::ValidationFlags::all(),
            crate::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
        module
    }

    /// Registers a two-dimensional image with a stable source identity.
    fn image(name: &str, logical_name: &str, class: ImageClass) -> OpaqueType {
        OpaqueType {
            name: name.into(),
            logical_name: logical_name.into(),
            inner: TypeInner::Image {
                dim: ImageDimension::D2,
                arrayed: false,
                class,
            },
        }
    }

    #[test]
    fn configured_opaque_types_keep_unused_logical_signatures() {
        let sampled = ImageClass::Sampled {
            kind: ScalarKind::Float,
            multi: false,
        };
        let configuration = Configuration {
            opaque_types: alloc::vec![
                image("Plane", "sampler2D", sampled),
                image("Rect", "sampler2DRect", sampled),
                image(
                    "Depth",
                    "sampler2DShadow",
                    ImageClass::Depth { multi: false }
                ),
            ],
            ..Configuration::default()
        };
        parse(
            r#"#version 450
            layout(binding=0) uniform Plane plane;
            layout(binding=1) uniform Rect rect;
            layout(binding=2) uniform Depth depth;
            float choose(Plane s) { return 1.0; }
            float choose(Rect s) { return 2.0; }
            float choose(Depth s) { return 3.0; }
            layout(location=0) out vec4 color;
            void main() { color=vec4(choose(plane)+choose(rect)+choose(depth)); }
        "#,
            &configuration,
        );
    }

    #[test]
    fn configured_storage_types_keep_builtin_and_user_value_candidates() {
        let configuration = Configuration {
            opaque_types: alloc::vec![image(
                "Image",
                "image2D",
                ImageClass::Storage {
                    format: StorageFormat::Rgba8Unorm,
                    access: StorageAccess::LOAD,
                }
            )],
            functions: alloc::vec![FunctionAlias {
                name: "family".into(),
                builtin: "imageLoad".into(),
                result_splat: None
            }],
        };
        let module = parse(
            r#"#version 450
            layout(binding=0,rgba8) readonly uniform Image source;
            vec4 family(Image s,float x) { return imageLoad(s,ivec2(x)); }
            layout(location=0) out vec4 color;
            void main() { color=family(source,ivec2(0))+family(source,0.5); }
        "#,
            &configuration,
        );
        assert!(module.types.iter().any(|(_, ty)| matches!(
            ty.inner,
            TypeInner::Image {
                class: ImageClass::Storage {
                    format: StorageFormat::Rgba8Unorm,
                    ..
                },
                ..
            }
        )));
    }

    #[test]
    fn configured_storage_writes_accept_native_image_parameters() {
        let configuration = Configuration {
            opaque_types: alloc::vec![image(
                "Image",
                "image2D",
                ImageClass::Storage {
                    format: StorageFormat::Rgba8Unorm,
                    access: StorageAccess::STORE,
                }
            )],
            functions: alloc::vec![FunctionAlias {
                name: "family".into(),
                builtin: "imageStore".into(),
                result_splat: None
            }],
        };
        parse(
            r#"#version 450
            layout(binding=0,rgba8) writeonly uniform Image target;
            void family(Image s,float x,vec4 value) { imageStore(s,ivec2(x),value); }
            layout(location=0) out vec4 color;
            void main() { family(target,ivec2(0),vec4(1.0)); family(target,0.5,vec4(0.5)); color=vec4(1.0); }
        "#,
            &configuration,
        );
    }

    #[test]
    fn configured_user_signatures_override_matching_builtin_candidates() {
        let configuration = Configuration {
            opaque_types: alloc::vec![image(
                "Image",
                "image2D",
                ImageClass::Storage {
                    format: StorageFormat::Rgba8Unorm,
                    access: StorageAccess::LOAD,
                }
            )],
            functions: alloc::vec![FunctionAlias {
                name: "family".into(),
                builtin: "imageLoad".into(),
                result_splat: None
            }],
        };
        for prototype in ["", "vec4 family(Image s,ivec2 p);"] {
            let source = alloc::format!(
                r#"#version 450
                layout(binding=0,rgba8) readonly uniform Image source;
                {prototype}
                vec4 family(Image s,ivec2 p) {{ return vec4(2.0); }}
                layout(location=0) out vec4 color;
                void main() {{ color=family(source,ivec2(0)); }}
            "#
            );
            let module = parse(&source, &configuration);
            assert!(!module
                .functions
                .iter()
                .any(
                    |(_, function)| function.expressions.iter().any(|(_, expression)| matches!(
                        expression,
                        &crate::Expression::ImageLoad { .. }
                    ))
                ));
        }
    }

    #[test]
    fn configured_user_signatures_override_builtin_conversion_ties() {
        let configuration = Configuration {
            opaque_types: alloc::vec![image(
                "Plane",
                "sampler2D",
                ImageClass::Sampled {
                    kind: ScalarKind::Float,
                    multi: false
                }
            )],
            functions: alloc::vec![FunctionAlias {
                name: "family".into(),
                builtin: "texture".into(),
                result_splat: None
            }],
        };
        let module = parse(
            r#"#version 450
            layout(binding=0) uniform Plane source;
            layout(binding=1) uniform sampler filtering;
            float family(Plane s,vec2 p) { return 2.0; }
            layout(location=0) out vec4 color;
            void main() { color=vec4(family(sampler2D(source,filtering),ivec2(0))); }
        "#,
            &configuration,
        );
        assert!(!module.functions.iter().any(|(_, function)| function
            .expressions
            .iter()
            .any(|(_, expression)| matches!(expression, &crate::Expression::ImageSample { .. }))));
    }

    #[test]
    fn configured_rejected_candidates_do_not_report_access_errors() {
        let configuration = Configuration {
            opaque_types: alloc::vec![image(
                "Image",
                "image2D",
                ImageClass::Storage {
                    format: StorageFormat::Rgba8Unorm,
                    access: StorageAccess::LOAD,
                }
            )],
            functions: alloc::vec![FunctionAlias {
                name: "family".into(),
                builtin: "imageStore".into(),
                result_splat: None
            }],
        };
        parse(
            r#"#version 450
            layout(binding=0,rgba8) readonly uniform Image source;
            void family(Image s,float x,vec4 value) {}
            layout(location=0) out vec4 color;
            void main() { family(source,0.5,vec4(1.0)); color=vec4(1.0); }
        "#,
            &configuration,
        );
    }

    #[test]
    fn configured_builtin_families_keep_independent_overloads_and_result_shapes() {
        let configuration = Configuration {
            functions: alloc::vec![
                FunctionAlias {
                    name: "first".into(),
                    builtin: "sin".into(),
                    result_splat: Some(VectorSize::Quad)
                },
                FunctionAlias {
                    name: "second".into(),
                    builtin: "sin".into(),
                    result_splat: None
                },
            ],
            ..Configuration::default()
        };
        parse(
            r#"#version 450
            float first(float x,float y) { return x+y; }
            float second(float x,float y) { return x-y; }
            layout(location=0) out vec4 color;
            void main() {
                float user=first(0.1,0.2)+second(0.1,0.2);
                color=first(0.5)+vec4(second(0.5)+user);
            }
        "#,
            &configuration,
        );
    }
}

#[cfg(test)]
mod shadow_tests {
    use super::*;
    use crate::{ImageClass, ImageDimension, ScalarKind, ShaderStage};

    #[test]
    fn configured_shadow_candidates_do_not_retype_regular_arguments() {
        let configuration = Configuration {
            opaque_types: alloc::vec![
                OpaqueType {
                    name: "Plane".into(),
                    logical_name: "sampler2D".into(),
                    inner: TypeInner::Image {
                        dim: ImageDimension::D2,
                        arrayed: false,
                        class: ImageClass::Sampled {
                            kind: ScalarKind::Float,
                            multi: false
                        },
                    }
                },
                OpaqueType {
                    name: "Depth".into(),
                    logical_name: "sampler2DShadow".into(),
                    inner: TypeInner::Image {
                        dim: ImageDimension::D2,
                        arrayed: false,
                        class: ImageClass::Depth { multi: false },
                    }
                },
            ],
            ..Configuration::default()
        };
        for shadow_first in [false, true] {
            let shadow = "float choose(Depth s) { return texture(sampler2DShadow(s,comparison),vec3(0.5)); }";
            let regular =
                "float choose(Plane s) { return texture(sampler2D(s,filtering),vec2(0.5)).x; }";
            let definitions = if shadow_first {
                alloc::format!("{shadow}{regular}")
            } else {
                alloc::format!("{regular}{shadow}")
            };
            let source = alloc::format!(
                r#"#version 450
                layout(binding=0) uniform Plane plane;
                layout(binding=1) uniform Depth depth;
                layout(binding=2) uniform sampler filtering;
                layout(binding=3) uniform samplerShadow comparison;
                {definitions}
                layout(location=0) out vec4 color;
                void main() {{ color=vec4(choose(plane)+choose(depth)); }}
            "#
            );
            let module = Frontend::default()
                .parse_with_configuration(
                    &super::super::Options::from(ShaderStage::Fragment),
                    &source,
                    &configuration,
                )
                .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
            crate::valid::Validator::new(
                crate::valid::ValidationFlags::all(),
                crate::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap();
        }
    }
}

#[cfg(test)]
mod regressions {
    use super::*;
    use crate::{
        ImageClass, ImageDimension, ScalarKind, ShaderStage, StorageAccess, StorageFormat,
    };

    /// Parses a configured fragment and returns validation data for its observable uses.
    fn parse(source: &str, configuration: &Configuration) -> (Module, crate::valid::ModuleInfo) {
        let module = Frontend::default()
            .parse_with_configuration(
                &super::super::Options::from(ShaderStage::Fragment),
                source,
                configuration,
            )
            .unwrap_or_else(|error| panic!("{}", error.emit_to_string(source)));
        let info = crate::valid::Validator::new(
            crate::valid::ValidationFlags::all(),
            crate::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
        (module, info)
    }

    #[test]
    fn configured_native_definitions_retain_builtin_candidates() {
        let configuration = Configuration {
            opaque_types: alloc::vec![OpaqueType {
                name: "Plane".into(),
                logical_name: "sourcePlane".into(),
                inner: TypeInner::Image {
                    dim: ImageDimension::D2,
                    arrayed: false,
                    class: ImageClass::Sampled {
                        kind: ScalarKind::Float,
                        multi: false
                    },
                },
            }],
            functions: alloc::vec![FunctionAlias {
                name: "family".into(),
                builtin: "texture".into(),
                result_splat: None,
            }],
        };
        for prototype in ["", "vec4 family(texture2D t,vec2 p);"] {
            let source = alloc::format!(
                r#"#version 450
                layout(binding=0) uniform Plane plane;
                layout(binding=1) uniform texture2D nativePlane;
                layout(binding=2) uniform sampler filtering;
                {prototype}
                vec4 family(texture2D t,vec2 p) {{ return vec4(2.0); }}
                layout(location=0) out vec4 color;
                void main() {{ color=family(nativePlane,vec2(0))+family(sampler2D(plane,filtering),vec2(0)); }}
            "#
            );
            let (module, _) = parse(&source, &configuration);
            assert!(module
                .functions
                .iter()
                .any(
                    |(_, function)| function.expressions.iter().any(|(_, expression)| matches!(
                        expression,
                        crate::Expression::ImageSample { .. }
                    ))
                ));
        }
    }

    #[test]
    fn configured_depth_overrides_win_native_conversion_ties() {
        let configuration = Configuration {
            opaque_types: alloc::vec![OpaqueType {
                name: "Tex".into(),
                logical_name: "sourceTexture".into(),
                inner: TypeInner::Image {
                    dim: ImageDimension::D2,
                    arrayed: false,
                    class: ImageClass::Sampled {
                        kind: ScalarKind::Float,
                        multi: false
                    },
                },
            }],
            functions: alloc::vec![FunctionAlias {
                name: "family".into(),
                builtin: "texture".into(),
                result_splat: None,
            }],
        };
        parse(
            r#"#version 450
            layout(binding=0) uniform Tex source;
            layout(binding=1) uniform samplerShadow comparison;
            vec4 family(Tex t,vec3 p) { return vec4(texture(sampler2DShadow(t,comparison),p)); }
            layout(location=0) out vec4 color;
            void main() { color=family(sampler2DShadow(source,comparison),ivec3(0)); }
        "#,
            &configuration,
        );
    }

    #[test]
    fn configured_opaque_overloads_match_complete_native_access() {
        let accesses = [
            StorageAccess::LOAD,
            StorageAccess::LOAD | StorageAccess::STORE,
            StorageAccess::STORE,
        ];
        let configuration = Configuration {
            opaque_types: ["Read", "ReadWrite", "Write"]
                .into_iter()
                .zip(accesses)
                .map(|(name, access)| OpaqueType {
                    name: name.into(),
                    logical_name: "image2D".into(),
                    inner: TypeInner::Image {
                        dim: ImageDimension::D2,
                        arrayed: false,
                        class: ImageClass::Storage {
                            format: StorageFormat::Rgba8Unorm,
                            access,
                        },
                    },
                })
                .collect(),
            ..Configuration::default()
        };
        for reverse in [false, true] {
            let mut definitions = [
                "float choose(Read s) { return 1.0; }",
                "float choose(ReadWrite s) { return 2.0; }",
                "float choose(Write s) { return 3.0; }",
            ];
            if reverse {
                definitions.reverse();
            }
            let definitions = definitions.concat();
            let source = alloc::format!(
                r#"#version 450
                layout(binding=0,rgba8) readonly uniform Read source;
                layout(binding=1,rgba8) uniform ReadWrite mixed;
                layout(binding=2,rgba8) writeonly uniform Write target;
                {definitions}
                layout(location=0) out vec4 color;
                void main() {{ color=vec4(choose(source)+choose(mixed)+choose(target)); }}
            "#
            );
            parse(&source, &configuration);
        }
    }

    #[test]
    fn configured_depth_promotion_retains_source_signatures() {
        for configured in [true, false] {
            let configuration = Configuration {
                opaque_types: if configured {
                    alloc::vec![OpaqueType {
                        name: "Tex".into(),
                        logical_name: "sourceTexture".into(),
                        inner: TypeInner::Image {
                            dim: ImageDimension::D2,
                            arrayed: false,
                            class: ImageClass::Sampled {
                                kind: ScalarKind::Float,
                                multi: false,
                            },
                        },
                    }]
                } else {
                    Vec::new()
                },
                ..Configuration::default()
            };
            let ty = if configured { "Tex" } else { "texture2D" };
            for prototype in [false, true] {
                let declaration = if prototype {
                    alloc::format!("float choose({ty});")
                } else {
                    String::new()
                };
                let source = alloc::format!(
                    r#"#version 450
                    layout(binding=0) uniform {ty} source;
                    layout(binding=1) uniform samplerShadow comparison;
                    {declaration}
                    float choose({ty} image) {{ return texture(sampler2DShadow(image,comparison),vec3(0.5)); }}
                    float nested({ty} image) {{ return choose(image); }}
                    layout(location=0) out vec4 color;
                    void main() {{ color=vec4(nested(source)+nested(source)+choose(source)); }}
                "#
                );
                let (module, _) = parse(&source, &configuration);
                for (_, function) in module.functions.iter() {
                    for argument in &function.arguments {
                        let argument_type = &module.types[argument.ty];
                        if matches!(argument_type.inner, TypeInner::Image { .. }) {
                            assert_eq!(argument_type.name.as_deref(), configured.then_some("Tex"));
                            assert!(matches!(
                                argument_type.inner,
                                TypeInner::Image {
                                    class: ImageClass::Depth { .. },
                                    ..
                                }
                            ));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn configured_opaque_overloads_distinguish_registered_and_native_types() {
        let configuration = Configuration {
            opaque_types: alloc::vec![OpaqueType {
                name: "Plane".into(),
                logical_name: "sampler2D".into(),
                inner: TypeInner::Image {
                    dim: ImageDimension::D2,
                    arrayed: false,
                    class: ImageClass::Sampled {
                        kind: ScalarKind::Float,
                        multi: false,
                    },
                },
            }],
            ..Configuration::default()
        };
        for registered_first in [false, true] {
            let registered = "float choose(Plane s) { return 1.0; }";
            let native = "float choose(texture2D s) { return 2.0; }";
            let definitions = if registered_first {
                alloc::format!("{registered}{native}")
            } else {
                alloc::format!("{native}{registered}")
            };
            let source = alloc::format!(
                r#"#version 450
                layout(binding=0) uniform Plane plane;
                layout(binding=1) uniform texture2D nativePlane;
                {definitions}
                layout(location=0) out vec4 color;
                void main() {{ color=vec4(choose(plane)+choose(nativePlane)); }}
            "#
            );
            let (module, _) = parse(&source, &configuration);
            let main = module
                .functions
                .iter()
                .find(|&(_, function)| function.name.as_deref() == Some("main"))
                .unwrap()
                .1;
            let parameter_names: Vec<_> = main
                .body
                .iter()
                .filter_map(|statement| match *statement {
                    crate::Statement::Call { function, .. } => {
                        let ty = module.functions[function].arguments[0].ty;
                        Some(module.types[ty].name.as_deref())
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(parameter_names, [Some("Plane"), None]);
        }
    }

    #[test]
    fn default_prototypes_override_native_conversion_ties() {
        for prototype in ["float sin(float x);", "float sin(float);"] {
            let source = alloc::format!(
                r#"#version 450
                {prototype}
                float sin(float x) {{ return x; }}
                layout(location=0) out vec4 color;
                void main() {{ color=vec4(sin(1)); }}
            "#
            );
            let (module, _) = parse(&source, &Configuration::default());
            assert!(!module
                .functions
                .iter()
                .any(
                    |(_, function)| function.expressions.iter().any(|(_, expression)| matches!(
                        expression,
                        crate::Expression::Math {
                            fun: crate::MathFunction::Sin,
                            ..
                        }
                    ))
                ));
        }
    }

    #[test]
    fn configured_unused_resource_arguments_keep_query_without_reads() {
        let configuration = Configuration {
            opaque_types: alloc::vec![
                OpaqueType {
                    name: "Plane".into(),
                    logical_name: "sampler2D".into(),
                    inner: TypeInner::Image {
                        dim: ImageDimension::D2,
                        arrayed: false,
                        class: ImageClass::Sampled {
                            kind: ScalarKind::Float,
                            multi: false
                        },
                    }
                },
                OpaqueType {
                    name: "Output".into(),
                    logical_name: "image2D".into(),
                    inner: TypeInner::Image {
                        dim: ImageDimension::D2,
                        arrayed: false,
                        class: ImageClass::Storage {
                            format: StorageFormat::Rgba8Unorm,
                            access: StorageAccess::STORE
                        },
                    }
                },
            ],
            ..Configuration::default()
        };
        let (module, info) = parse(
            r#"#version 450
            layout(binding=0) uniform Plane source;
            layout(binding=1,rgba8) writeonly uniform Output outputImage;
            layout(binding=2) uniform sampler filtering;
            void noop(Plane a,Output b,sampler c) {}
            layout(location=0) out vec4 color;
            void main() { noop(source,outputImage,filtering); color=vec4(1.0); }
        "#,
            &configuration,
        );
        for (handle, global) in module.global_variables.iter() {
            if global.binding.is_some() {
                assert_eq!(
                    info.get_entry_point(0)[handle],
                    crate::valid::GlobalUse::QUERY
                );
            }
        }
    }

    #[test]
    fn configured_rejected_shadow_candidates_do_not_retype_call_arguments() {
        let configuration = Configuration {
            opaque_types: alloc::vec![OpaqueType {
                name: "Unused".into(),
                logical_name: "sampler2D".into(),
                inner: TypeInner::Image {
                    dim: ImageDimension::D2,
                    arrayed: false,
                    class: ImageClass::Sampled {
                        kind: ScalarKind::Float,
                        multi: false
                    },
                },
            }],
            ..Configuration::default()
        };
        parse(
            r#"#version 450
            layout(binding=0) uniform texture2D source;
            layout(binding=3) uniform texture2D depthSource;
            layout(binding=1) uniform sampler filtering;
            layout(binding=2) uniform samplerShadow comparison;
            float choose(texture2D s,vec2 p) { return texture(sampler2D(s,filtering),p).x; }
            float choose(texture2D s,float p) { return texture(sampler2DShadow(s,comparison),vec3(p)); }
            layout(location=0) out vec4 color;
            void main() { color=vec4(choose(source,vec2(0.5))+choose(depthSource,0.5)); }
        "#,
            &configuration,
        );
    }

    #[test]
    fn configured_overload_identity_uses_constant_expression_types() {
        let source = r#"#version 450
            vec2 sin(vec2 x) { return x; }
            const float c = sin(1);
            layout(location=0) out vec4 color;
            void main() { color=vec4(c); }
        "#;
        parse(source, &Configuration::default());
        let configuration = Configuration {
            opaque_types: alloc::vec![OpaqueType {
                name: "Unused".into(),
                logical_name: "sampler2D".into(),
                inner: TypeInner::Image {
                    dim: ImageDimension::D2,
                    arrayed: false,
                    class: ImageClass::Sampled {
                        kind: ScalarKind::Float,
                        multi: false
                    },
                },
            }],
            ..Configuration::default()
        };
        parse(source, &configuration);
    }

    #[test]
    fn configured_scalar_prototypes_can_override_native_signatures() {
        let configuration = Configuration {
            functions: alloc::vec![FunctionAlias {
                name: "family".into(),
                builtin: "sin".into(),
                result_splat: None
            }],
            ..Configuration::default()
        };
        for prototype in ["float family(float x);", "float family(float);"] {
            let source = alloc::format!(
                r#"#version 450
                {prototype}
                float family(float x) {{ return x; }}
                layout(location=0) out vec4 color;
                void main() {{ color=vec4(family(0.5)); }}
            "#
            );
            parse(&source, &configuration);
        }
    }
}

#[cfg(test)]
mod unnamed_prototype_tests {
    use super::*;
    use crate::{ImageClass, ImageDimension, ScalarKind, ShaderStage};

    #[test]
    fn configured_unnamed_prototypes_retain_distinct_opaque_types() {
        let configuration = Configuration {
            opaque_types: alloc::vec![
                OpaqueType {
                    name: "Plane".into(),
                    logical_name: "sampler2D".into(),
                    inner: TypeInner::Image {
                        dim: ImageDimension::D2,
                        arrayed: false,
                        class: ImageClass::Sampled {
                            kind: ScalarKind::Float,
                            multi: false
                        },
                    }
                },
                OpaqueType {
                    name: "Depth".into(),
                    logical_name: "sampler2DShadow".into(),
                    inner: TypeInner::Image {
                        dim: ImageDimension::D2,
                        arrayed: false,
                        class: ImageClass::Depth { multi: false },
                    }
                },
            ],
            ..Configuration::default()
        };
        let source = r#"#version 450
            layout(binding=0) uniform Plane plane;
            layout(binding=1) uniform Depth depth;
            float choose(Plane);
            float choose(Depth);
            float choose(Plane s) { return 1.0; }
            float choose(Depth s) { return 2.0; }
            layout(location=0) out vec4 color;
            void main() { color=vec4(choose(plane)+choose(depth)); }
        "#;
        let module = Frontend::default()
            .parse_with_configuration(
                &super::super::Options::from(ShaderStage::Fragment),
                source,
                &configuration,
            )
            .unwrap_or_else(|error| panic!("{}", error.emit_to_string(source)));
        crate::valid::Validator::new(
            crate::valid::ValidationFlags::all(),
            crate::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
}
