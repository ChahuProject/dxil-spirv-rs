//! Internal model over a parsed rspirv module.
//!
//! This is a lightweight index of the type / decoration / variable /
//! access-chain information the pass needs, computed once from the parsed
//! module. It is deliberately minimal — only the constructs relevant to
//! stride-4 cbuffer views are captured; everything else is left untouched
//! and round-tripped verbatim.

use rspirv::dr::{Instruction, Module};
use rspirv::spirv::{Decoration, Op, StorageClass};
use std::collections::{HashMap, HashSet};

/// A single global-scope variable we care about.
#[derive(Debug, Clone, Copy)]
pub struct Variable {
    /// `OpVariable` result id.
    pub id: u32,
    /// The `OpTypePointer` id used by the variable.
    pub pointer_type: u32,
    /// The pointee type id (expected to be an `OpTypeStruct`).
    pub struct_type: u32,
    /// Storage class (`Uniform` or `StorageBuffer`).
    pub storage_class: StorageClass,
}

/// Index of everything the pass inspects.
#[derive(Debug, Default)]
pub struct ModuleInfo {
    /// `OpTypeStruct` id -> member type ids.
    pub struct_members: HashMap<u32, Vec<u32>>,
    /// `OpTypeArray` id -> (element type id, length constant id).
    pub array_info: HashMap<u32, (u32, u32)>,
    /// `OpTypeVector` id -> (component type id, component count).
    pub vector_info: HashMap<u32, (u32, u32)>,
    /// `OpTypeFloat` id -> bit width.
    pub float_width: HashMap<u32, u32>,
    /// `OpTypeInt` id -> (bit width, signedness).
    pub int_info: HashMap<u32, (u32, u32)>,
    /// `OpConstant` id -> 32-bit value (IdRef-form operands).
    pub constants: HashMap<u32, u32>,
    /// Array type id -> ArrayStride decoration value.
    pub array_strides: HashMap<u32, u32>,
    /// (struct id, member index) -> Offset decoration value.
    pub member_offsets: HashMap<(u32, u32), u32>,
    /// Struct ids decorated Block or BufferBlock.
    pub block_structs: HashSet<u32>,
    /// Variables in Uniform / StorageBuffer storage classes.
    pub variables: Vec<Variable>,
    /// Variable id -> (DescriptorSet, Binding) decorations.
    pub bindings: HashMap<u32, (Option<u32>, Option<u32>)>,
    /// `OpName` id -> name (debug / error reporting only).
    pub names: HashMap<u32, String>,
}

impl ModuleInfo {
    /// Returns the type id of the scalar base (float or int) if `type_id`
    /// is a 32-bit scalar, else `None`.
    pub fn scalar_base_32(&self, type_id: u32) -> Option<u32> {
        if self.float_width.get(&type_id) == Some(&32) {
            return Some(type_id);
        }
        if let Some((width, _)) = self.int_info.get(&type_id) {
            if *width == 32 {
                return Some(type_id);
            }
        }
        None
    }

    /// `true` if `type_id` is an `OpTypeVector` of 4 × 32-bit scalars.
    pub fn is_vec4_of_32(&self, type_id: u32) -> bool {
        let (comp, count) = match self.vector_info.get(&type_id) {
            Some(v) => *v,
            None => return false,
        };
        count == 4 && self.scalar_base_32(comp).is_some()
    }

    /// Variable name if any (fallback: numeric id).
    pub fn var_name(&self, var_id: u32) -> String {
        self.names
            .get(&var_id)
            .cloned()
            .unwrap_or_else(|| format!("#{var_id}"))
    }
}

/// Scalar kind of a 32-bit scalar type (for diagnostics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarKind {
    /// 32-bit IEEE-754 float.
    Float,
    /// 32-bit integer.
    Int,
}

/// Resolve the scalar kind of a 32-bit scalar type id.
pub fn scalar_kind(info: &ModuleInfo, type_id: u32) -> Option<ScalarKind> {
    if info.float_width.get(&type_id) == Some(&32) {
        Some(ScalarKind::Float)
    } else if let Some((width, _)) = info.int_info.get(&type_id) {
        if *width == 32 {
            Some(ScalarKind::Int)
        } else {
            None
        }
    } else {
        None
    }
}

/// Extracts the `IdRef` value of an operand, if the operand is an `IdRef`.
pub fn as_id_ref(op: &rspirv::dr::Operand) -> Option<u32> {
    match op {
        rspirv::dr::Operand::IdRef(id) => Some(*id),
        _ => None,
    }
}

/// Computes the number of times each result id is referenced across the whole
/// module (types, annotations, entry points, functions).
///
/// This is used to decide whether a type instruction is still live after the
/// rewrite (dead types are then removed).
pub fn reference_counts(module: &Module) -> HashMap<u32, u32> {
    let mut counts: HashMap<u32, u32> = HashMap::new();
    let mut record = |inst: &Instruction| {
        for op in &inst.operands {
            if let Some(id) = as_id_ref(op) {
                *counts.entry(id).or_insert(0) += 1;
            }
        }
    };
    for inst in module
        .capabilities
        .iter()
        .chain(&module.extensions)
        .chain(&module.ext_inst_imports)
        .chain(module.memory_model.iter())
        .chain(&module.entry_points)
        .chain(&module.execution_modes)
        .chain(&module.debug_string_source)
        .chain(&module.debug_names)
        .chain(&module.annotations)
        .chain(&module.types_global_values)
    {
        record(inst);
    }
    for func in &module.functions {
        if let Some(def) = &func.def {
            record(def);
        }
        for param in &func.parameters {
            record(param);
        }
        for block in &func.blocks {
            for inst in &block.instructions {
                record(inst);
            }
        }
        if let Some(end) = &func.end {
            record(end);
        }
    }
    counts
}

/// Builds the [`ModuleInfo`] index from a parsed module.
pub fn analyze(module: &Module) -> ModuleInfo {
    let mut info = ModuleInfo::default();

    for inst in &module.types_global_values {
        match inst.class.opcode {
            Op::TypeStruct => {
                let id = inst.result_id.expect("OpTypeStruct has result id");
                let members = inst
                    .operands
                    .iter()
                    .filter_map(as_id_ref)
                    .collect::<Vec<u32>>();
                info.struct_members.insert(id, members);
            }
            Op::TypeArray => {
                let id = inst.result_id.expect("OpTypeArray has result id");
                let elem = as_id_ref(inst.operands.first().unwrap());
                let len = as_id_ref(inst.operands.get(1).unwrap());
                if let (Some(elem), Some(len)) = (elem, len) {
                    info.array_info.insert(id, (elem, len));
                }
            }
            Op::TypeVector => {
                let id = inst.result_id.expect("OpTypeVector has result id");
                let comp = as_id_ref(inst.operands.first().unwrap());
                let count = inst.operands.get(1).and_then(|o| match o {
                    rspirv::dr::Operand::LiteralBit32(v) => Some(*v),
                    _ => None,
                });
                if let (Some(comp), Some(count)) = (comp, count) {
                    info.vector_info.insert(id, (comp, count));
                }
            }
            Op::TypeFloat => {
                let id = inst.result_id.expect("OpTypeFloat has result id");
                let width = inst.operands.first().and_then(|o| match o {
                    rspirv::dr::Operand::LiteralBit32(v) => Some(*v),
                    _ => None,
                });
                if let Some(width) = width {
                    info.float_width.insert(id, width);
                }
            }
            Op::TypeInt => {
                let id = inst.result_id.expect("OpTypeInt has result id");
                let width = inst.operands.first().and_then(|o| match o {
                    rspirv::dr::Operand::LiteralBit32(v) => Some(*v),
                    _ => None,
                });
                let signedness = inst.operands.get(1).and_then(|o| match o {
                    rspirv::dr::Operand::LiteralBit32(v) => Some(*v),
                    _ => None,
                });
                if let (Some(width), Some(signedness)) = (width, signedness) {
                    info.int_info.insert(id, (width, signedness));
                }
            }
            Op::Constant => {
                let id = inst.result_id.expect("OpConstant has result id");
                // 32-bit scalar constants are stored as a single IdRef-form
                // literal in rspirv's representation.
                if let Some(v) = as_id_ref(
                    inst.operands
                        .first()
                        .unwrap_or(&rspirv::dr::Operand::LiteralBit32(0)),
                ) {
                    info.constants.insert(id, v);
                } else if let rspirv::dr::Operand::LiteralBit32(v) = inst.operands.first().unwrap()
                {
                    info.constants.insert(id, *v);
                }
            }
            Op::Variable => {
                let id = inst.result_id.expect("OpVariable has result id");
                let pointer_type = inst.result_type.expect("OpVariable has result type");
                let storage_class = inst.operands.first().and_then(|o| match o {
                    rspirv::dr::Operand::StorageClass(sc) => Some(*sc),
                    _ => None,
                });
                if let Some(sc) = storage_class {
                    if sc == StorageClass::Uniform || sc == StorageClass::StorageBuffer {
                        // resolve pointee through OpTypePointer
                        if let Some(pointee) = module
                            .types_global_values
                            .iter()
                            .find(|t| {
                                t.class.opcode == Op::TypePointer
                                    && t.result_id == Some(pointer_type)
                            })
                            .and_then(|t| as_id_ref(t.operands.last().unwrap()))
                        {
                            info.variables.push(Variable {
                                id,
                                pointer_type,
                                struct_type: pointee,
                                storage_class: sc,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    for inst in &module.annotations {
        match inst.class.opcode {
            Op::Decorate => {
                let target = as_id_ref(inst.operands.first().unwrap());
                let (Some(target), Some(dec)) = (target, inst.operands.get(1)) else {
                    continue;
                };
                let dec = match dec {
                    rspirv::dr::Operand::Decoration(d) => *d,
                    _ => continue,
                };
                match dec {
                    Decoration::Block => {
                        info.block_structs.insert(target);
                    }
                    Decoration::BufferBlock => {
                        info.block_structs.insert(target);
                    }
                    Decoration::ArrayStride => {
                        if let rspirv::dr::Operand::LiteralBit32(v) = inst.operands.get(2).unwrap()
                        {
                            info.array_strides.insert(target, *v);
                        }
                    }
                    Decoration::DescriptorSet => {
                        if let rspirv::dr::Operand::LiteralBit32(v) = inst.operands.get(2).unwrap()
                        {
                            let e = info.bindings.entry(target).or_insert((None, None));
                            e.0 = Some(*v);
                        }
                    }
                    Decoration::Binding => {
                        if let rspirv::dr::Operand::LiteralBit32(v) = inst.operands.get(2).unwrap()
                        {
                            let e = info.bindings.entry(target).or_insert((None, None));
                            e.1 = Some(*v);
                        }
                    }
                    _ => {}
                }
            }
            Op::MemberDecorate => {
                let struct_id = as_id_ref(inst.operands.first().unwrap());
                let member = inst.operands.get(1).and_then(|o| match o {
                    rspirv::dr::Operand::LiteralBit32(v) => Some(*v),
                    _ => None,
                });
                let dec = inst.operands.get(2).and_then(|o| match o {
                    rspirv::dr::Operand::Decoration(d) => Some(*d),
                    _ => None,
                });
                if let (Some(struct_id), Some(member), Some(Decoration::Offset)) =
                    (struct_id, member, dec)
                {
                    if let rspirv::dr::Operand::LiteralBit32(v) = inst.operands.get(3).unwrap() {
                        info.member_offsets.insert((struct_id, member), *v);
                    }
                }
            }
            _ => {}
        }
    }

    for inst in &module.debug_names {
        if inst.class.opcode == Op::Name {
            let target = as_id_ref(inst.operands.first().unwrap());
            if let (Some(target), Some(rspirv::dr::Operand::LiteralString(name))) =
                (target, inst.operands.get(1))
            {
                info.names.insert(target, name.clone());
            }
        }
    }

    info
}
