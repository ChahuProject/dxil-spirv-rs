# Changelog

Current Status: Version 0.1.0+dxil-spirv.2.72.1 | Rust Edition 2024 | 829/829 shader tests passing (100.0%) | CI green on Windows, Linux, macOS

This document records the chronological development history of dxil-spirv-rs, reconstructed directly from git history and commit bodies.

## Milestones

### 1. Initial Bindings and Build Infrastructure
- Date: 2026-08-15
- Key Commits: `b794e5f`, `670be82`, `8b36488`, `c465d4a`, `72b8ed5`, `5f991f6`, `01ef4c4`, `b02dd90`, `76d0456`, `218e11a`, `e6c51ce`

The project began with a two-crate workspace topology. `dxil-spirv-sys` handles native CMake compilation and raw bindgen FFI bindings, while `dxil-spirv` provides a safe, idiomatic Rust API. The upstream C++ codebase from `HansKristian-Work/dxil-spirv` was pinned as a git submodule.

Early implementation work established the core RAII types `ParsedBlob` and `Converter`, along with the `convert_to_spirv()` one-shot helper function. The wrapper was quickly expanded to cover all 50+ options in `ConverterOption`, compute shader workgroup dimensions, entry point selectors, wave size heuristics, and LLVM IR disassembly accessors.

Bridging Rust closures to C callbacks required solving a fat-pointer limitation in Rust. Trait objects like `Box<dyn FnMut...>` cannot cast directly to a thin `*mut c_void` userdata pointer. A double-boxing pattern (`Box<Box<dyn FnMut...>>`) solved this by creating a stable heap address for the outer pointer. Trampolines wrap closure calls in `std::panic::catch_unwind` to prevent panics from unwinding across the `extern "C"` boundary. This pattern was applied across all 8 callback remappers:
- SRV remapper (Shader Resource Views)
- UAV remapper (Unordered Access Views)
- CBV remapper (Constant Buffer Views)
- Sampler remapper
- Vertex Input remapper
- Stage Input remapper
- Stage Output remapper
- Stream Output remapper

Static linking required identifying the complete 9-library closure in strict dependency order:
1. `dxil-spirv-c-static` (C API export surface)
2. `dxil-converter` (core DXIL to SPIR-V translation logic)
3. `spirv-module` (SPIR-V binary instruction builder)
4. `dxil-utils` (shared utilities and container parsers)
5. `dxil-debug` (disassembly and debug printing)
6. `dxbc-spirv` (legacy DXBC SM4/SM5 translation fallback)
7. `glslang-spirv-builder` (SPIR-V AST construction backend)
8. `llvm-bc` (LLVM bitcode container reader)
9. `bc-decoder` (low-level bitstream decoder)

On Windows, `CMAKE_MSVC_RUNTIME_LIBRARY` was configured dynamically to match Rust's debug and release CRT profiles (`MultiThreaded$<$<CONFIG:Debug>:Debug>DLL`). This resolved unresolved linker symbols for `_CrtDbgReport` and `_calloc_dbg`. Semver build metadata (`0.1.0+dxil-spirv.2.72.1`) was adopted to track the upstream C API version string directly.

Reference architectures from `grovesNL/spirv_cross` and `SnowflakePowered/spirv-cross2-rs` were studied to establish sound binding conventions. Reference repositories were configured as on-demand clones within `.agents/skills/sync-upstream/` so downstream crate consumers do not pull unnecessary source trees.

### 2. API Coverage Completion
- Date: 2026-08-15
- Key Commits: `b7fc828`, `3310b19`

Dependencies were upgraded to bindgen 0.72 and thiserror 2.0. A dedicated compile-time test (`tests/api_coverage.rs`) was introduced to track whether any upstream C functions were missing from the safe layer.

The safe wrapper was expanded with 23 additional function bindings to expose the entire upstream C surface:
- Root signature parameter mappings and descriptor tables (`add_root_descriptor_mapping`, `set_root_constant_word_count`, `add_local_root_constants`, `begin_local_root_descriptor_table`, `add_local_root_descriptor_table`, `end_local_root_descriptor_table`)
- Work Graphs entry points (`node_input`, `num_node_outputs`, `node_output` for SM6.8 mesh nodes)
- RDAT subobject parsing for DirectX Raytracing state objects (`get_num_rdat_subobjects`, `get_rdat_subobject`)
- Resource scanning for pre-conversion introspection (`scan_resources`)
- Thread allocator memory arena management (`ThreadAllocatorContext`)
- Thread-local log callback registration (`set_thread_log_callback`)
- Direct DXIL bitcode parsing without DXBC container overhead (`parse_dxil`)

New typed data structures were introduced to represent raw C constructs safely: `ResourceClass`, `MetaDescriptor`, `MetaDescriptorKind`, `RdatSubobject`, `RdatSubobjectKind`, `NodeInputData`, `NodeOutputData`, and `LogLevel`.

Experimental feature flags (`-DDXIL_SPV_ENABLE_EXPERIMENTAL_WORKGRAPHS` and `-DDXIL_SPV_ENABLE_EXPERIMENTAL_MULTIVIEW`) were added to bindgen flags in `build.rs`. Because the upstream C++ implementation always compiles these paths, the Rust wrapper now exposes all 64 functions from the C API (`KNOWN_MISSING` dropped to 0).

### 3. End-to-End Test Infrastructure
- Date: 2026-08-16
- Key Commits: `41260c8`, `3558baf`, `5e18ce6`, `38f713a`

A third crate, `dxil-spirv-tests`, was added to validate conversions against real-world shaders. The build script synchronizes 1,550 shader source files and 839 reference files from `dxil-spirv-sys/dxil-spirv/shaders/` and `reference/` on every build.

The harness automates downloading Microsoft DXC 1.9.2602.17 to compile HLSL sources into DXIL bytecode. This DXC version was specifically chosen because it provides initial production support for Shader Model 6.9.

Because upstream C++ assertions call `abort()` on invalid inputs, running conversions in-process would terminate the entire test runner. The test harness isolates each shader conversion inside a dedicated child process spawned with `std::process::Command`. Child processes receive target shaders via `DXIL_SPIRV_TEST_CHILD_SHADER` and communicate results back through structured stdout messages (`__DXIL_SPIRV_RESULT__|status|spirv_len|error`).

GLSL round-trip validation was integrated using `spirv-cross2` 0.7.1, ensuring generated SPIR-V decompiles cleanly into GLSL 460. An opt-in strict comparison mode (`DXIL_SPIRV_STRICT_GLSL=1`) checks MD5 hashes against upstream reference files. The `test_metrics_report` suite was created with hard assertions demanding zero unexpected failures and zero skipped shaders. Initial test execution passed 47/48 stages and 67/159 resources.

### 4. Regression Baseline and Known-Failure Classification
- Date: 2026-08-16
- Key Commits: `e5187dd`, `488afa3`, `568dda3`, `2aa1e90`, `ca2598e`

Test suite coverage was expanded across all 24 upstream shader categories, eliminating blind spots: `ags`, `alloca-opts`, `auto-barrier`, `control-flow`, `descriptor_qa`, `dxil-builtin`, `fp16`, `heap-robustness`, `instrumentation`, `llvm-builtin`, `memory-model`, `nvapi`, `opts`, `raw-access`, `resources`, `rov`, `sampler-feedback`, `semantics`, `stages`, `vectorization`, `view-instancing`, `vkmm`, `asm`, and root shaders. Raw LLVM bitcode files in `asm/*.bc.dxil` were wired to `dxil_spirv::parse_dxil`.

The known-failure classification logic in `requires_complex_remapper` was updated to execute only after a conversion attempt fails. Shaders that convert successfully are no longer hidden as known failures. This adjustment immediately lifted the pass rate from 66.3% to 76.2% (632 passing, 197 known failures).

A regression baseline file (`tests/regression_baseline.json`) was generated to catch any pass-to-fail regressions between code changes. Update workflows were wired through the `DXIL_SPIRV_UPDATE_BASELINE=1` environment variable. Child processes received a 30-second watchdog timer to kill infinite loops. `test_completeness_check` gained a guard against empty shader directories, preventing false-positive passes when submodules are missing.

Archive extraction was hardened by upgrading `zip` to version 8.6.0 with deflate support. The extraction logic was also scoped strictly to `bin/x64/` to avoid picking up 32-bit DXC binaries.

### 5. Reaching 100% Shader Pass Rate
- Date: 2026-08-16
- Key Commits: `4e77570`, `47daf64`, `5b89e37`

Closing the remaining test failures required matching upstream CLI options and remapping configurations across three progressive phases.

Phase 1 added nine missing option mappings in `configure_converter()`:
- `DescriptorQa` for `.descriptor-qa.` (version 2, sets 10/11, hash `0xdeadbeef`)
- `InstructionInstrumentation` for `.bda-instrumentation.` (buffer sync validation)
- `VulkanMemoryModel` for `.vkmm.`
- `Nvapi` for `.nvapi.` (register 127, space 0)
- `Float8Support` for `.full-wmma.` (FP8 arithmetic and cooperative matrix)
- `ShaderQuirk::GroupSharedAutoBarrier` for `.auto-group-shared-barrier.`
- `MixedFloatDotProduct` for `.mixed-float-dot-product.`
- `OutputSwizzle` for `.rt-swizzle.`
- `RawAccessChainsNv` for `.raw-access-chains.`

These option mappings reduced known failures from 197 to 194.

Phase 2 tackled fundamental binding defaults:
- SSBO alignment: Upstream library defaults to 16 bytes, but the CLI tool defaults to 1 byte. Adding `SsboAlignment { alignment: 1 }` as a base option resolved offset errors across all non-bindless SSBO shaders.
- Bindless push constants: Allocated at least 8 words for `.bindless.` and 4 additional words for descriptor QA, alongside 64 root parameter mappings.
- Root descriptors: Enabled `set_root_descriptor_count(4)` and Buffer Device Address (BDA) for `.root-descriptor.` shaders.

These changes dropped known failures from 194 to 9, achieving a 98.9% pass rate.

Phase 3 resolved the final 9 failures with three targeted fixes:
1. BDA instrumentation: Root descriptor BDA was forcing the RTAS heap to BufferDeviceAddress, but instrumentation requires an SSBO introspection buffer. Added a flag to skip BDA override on the RTAS heap (6 shaders).
2. Local root signatures: Replaced manual descriptor table calls with upstream-equivalent `add_local_root_constants(15, 0, 5)`, `add_local_root_constants(15, 1, 6)`, and `add_local_root_descriptor()`, paired with `PhysicalStorageBuffer` (2 shaders).
3. Heap robustness: Corrected meta descriptor kinds to use `ResourceDescriptorHeapSize` as a UBO constant at set 10 binding 20, and `RawDescriptorHeapView` as a UBO BDA at set 10 binding 21 (1 shader).

With these fixes, all 829 shaders in the upstream test suite pass completely (100.0% pass rate, 0 known failures, 0 regressions).

### 6. Edition 2024 Migration and Code Formatting
- Date: 2026-08-16
- Key Commits: `ed19ee0`, `55fa6fa`

The entire workspace was migrated to Rust edition 2024, setting `rust-version = "1.85"` in `Cargo.toml`. All Rust source files were reformatted to conform to edition 2024 style rules and module import ordering.

A root `rustfmt.toml` file was added to enforce workspace-wide formatting standards. An empty `.rustfmt.toml` file was placed in `dxil-spirv-sys/generated/` to prevent rustfmt from altering the generated `bindings.rs` file (per rust-lang/rustfmt#4264).

CI workflows were restructured into three dedicated jobs with separate cache scopes:
- `fmt`: Fast formatting gate (~30s) running without submodule checkout.
- `clippy`: Check-level analysis and lint verification.
- `build-test`: Full C++ compilation and test execution.

Artifact sharing was configured so `cargo build` and `cargo test` share compiled dependencies without rebuilding native C++ libraries multiple times.

### 7. Cross-Platform CI Repair and Link Fixes
- Date: 2026-08-16
- Key Commits: `ed19ee0`, `6d7757f`, `5163058`, `ce156ac`, `dde03e4`, `262a41e`, `a923f9b`, `5ecba0b`

Multiple platform-specific link and compilation issues were identified and resolved across Windows, Linux, and macOS runners.

The MSVC-specific `/EHsc` compiler flag was restricted to `target_env == "msvc"`. GCC and Clang reject this flag during CMake compiler feature checks with errors like `no such file or directory: '/EHsc'`.

The CI caching configuration was refined by removing the `target/` cache directory. Restoring stale target directories caused link failures when CMake static libraries moved or became invalidated across runs.

`register_lib_dirs()` in `build.rs` was updated to search for `.a` archives alongside `.lib` files. Without this change, Unix linkers failed to find static libraries (`libdxbc-spirv.a`, `libglslang-spirv-builder.a`, `libllvm-bc.a`, `libbc-decoder.a`) nested inside CMake subdirectories.

Bindgen generated `dxil_spv_option` as `c_int` (signed) on Windows and `c_uint` (unsigned) on Linux and macOS because the C header contains no negative enumerators. The safe wrapper normalized all enum fields to `u32`. A crate-level `#![allow(clippy::unnecessary_cast)]` attribute was added so cross-platform casts compile cleanly under `-D warnings`.

Explicit dynamic link directives were added for C++ standard libraries (`c++` on macOS, `stdc++` on Linux). On non-Windows platforms where DXC binaries cannot run, `build.rs` emits `cargo:rustc-cfg=dxc_unavailable`, allowing the test suite to skip shader execution gracefully while still verifying library builds.

### 8. Documentation Restructure
- Date: 2026-08-16
- Key Commits: `4e77570`, `38f713a`, `55fa6fa`, `ed19ee0`

Project documentation was reorganized into dedicated files under `docs/` to provide clear separation of concerns for users and contributors.

`docs/README.md` acts as the central hub and defines maintenance policies. `docs/usage.md` provides an end-to-end guide for crate consumers, detailing conversions, remapper callbacks, root layouts, logging, and memory arenas.

`docs/architecture.md` covers internal design topics for developers, including crate topology, the 9-library static link closure, FFI safety boundaries, and the cross-platform pitfall ledger. `docs/testing.md` documents test harness architecture, shader markers, and baseline mechanics.

`docs/platform-support.md` details supported OS and architecture targets. This changelog was added to track project evolution across each major milestone.

## How to Update

Every pull request that introduces user-visible API changes, modifies internal build scripts, or alters shader test coverage must add an entry to this changelog. Place the new entry under the latest milestone or create a new section if the change represents a distinct development phase. Reference the relevant commit hashes, describe what changed, explain why the change was necessary, and include updated verification metrics whenever test results are affected.
