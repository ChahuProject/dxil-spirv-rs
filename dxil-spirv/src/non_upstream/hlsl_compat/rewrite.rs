//! Rewrites the module: vec4-aligns stride-4 cbuffer views.
//!
//! For each [`CbufferTarget`](super::detect::CbufferTarget):
//!
//! * **In-place** (no vec4 alias): the view variable's wrapper struct is
//!   replaced by `struct { float4[N/4] }` (ArrayStride 16), and every access
//!   chain `[member, i]` becomes `[member, i/4, i%4]`.
//! * **Merge** (vec4 alias found): every access chain is redirected to the
//!   alias variable (same index rewrite), and the scalar view variable is
//!   dropped.
//!
//! Both paths describe exactly the same bytes (a cbuffer's memory layout is a
//! flat sequence of vec4 registers), so the transformation is layout- and
//! semantics-preserving.

use super::detect::{AccessUse, CbufferTarget};
use super::error::HlslCompatError;
use rspirv::dr::{Instruction, Module, Operand};
use rspirv::spirv::{Op, StorageClass};
use std::collections::{BTreeSet, HashMap};

/// Allocates fresh result ids beyond the module's current bound.
struct IdAlloc {
    bound: u32,
}

impl IdAlloc {
    fn new(bound: u32) -> Self {
        Self { bound }
    }

    fn alloc(&mut self) -> u32 {
        let id = self.bound;
        self.bound += 1;
        id
    }
}

/// Finds the module's `OpTypeInt(32, unsigned)` id (used for array indices).
fn find_u32_type(module: &Module) -> Option<u32> {
    module.types_global_values.iter().find_map(|inst| {
        if inst.class.opcode != Op::TypeInt {
            return None;
        }
        let width = match inst.operands.first()? {
            Operand::LiteralBit32(w) => *w,
            _ => return None,
        };
        let signed = match inst.operands.get(1)? {
            Operand::LiteralBit32(s) => *s,
            _ => return None,
        };
        if width == 32 && signed == 0 {
            inst.result_id
        } else {
            None
        }
    })
}

/// Finds an existing `OpTypeVector(scalar, 4)` for the given scalar type, if
/// any (avoids duplicate vector types).
fn find_vec4_of(module: &Module, scalar_type: u32) -> Option<u32> {
    module.types_global_values.iter().find_map(|inst| {
        if inst.class.opcode != Op::TypeVector {
            return None;
        }
        let comp = match inst.operands.first()? {
            Operand::IdRef(c) => *c,
            _ => return None,
        };
        let count = match inst.operands.get(1)? {
            Operand::LiteralBit32(c) => *c,
            _ => return None,
        };
        if comp == scalar_type && count == 4 {
            inst.result_id
        } else {
            None
        }
    })
}

/// Appends a new `OpConstant` (32-bit) to the module's global scope and
/// returns its id.
///
/// SPIR-V only requires definition-before-use for global instructions; the
/// referenced u32 type and any prior constants are always defined earlier in
/// the module, so appending is safe. (An earlier revision inserted before the
/// first `OpVariable`; that broke modules where the u32 type is declared
/// *after* some variables.)
fn push_u32_constant(
    module: &mut Module,
    alloc: &mut IdAlloc,
    u32_type: u32,
    value: u32,
) -> u32 {
    let id = alloc.alloc();
    let inst = Instruction::new(
        Op::Constant,
        Some(u32_type),
        Some(id),
        vec![Operand::IdRef(value)],
    );
    module.types_global_values.push(inst);
    id
}

/// Resolves a `u32` constant value by id, if the id denotes an
/// `OpConstant` of the module's u32 type.
///
/// rspirv represents the literal both as `IdRef` (modules produced by the
/// parser) and as `LiteralBit32` (modules produced by the builder); both are
/// accepted here.
fn resolve_u32_const(module: &Module, u32_type: u32, const_id: u32) -> Option<u32> {
    module.types_global_values.iter().find_map(|inst| {
        if inst.class.opcode == Op::Constant
            && inst.result_id == Some(const_id)
            && inst.result_type == Some(u32_type)
        {
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

/// Resolves (or creates) a `u32` constant with the given value, reusing an
/// existing one if present.
fn ensure_u32_constant(
    module: &mut Module,
    alloc: &mut IdAlloc,
    u32_type: u32,
    value: u32,
) -> u32 {
    if let Some(existing) = module.types_global_values.iter().find_map(|inst| {
        if inst.class.opcode == Op::Constant && inst.result_type == Some(u32_type) {
            match inst.operands.first() {
                Some(Operand::IdRef(v)) if *v == value => inst.result_id,
                Some(Operand::LiteralBit32(v)) if *v == value => inst.result_id,
                _ => None,
            }
        } else {
            None
        }
    }) {
        return existing;
    }
    push_u32_constant(module, alloc, u32_type, value)
}

/// Locates the `OpVariable` instruction for `var_id` in the global scope.
fn find_variable_pos(module: &Module, var_id: u32) -> Option<usize> {
    module.types_global_values.iter().position(|inst| {
        inst.class.opcode == Op::Variable && inst.result_id == Some(var_id)
    })
}

/// Builds the replacement wrapper type for an in-place rewrite:
/// `struct { float4[N/4] (stride 16) }` plus its Uniform pointer type.
/// All instructions are inserted *before* `var_pos` (the variable that will
/// reference them), preserving definition-before-use ordering. Returns
/// (new_struct_id, new_pointer_id).
#[allow(clippy::too_many_arguments)]
fn build_vec4_wrapper_type(
    module: &mut Module,
    alloc: &mut IdAlloc,
    u32_type: u32,
    scalar_type: u32,
    array_len: u32, // N
    var_pos: usize,
) -> Result<(u32, u32), HlslCompatError> {
    let len = array_len / 4;
    let len_const = alloc.alloc();
    let vec4_type = match find_vec4_of(module, scalar_type) {
        Some(t) => t,
        None => {
            let t = alloc.alloc();
            let inst = Instruction::new(
                Op::TypeVector,
                None,
                Some(t),
                vec![Operand::IdRef(scalar_type), Operand::LiteralBit32(4)],
            );
            module.types_global_values.insert(var_pos, inst);
            t
        }
    };
    let array_type = alloc.alloc();
    let struct_type = alloc.alloc();
    let pointer_type = alloc.alloc();

    // Insert in dependency order right before the variable: length constant
    // must precede the array type; the vector may have been inserted above.
    module.types_global_values.insert(
        var_pos,
        Instruction::new(
            Op::Constant,
            Some(u32_type),
            Some(len_const),
            vec![Operand::IdRef(len)],
        ),
    );
    module.types_global_values.insert(
        var_pos,
        Instruction::new(
            Op::TypeArray,
            None,
            Some(array_type),
            vec![Operand::IdRef(vec4_type), Operand::IdRef(len_const)],
        ),
    );
    module.types_global_values.insert(
        var_pos,
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_type),
            vec![Operand::IdRef(array_type)],
        ),
    );
    module.types_global_values.insert(
        var_pos,
        Instruction::new(
            Op::TypePointer,
            None,
            Some(pointer_type),
            vec![
                Operand::StorageClass(StorageClass::Uniform),
                Operand::IdRef(struct_type),
            ],
        ),
    );

    // Block on the new struct, ArrayStride 16 on the new array, member
    // Offset 0 on the new struct.
    module.annotations.push(Instruction::new(
        Op::Decorate,
        None,
        None,
        vec![
            Operand::IdRef(struct_type),
            Operand::Decoration(rspirv::spirv::Decoration::Block),
        ],
    ));
    module.annotations.push(Instruction::new(
        Op::Decorate,
        None,
        None,
        vec![
            Operand::IdRef(array_type),
            Operand::Decoration(rspirv::spirv::Decoration::ArrayStride),
            Operand::LiteralBit32(16),
        ],
    ));
    module.annotations.push(Instruction::new(
        Op::MemberDecorate,
        None,
        None,
        vec![
            Operand::IdRef(struct_type),
            Operand::LiteralBit32(0),
            Operand::Decoration(rspirv::spirv::Decoration::Offset),
            Operand::LiteralBit32(0),
        ],
    ));

    Ok((struct_type, pointer_type))
}

/// Removes `var_id` from every entry point's interface list.
fn remove_from_entry_point_interfaces(module: &mut Module, var_id: u32) {
    for ep in &mut module.entry_points {
        let mut new_ops = Vec::with_capacity(ep.operands.len());
        for (pos, op) in ep.operands.iter().enumerate() {
            if pos >= 3 && matches!(op, Operand::IdRef(id) if *id == var_id) {
                continue;
            }
            new_ops.push(op.clone());
        }
        ep.operands = new_ops;
    }
}

/// Removes decorations and debug names that reference `var_id` so the
/// dropped variable leaves no dangling references behind.
fn remove_var_metadata(module: &mut Module, var_id: u32) {
    module.annotations.retain(|inst| {
        !(inst.class.opcode == Op::Decorate
            && inst.operands.first().and_then(|o| match o {
                Operand::IdRef(id) => Some(*id),
                _ => None,
            }) == Some(var_id))
    });
    module.debug_names.retain(|inst| {
        !(inst.class.opcode == Op::Name
            && inst.operands.first().and_then(|o| match o {
                Operand::IdRef(id) => Some(*id),
                _ => None,
            }) == Some(var_id))
    });
}

/// Rewrites every access chain of all targets.
///
/// `plan` maps each access chain to the variable it must be redirected to
/// (the view itself for in-place, the vec4 alias for merges). Chains are
/// processed in reverse instruction order **across all targets and blocks**,
/// so div/mod instructions inserted for one chain never shift the position of
/// chains that have not been processed yet.
///
/// Dynamic indices get `OpUDiv`/`OpUMod` instructions inserted immediately
/// before the access chain; static indices use the pre-created constants in
/// `static_split`.
#[allow(clippy::too_many_arguments)]
fn rewrite_accesses(
    module: &mut Module,
    alloc: &mut IdAlloc,
    u32_type: u32,
    plan: &mut [(u32, AccessUse)],
    member_const: u32,
    static_split: &HashMap<u32, (u32, u32)>,
    static_index_value: &HashMap<(usize, usize, usize), u32>,
    four_const: u32,
) {
    // Reverse order: highest (function, block, instruction) first.
    plan.sort_by_key(|(_, u)| std::cmp::Reverse((u.function_idx, u.block_idx, u.inst_idx)));

    // Group by block; within a block the plan is already in reverse order.
    // (inst_idx, target_var, use_info)
    type BlockPlan = Vec<(usize, u32, AccessUse)>;
    let mut by_block: HashMap<(usize, usize), BlockPlan> = HashMap::new();
    for (target_var, use_info) in plan.iter() {
        by_block
            .entry((use_info.function_idx, use_info.block_idx))
            .or_default()
            .push((use_info.inst_idx, *target_var, *use_info));
    }

    for ((f_idx, b_idx), entries) in by_block {
        let func = &mut module.functions[f_idx];
        let block = &mut func.blocks[b_idx];

        for (inst_idx, target_var, use_info) in entries {
            let orig = block.instructions[inst_idx].clone();

            let (div_id, mod_id) = if use_info.is_dynamic {
                let index_id = match orig.operands.get(2) {
                    Some(Operand::IdRef(id)) => *id,
                    _ => continue, // already validated by detect; defensive
                };
                // Insert OpUDiv / OpUMod immediately before the access chain.
                let div_id = alloc.alloc();
                let mod_id = alloc.alloc();
                let div = Instruction::new(
                    Op::UDiv,
                    Some(u32_type),
                    Some(div_id),
                    vec![Operand::IdRef(index_id), Operand::IdRef(four_const)],
                );
                let umod = Instruction::new(
                    Op::UMod,
                    Some(u32_type),
                    Some(mod_id),
                    vec![Operand::IdRef(index_id), Operand::IdRef(four_const)],
                );
                block.instructions.insert(inst_idx, div);
                block.instructions.insert(inst_idx + 1, umod);
                (div_id, mod_id)
            } else {
                // Static: use the pre-created quotient/remainder constants.
                let value = *static_index_value
                    .get(&(f_idx, b_idx, inst_idx))
                    .expect("static index must have been pre-scanned");
                let (div_id, mod_id) = *static_split
                    .get(&value)
                    .expect("static split constants must have been pre-created");
                (div_id, mod_id)
            };

            let new_ac = Instruction::new(
                orig.class.opcode,
                orig.result_type,
                orig.result_id,
                vec![
                    Operand::IdRef(target_var),
                    Operand::IdRef(member_const),
                    Operand::IdRef(div_id),
                    Operand::IdRef(mod_id),
                ],
            );
            // The original access chain now sits two slots further down
            // (two div/mod instructions were inserted before it).
            if use_info.is_dynamic {
                block.instructions[inst_idx + 2] = new_ac;
            } else {
                block.instructions[inst_idx] = new_ac;
            }
        }
    }
}

/// Applies all rewrite targets to the module. Returns the number of targets
/// that were successfully rewritten (targets that could not be handled are
/// skipped and do not fail the pass).
pub fn apply(module: &mut Module, targets: &[CbufferTarget]) -> Result<usize, HlslCompatError> {
    let Some(u32_type) = find_u32_type(module) else {
        return Err(HlslCompatError::Unsupported(
            "module has no unsigned 32-bit integer type for index arithmetic".into(),
        ));
    };

    let bound = module
        .header
        .as_ref()
        .map(|h| h.bound)
        .ok_or_else(|| HlslCompatError::InvalidSpirv("missing module header".into()))?;
    let mut alloc = IdAlloc::new(bound);

    // Pre-scan: resolve every static index value used by the access chains so
    // all quotient/remainder constants exist before any function mutation.
    let mut static_values: BTreeSet<u32> = BTreeSet::new();
    let mut static_index_value: HashMap<(usize, usize, usize), u32> = HashMap::new();
    for target in targets {
        for ac in &target.accesses {
            let inst = &module.functions[ac.function_idx].blocks[ac.block_idx].instructions
                [ac.inst_idx];
            if let Some(Operand::IdRef(idx_id)) = inst.operands.get(2) {
                if let Some(value) = resolve_u32_const(module, u32_type, *idx_id) {
                    static_values.insert(value);
                    static_index_value.insert((ac.function_idx, ac.block_idx, ac.inst_idx), value);
                }
                // Note: if the constant cannot be resolved (e.g. non-u32
                // index, impossible in valid SPIR-V), the access chain is
                // treated as dynamic below.
            }
        }
    }

    let mut static_split: HashMap<u32, (u32, u32)> = HashMap::new();
    for value in static_values {
        let div = push_u32_constant(module, &mut alloc, u32_type, value / 4);
        let m = push_u32_constant(module, &mut alloc, u32_type, value % 4);
        static_split.insert(value, (div, m));
    }

    // The literal `4` used for dynamic division/modulo.
    let four_const = push_u32_constant(module, &mut alloc, u32_type, 4);
    // Member index of the (single-member) wrapper structs: always 0.
    let member_const = ensure_u32_constant(module, &mut alloc, u32_type, 0);

    let mut rewritten = 0usize;
    let mut plan: Vec<(u32, AccessUse)> = Vec::new();

    for target in targets {
        let view = &target.view;
        let target_var = match &target.alias {
            Some(alias) => alias.var_id,
            None => view.var_id,
        };

        let Some(var_pos) = find_variable_pos(module, view.var_id) else {
            continue; // variable vanished between analysis and rewrite
        };

        if target.alias.is_none() {
            // In-place: replace the variable's type with the vec4 wrapper.
            let (_, new_pointer) = build_vec4_wrapper_type(
                module,
                &mut alloc,
                u32_type,
                view.scalar_type_id,
                view.array_len,
                var_pos,
            )?;
            // Re-locate the variable (insertions shifted positions above).
            let var_pos = find_variable_pos(module, view.var_id).expect("variable still present");
            module.types_global_values[var_pos].result_type = Some(new_pointer);
        } else {
            // Merge: drop the scalar view variable; the vec4 alias already
            // declares the canonical type.
            remove_from_entry_point_interfaces(module, view.var_id);
            remove_var_metadata(module, view.var_id);
            let var_pos = find_variable_pos(module, view.var_id).expect("variable still present");
            module.types_global_values.remove(var_pos);
        }

        for ac in &target.accesses {
            plan.push((target_var, *ac));
        }
        rewritten += 1;
    }

    rewrite_accesses(
        module,
        &mut alloc,
        u32_type,
        &mut plan,
        member_const,
        &static_split,
        &static_index_value,
        four_const,
    );

    // Update the module bound so freshly allocated ids are covered.
    if let Some(header) = module.header.as_mut() {
        header.bound = alloc.bound;
    }

    Ok(rewritten)
}
