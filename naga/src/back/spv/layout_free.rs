//! Separate logical function/private allocations from host interface layouts.

use alloc::{collections::BTreeMap, vec::Vec};
use spirv::{Op, StorageClass, Word};

use super::Error;

struct Types {
    declarations: BTreeMap<Word, Vec<Word>>,
    constants: BTreeMap<Word, Vec<Word>>,
    logical: BTreeMap<Word, Word>,
    constant_copies: BTreeMap<(Word, Word), Word>,
    next_id: Word,
}

impl Types {
    /// Allocate a result ID beyond every ID in the original module.
    const fn id(&mut self) -> Word {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Clone aggregate types without copying their memory-layout annotations.
    fn logical_type(&mut self, id: Word, output: &mut Vec<Word>) -> Word {
        if let Some(&copy) = self.logical.get(&id) {
            return copy;
        }
        let Some(mut declaration) = self.declarations.get(&id).cloned() else {
            return id;
        };
        let op = declaration[0] & 0xffff;
        if op != Op::TypeArray as Word && op != Op::TypeStruct as Word {
            return id;
        }
        let copy = self.id();
        self.logical.insert(id, copy);
        declaration[1] = copy;
        let end = if op == Op::TypeArray as Word {
            3
        } else {
            declaration.len()
        };
        for child in &mut declaration[2..end] {
            *child = self.logical_type(*child, output);
        }
        output.extend(declaration);
        copy
    }

    /// Rebuild constant initializers with the allocation's logical aggregate type.
    fn logical_constant(
        &mut self,
        id: Word,
        ty: Word,
        output: &mut Vec<Word>,
    ) -> Result<Word, Error> {
        if let Some(&copy) = self.constant_copies.get(&(id, ty)) {
            return Ok(copy);
        }
        let Some(mut constant) = self.constants.get(&id).cloned() else {
            return Err(Error::Validation("missing aggregate variable initializer"));
        };
        if constant[1] == ty {
            return Ok(id);
        }
        let original_type = self.declarations[&constant[1]].clone();
        let op = constant[0] & 0xffff;
        if op == Op::ConstantComposite as Word {
            for (index, child) in constant[3..].iter_mut().enumerate() {
                let child_type = if original_type[0] & 0xffff == Op::TypeArray as Word {
                    original_type[2]
                } else {
                    original_type[index + 2]
                };
                if let Some(&logical) = self.logical.get(&child_type) {
                    *child = self.logical_constant(*child, logical, output)?;
                }
            }
        } else if op != Op::ConstantNull as Word {
            return Err(Error::Validation(
                "unsupported aggregate variable initializer",
            ));
        }
        let copy = self.id();
        constant[1] = ty;
        constant[2] = copy;
        output.extend(constant);
        self.constant_copies.insert((id, ty), copy);
        Ok(copy)
    }
}

#[cfg(all(test, feature = "wgsl-in"))]
mod tests {
    use super::*;
    use crate::back::spv::{Options, PipelineOptions, Writer};
    use alloc::{string::String, vec};

    const SOURCE: &str = r#"
struct Pair { a: u32, b: u32 }
@group(0) @binding(0) var<storage, read_write> pairs: array<Pair>;
var<private> table: array<Pair, 2> = array(Pair(1u, 2u), Pair(3u, 4u));
fn mutate(value: ptr<function, Pair>) { (*value).a += 1u; }
@compute @workgroup_size(1)
fn main() {
    var local = pairs[0];
    mutate(&local);
    var nested: array<Pair, 2> = table;
    nested[1].b += local.a;
    pairs[0] = nested[1];
    table = nested;
}
"#;

    #[test]
    fn function_private_and_interface_values_keep_valid_distinct_layouts() {
        let module = crate::front::wgsl::parse_str(SOURCE).unwrap();
        let info = crate::valid::Validator::new(
            crate::valid::ValidationFlags::all(),
            crate::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
        let pipeline = PipelineOptions {
            shader_stage: crate::ShaderStage::Compute,
            entry_point: "main".into(),
        };
        for version in [(1, 3), (1, 6)] {
            let options = Options {
                lang_version: version,
                ..Default::default()
            };
            let mut writer = Writer::new(&options).unwrap();
            let mut words = vec![0x12345678];
            writer
                .write(&module, &info, Some(&pipeline), &None, &mut words)
                .unwrap();
            assert_eq!(words.remove(0), 0x12345678);
            rspirv::dr::load_words(&words).unwrap();
            let validator = std::env::var_os("SPIRV_VAL").unwrap_or_else(|| "spirv-val".into());
            if std::process::Command::new(&validator)
                .arg("--version")
                .output()
                .is_err()
            {
                std::eprintln!("missing fixture: spirv-val for Vulkan layout validation");
                continue;
            }
            let mut child = std::process::Command::new(&validator)
                .args([
                    "--target-env",
                    if version == (1, 3) {
                        "vulkan1.1"
                    } else {
                        "vulkan1.3"
                    },
                ])
                .stdin(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(
                    &words
                        .iter()
                        .flat_map(|word| word.to_le_bytes())
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            let result = child.wait_with_output().unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
}

/// SPIR-V 1.5+ forbids explicit memory layouts on Function and Private variables.
/// Keep interface types intact and convert whole values at allocation boundaries.
pub(super) fn legalize(words: &mut Vec<Word>) -> Result<(), Error> {
    if words[1] < 0x0001_0500 {
        return Ok(());
    }
    let mut instructions = Vec::new();
    let mut offset = 5;
    while offset < words.len() {
        let count = (words[offset] >> 16) as usize;
        if count == 0 || count > words.len() - offset {
            return Err(Error::Validation("malformed generated SPIR-V instruction"));
        }
        instructions.push(words[offset..offset + count].to_vec());
        offset += count;
    }
    let mut types = Types {
        declarations: BTreeMap::new(),
        constants: BTreeMap::new(),
        logical: BTreeMap::new(),
        constant_copies: BTreeMap::new(),
        next_id: words[3],
    };
    let mut pointers = BTreeMap::new();
    for instruction in &instructions {
        match instruction[0] & 0xffff {
            op if op == Op::TypeArray as Word || op == Op::TypeStruct as Word => {
                types
                    .declarations
                    .insert(instruction[1], instruction.clone());
            }
            op if op == Op::TypePointer as Word => {
                pointers.insert(instruction[1], (instruction[2], instruction[3]));
            }
            op if op == Op::ConstantNull as Word || op == Op::ConstantComposite as Word => {
                types.constants.insert(instruction[2], instruction.clone());
            }
            _ => {}
        }
    }
    let mut pointer_values = BTreeMap::new();
    for instruction in &instructions {
        if instruction.len() < 3 || !pointers.contains_key(&instruction[1]) {
            continue;
        }
        match instruction[0] & 0xffff {
            op if op == Op::Name as Word
                || op == Op::Decorate as Word
                || op == Op::DecorateId as Word
                || op == Op::TypePointer as Word
                || op == Op::TypeForwardPointer as Word => {}
            _ => {
                pointer_values.insert(instruction[2], instruction[1]);
            }
        }
    }
    let mut output = words[..5].to_vec();
    let mut logical_pointers = BTreeMap::new();
    let mut initialized_functions = false;
    for mut instruction in instructions.iter().cloned() {
        let op = instruction[0] & 0xffff;
        if op == Op::TypePointer as Word {
            let (_, original) = pointers[&instruction[1]];
            if instruction[2] == StorageClass::Function as Word
                || instruction[2] == StorageClass::Private as Word
            {
                let logical = types.logical_type(original, &mut output);
                if logical != original {
                    instruction[3] = logical;
                    logical_pointers.insert(instruction[1], (original, logical));
                }
            }
        }
        if op == Op::Function as Word && !initialized_functions {
            for variable in &instructions {
                if variable[0] & 0xffff == Op::Variable as Word && variable.len() == 5 {
                    if let Some(&(_, logical)) = logical_pointers.get(&variable[1]) {
                        types.logical_constant(variable[4], logical, &mut output)?;
                    }
                }
            }
            initialized_functions = true;
        }
        if op == Op::Variable as Word && instruction.len() == 5 {
            if let Some(&(_, logical)) = logical_pointers.get(&instruction[1]) {
                instruction[4] = types.logical_constant(instruction[4], logical, &mut output)?;
            }
        }
        if op == Op::Load as Word || op == Op::Store as Word {
            let pointer = instruction[if op == Op::Load as Word { 3 } else { 1 }];
            if let Some((original, logical)) = pointer_values
                .get(&pointer)
                .and_then(|pointer_type| logical_pointers.get(pointer_type))
                .copied()
            {
                let copy = types.id();
                if op == Op::Load as Word {
                    let result = instruction[2];
                    instruction[1] = logical;
                    instruction[2] = copy;
                    output.extend(instruction);
                    output.extend([(4 << 16) | Op::CopyLogical as Word, original, result, copy]);
                    continue;
                } else {
                    output.extend([
                        (4 << 16) | Op::CopyLogical as Word,
                        logical,
                        copy,
                        instruction[2],
                    ]);
                    instruction[2] = copy;
                }
            }
        }
        output.extend(instruction);
    }
    output[3] = types.next_id;
    *words = output;
    Ok(())
}
