//! NON-UPSTREAM EXTENSION — stage-output `Location` deduplication.
//!
//! **This module is not part of upstream dxil-spirv.** It is added by
//! dxil-spirv-rs and only exists when the `non-upstream-hlsl-compat` crate
//! feature is enabled (off by default).
//!
//! # Why this exists
//!
//! D3D allows multiple shader outputs to share a single output register by
//! packing disjoint component masks (`o0.xy` + `o0.z`). dxbc-spirv / dxil-spirv
//! translate such outputs into separate SPIR-V variables that share the same
//! `Location` (distinguished by `Component` decorations) — valid SPIR-V that
//! Vulkan consumers handle correctly.
//!
//! The spirv-cross2 **HLSL** backend, however, cannot express shared
//! locations: HLSL semantics have no component concept, so it emits duplicate
//! semantics (e.g. two `TEXCOORD0` outputs) and DXC rejects the result with
//! `error: Semantic 'TEXCOORD' overlap at 0.`
//!
//! [`deduplicate_stage_output_locations`] relocates every colliding output
//! variable to the next free `Location`, producing unique semantics that the
//! HLSL backend (and DXC) can consume. Semantics *names* change (they are
//! derived from `Location`), but the data layout and program behavior are
//! untouched — this pass exists for **display / re-compilation** pipelines
//! only, never for Vulkan execution.
//!
//! The pass is **idempotent** and a **no-op** for modules without location
//! collisions (the input words are returned unchanged).

use super::error::HlslCompatError;
use super::ir::as_id_ref;
use super::NormalizeOutput;
use rspirv::binary::Assemble;
use rspirv::dr::{Instruction, Module, Operand};
use rspirv::spirv::{Decoration, Op, StorageClass};
use std::collections::{HashMap, HashSet};

/// Output variables (storage class `Output`), their `Location` decorations,
/// and the set of variables decorated `BuiltIn` (system values).
fn collect_output_locations(module: &Module) -> (Vec<u32>, HashMap<u32, u32>, HashSet<u32>) {
    let mut output_vars = Vec::new();
    let mut locations = HashMap::new();
    let mut builtins = HashSet::new();

    for inst in &module.types_global_values {
        if inst.class.opcode == Op::Variable {
            let storage = inst.operands.first().and_then(|op| match op {
                Operand::StorageClass(sc) => Some(*sc),
                _ => None,
            });
            if storage == Some(StorageClass::Output) {
                if let Some(id) = inst.result_id {
                    output_vars.push(id);
                }
            }
        }
    }

    for inst in &module.annotations {
        if inst.class.opcode != Op::Decorate {
            continue;
        }
        let Some(target) = as_id_ref(inst.operands.first().unwrap()) else {
            continue;
        };
        let Some(Operand::Decoration(dec)) = inst.operands.get(1) else {
            continue;
        };
        match dec {
            Decoration::Location => {
                if let Some(Operand::LiteralBit32(v)) = inst.operands.get(2) {
                    locations.insert(target, *v);
                }
            }
            Decoration::BuiltIn => {
                builtins.insert(target);
            }
            _ => {}
        }
    }

    (output_vars, locations, builtins)
}

/// Non-upstream extension: relocates colliding stage-output variables so every
/// user output has a unique `Location`.
///
/// * **Input**: any SPIR-V words. Not a SPIR-V module → error.
/// * **Output**: the same module with every `Location` shared by more than one
///   output variable reassigned to the next free location (in variable
///   declaration order; the first variable at a location keeps it). `BuiltIn`
///   outputs (e.g. `SV_Position`) are never touched.
/// * **No-op**: modules without collisions are returned verbatim.
///
/// [`NormalizeOutput::rewritten`] reports how many variables were relocated.
pub fn deduplicate_stage_output_locations(spirv: &[u32]) -> Result<NormalizeOutput, HlslCompatError> {
    let mut loader = rspirv::dr::Loader::new();
    rspirv::binary::parse_words(spirv, &mut loader).map_err(|e| {
        HlslCompatError::InvalidSpirv(format!("failed to parse input module: {e}"))
    })?;
    let mut module = loader.module();

    let (output_vars, locations, builtins) = collect_output_locations(&module);

    // First variable at each location keeps it; later ones are relocated.
    let mut mut_used: HashSet<u32> = locations
        .iter()
        .filter(|(var, _)| !builtins.contains(var))
        .map(|(_, loc)| *loc)
        .collect();
    let mut seen: HashSet<u32> = HashSet::new();
    let mut assigned: HashMap<u32, u32> = HashMap::new();

    for var in &output_vars {
        if builtins.contains(var) {
            continue;
        }
        let Some(loc) = locations.get(var).copied() else {
            continue;
        };
        if seen.insert(loc) {
            continue; // first variable at this location keeps it
        }
        // Collision: assign the next free location (≥ current max + 1).
        let new_loc = next_free(&mut_used);
        mut_used.insert(new_loc);
        assigned.insert(*var, new_loc);
    }

    if assigned.is_empty() {
        return Ok(NormalizeOutput {
            spirv: spirv.to_vec(),
            rewritten: 0,
            skipped: vec![],
        });
    }

    // Drop the old Location decorations of relocated variables, then append
    // the new ones. All other annotations (Component, BuiltIn, …) are kept.
    let mut annotations: Vec<Instruction> = module
        .annotations
        .iter()
        .filter(|inst| {
            !(inst.class.opcode == Op::Decorate
                && as_id_ref(inst.operands.first().unwrap()).is_some_and(|id| assigned.contains_key(&id))
                && matches!(inst.operands.get(1), Some(Operand::Decoration(Decoration::Location))))
        })
        .cloned()
        .collect();

    let mut new_ids: Vec<u32> = assigned.keys().copied().collect();
    new_ids.sort_unstable();
    for var_id in new_ids {
        annotations.push(Instruction::new(
            Op::Decorate,
            None,
            None,
            vec![
                Operand::IdRef(var_id),
                Operand::Decoration(Decoration::Location),
                Operand::LiteralBit32(assigned[&var_id]),
            ],
        ));
    }
    module.annotations = annotations;

    Ok(NormalizeOutput {
        spirv: module.assemble(),
        rewritten: assigned.len(),
        skipped: vec![],
    })
}

/// Smallest unused location at or above `max(used) + 1`.
fn next_free(used: &HashSet<u32>) -> u32 {
    let mut candidate = used.iter().copied().max().unwrap_or(0) + 1;
    while used.contains(&candidate) {
        candidate += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    use rspirv::binary::Assemble;
    use rspirv::dr::{Builder, Operand};
    use rspirv::spirv::{
        AddressingModel, Capability, Decoration, ExecutionModel, FunctionControl, MemoryModel,
        StorageClass,
    };

    /// Builds a vertex module with two Output variables sharing Location 0
    /// (packed register case: `o0.xy` + `o0.z`).
    fn build_collision_module() -> Vec<u32> {
        let mut b = Builder::new();
        b.memory_model(AddressingModel::Logical, MemoryModel::GLSL450);
        b.capability(Capability::Shader);

        let void = b.type_void();
        let f32 = b.type_float(32, None);
        let vec2 = b.type_vector(f32, 2);
        let ptr_out_vec2 = b.type_pointer(None, StorageClass::Output, vec2);
        let ptr_out_f32 = b.type_pointer(None, StorageClass::Output, f32);

        let out_xy = b.variable(ptr_out_vec2, None, StorageClass::Output, None);
        let out_z = b.variable(ptr_out_f32, None, StorageClass::Output, None);
        b.decorate(out_xy, Decoration::Location, [Operand::LiteralBit32(0)]);
        b.decorate(out_z, Decoration::Location, [Operand::LiteralBit32(0)]);
        b.decorate(out_z, Decoration::Component, [Operand::LiteralBit32(2)]);

        let ftype = b.type_function(void, vec![]);
        let func = b.begin_function(void, None, FunctionControl::NONE, ftype).unwrap();
        b.entry_point(ExecutionModel::Vertex, func, "main", []);
        b.begin_block(None).unwrap();
        b.ret().unwrap();
        b.end_function().unwrap();

        b.module().clone().assemble()
    }

    #[test]
    fn relocates_colliding_outputs() {
        let spirv = build_collision_module();
        let out = deduplicate_stage_output_locations(&spirv).expect("pass");
        assert_eq!(out.rewritten, 1, "one variable should be relocated");
        assert_ne!(out.spirv, spirv, "output must differ from input");

        let (_, locations, _) = collect_output_locations(&loader_module(&out.spirv));
        let values: Vec<u32> = locations.values().copied().collect();
        assert_eq!(values.len(), 2);
        assert_ne!(values[0], values[1], "locations must be unique");
        assert!(values.contains(&0), "first variable keeps location 0");
    }

    #[test]
    fn noop_without_collisions() {
        let spirv = build_collision_module();
        let out = deduplicate_stage_output_locations(&spirv).expect("pass");
        let out2 = deduplicate_stage_output_locations(&out.spirv).expect("pass");
        assert_eq!(out2.rewritten, 0, "second pass must be a no-op");
        assert_eq!(out2.spirv, out.spirv, "idempotent");
    }

    #[test]
    fn invalid_input_errors() {
        assert!(deduplicate_stage_output_locations(&[]).is_err());
    }

    fn loader_module(spirv: &[u32]) -> Module {
        let mut loader = rspirv::dr::Loader::new();
        rspirv::binary::parse_words(spirv, &mut loader).expect("parse");
        loader.module()
    }
}
