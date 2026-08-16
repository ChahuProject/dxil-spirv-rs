//! NON-UPSTREAM EXTENSION — HLSL-compatible cbuffer layout normalization.
//!
//! **This module is not part of upstream dxil-spirv.** It is added by
//! dxil-spirv-rs and only exists when the `non-upstream-hlsl-compat` crate
//! feature is enabled (off by default). The upstream C++ library is never
//! modified; this is a pure SPIR-V post-processing step on the converted
//! output.
//!
//! # Why this exists
//!
//! Upstream dxbc-spirv can emit a constant buffer as a *scalar view*: a
//! `struct { float[N] }` member with `ArrayStride 4`. This is valid std140,
//! but the spirv-cross2 **HLSL** backend cannot express it — its cbuffer
//! model is vec4-register based — and rejects the module with
//! "cannot be expressed with either HLSL packing layout or packoffset".
//! GLSL/MSL/JSON decompilation of the same shaders succeeds.
//!
//! [`vec4_align_cbuffers`] rewrites such views to `struct { float4[N/4] }`
//! (`ArrayStride 16`) and rewrites every access chain accordingly, producing
//! SPIR-V that the HLSL backend can consume. Both layouts describe exactly
//! the same bytes, so the transformation is layout- and semantics-preserving.
//!
//! See `docs/non-upstream/hlsl-compat-rationale.md` in the repository for the
//! full reproduction steps, root-cause analysis, and validation.

pub mod detect;
mod error;
pub mod ir;
mod rewrite;

pub use detect::{AccessUse, CbufferTarget, Skipped, Stride4View, Vec4Alias};
pub use error::HlslCompatError;
pub use ir::{analyze, ModuleInfo, ScalarKind, Variable};

use rspirv::binary::Assemble;

/// Result of a [`vec4_align_cbuffers`] run.
#[derive(Debug)]
pub struct NormalizeOutput {
    /// The normalized SPIR-V words.
    pub spirv: Vec<u32>,
    /// Number of cbuffer views that were rewritten or merged.
    pub rewritten: usize,
    /// Scalar views that were found but left untouched (with reasons).
    pub skipped: Vec<Skipped>,
}

/// Non-upstream extension: rewrites stride-4 scalar cbuffer views into
/// vec4-aligned form so the result can be decompiled to HLSL.
///
/// * **Input**: any SPIR-V words (typically the output of
///   `dxil_spirv::convert_to_spirv`). Not a SPIR-V module → error.
/// * **Output**: the same module with Uniform-block `float[N]` (stride 4)
///   cbuffer views replaced by `float4[N/4]` (stride 16), and every access
///   chain into them rewritten (`[member, i]` → `[member, i/4, i%4]`;
///   dynamic indices become `OpUDiv`/`OpUMod`).
/// * **Merging**: if the same cbuffer also exists as a vec4 view (same
///   descriptor binding, same byte size), the scalar view's access chains are
///   redirected into it and the duplicate variable is dropped.
/// * **Skipping**: views that cannot be rewritten safely (non-32-bit scalars,
///   length not divisible by 4, vector loads off the scalar array, unsafe
///   references, …) are left untouched; the pass never fails because of one.
///
/// The pass is **idempotent** and a **no-op** for modules without matching
/// views (the input words are returned unchanged). The byte layout of every
/// buffer is preserved, so the transformation is semantics-preserving: GLSL
/// and MSL decompilation results are unaffected.
pub fn vec4_align_cbuffers(spirv: &[u32]) -> Result<NormalizeOutput, HlslCompatError> {
    let mut loader = rspirv::dr::Loader::new();
    rspirv::binary::parse_words(spirv, &mut loader).map_err(|e| {
        HlslCompatError::InvalidSpirv(format!("failed to parse input module: {e}"))
    })?;
    let mut module = loader.module();

    let info = ir::analyze(&module);
    let (targets, skipped) = detect::find_targets(&module, &info);

    if targets.is_empty() {
        // No-op: hand the input back verbatim.
        return Ok(NormalizeOutput {
            spirv: spirv.to_vec(),
            rewritten: 0,
            skipped,
        });
    }

    let rewritten = rewrite::apply(&mut module, &targets)?;
    let output = module.assemble();

    Ok(NormalizeOutput {
        spirv: output,
        rewritten,
        skipped,
    })
}
