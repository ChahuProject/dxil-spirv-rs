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

    cfg.define("DXIL_SPIRV_CLI", "OFF")
        .define("DXIL_SPIRV_NATIVE_LLVM", "OFF")
        .define("CMAKE_BUILD_TYPE", "Release")
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

    for lib in [
        "dxil-spirv-c-static",
        "dxil-converter",
        "spirv-module",
        "dxil-utils",
        "dxil-debug",
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
            } else if path.extension().map_or(false, |e| e == "lib") {
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

    let bindings = bindgen::Builder::default()
        .header(header.display().to_string())
        .clang_arg(format!("-I{}", upstream.display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_function("dxil_spv_.*")
        .allowlist_type("dxil_spv_.*")
        .allowlist_var("DXIL_SPV_.*")
        .derive_debug(true)
        .derive_default(true)
        .generate()
        .expect("bindgen failed on dxil_spirv_c.h");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("failed to write bindings.rs");

    // Keep a copy for local inspection.
    let _ = std::fs::create_dir_all(manifest_dir.join("generated"));
    let _ = bindings.write_to_file(manifest_dir.join("generated").join("bindings.rs"));
}
