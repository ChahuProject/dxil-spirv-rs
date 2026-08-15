//! Compile-time + runtime guard: every exported `dxil_spv_*` function from
//! the upstream C API must be either wrapped by the safe layer or explicitly
//! listed in `KNOWN_MISSING` with a justification.
//!
//! This test **fails hard** when a new upstream version adds a function that
//! we have not wrapped, preventing silent API-surface drift.
//!
//! ## How it works
//!
//! 1. Reads `dxil_spirv_c.h` (the vendored upstream header) and extracts every
//!    `dxil_spv_*` function name marked `DXIL_SPV_PUBLIC_API`.
//! 2. Reads every `.rs` file under `dxil-spirv/src/` and searches for a
//!    textual reference to each upstream function name.
//! 3. Any upstream function **not** referenced in the safe layer **and** not
//!    in `KNOWN_MISSING` causes a test failure.
//!
//! ## Maintenance protocol
//!
//! - **New upstream function appears?** The test fails. Either wrap it in
//!   the safe layer (preferred) or add it to `KNOWN_MISSING` with a comment
//!   explaining why it is intentionally skipped.
//! - **Wrapped a previously-missing function?** Remove it from
//!   `KNOWN_MISSING` — the list should only shrink over time.
//! - **Deleting from `KNOWN_MISSING` without wrapping** is a regression;
//!   the test will catch it on the next run.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Functions we are **deliberately** not wrapping yet, with justification.
///
/// Each entry is `(function_name, reason)`.
/// **Goal: this list should shrink to zero.** Every entry is technical debt.
///
/// When you wrap one of these functions in the safe layer, delete its entry.
/// When upstream adds a new function, this test will fail until you either
/// wrap it or add it here with a reason.
const KNOWN_MISSING: &[(&str, &str)] = &[
    // ── Thread allocator context ────────────────────────────────────────
    // These manage a thread-local custom allocator for the C++ side.
    // Only needed if you want to plug in a custom heap (e.g. for embedded
    // or arena allocation). Rust's global allocator already covers the
    // common case, so these are low-priority.
    (
        "dxil_spv_begin_thread_allocator_context",
        "custom allocator hook; Rust global allocator suffices for typical use",
    ),
    (
        "dxil_spv_end_thread_allocator_context",
        "custom allocator hook; see begin_thread_allocator_context",
    ),
    (
        "dxil_spv_reset_thread_allocator_context",
        "custom allocator hook; see begin_thread_allocator_context",
    ),
    // ── Thread log callback ─────────────────────────────────────────────
    // C-style variadic log callback from the upstream library.
    // Useful for debugging but not essential for conversion correctness.
    // Would need a `log` crate bridge to be idiomatic.
    (
        "dxil_spv_set_thread_log_callback",
        "logging hook; low priority, needs `log` crate bridge design",
    ),
    // ── dxil_spv_parse_dxil (indirectly covered) ───────────────────────
    // `dxil_spv_parse_dxil_blob` (which we wrap) calls this internally for
    // raw DXIL bitcode. We expose the blob-level API which is the public
    // entry point; `parse_dxil` is a lower-level detail.
    (
        "dxil_spv_parse_dxil",
        "indirectly covered via dxil_spv_parse_dxil_blob",
    ),
    // ── Root signature / descriptor mapping (SM5.1+ root signatures) ────
    // These map D3D12 root signature concepts (root constants, root
    // descriptors, descriptor tables) to Vulkan descriptor set/binding
    // assignments. Essential for correct conversion of shaders that use
    // explicit root signatures beyond simple binding.
    //
    // TODO: wrap all of these — they are the highest-priority gap.
    (
        "dxil_spv_converter_set_root_constant_word_count",
        "TODO: root signature support — high priority",
    ),
    (
        "dxil_spv_converter_set_root_descriptor_count",
        "TODO: root signature support — high priority",
    ),
    (
        "dxil_spv_converter_add_local_root_constants",
        "TODO: root signature support — high priority",
    ),
    (
        "dxil_spv_converter_add_local_root_descriptor",
        "TODO: root signature support — high priority",
    ),
    (
        "dxil_spv_converter_add_local_root_descriptor_table",
        "TODO: root signature support — high priority",
    ),
    (
        "dxil_spv_converter_begin_local_root_descriptor_table",
        "TODO: root signature support — high priority",
    ),
    (
        "dxil_spv_converter_end_local_root_descriptor_table",
        "TODO: root signature support — high priority",
    ),
    (
        "dxil_spv_converter_add_root_parameter_mapping",
        "TODO: root signature support — high priority",
    ),
    (
        "dxil_spv_converter_add_root_descriptor_mapping",
        "TODO: root signature support — high priority",
    ),
    // ── Work Graphs / mesh node (SM6.8+) ────────────────────────────────
    // Node input/output introspection for Work Graphs and mesh shader
    // node pipelines. Only relevant for cutting-edge SM6.8 workloads.
    (
        "dxil_spv_parsed_blob_get_entry_point_node_input",
        "SM6.8 Work Graphs; niche feature",
    ),
    (
        "dxil_spv_parsed_blob_get_entry_point_num_node_outputs",
        "SM6.8 Work Graphs; niche feature",
    ),
    (
        "dxil_spv_parsed_blob_get_entry_point_node_output",
        "SM6.8 Work Graphs; niche feature",
    ),
    // ── Resource scanning ───────────────────────────────────────────────
    // Callback-based resource enumeration on the parsed blob before
    // conversion. Users can achieve the same by running the converter and
    // inspecting the SPIR-V, so this is a convenience API.
    (
        "dxil_spv_parsed_blob_scan_resources",
        "convenience API; same info available post-conversion",
    ),
    // ── Multiview compatibility ─────────────────────────────────────────
    (
        "dxil_spv_converter_is_multiview_compatible",
        "niche Vulkan multiview check; low priority",
    ),
    // ── RDAT subobjects (State Objects / DXR) ───────────────────────────
    // RDAT (Runtime Data) subobject extraction for raytracing state
    // objects. Only relevant for DXR pipelines.
    (
        "dxil_spv_parsed_blob_get_num_rdat_subobjects",
        "DXR state object introspection; niche feature",
    ),
    (
        "dxil_spv_parsed_blob_get_rdat_subobject",
        "DXR state object introspection; niche feature",
    ),
    // ── Non-semantic debug info ─────────────────────────────────────────
    (
        "dxil_spv_converter_add_non_semantic_debug_info",
        "debug info passthrough; low priority",
    ),
    // ── Meta descriptor ─────────────────────────────────────────────────
    (
        "dxil_spv_converter_set_meta_descriptor",
        "meta descriptor API; low priority",
    ),
];

/// Returns the path to the vendored `dxil_spirv_c.h` relative to the
/// workspace root (which is the parent of `CARGO_MANIFEST_DIR` since we
/// are in the `dxil-spirv` member crate).
fn upstream_header_path() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    Path::new(&manifest_dir)
        .parent()
        .expect("workspace root")
        .join("dxil-spirv-sys")
        .join("dxil-spirv")
        .join("dxil_spirv_c.h")
}

/// Extract all `dxil_spv_*` function names declared with
/// `DXIL_SPV_PUBLIC_API` in the upstream header.
fn extract_upstream_exports(header_text: &str) -> BTreeSet<String> {
    let mut exports = BTreeSet::new();
    // Match patterns like:
    //   DXIL_SPV_PUBLIC_API void dxil_spv_...(args);
    //   DXIL_SPV_PUBLIC_API dxil_spv_result dxil_spv_...(args);
    //   DXIL_SPV_PUBLIC_API const char *dxil_spv_...(args);
    //
    // The function name always starts with `dxil_spv_` and is followed by `(`.
    for line in header_text.lines() {
        let line = line.trim();
        if !line.starts_with("DXIL_SPV_PUBLIC_API") {
            continue;
        }
        // Find the function name: the last `dxil_spv_*` token before `(`.
        if let Some(paren_pos) = line.find('(') {
            let before_paren = &line[..paren_pos];
            // Walk backwards from the end to find the function name.
            if let Some(name_start) = before_paren.rfind("dxil_spv_") {
                let name = before_paren[name_start..].trim_end_matches(|c: char| c == '*' || c == ' ');
                if !name.is_empty() {
                    exports.insert(name.to_string());
                }
            }
        }
    }
    exports
}

/// Read all `.rs` source files under `dxil-spirv/src/` and return their
/// concatenated content for symbol searching.
fn read_safe_layer_sources() -> String {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let src_dir = Path::new(&manifest_dir).join("src");
    let mut combined = String::new();
    read_rs_files_recursive(&src_dir, &mut combined);
    combined
}

fn read_rs_files_recursive(dir: &Path, out: &mut String) {
    if !dir.is_dir() {
        return;
    }
    for entry in fs::read_dir(dir).expect("failed to read src directory") {
        let entry = entry.expect("failed to read directory entry");
        let path = entry.path();
        if path.is_dir() {
            read_rs_files_recursive(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
            out.push_str(&content);
            out.push('\n');
        }
    }
}

/// Check whether a symbol name appears in the source text as a word
/// (not as part of a longer identifier).
fn source_references_symbol(source: &str, symbol: &str) -> bool {
    for (i, _) in source.match_indices(symbol) {
        let before_ok = i == 0
            || !source.as_bytes()[i - 1].is_ascii_alphanumeric() && source.as_bytes()[i - 1] != b'_';
        let after = i + symbol.len();
        let after_ok = after >= source.len()
            || !source.as_bytes()[after].is_ascii_alphanumeric() && source.as_bytes()[after] != b'_';
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

#[test]
fn all_upstream_exports_are_wrapped_or_documented() {
    let header_path = upstream_header_path();
    let header_text = fs::read_to_string(&header_path).unwrap_or_else(|e| {
        panic!(
            "failed to read upstream header at {}: {e}\n\
             Is the submodule initialized? Run: git submodule update --init --recursive",
            header_path.display()
        )
    });

    let exports = extract_upstream_exports(&header_text);
    assert!(
        !exports.is_empty(),
        "no upstream exports found — is the header path correct? ({})",
        header_path.display()
    );

    let known_missing: BTreeSet<&str> = KNOWN_MISSING.iter().map(|(name, _)| *name).collect();

    let source = read_safe_layer_sources();

    let mut unwrapped_and_undocumented = Vec::new();

    for func in &exports {
        let is_wrapped = source_references_symbol(&source, func);
        let is_known_missing = known_missing.contains(func.as_str());

        if !is_wrapped && !is_known_missing {
            unwrapped_and_undocumented.push(func.clone());
        }
    }

    if !unwrapped_and_undocumented.is_empty() {
        let mut msg = String::from(
            "\n\nThe following upstream dxil_spv_* functions are neither wrapped \
             in the safe layer nor listed in KNOWN_MISSING:\n\n",
        );
        for f in &unwrapped_and_undocumented {
            msg.push_str(&format!("  - {f}\n"));
        }
        msg.push_str(
            "\nTo fix: either wrap them in the safe layer, or add them to \
             KNOWN_MISSING in tests/api_coverage.rs with a justification.\n",
        );
        panic!("{msg}");
    }

    // Also verify that every KNOWN_MISSING entry actually exists in the
    // upstream header (guard against typos and stale entries after
    // upstream removes a function).
    for (name, _reason) in KNOWN_MISSING {
        assert!(
            exports.contains(*name),
            "KNOWN_MISSING entry '{name}' does not match any upstream export.\n\
             Did upstream remove it? Or is there a typo?\n\
             Remove stale entries from KNOWN_MISSING."
        );
    }

    // Verify that KNOWN_MISSING entries are NOT accidentally wrapped
    // (if someone wraps a function but forgets to remove it from the list,
    // that's harmless but should be cleaned up).
    let mut wrapped_but_still_listed = Vec::new();
    for (name, _) in KNOWN_MISSING {
        if source_references_symbol(&source, name) {
            wrapped_but_still_listed.push(*name);
        }
    }
    if !wrapped_but_still_listed.is_empty() {
        let mut msg = String::from(
            "\n\nThe following KNOWN_MISSING entries are now referenced in the safe layer.\n\
             Please remove them from KNOWN_MISSING:\n\n",
        );
        for f in &wrapped_but_still_listed {
            msg.push_str(&format!("  - {f}\n"));
        }
        // This is a soft warning, not a hard failure — the important thing
        // is that all upstream functions are covered. But keeping the list
        // clean prevents confusion.
        eprintln!("{msg}");
    }
}
