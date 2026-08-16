//! NON-UPSTREAM extension unit tests for `hlsl_compat::vec4_align_cbuffers`.
//!
//! These construct SPIR-V modules with the rspirv builder (no C++ conversion
//! involved) and verify the pass behaviour:
//!
//! * merging a scalar view into its vec4 alias (dual view),
//! * in-place rewrite of a lone scalar view (static index splitting),
//! * idempotent no-op on clean modules,
//! * skipping views that cannot be padded,
//! * rejecting non-SPIR-V input.
//!
//! Run with:
//! ```text
//! cargo test -p dxil-spirv --features non-upstream-hlsl-compat --test non_upstream_hlsl_compat
//! ```

#![cfg(feature = "non-upstream-hlsl-compat")]

use std::collections::HashMap;

use dxil_spirv::non_upstream::hlsl_compat::{self, ir as hc_ir};
use rspirv::binary::Assemble;
use rspirv::dr::{Builder, Operand};
use rspirv::spirv::{AddressingModel, Decoration, FunctionControl, MemoryModel, Op, StorageClass};

struct ModuleIds {
    var_scalar: u32,
    var_vec4: u32,
}

/// Builds a module with the dual-view pattern:
///
/// * `var_scalar`: Uniform block `struct { float[8] (stride 4) }`, binding (0,0)
/// * `var_vec4`:   Uniform block `struct { float4[2] (stride 16) }`, binding (0,0)
/// * one function that loads `var_scalar[0][param]` (dynamic index).
fn build_dual_view() -> (Vec<u32>, ModuleIds) {
    let mut b = Builder::new();
    b.memory_model(AddressingModel::Logical, MemoryModel::GLSL450);

    let void = b.type_void();
    let f32 = b.type_float(32, None);
    let u32 = b.type_int(32, 0);
    let c0 = b.constant_bit32(u32, 0);
    let c2 = b.constant_bit32(u32, 2);
    let c8 = b.constant_bit32(u32, 8);

    let vec4 = b.type_vector(f32, 4);
    let arr_scalar = b.type_array(f32, c8);
    let struct_scalar = b.type_struct(vec![arr_scalar]);
    let ptr_scalar = b.type_pointer(None, StorageClass::Uniform, struct_scalar);
    let arr_vec4 = b.type_array(vec4, c2);
    let struct_vec4 = b.type_struct(vec![arr_vec4]);
    let ptr_vec4 = b.type_pointer(None, StorageClass::Uniform, struct_vec4);

    let var_scalar = b.variable(ptr_scalar, None, StorageClass::Uniform, None);
    let var_vec4 = b.variable(ptr_vec4, None, StorageClass::Uniform, None);

    for (s, block) in [(struct_scalar, true), (struct_vec4, false)] {
        b.decorate(s, Decoration::Block, []);
        b.member_decorate(s, 0, Decoration::Offset, [Operand::LiteralBit32(0)]);
        let _ = block;
    }
    b.decorate(
        arr_scalar,
        Decoration::ArrayStride,
        [Operand::LiteralBit32(4)],
    );
    b.decorate(arr_vec4, Decoration::ArrayStride, [Operand::LiteralBit32(16)]);
    for var in [var_scalar, var_vec4] {
        b.decorate(var, Decoration::DescriptorSet, [Operand::LiteralBit32(0)]);
        b.decorate(var, Decoration::Binding, [Operand::LiteralBit32(0)]);
    }

    // Function: load var_scalar[0][param].
    let ftype = b.type_function(void, vec![u32]);
    b.begin_function(void, None, FunctionControl::NONE, ftype)
        .unwrap();
    let param = b.function_parameter(u32).unwrap();
    b.begin_block(None).unwrap();
    let ptr_f32 = b.type_pointer(None, StorageClass::Uniform, f32);
    let ac = b
        .access_chain(ptr_f32, None, var_scalar, [c0, param])
        .unwrap();
    let _ = b.load(f32, None, ac, None, []).unwrap();
    b.ret().unwrap();
    b.end_function().unwrap();

    let ids = ModuleIds {
        var_scalar,
        var_vec4,
    };
    (b.module().clone().assemble(), ids)
}

/// Builds a module with a lone scalar view and a **static** index:
/// `struct { float[8] (stride 4) }` accessed as `[0][5]`.
fn build_single_view_static() -> (Vec<u32>, u32) {
    let mut b = Builder::new();
    b.memory_model(AddressingModel::Logical, MemoryModel::GLSL450);

    let void = b.type_void();
    let f32 = b.type_float(32, None);
    let u32 = b.type_int(32, 0);
    let c0 = b.constant_bit32(u32, 0);
    let c5 = b.constant_bit32(u32, 5);
    let c8 = b.constant_bit32(u32, 8);

    let arr = b.type_array(f32, c8);
    let s = b.type_struct(vec![arr]);
    let ptr = b.type_pointer(None, StorageClass::Uniform, s);
    let var = b.variable(ptr, None, StorageClass::Uniform, None);

    b.decorate(s, Decoration::Block, []);
    b.member_decorate(s, 0, Decoration::Offset, [Operand::LiteralBit32(0)]);
    b.decorate(arr, Decoration::ArrayStride, [Operand::LiteralBit32(4)]);
    b.decorate(var, Decoration::DescriptorSet, [Operand::LiteralBit32(0)]);
    b.decorate(var, Decoration::Binding, [Operand::LiteralBit32(0)]);

    let ftype = b.type_function(void, vec![]);
    b.begin_function(void, None, FunctionControl::NONE, ftype)
        .unwrap();
    b.begin_block(None).unwrap();
    let ptr_f32 = b.type_pointer(None, StorageClass::Uniform, f32);
    let ac = b
        .access_chain(ptr_f32, None, var, [c0, c5])
        .unwrap();
    let _ = b.load(f32, None, ac, None, []).unwrap();
    b.ret().unwrap();
    b.end_function().unwrap();

    (b.module().clone().assemble(), var)
}

/// Builds a module with a lone scalar view whose `float4` type exists but is
/// declared **after** the cbuffer variable — the shape dxil-spirv actually
/// emits. The rewrite must move such a TypeVector before the newly inserted
/// TypeArray (definition-before-use).
fn build_vec4_declared_after_var() -> (Vec<u32>, u32) {
    let mut b = Builder::new();
    b.memory_model(AddressingModel::Logical, MemoryModel::GLSL450);

    let void = b.type_void();
    let f32 = b.type_float(32, None);
    let u32 = b.type_int(32, 0);
    let c0 = b.constant_bit32(u32, 0);
    let c5 = b.constant_bit32(u32, 5);
    let c8 = b.constant_bit32(u32, 8);

    let arr = b.type_array(f32, c8);
    let s = b.type_struct(vec![arr]);
    let ptr = b.type_pointer(None, StorageClass::Uniform, s);
    let var = b.variable(ptr, None, StorageClass::Uniform, None);

    // The float4 type is created *after* the variable on purpose: builder
    // appends type instructions in call order, so the TypeVector lands after
    // the OpVariable in types_global_values.
    let _vec4 = b.type_vector(f32, 4);

    b.decorate(s, Decoration::Block, []);
    b.member_decorate(s, 0, Decoration::Offset, [Operand::LiteralBit32(0)]);
    b.decorate(arr, Decoration::ArrayStride, [Operand::LiteralBit32(4)]);
    b.decorate(var, Decoration::DescriptorSet, [Operand::LiteralBit32(0)]);
    b.decorate(var, Decoration::Binding, [Operand::LiteralBit32(0)]);

    let ftype = b.type_function(void, vec![]);
    b.begin_function(void, None, FunctionControl::NONE, ftype)
        .unwrap();
    b.begin_block(None).unwrap();
    let ptr_f32 = b.type_pointer(None, StorageClass::Uniform, f32);
    let ac = b
        .access_chain(ptr_f32, None, var, [c0, c5])
        .unwrap();
    let _ = b.load(f32, None, ac, None, []).unwrap();
    b.ret().unwrap();
    b.end_function().unwrap();

    (b.module().clone().assemble(), var)
}

/// Builds a clean module with only a vec4-aligned cbuffer (nothing to do).
fn build_clean() -> Vec<u32> {
    let mut b = Builder::new();
    b.memory_model(AddressingModel::Logical, MemoryModel::GLSL450);

    let void = b.type_void();
    let f32 = b.type_float(32, None);
    let u32 = b.type_int(32, 0);
    let c0 = b.constant_bit32(u32, 0);
    let c2 = b.constant_bit32(u32, 2);

    let vec4 = b.type_vector(f32, 4);
    let arr = b.type_array(vec4, c2);
    let s = b.type_struct(vec![arr]);
    let ptr = b.type_pointer(None, StorageClass::Uniform, s);
    let var = b.variable(ptr, None, StorageClass::Uniform, None);

    b.decorate(s, Decoration::Block, []);
    b.member_decorate(s, 0, Decoration::Offset, [Operand::LiteralBit32(0)]);
    b.decorate(arr, Decoration::ArrayStride, [Operand::LiteralBit32(16)]);
    b.decorate(var, Decoration::DescriptorSet, [Operand::LiteralBit32(0)]);
    b.decorate(var, Decoration::Binding, [Operand::LiteralBit32(0)]);

    let ftype = b.type_function(void, vec![]);
    b.begin_function(void, None, FunctionControl::NONE, ftype)
        .unwrap();
    b.begin_block(None).unwrap();
    let ptr_f32 = b.type_pointer(None, StorageClass::Uniform, f32);
    let ac = b
        .access_chain(ptr_f32, None, var, [c0, c0])
        .unwrap();
    let _ = b.load(f32, None, ac, None, []).unwrap();
    b.ret().unwrap();
    b.end_function().unwrap();

    b.module().clone().assemble()
}

/// Parses words back into a module for inspection.
fn parse(words: &[u32]) -> rspirv::dr::Module {
    let mut loader = rspirv::dr::Loader::new();
    rspirv::binary::parse_words(words, &mut loader).expect("parse");
    loader.module()
}

/// Resolves the value of a u32 `OpConstant` by id (accepts both the
/// `IdRef` and `LiteralBit32` operand forms rspirv uses).
fn constant_value(module: &rspirv::dr::Module, id: u32) -> Option<u32> {
    module.types_global_values.iter().find_map(|inst| {
        if inst.class.opcode == Op::Constant && inst.result_id == Some(id) {
            match inst.operands.first() {
                Some(Operand::IdRef(v)) => Some(*v),
                Some(Operand::LiteralBit32(v)) => Some(*v),
                _ => None,
            }
        } else {
            None
        }
    })
}

/// Collects all access chain base ids in the module's functions.
fn access_chain_bases(module: &rspirv::dr::Module) -> Vec<(u32, Vec<u32>)> {
    let mut out = vec![];
    for func in &module.functions {
        for block in &func.blocks {
            for inst in &block.instructions {
                if matches!(inst.class.opcode, Op::AccessChain | Op::InBoundsAccessChain) {
                    let base = match inst.operands.first() {
                        Some(Operand::IdRef(b)) => *b,
                        _ => 0,
                    };
                    let indices = inst.operands[1..]
                        .iter()
                        .filter_map(|o| match o {
                            Operand::IdRef(i) => Some(*i),
                            _ => None,
                        })
                        .collect();
                    out.push((base, indices));
                }
            }
        }
    }
    out
}

#[test]
fn merge_dual_view() {
    let (words, ids) = build_dual_view();
    let out = hlsl_compat::vec4_align_cbuffers(&words).expect("pass");
    assert_eq!(out.rewritten, 1, "exactly one view rewritten");
    assert!(out.skipped.is_empty(), "nothing should be skipped");

    let module = parse(&out.spirv);
    let info = hc_ir::analyze(&module);

    // The scalar view variable must be gone; the vec4 alias must remain.
    assert!(
        !info.variables.iter().any(|v| v.id == ids.var_scalar),
        "scalar view variable must be removed"
    );
    assert!(
        info.variables.iter().any(|v| v.id == ids.var_vec4),
        "vec4 alias must remain"
    );

    // The access chain must be redirected to the alias with 3 indices
    // (member, quotient, remainder).
    let chains = access_chain_bases(&module);
    assert_eq!(chains.len(), 1, "one access chain");
    let (base, indices) = &chains[0];
    assert_eq!(*base, ids.var_vec4, "access chain redirected to vec4 alias");
    assert_eq!(indices.len(), 3, "[member, i/4, i%4]");
    // Member index must be a constant with value 0.
    let member_val = constant_value(&module, indices[0]).expect("member index is a constant");
    assert_eq!(member_val, 0, "member index 0");

    // OpUDiv and OpUMod must be present.
    let has_udiv = module.functions.iter().any(|f| {
        f.blocks
            .iter()
            .any(|b| b.instructions.iter().any(|i| i.class.opcode == Op::UDiv))
    });
    let has_umod = module.functions.iter().any(|f| {
        f.blocks
            .iter()
            .any(|b| b.instructions.iter().any(|i| i.class.opcode == Op::UMod))
    });
    assert!(has_udiv && has_umod, "dynamic index split into UDiv/UMod");
}

#[test]
fn in_place_rewrite_static_index() {
    let (words, var_id) = build_single_view_static();
    let out = hlsl_compat::vec4_align_cbuffers(&words).expect("pass");
    assert_eq!(out.rewritten, 1);

    let module = parse(&out.spirv);
    let info = hc_ir::analyze(&module);

    // The variable survives (in-place), its wrapper is now vec4-aligned.
    let var = info
        .variables
        .iter()
        .find(|v| v.id == var_id)
        .expect("variable still present");
    assert!(info.block_structs.contains(&var.struct_type));
    let members = info.struct_members.get(&var.struct_type).expect("members");
    assert_eq!(members.len(), 1);
    let (elem, len_const) = info.array_info.get(&members[0]).expect("array");
    assert!(info.is_vec4_of_32(*elem), "element is float4");
    let len = info.constants.get(len_const).copied().unwrap();
    assert_eq!(len, 2, "float[8] -> float4[2]");
    assert_eq!(info.array_strides.get(&members[0]), Some(&16));

    // Static index 5 -> [0, 1, 1] (5/4 == 1, 5%4 == 1).
    let chains = access_chain_bases(&module);
    assert_eq!(chains.len(), 1);
    let (base, indices) = &chains[0];
    assert_eq!(*base, var_id);
    assert_eq!(indices.len(), 3);
    let values: Vec<u32> = indices
        .iter()
        .map(|i| constant_value(&module, *i).unwrap_or(u32::MAX))
        .collect();
    assert_eq!(values, vec![0, 1, 1], "static [0, 5] split into [0, 5/4, 5%4]");
}

/// Regression: with a **lone** scalar view the module may have no `float4`
/// type yet, so the pass must insert a fresh `OpTypeVector`. It must be
/// emitted *before* the `OpTypeArray` that references it — SPIR-V forbids
/// forward references in the type system, and spirv-cross's parser rejects
/// (asserts on) such modules. This test pins the definition-before-use
/// ordering of every type/constant in the rewritten module.
#[test]
fn in_place_rewrite_types_defined_before_use() {
    let (words, _) = build_single_view_static();
    let out = hlsl_compat::vec4_align_cbuffers(&words).expect("pass");

    let module = parse(&out.spirv);
    let mut pos: HashMap<u32, usize> = HashMap::new();
    let mut type_arrays: Vec<(usize, u32, u32)> = Vec::new(); // (pos, component_id, length_id)
    for (i, inst) in module.types_global_values.iter().enumerate() {
        if let Some(id) = inst.result_id {
            pos.insert(id, i);
        }
        if inst.class.opcode == Op::TypeArray {
            let component = match inst.operands.first() {
                Some(Operand::IdRef(v)) => *v,
                _ => 0,
            };
            let len = match inst.operands.get(1) {
                Some(Operand::IdRef(v)) => *v,
                _ => 0,
            };
            type_arrays.push((i, component, len));
        }
    }
    assert!(!type_arrays.is_empty(), "rewrite must emit a TypeArray");

    for (array_pos, component_id, len_id) in type_arrays {
        let component_pos = pos.get(&component_id).copied().expect("component type defined");
        assert!(
            component_pos < array_pos,
            "component type (id {component_id}) must precede its TypeArray at {array_pos}"
        );
        let len_pos = pos.get(&len_id).copied().expect("length constant defined");
        assert!(
            len_pos < array_pos,
            "length constant (id {len_id}) must precede its TypeArray at {array_pos}"
        );
    }
}

/// Regression: dxil-spirv output commonly declares the `float4` type *after*
/// the cbuffer variable. The in-place rewrite must move such a TypeVector
/// before the newly inserted TypeArray — otherwise the module contains a
/// forward-referenced type and spirv-cross rejects it (parse failure /
/// `assert(0)` in mark_used_as_array_length).
#[test]
fn in_place_rewrite_moves_vec4_declared_after_variable() {
    let (words, var_id) = build_vec4_declared_after_var();
    let out = hlsl_compat::vec4_align_cbuffers(&words).expect("pass");
    assert_eq!(out.rewritten, 1);

    let module = parse(&out.spirv);
    // The variable must still be bound to the new vec4 wrapper.
    let info = hc_ir::analyze(&module);
    let var = info
        .variables
        .iter()
        .find(|v| v.id == var_id)
        .expect("variable still present");
    let members = info.struct_members.get(&var.struct_type).expect("members");
    assert_eq!(members.len(), 1);
    let (elem, _) = info.array_info.get(&members[0]).expect("array");
    assert!(info.is_vec4_of_32(*elem), "element is float4");

    // Definition-before-use: every TypeArray's component and length must be
    // defined before the TypeArray itself.
    let mut pos: HashMap<u32, usize> = HashMap::new();
    let mut type_arrays: Vec<(usize, u32, u32)> = Vec::new();
    for (i, inst) in module.types_global_values.iter().enumerate() {
        if let Some(id) = inst.result_id {
            pos.insert(id, i);
        }
        if inst.class.opcode == Op::TypeArray {
            let component = match inst.operands.first() {
                Some(Operand::IdRef(v)) => *v,
                _ => 0,
            };
            let len = match inst.operands.get(1) {
                Some(Operand::IdRef(v)) => *v,
                _ => 0,
            };
            type_arrays.push((i, component, len));
        }
    }
    assert!(!type_arrays.is_empty(), "rewrite must emit a TypeArray");
    for (array_pos, component_id, len_id) in type_arrays {
        let component_pos = pos.get(&component_id).copied().expect("component type defined");
        assert!(
            component_pos < array_pos,
            "component type (id {component_id}) must precede its TypeArray at {array_pos}"
        );
        let len_pos = pos.get(&len_id).copied().expect("length constant defined");
        assert!(
            len_pos < array_pos,
            "length constant (id {len_id}) must precede its TypeArray at {array_pos}"
        );
    }
}

#[test]
fn idempotent_noop_on_clean_module() {
    let words = build_clean();
    let out = hlsl_compat::vec4_align_cbuffers(&words).expect("pass");
    assert_eq!(out.rewritten, 0);
    assert_eq!(out.spirv, words, "no-op must return input verbatim");
}

#[test]
fn invalid_input_rejected() {
    let err = hlsl_compat::vec4_align_cbuffers(&[0xdead_beef, 1, 2, 3]).unwrap_err();
    assert!(
        matches!(err, hlsl_compat::HlslCompatError::InvalidSpirv(_)),
        "garbage words must yield InvalidSpirv, got {err:?}"
    );
}
