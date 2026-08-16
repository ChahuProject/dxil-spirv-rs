# Developer Architecture

[English](architecture.md) | [中文](architecture.zh-CN.md)

This document describes the internal architecture of `dxil-spirv-rs`, including crate topology, the native build and static linking pipeline, FFI boundary rules, lifetime management patterns, thread safety contracts, test harness infrastructure, and recorded platform lessons.

## 1. Crate Topology

The workspace separates concerns across three crates, following the pattern established by modern Rust native binding ecosystems such as `spirv-cross2-rs` and `spirv_cross`:

```text
dxil-spirv (safe idiomatic Rust wrapper)
    │
    ▼ (depends on)
dxil-spirv-sys (build.rs + CMake compilation + raw bindgen FFI)
    │
    ▼ (compiles and links)
dxil-spirv (vendored C++ submodule at dxil-spirv-sys/dxil-spirv)

dxil-spirv-tests (out-of-tree end-to-end test suite + DXC compilation harness)
```

### `dxil-spirv-sys`

`dxil-spirv-sys` is the low-level FFI substrate:
- Submodule: Vendors upstream `HansKristian-Work/dxil-spirv` at `dxil-spirv-sys/dxil-spirv`.
- Build script: `build.rs` orchestrates CMake compilation of the `dxil-spirv-c-static` target, emits link search flags for all constituent static libraries, links the platform C++ standard library, and executes `bindgen` on `dxil_spirv_c.h`.
- Output: Emits raw bindings into `OUT_DIR/bindings.rs` and copies a mirror to `dxil-spirv-sys/generated/bindings.rs` for offline inspection.

### `dxil-spirv`

`dxil-spirv` is the safe, idiomatic public interface:
- RAII wrappers: Encapsulates raw C handles in `ParsedBlob` (`dxil_spv_parsed_blob`) and `Converter` (`dxil_spv_converter`), ensuring memory is freed when dropped.
- Typed conversions: Maps C enums and tagged option structs to Rust enums like `ShaderStage` and `ConverterOption`.
- Trampoline layer: Bridges safe Rust `FnMut` closures to C remapper callbacks using a double-box allocation pattern with panic catches.
- Error handling: Converts `dxil_spv_result` status codes into typed `Result<T, Error>` via `thiserror`.

### `dxil-spirv-tests`

`dxil-spirv-tests` is the verification harness:
- Test dataset: Synchronizes 829 test shaders from the upstream submodule.
- DXC integration: Automates downloading and running the DirectX Shader Compiler (`dxc.exe`) on Windows to compile HLSL sources into DXIL bitcode blobs.
- Subprocess isolation: Executes each shader conversion test in a separate child process so upstream C++ assertions do not terminate the entire test runner.

## 2. Native Build Pipeline and Static Link Closure

Upstream `dxil-spirv` is a CMake project written in C++17. `dxil-spirv-sys/build.rs` compiles the static C API target using the `cmake` crate.

### Comparison with Reference Binding Crates

The binding architecture combines ideas from two mature predecessors:
- `grovesNL/spirv_cross`: Uses a classic build script and raw FFI layer. Unlike `spirv_cross` which builds vendored source with the `cc` crate, `dxil-spirv-sys` uses the `cmake` crate because upstream relies on CMake target graphs and subproject definitions.
- `SnowflakePowered/spirv-cross2-rs`: Demonstrates modern soundness patterns, strict `-sys` separation, Arc-guarded context lifetimes, and upstream-pinned semver metadata.

### Static Link Closure

The `dxil-spirv-c-static` target depends on multiple internal static libraries. Static linkers resolve symbols sequentially, so libraries must be declared in strict dependent-before-dependency order.

The exact 9-library link closure is:

```text
1. dxil-spirv-c-static      (C API export surface)
2. dxil-converter           (Core DXIL to SPIR-V translation logic)
3. spirv-module             (SPIR-V binary instruction builder)
4. dxil-utils               (Shared utilities and container parsers)
5. dxil-debug               (Disassembly and debug printing)
6. dxbc-spirv               (Legacy DXBC SM4/SM5 translation fallback)
7. glslang-spirv-builder    (SPIR-V AST construction backend)
8. llvm-bc                  (LLVM bitcode container reader)
9. bc-decoder               (Low-level bitstream decoder)
```

If the linker reports unresolved `LLVMBC::*` or `spv::Builder::*` symbols, a library is missing or mis-ordered in this list.

Two upstream targets are deliberately omitted from this closure:
- `dxil-spirv-headers`: Header-only CMake interface target; generates no archive file.
- `spirv-cross` / `spirv-tools`: Optional upstream CLI tool dependencies; not part of the library link closure.

### Library Discovery Across Configurations

CMake builds static targets inside distinct subdirectories. On Windows (MSVC), output lands in per-config directories such as `Release/` or `Debug/`. On Unix systems, individual static archives land in per-target build subtrees.

`build.rs` implements `register_lib_dirs()`, which walks the CMake build output directory recursively. Whenever a directory containing `.lib` (Windows) or `.a` (Linux/macOS) archives is found, it emits a `cargo:rustc-link-search=native=<dir>` directive.

### Bindgen Workflow and Layout Verification

`build.rs` generates Rust bindings at compile time:
- Header: `dxil-spirv-sys/dxil-spirv/dxil_spirv_c.h`.
- Allowlists: Functions matching `dxil_spv_.*`, types matching `dxil_spv_.*`, and variables matching `DXIL_SPV_.*`.
- Layout tests: Bindgen struct layout tests remain enabled. Unlike `spirv_cross` which disables layout tests, `dxil-spirv-sys` keeps them because the upstream struct set is stable. If upstream alters field alignment or padding, these tests fail immediately during `cargo test`.

## 3. FFI Boundary Rules

Interfacing Rust with the upstream C API requires specific boundary handling rules:

### Casted Macro Constants

Macros defined with type casts, such as `#define DXIL_SPV_TRUE ((dxil_spv_bool)1)`, are skipped by bindgen. Rust code must not attempt to reference `sys::DXIL_SPV_TRUE` or `sys::DXIL_SPV_FALSE`. Instead, pass integer literals `1` or `0` typed as `sys::dxil_spv_bool` (`u8`).

### Anonymous Unions

Bindgen models C anonymous unions as nested private union types that cannot be constructed via struct literals. When converting safe structs to raw FFI structs:
1. Initialize the struct via `Default::default()`.
2. Assign the target fields explicitly.
3. Annotate the `From` implementation with `#[allow(clippy::field_reassign_with_default)]`.

### Enum Signedness Normalization

Depending on target platform headers, Clang and bindgen emit C enums as signed integers (`c_int` / `i32`) or unsigned integers (`c_uint` / `u32`). For example, enums containing negative sentinels become signed, while purely positive sets become unsigned.

The safe wrapper normalizes all enum discriminants and option tags to `u32`. Explicit casts like `raw as u32` or `tag as sys::dxil_spv_option` are required on some platforms while being no-ops on others. To avoid compiler warnings on homogeneous targets, `dxil-spirv/src/lib.rs` permits `#![allow(clippy::unnecessary_cast)]` across the entire crate.

## 4. FFI Callback Trampoline Pattern

Upstream `dxil-spirv` provides eight remapper callbacks on `Converter` (SRV, UAV, CBV, sampler, vertex input, stage input, stage output, stream output) plus one on `ParsedBlob::scan_resources`.

`spirv_cross` lacks callback APIs, so `dxil-spirv-rs` establishes a custom double-box trampoline pattern in `dxil-spirv/src/remapper.rs`.

```text
Rust Closure: Box<dyn FnMut(&D3d) -> Option<Vulkan> + Send>
                           │
                 Box::new (outer box)
                           │
                           ▼
                 *mut Box<dyn FnMut...>  <─── Thin Pointer (*mut c_void userdata)
                           │
                 C Library Converter
                           │
                 (invokes trampoline during dxil_spv_converter_run)
                           │
                           ▼
                  extern "C" trampoline
                           │
       ┌───────────────────┴───────────────────┐
       ▼                                       ▼
&mut **(userdata as *mut Box<...>)   catch_unwind block
(dereference twice to get &mut dyn)  (returns 0 / DXIL_SPV_FALSE on panic)
```

### Double-Boxing for Thin Pointers

A trait object `Box<dyn FnMut...>` is a fat pointer consisting of two words (data pointer and vtable pointer). The C API accepts a single `*mut c_void` userdata pointer. Casting a fat pointer directly to `*mut c_void` is invalid in Rust.

The solution wraps the fat pointer inside an outer heap box:
1. Construct `Box<Box<dyn FnMut...>>`.
2. Obtain a thin raw pointer to the outer box: `(&mut *holder.closure) as *mut Box<dyn FnMut...> as *mut c_void`.
3. In the trampoline, cast `userdata` back to `*mut Box<dyn FnMut...>` and dereference twice (`&mut **ptr`) to access the inner closure.

### Lifetime and Ownership Model

The `Converter` instance owns the remapper closures inside an internal `Option<Box<RemapperHolder>>`:
- While conversion runs, the outer box allocation remains fixed in memory at a stable address.
- In `Converter::drop`, `self._remappers.take()` drops the Rust closures first, followed by `dxil_spv_converter_free`. This guarantees C code never holds a dangling userdata pointer.
- `Box::into_raw` is avoided because holding both an active raw pointer and an owned `Box` creates duplicate free risks.

### Panic Boundary Safety

Rust panics must never unwind across an `extern "C"` ABI boundary. Doing so causes undefined behavior or immediate process aborts.

Every callback trampoline encloses the closure invocation inside `std::panic::catch_unwind(std::panic::AssertUnwindSafe(...))`. If a panic occurs:
- The panic is caught within the trampoline.
- The trampoline returns `0` (`DXIL_SPV_FALSE`), signaling failure to the C++ core.

### Keep-Alive Companions

Options passed to `Converter::add_option` may contain raw pointers referencing heap buffers (such as `Vec<u32>` output swizzles or `CString` file paths). The `RawOptionData` enum in `dxil-spirv/src/options.rs` stores these backing allocations alongside the C struct during the FFI call. Since they are held purely for lifetime duration and not read back by Rust, the enum is annotated with `#[allow(dead_code)]`.

## 5. Thread Safety Contract

`dxil-spirv-rs` enforces explicit concurrency invariants:

```rust
// In dxil-spirv/src/converter.rs:
unsafe impl Send for Converter {}
// Sync is deliberately omitted.

// In dxil-spirv/src/parsed_blob.rs:
unsafe impl Send for ParsedBlob {}
// Sync is deliberately omitted.
```

### Why `Send` is Implemented

Upstream conversion is completely synchronous and self-contained. Remapper callbacks execute solely during `dxil_spv_converter_run` on the thread calling that function. There are no background worker threads, no hidden thread-local capture across runs, and no shared global state during conversion. Moving a `Converter` or `ParsedBlob` between threads is safe.

### Why `Sync` is Not Implemented

Concurrent invocations of `dxil_spv_converter_run` or mutating option setters on the same handle from multiple threads cause data races in the C++ object. Therefore, neither `Converter` nor `ParsedBlob` implements `Sync`. Shared access across threads requires external synchronization via `Mutex` or `RwLock`.

Remapper closures only require `Send + 'static`, not `Sync`.

## 6. C Runtime (CRT) and Exception Handling

Windows MSVC builds require precise alignment between Rust and C++ runtime configurations.

### MSVC CRT Matching

`dxil-spirv-sys/build.rs` configures CMake with:

```rust
cfg.define("CMAKE_MSVC_RUNTIME_LIBRARY", "MultiThreaded$<$<CONFIG:Debug>:Debug>DLL");
```

This instructs MSVC to link the dynamic C runtime:
- Debug Rust build (`PROFILE=debug`) links `MSVCRTD.lib` via `Debug` CMake configuration.
- Release Rust build links `MSVCRT.lib` via `Release` CMake configuration.

Mismatches cause unresolved symbol errors at final link time for functions like `_CrtDbgReport` or `_calloc_dbg`.

### Exception Flags and RTTI

Upstream GCC and Clang build flags specify `-fno-exceptions -fno-rtti`, but the library code uses no `try`/`catch`/`throw` statements and replaces native RTTI with custom LLVM-style `isa<>` and `dyn_cast<>` templates.
- On MSVC, `build.rs` passes `/EHsc` to enable structured C++ exception handling and silence STL warning `C4530`.
- On GCC and Clang (Linux/macOS), exception handling is enabled by default for `.cpp` files. Passing `/EHsc` to GCC or Clang causes compiler configuration checks in CMake to fail with `no such file or directory: '/EHsc'`. The flag is gated strictly on `target_env == "msvc"`.

## 7. C++ Standard Library Linking

Rust's compiler links C runtime libraries automatically, but it does not link the C++ standard library when linking static C++ archives.

Upstream `dxil-spirv` relies on `operator new`, `operator delete`, standard containers (`std::vector`, `std::string`, `std::unordered_map`), and RTTI constructs.

To resolve these symbols on non-MSVC toolchains, `dxil-spirv-sys/build.rs` emits explicit dynamic link directives:
- macOS: `println!("cargo:rustc-link-lib=dylib=c++");` (libc++)
- Linux, BSDs, and Unix targets: `println!("cargo:rustc-link-lib=dylib=stdc++");` (libstdc++)
- Windows MSVC: Handled automatically by the default CRT configuration.

For full platform matrix details, refer to [platform-support.md](platform-support.md).

## 8. Cross-Platform Pitfall Ledger

The following table records concrete integration issues, their root causes, and the commits that resolved them:

| Area | Symptom | Root Cause | Fix | Commit |
|---|---|---|---|---|
| Library Discovery | Link failure: unable to find `-ldxbc-spirv` on Linux/macOS | `register_lib_dirs` only checked for `.lib` files, ignoring `.a` archives on Unix | Search for both `.lib` and `.a` extensions in recursive walker | `5163058` |
| Bindgen Types | Compiler error `E0308`: mismatched types in enum matches on Linux | Clang generated `c_uint` on Linux but `c_int` on Windows for enums without negative values | Normalize enums to `u32` and cast explicitly across FFI boundaries | `ce156ac` |
| Clippy Lint | Lint warning `clippy::unnecessary_cast` on Windows targets | Normalizing casts are required on Linux but redundant on Windows | Apply `#![allow(clippy::unnecessary_cast)]` crate-wide in safe layer | `5ecba0b` |
| CMake Compiler Check | CMake configuration failed: `no such file or directory: '/EHsc'` | `/EHsc` is an MSVC-specific flag that breaks GCC and Clang CLI parsers | Gate `/EHsc` behind target check `CARGO_CFG_TARGET_ENV == "msvc"` | `ed19ee0` |
| Linker Symbols | Unresolved `std::__cxx11` / `operator new` on Linux and macOS | rustc links C runtime but does not automatically pull C++ stdlib | Emit `cargo:rustc-link-lib=dylib=stdc++` on Linux and `c++` on macOS | `dde03e4` |
| Test Harness | E2E test failures on Linux/macOS due to missing DXC binary | Microsoft DXC release archive provides Windows `dxc.exe` x64 binary | Skip DXC compilation gracefully on non-Windows platforms or when `DXC_PATH` is unset | `262a41e` |
| CI Cache | Stale C++ object files linked after submodule updates in CI | Caching full `target/` directory retained outdated native archives | Strip fragile `target/` cache from CI workflows; rely on Cargo dependency caching | `6d7757f` |
| Directory Recursion | Link failure: missing symbols from nested subprojects (`llvm-bc`) | CMake nested static targets in separate subdirectories | Recurse into all subfolders in `register_lib_dirs` | `5163058` |

## 9. Experimental and Conditional API Surface

Upstream gates certain C API declarations behind preprocessor defines in `dxil_spirv_c.h`:

| Macro | Functions Exposed | Upstream Default | Rust Wrapper Handling |
|---|---|---|---|
| `DXIL_SPV_ENABLE_EXPERIMENTAL_WORKGRAPHS` | `dxil_spv_parsed_blob_get_entry_point_node_input`, `dxil_spv_parsed_blob_get_entry_point_num_node_outputs`, `dxil_spv_parsed_blob_get_entry_point_node_output` | Enabled in `dxil_spirv_c.cpp` | Passed `-DDXIL_SPV_ENABLE_EXPERIMENTAL_WORKGRAPHS` to bindgen; safe layer always wraps |
| `DXIL_SPV_ENABLE_EXPERIMENTAL_MULTIVIEW` | `dxil_spv_converter_is_multiview_compatible` | Enabled in `dxil_spirv_c.cpp` | Passed `-DDXIL_SPV_ENABLE_EXPERIMENTAL_MULTIVIEW` to bindgen; safe layer always wraps |

Because `dxil_spirv_c.cpp` hardcodes these defines, compiled static libraries always contain the symbols. Passing matching `-D` arguments in `dxil-spirv-sys/build.rs` ensures bindgen emits corresponding function signatures.

### Switch Maintenance Checklist

If upstream alters existing feature flags or introduces new conditional switches:
1. Update bindgen `-D` definitions in `dxil-spirv-sys/build.rs` inside `generate_bindings()`.
2. Update CMake compilation options in `dxil-spirv-sys/build.rs` inside `build_with_cmake()`.
3. Add or adjust corresponding `#[cfg(feature = "...")]` gates in `dxil-spirv/src/`.
4. Update feature declarations in `dxil-spirv/Cargo.toml`.
5. Update documentation in `docs/usage.md` and `docs/architecture.md`.
6. Update `dxil-spirv/tests/api_coverage.rs` to verify new functions are tracked.

## 10. Upstream Versioning and Synchronization

`dxil-spirv-rs` tracks upstream `HansKristian-Work/dxil-spirv`, which operates on a rolling master branch without tagged releases.

### Semver Build Metadata Rule

The crate version in workspace `Cargo.toml` (`[workspace.package]`) uses semver build metadata to record the pinned upstream C API version:

```text
<crate-version>+dxil-spirv.<UPSTREAM_MAJOR.MINOR.PATCH>
Example: 0.1.0+dxil-spirv.2.72.1
```

- The `+dxil-spirv.X.Y.Z` suffix mirrors the `DXIL_SPV_API_VERSION_MAJOR`, `DXIL_SPV_API_VERSION_MINOR`, and `DXIL_SPV_API_VERSION_PATCH` definitions from `dxil_spirv_c.h`.
- Crates.io parses build metadata without using it for version precedence, keeping dependency resolution standard.
- The base version (`0.1.0`) follows standard semver: breaking changes in the safe Rust wrapper trigger minor or major bumps, while internal updates or backward-compatible additions trigger patch bumps.

## 11. End-to-End Test Infrastructure Architecture

The `dxil-spirv-tests` crate provides automated verification against the full upstream shader test suite.

### Test Data Flow

```text
dxil-spirv-sys/dxil-spirv/shaders/   ──sync──▶  tests/shaders/   (git-ignored)
dxil-spirv-sys/dxil-spirv/reference/ ──sync──▶  tests/reference/ (git-ignored)
                                               │
                                               ▼ DXC 1.9.2602.17
                                          tests/shaders/*.dxil
```

### Core Architecture Components

The test infrastructure consists of three primary modules:
- `dxil-spirv-tests/build.rs`: Synchronizes shader directories, downloads Microsoft DXC release assets, and compiles HLSL test shaders into binary DXIL containers.
- `dxil-spirv-tests/tests/harness.rs`: Drives shader conversions, configures remapper state, and normalizes output.
- `dxil-spirv-tests/tests/e2e.rs`: Implements test suites covering suite completeness, category smoke tests, regression baselines, and conversion metrics.

### Subprocess Isolation Pattern

Upstream C++ translation code contains strict debug assertions (such as `SpvBuilder.cpp:754`). When an unsupported or malformed instruction sequence triggers a C++ assertion, the abort terminates the host process immediately.

`dxil-spirv-tests` executes every shader conversion inside an isolated child process using `std::process::Command`. If an assertion fires:
- The child process exits with an error status code.
- The parent test harness catches the failure without terminating the remaining test suite.

### DXC Version Lock

The test harness pins `DXC_VERSION` to `1.9.2602.17` in `dxil-spirv-tests/build.rs`. This release provides the first stable production compiler for Shader Model 6.9 (SM6.9). Downgrading DXC causes SM6.9 shaders to fail during compilation.

### Known Failure Tracking and Completeness Gate

Certain shaders in the upstream test suite require per-shader custom remapper callbacks. The test harness classifies these cases through `requires_complex_remapper()` in `harness.rs`, tagging them as `KnownFailure`.

This design preserves two goals:
1. `test_completeness_check` enforces that all 829 upstream shaders are tracked and accounted for.
2. The exact known failure rate (~33.7%, or 279/829 shaders) is measured continuously without breaking automated build gates.
