//! Build script: compiles the vendored `dxil-spirv` C++ library via CMake and
//! generates Rust FFI bindings from `dxil_spirv_c.h` via bindgen.
//!
//! The upstream CMake project provides a `dxil-spirv-c-static` target that
//! bundles the full DXIL/DXBC → SPIR-V converter (including the embedded
//! `dxbc-spirv` fallback for SM4/SM5 containers). We build it as a static
//! library and link it into the final Rust artifact.

use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let upstream = manifest_dir.join("dxil-spirv");

    if !upstream.join("CMakeLists.txt").exists() {
        panic!(
            "dxil-spirv submodule is not initialized at `{}`.\n\
             Run: git submodule update --init --recursive",
            upstream.display()
        );
    }

    let dst = build_with_cmake(&upstream);
    link_static(&dst);
    generate_bindings(&upstream, &manifest_dir);

    println!("cargo:rerun-if-changed=build.rs");
}

/// Configure and build the upstream CMake project, returning the CMake
/// install prefix that contains the compiled static libraries.
fn build_with_cmake(upstream: &Path) -> PathBuf {
    let mut cfg = cmake::Config::new(upstream);

    // Match the Rust profile so the C++ runtime (CRT) is consistent with the
    // Rust side: debug Rust links MSVCRTD, release Rust links MSVCRT. Mixing
    // them produces unresolved `_CrtDbgReport` / `_calloc_dbg` symbols.
    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".to_string());
    let build_type = if profile == "debug" {
        "Debug"
    } else {
        "Release"
    };

    cfg.define("DXIL_SPIRV_CLI", "OFF")
        .define("DXIL_SPIRV_NATIVE_LLVM", "OFF")
        .define("CMAKE_BUILD_TYPE", build_type)
        // Use the dynamic CRT (/MD or /MDd) to match Rust's default linkage.
        .define(
            "CMAKE_MSVC_RUNTIME_LIBRARY",
            "MultiThreaded$<$<CONFIG:Debug>:Debug>DLL",
        )
        .profile(build_type)
        // Only build what we need: the static C API target.
        .build_target("dxil-spirv-c-static")
        .cxxflag("/EHsc"); // MSVC: enable C++ exceptions (dxil-spirv needs them)

    cfg.build()
}

/// Emit link directives for the dxil-spirv static libraries.
fn link_static(dst: &Path) {
    // CMake places built libraries under <dst>/build (target dirs vary).
    let build_dir = dst.join("build");

    for search_dir in [&build_dir, dst] {
        println!("cargo:rustc-link-search=native={}", search_dir.display());
        // CMake on MSVC emits per-config subdirectories.
        for config in ["Release", "Debug", "RelWithDebInfo"] {
            println!(
                "cargo:rustc-link-search=native={}",
                search_dir.join(config).display()
            );
        }
    }

    // Recursively register every subdirectory that may hold a .lib, since the
    // upstream targets (dxil-converter, spirv-module, dxil-utils, dxil-debug,
    // third_party/…) land in their own build folders.
    register_lib_dirs(&build_dir);

    // Order matters for static linking: dependents come before their
    // dependencies. dxil-converter pulls in the LLVM bitcode reader
    // (llvm-bc / bc-decoder), the DXBC fallback (dxbc-spirv), and the
    // glslang SPIR-V builder used by spirv-module.
    for lib in [
        "dxil-spirv-c-static",
        "dxil-converter",
        "spirv-module",
        "dxil-utils",
        "dxil-debug",
        "dxbc-spirv",
        "glslang-spirv-builder",
        "llvm-bc",
        "bc-decoder",
    ] {
        println!("cargo:rustc-link-lib=static={lib}");
    }
}

/// Recursively walk `dir` and register any directory containing `.lib` files
/// as a native link search path.
fn register_lib_dirs(dir: &Path) {
    if !dir.is_dir() {
        return;
    }
    let mut has_lib = false;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                register_lib_dirs(&path);
            } else if path.extension().is_some_and(|e| e == "lib") {
                has_lib = true;
            }
        }
    }
    if has_lib {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }
}

/// Run bindgen against the upstream C header and write the bindings into
/// `OUT_DIR`.
fn generate_bindings(upstream: &Path, manifest_dir: &Path) {
    let header = upstream.join("dxil_spirv_c.h");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut builder = bindgen::Builder::default()
        .header(header.display().to_string())
        .clang_arg(format!("-I{}", upstream.display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_function("dxil_spv_.*")
        .allowlist_type("dxil_spv_.*")
        .allowlist_var("DXIL_SPV_.*")
        .derive_debug(true)
        .derive_default(true);

    // The upstream dxil_spirv_c.h gates some API surface behind
    // preprocessor macros. The corresponding features are always compiled
    // into the C++ library (upstream dxil_spirv_c.cpp hardcodes the
    // #defines), but bindgen cannot see them unless we pass the same
    // defines here. Keep this list in sync with dxil_spirv_c.cpp.
    //
    // NOTE: If upstream ever makes these conditional in the .cpp as well,
    // we must also add matching CMake compile definitions in
    // build_with_cmake() and gate the safe layer with cargo features.
    builder = builder
        .clang_arg("-DDXIL_SPV_ENABLE_EXPERIMENTAL_WORKGRAPHS")
        .clang_arg("-DDXIL_SPV_ENABLE_EXPERIMENTAL_MULTIVIEW");

    let bindings = builder
        .generate()
        .expect("bindgen failed on dxil_spirv_c.h");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("failed to write bindings.rs");

    // Keep a copy for local inspection.
    let _ = std::fs::create_dir_all(manifest_dir.join("generated"));
    let _ = bindings.write_to_file(manifest_dir.join("generated").join("bindings.rs"));
}
