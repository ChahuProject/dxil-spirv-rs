//! NON-UPSTREAM extension e2e tests for the hlsl-compat cbuffer normalization.
//!
//! These tests are **not** part of the upstream-facing test suite. They only
//! run when the `non-upstream-hlsl-compat` feature is enabled:
//!
//! ```text
//! cargo test -p dxil-spirv-tests --features non-upstream-hlsl-compat --test non_upstream::hlsl_compat_e2e
//! ```
//!
//! Scope:
//! * target shaders (previously failing HLSL) must compile to HLSL after the
//!   pass;
//! * shaders without stride-4 cbuffer views must be returned verbatim
//!   (idempotent no-op);
//! * no shader may regress: GLSL compilation must still succeed after the
//!   pass.

#![cfg(feature = "non-upstream-hlsl-compat")]

use dxil_spirv::non_upstream::hlsl_compat;
use spirv_cross2::compile::hlsl::HlslShaderModel;
use spirv_cross2::compile::CompilableTarget;
use spirv_cross2::targets::Hlsl;
use spirv_cross2::{Compiler, Module};
use std::path::Path;

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn convert_dxil(rel: &str) -> Result<Vec<u32>, String> {
    let path = workspace_root().join("tests/shaders").join(rel);
    let blob = std::fs::read(&path).map_err(|e| format!("read {rel}: {e}"))?;
    let parsed = dxil_spirv::ParsedBlob::parse(&blob).map_err(|e| format!("parse: {e}"))?;
    let converter = dxil_spirv::Converter::new(&parsed).map_err(|e| format!("new: {e}"))?;
    converter.run().map_err(|e| format!("run: {e}"))?;
    converter.compiled_spirv().map_err(|e| format!("spirv: {e}"))
}

fn hlsl_ok(spirv: &[u32]) -> bool {
    let module = Module::from_words(spirv);
    let Ok(compiler) = Compiler::<Hlsl>::new(module) else {
        return false;
    };
    let mut options = Hlsl::options();
    options.shader_model = HlslShaderModel::ShaderModel5_1;
    compiler.compile(&options).is_ok()
}

fn glsl_ok(spirv: &[u32]) -> bool {
    let module = Module::from_words(spirv);
    let Ok(compiler) = Compiler::<spirv_cross2::targets::Glsl>::new(module) else {
        return false;
    };
    let options = spirv_cross2::targets::Glsl::options();
    compiler.compile(&options).is_ok()
}

/// Shaders whose HLSL compilation previously failed with the cbuffer layout
/// error ("cannot be expressed with either HLSL packing layout or
/// packoffset"). These must compile to HLSL after `vec4_align_cbuffers`.
const TARGET_SHADERS: &[&str] = &[
    "alloca-opts/float4-array-load.dxil",
    "alloca-opts/float4-array-load.bindless.dxil",
    "alloca-opts/float4-array-load.bindless.root-constants.dxil",
    "alloca-opts/float4-array-load.root-constant.dxil",
    "alloca-opts/float4-array-load.root-descriptor.dxil",
    "alloca-opts/float4-array-load.root-descriptor.root-constants.dxil",
    "alloca-opts/load-different.dxil",
    "alloca-opts/matrix-load.dxil",
    "alloca-opts/out-of-order-load.dxil",
    "alloca-opts/uint4-array-load.dxil",
];

#[test]
fn target_shaders_compile_to_hlsl_after_normalization() {
    let mut rewritten_total = 0;
    let mut skipped_total = 0;
    for rel in TARGET_SHADERS {
        let spirv = convert_dxil(rel).unwrap_or_else(|e| panic!("{rel}: {e}"));
        // Sanity: these shaders must fail before the pass.
        if hlsl_ok(&spirv) {
            eprintln!("NOTE: {rel} already compiles to HLSL; keeping as regression guard");
        }
        let out = hlsl_compat::vec4_align_cbuffers(&spirv)
            .unwrap_or_else(|e| panic!("{rel}: pass failed: {e}"));
        rewritten_total += out.rewritten;
        skipped_total += out.skipped.len();
        assert!(
            hlsl_ok(&out.spirv),
            "{rel}: HLSL still fails after vec4_align_cbuffers (rewritten={}, skipped={})",
            out.rewritten,
            out.skipped.len()
        );
        // The pass must not break GLSL.
        assert!(glsl_ok(&out.spirv), "{rel}: GLSL broken after pass");
    }
    assert!(
        rewritten_total > 0,
        "expected at least one rewritten cbuffer view across target shaders"
    );
    eprintln!(
        "target shaders: rewritten views total={rewritten_total}, skipped total={skipped_total}"
    );
}

#[test]
fn noop_on_clean_modules() {
    // Shaders without stride-4 cbuffer views: the pass must return the input
    // words verbatim (idempotent no-op).
    for rel in ["alloca-opts/double-array-load.dxil", "alloca-opts/bad-stride.dxil"] {
        let path = workspace_root().join("tests/shaders").join(rel);
        if !path.exists() {
            eprintln!("skipping {rel}: not compiled");
            continue;
        }
        let spirv = convert_dxil(rel).expect("convert");
        let out = hlsl_compat::vec4_align_cbuffers(&spirv).expect("pass");
        assert_eq!(out.rewritten, 0, "{rel}: unexpectedly rewritten");
        assert_eq!(out.spirv, spirv, "{rel}: no-op must return input verbatim");
    }
}

#[test]
fn alloca_opts_never_regress() {
    // Every alloca-opts shader: after the pass, HLSL must not go from
    // "compiles" to "fails", and GLSL must still compile.
    let dir = workspace_root().join("tests/shaders/alloca-opts");
    let entries = std::fs::read_dir(&dir).expect("read alloca-opts dir");
    let mut checked = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("dxil") {
            continue;
        }
        let rel = format!("alloca-opts/{}", path.file_name().unwrap().to_string_lossy());
        let spirv = convert_dxil(&rel).expect("convert");
        let before_ok = hlsl_ok(&spirv);
        let before_glsl = glsl_ok(&spirv);
        let out = hlsl_compat::vec4_align_cbuffers(&spirv).expect("pass");
        let after_ok = hlsl_ok(&out.spirv);
        let after_glsl = glsl_ok(&out.spirv);
        assert!(
            !before_ok || after_ok,
            "{rel}: HLSL regressed (ok -> fail) after pass"
        );
        assert!(
            !before_glsl || after_glsl,
            "{rel}: GLSL regressed (ok -> fail) after pass"
        );
        checked += 1;
    }
    assert!(checked >= 10, "expected to check >= 10 shaders, checked {checked}");
}
