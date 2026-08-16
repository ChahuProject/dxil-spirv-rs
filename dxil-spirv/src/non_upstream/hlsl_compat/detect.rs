//! Detection of stride-4 scalar cbuffer views and their vec4-aligned aliases.
//!
//! Upstream dxbc-spirv can emit a constant buffer as a *scalar view*:
//!
//! ```text
//! OpTypeStruct { float[N] (ArrayStride 4) }
//! ```
//!
//! (the "wrapper" form: a single member that is a 32-bit scalar array with
//! stride 4). This layout is legal std140, but the spirv-cross2 HLSL backend
//! cannot express it (its cbuffer model is vec4-register based), so HLSL
//! decompilation fails. The same cbuffer frequently also exists as a *vec4
//! view* in the same module (a second Uniform block with the same binding,
//! `float4[M]` with stride 16, where `M * 4 == N`).
//!
//! This module finds such views, pairs each scalar view with its vec4 alias
//! (if any), and checks whether every access chain into the view can be
//! rewritten safely. Anything unsafe is skipped (left untouched).

use super::ir::{self, ModuleInfo, Variable};
use rspirv::dr::Module;
use rspirv::spirv::{Op, StorageClass};
use std::collections::HashMap;

/// A stride-4 scalar array cbuffer view.
#[derive(Debug, Clone, Copy)]
pub struct Stride4View {
    /// `OpVariable` id of the scalar view.
    pub var_id: u32,
    /// Wrapper `OpTypeStruct` id (single member).
    pub struct_id: u32,
    /// Member index of the array inside the wrapper struct (0).
    pub member_index: u32,
    /// The `float[N]` array type id.
    pub array_type_id: u32,
    /// Array length `N`.
    pub array_len: u32,
    /// The 32-bit scalar (float/int) type id.
    pub scalar_type_id: u32,
    /// Total size in bytes (`N * 4`).
    pub total_bytes: u32,
}

/// A vec4-aligned view of the same cbuffer (`float4[M]`, stride 16).
#[derive(Debug, Clone, Copy)]
pub struct Vec4Alias {
    /// `OpVariable` id of the vec4 view (the merge target).
    pub var_id: u32,
    /// Wrapper `OpTypeStruct` id.
    pub struct_id: u32,
    /// Member index of the `float4[M]` array inside its wrapper.
    pub member_index: u32,
    /// Array length `M` (= `N / 4`).
    pub array_len: u32,
}

/// Location of an `OpAccessChain` (or `OpInBoundsAccessChain`) instruction.
#[derive(Debug, Clone, Copy)]
pub struct AccessUse {
    /// Index into `rspirv::dr::Module::functions`.
    pub function_idx: usize,
    /// Index into `rspirv::dr::Function::blocks`.
    pub block_idx: usize,
    /// Index into `block.instructions`.
    pub inst_idx: usize,
    /// `true` if the array index is a runtime value (needs OpUDiv/OpUMod).
    pub is_dynamic: bool,
}

/// A rewrite target: a scalar view plus (optionally) its vec4 alias, with the
/// access chains that must be rewritten.
#[derive(Debug)]
pub struct CbufferTarget {
    /// The scalar view to rewrite.
    pub view: Stride4View,
    /// The vec4 alias to merge into, if one exists.
    pub alias: Option<Vec4Alias>,
    /// Every access chain rooted at the view that must be rewritten.
    pub accesses: Vec<AccessUse>,
}

/// A scalar view that was found but cannot be safely rewritten.
#[derive(Debug)]
pub struct Skipped {
    /// `OpVariable` id of the skipped view.
    pub var_id: u32,
    /// Human-readable reason for skipping.
    pub reason: &'static str,
}

fn get_u32_const(info: &ModuleInfo, const_id: u32) -> Option<u32> {
    info.constants.get(&const_id).copied()
}

/// Inspects a Uniform block variable and returns the stride-4 scalar array
/// view descriptor if it matches the wrapper form.
fn find_stride4_view(info: &ModuleInfo, var: &Variable) -> Option<Stride4View> {
    if var.storage_class != StorageClass::Uniform {
        return None;
    }
    if !info.block_structs.contains(&var.struct_type) {
        return None;
    }
    let members = info.struct_members.get(&var.struct_type)?;
    if members.len() != 1 {
        return None; // wrapper must be a single-member struct
    }
    let array_type = members[0];
    let (elem_type, len_const) = *info.array_info.get(&array_type)?;
    let scalar_type = info.scalar_base_32(elem_type)?; // must be a plain 32-bit scalar
    if info.array_strides.get(&array_type) != Some(&4) {
        return None;
    }
    if info.member_offsets.get(&(var.struct_type, 0)) != Some(&0) {
        return None; // member must sit at offset 0
    }
    let len = get_u32_const(info, len_const)?;
    if len == 0 || len % 4 != 0 {
        return None; // cannot be vec4-aligned
    }
    Some(Stride4View {
        var_id: var.id,
        struct_id: var.struct_type,
        member_index: 0,
        array_type_id: array_type,
        array_len: len,
        scalar_type_id: scalar_type,
        total_bytes: len * 4,
    })
}

/// Looks for a vec4 view of the same cbuffer: same binding, same total size,
/// a single-member struct holding `float4[length]` with stride 16.
fn find_vec4_alias(info: &ModuleInfo, view: &Stride4View) -> Option<Vec4Alias> {
    let binding = info.bindings.get(&view.var_id).copied();
    for var in &info.variables {
        if var.id == view.var_id {
            continue;
        }
        if var.storage_class != StorageClass::Uniform {
            continue;
        }
        if !info.block_structs.contains(&var.struct_type) {
            continue;
        }
        if info.bindings.get(&var.id).copied() != binding {
            continue; // must be the same descriptor
        }
        let members = info.struct_members.get(&var.struct_type)?;
        if members.len() != 1 {
            continue;
        }
        let array_type = members[0];
        let (elem_type, len_const) = *info.array_info.get(&array_type)?;
        if !info.is_vec4_of_32(elem_type) {
            continue;
        }
        if info.array_strides.get(&array_type) != Some(&16) {
            continue;
        }
        if info.member_offsets.get(&(var.struct_type, 0)) != Some(&0) {
            continue;
        }
        let len = get_u32_const(info, len_const)?;
        if len * 16 != view.total_bytes {
            continue; // must describe the same byte range
        }
        return Some(Vec4Alias {
            var_id: var.id,
            struct_id: var.struct_type,
            member_index: 0,
            array_len: len,
        });
    }
    None
}

/// Collects every access chain rooted directly at `var_id` and checks whether
/// all of them are safe to rewrite. A use is safe iff:
///
/// * it is an `OpAccessChain` / `OpInBoundsAccessChain` whose base is the
///   variable directly, with exactly two indices (`[member, index]`), and
/// * every consumer of the access chain result is an `OpLoad` with a 32-bit
///   scalar result type, and
/// * the variable itself is only referenced from access-chain bases and
///   entry-point interface lists (decorations / debug names may remain; they
///   are harmless if the variable is dropped).
///
/// Any other use (stores, nested access chains, array length queries,
/// non-semantic debug references, …) marks the whole view as unsafe.
fn collect_accesses(
    module: &Module,
    info: &ModuleInfo,
    view: &Stride4View,
) -> Option<Vec<AccessUse>> {
    // First pass: locate the access chains rooted at the view variable.
    let mut ac_by_id: HashMap<u32, AccessUse> = HashMap::new();
    for (f_idx, func) in module.functions.iter().enumerate() {
        for (b_idx, block) in func.blocks.iter().enumerate() {
            for (i_idx, inst) in block.instructions.iter().enumerate() {
                if matches!(inst.class.opcode, Op::AccessChain | Op::InBoundsAccessChain) {
                    let base = inst.operands.first().and_then(ir::as_id_ref)?;
                    if base != view.var_id {
                        continue;
                    }
                    let indices = &inst.operands[1..];
                    if indices.len() != 2 {
                        return None; // expected exactly [member, index]
                    }
                    // The member index is a constant id; compare its value.
                    let member_const = ir::as_id_ref(&indices[0])?;
                    let member_value = get_u32_const(info, member_const).unwrap_or(u32::MAX);
                    if member_value != view.member_index {
                        return None;
                    }
                    let index = ir::as_id_ref(&indices[1])?;
                    let is_dynamic = !info.constants.contains_key(&index);
                    let ac_id = inst.result_id?;
                    ac_by_id.insert(
                        ac_id,
                        AccessUse {
                            function_idx: f_idx,
                            block_idx: b_idx,
                            inst_idx: i_idx,
                            is_dynamic,
                        },
                    );
                }
            }
        }
    }

    // Second pass: validate every reference to the view variable and every
    // reference to each access chain result.
    let mut verified: std::collections::HashSet<u32> = Default::default();

    for inst in &module.entry_points {
        for (pos, op) in inst.operands.iter().enumerate() {
            if ir::as_id_ref(op) == Some(view.var_id) && pos < 3 {
                return None; // referenced outside the interface list
            }
        }
    }

    for func in &module.functions {
        for block in &func.blocks {
            for inst in &block.instructions {
                for (pos, op) in inst.operands.iter().enumerate() {
                    let Some(id) = ir::as_id_ref(op) else {
                        continue;
                    };
                    if id == view.var_id {
                        let is_ac_base = matches!(
                            inst.class.opcode,
                            Op::AccessChain | Op::InBoundsAccessChain
                        ) && pos == 0;
                        if !is_ac_base {
                            return None;
                        }
                    }
                    if ac_by_id.contains_key(&id) {
                        match inst.class.opcode {
                            Op::Load => {
                                if inst.operands.first().and_then(ir::as_id_ref) != Some(id) {
                                    return None;
                                }
                                // Vector load off a scalar array cannot be
                                // rewritten: skip the whole view.
                                info.scalar_base_32(inst.result_type?)?;
                                verified.insert(id);
                            }
                            _ => return None, // store / nested chain / anything else
                        }
                    }
                }
            }
        }
    }

    // Every access chain must be consumed by at least one scalar load.
    if verified.len() != ac_by_id.len() {
        return None;
    }

    Some(ac_by_id.into_values().collect())
}

/// Finds all safe rewrite targets in the module.
pub fn find_targets(module: &Module, info: &ModuleInfo) -> (Vec<CbufferTarget>, Vec<Skipped>) {
    let mut targets = Vec::new();
    let mut skipped = Vec::new();

    for var in &info.variables {
        let Some(view) = find_stride4_view(info, var) else {
            continue;
        };
        let alias = find_vec4_alias(info, &view);
        match collect_accesses(module, info, &view) {
            Some(accesses) => targets.push(CbufferTarget {
                view,
                alias,
                accesses,
            }),
            None => skipped.push(Skipped {
                var_id: var.id,
                reason: "unsafe access pattern",
            }),
        }
    }

    (targets, skipped)
}
