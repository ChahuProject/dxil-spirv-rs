# CI Architecture

This document describes the Continuous Integration (CI) architecture for `dxil-spirv-rs`, defined in `.github/workflows/ci.yml`. It details the job topology, platform verification strategy, caching policies, local pre-push validation, and the historical ledger of CI pitfalls and resolutions.

For platform requirements and supported target triples, see [platform-support.md](platform-support.md). For test suite mechanics and shader coverage metrics, see [testing.md](testing.md).

## Job Overview and Workflow Topology

CI executes on GitHub Actions for every push and pull request targeting the `main` branch. The workflow is partitioned into three jobs that separate fast sanity checks from heavy native compilation passes.

| Job ID in `ci.yml` | Step Name | Runner OS | Timeout | Execution Commands |
|---|---|---|---|---|
| `fmt` | `rustfmt` | `ubuntu-latest` | 5 min | `cargo fmt --all -- --check` |
| `clippy` | `clippy (${{ matrix.os }})` | `windows-latest`<br>`ubuntu-latest`<br>`macos-latest` | 30 min | `cargo clippy --workspace --all-targets -- -D warnings` |
| `build-test` | `build & test (${{ matrix.os }})` | `windows-latest`<br>`ubuntu-latest`<br>`macos-latest` | 45 min | `cargo build --workspace --all-targets --verbose`<br>`cargo test --workspace --verbose` |

### Pipeline Structure and Rationale

The pipeline enforces three stages of verification:

1. **Fast Formatting Gate (`fmt`)**:
   Runs on a single `ubuntu-latest` runner. It does not check out submodules or install C++ dependencies, completing in under 30 seconds. If code style or import order violates `rustfmt.toml` (edition 2024), the pipeline halts immediately before spinning up heavier matrix runners.

2. **Cross-Platform Lint Gate (`clippy`)**:
   Runs across all three operating systems with `strategy.fail-fast: false`. Clippy operates primarily on check-level artifacts. Running it as an independent matrix job surfaces platform-specific lint regressions (such as cast lints or target configuration warnings) without blocking on full test execution.

3. **Consolidated Build and Test Gate (`build-test`)**:
   Runs across all three operating systems. Compiles the upstream C++ static library (`dxil-spirv-c-static`) via CMake and executes unit, safe wrapper, and integration test suites. Running `cargo build --workspace --all-targets` followed by `cargo test --workspace` ensures the C++ core and test binaries are compiled once within the same target workspace.

### Runner Setup and Host Dependencies

On Linux (`ubuntu-latest`), CI provisions build dependencies via `apt-get`:
- `cmake` and `ninja-build`: Used by `cmake-rs` to configure and compile upstream C++ sources.
- `libclang-dev` and `clang`: Required by `bindgen` to parse `dxil_spirv_c.h` during `dxil-spirv-sys` compilation.

On macOS (`macos-latest`, Apple Silicon `aarch64-apple-darwin`), CI provisions dependencies via Homebrew:
- `cmake` and `ninja`: Build tooling for the C++ static core.
- `llvm`: Provides `libclang` for `bindgen`.

On Windows (`windows-latest`, `x86_64-pc-windows-msvc`), the runner image includes MSVC and CMake natively.

## Platform Strategy and Test Harness Divergence

The repository maintains three workspace crates:
- `dxil-spirv-sys`: Compiles the vendored C++ core and provides raw FFI bindings.
- `dxil-spirv`: Safe, idiomatic Rust API wrappers.
- `dxil-spirv-tests`: Integration test suite and 829-shader regression harness.

The core libraries (`dxil-spirv` and `dxil-spirv-sys`) compile and run with 100% feature parity across Windows, Linux, and macOS. However, the end-to-end test harness diverges due to external compiler availability.

### The DXC Binary Caveat and `dxc_unavailable`

Compiling HLSL test shaders to DXIL bytecode requires Microsoft DirectX Shader Compiler (DXC). In `dxil-spirv-tests/build.rs`, the build script searches for DXC in the following sequence:
1. Cached executable under `target/dxc/1.9.2602.17/dxc.exe`.
2. Path specified in the `DXC_PATH` environment variable.
3. System `PATH` (`dxc --version`).
4. Windows Kits directory fallback (`C:\Program Files (x86)\Windows Kits\10\bin`).
5. Automatic download of official Microsoft release `v1.9.2602` (`dxc_2026_02_20.zip`).

Because Microsoft publishes DXC release archives containing Windows x64 binaries, the downloaded `dxc.exe` cannot execute on Linux or macOS kernels (failing with `Permission denied` or `Exec format error`).

To keep CI green across Unix runners without disabling integration test compilation:

1. `dxil-spirv-tests/build.rs` invokes `is_dxc_runnable(&dxc_path)` to test process execution via `--version`.
2. When non-runnable, `build.rs` emits `cargo:rustc-cfg=dxc_unavailable`.
3. To comply with compiler lint requirements, `build.rs` unconditionally emits `cargo:rustc-check-cfg=cfg(dxc_unavailable)`.
4. In `dxil-spirv-tests/tests/e2e.rs`, test entry points (`test_smoke`, `run_category`, `test_metrics_report`) evaluate `if cfg!(dxc_unavailable)` and exit early with a skip notification.
5. Windows CI runners execute the complete end-to-end suite across all 829 shaders. Linux and macOS CI runners compile all test harnesses, execute all unit tests, and validate the safe API wrapper without failing completeness checks.

## Caching Policy

CI uses GitHub Actions caching (`actions/cache@v4`) with strict cache boundaries.

### What is Cached

- `~/.cargo/registry`: Downloaded crate indexes and package archives.
- `~/.cargo/git`: Cloned git dependency checkouts.

Cache keys are isolated per runner operating system and keyed by lockfile hash:
```yaml
key: ${{ runner.os }}-build-crates-${{ hashFiles('**/Cargo.lock') }}
restore-keys: ${{ runner.os }}-build-crates-
```

### Why `target/` is NOT Cached (The Stale CMake Artifact Lesson)

An earlier CI iteration cached the `target/` directory to avoid rebuilding the C++ core on unchanged runs. This introduced persistent link failures:

1. **Invisible Build Artifacts**: The upstream C++ static libraries (`libdxbc-spirv.a`, `libglslang-spirv-builder.a`, `libllvm-bc.a`, `libbc-decoder.a`, or `.lib` on Windows) are generated by CMake under `target/debug/build/dxil-spirv-sys-*/out/build/`.
2. **Fingerprint Disconnect**: Cargo tracks Rust source files and `build.rs` scripts, but does not monitor intermediate CMake build outputs.
3. **Stale Link References**: Restoring `target/` from cache brought back precompiled Rust `.rlib` archives containing absolute link paths pointing to missing or relocated CMake build directories.
4. **Failure Mode**: When compiling test binaries, the linker failed with `unable to find library -ldxbc-spirv` because the referenced static archive did not exist at the restored path.

Dropping `target/` from the cache ensures the C++ core builds fresh via CMake on every CI run. While this adds a modest compilation step, it eliminates linker path drift.

## The Pitfall Ledger (CI Fix History)

The following ledger details the sequence of CI failure modes, their root causes, and the concrete fixes implemented across commits `ed19ee0..5ecba0b`.

### Pitfall 1: MSVC `/EHsc` Flag Passed Unconditionally to GCC and Clang

- **Commit**: `ed19ee0` ("Migrate to edition 2024, fix cross-platform CI, add rustfmt config")
- **Symptom**: CMake configuration failed on Linux and macOS CI runners with:
  ```text
  c++: error: no such file or directory: '/EHsc'
  ```
- **Root Cause**: `dxil-spirv-sys/build.rs` unconditionally passed `.cxxflag("/EHsc")` to `cmake::Config`. MSVC requires `/EHsc` to enable C++ structured exception handling, but GCC and Clang treat `/EHsc` as an unrecognized file argument.
- **Fix**: Restricted the flag to MSVC targets in `dxil-spirv-sys/build.rs`:
  ```rust
  if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
      cfg.cxxflag("/EHsc");
  }
  ```
  GCC and Clang enable C++ exceptions by default for C++ sources.

### Pitfall 2: Stale `target/` Cache Producing Missing Static Archive Links

- **Commit**: `6d7757f` ("Fix CI: drop fragile target/ cache causing stale C++ lib links")
- **Symptom**: Linux and macOS builds failed at the final link stage with:
  ```text
  error: linking with `cc` failed: exit status: 1
  = note: /usr/bin/ld: cannot find -ldxbc-spirv: No such file or directory
  ```
- **Root Cause**: The CI workflow cached `target/`. Cached `.rlib` files contained hardcoded paths to CMake static archives that were absent on newly provisioned runner environments.
- **Fix**: Removed `target` from cache paths in `.github/workflows/ci.yml`. Cached only `~/.cargo/registry` and `~/.cargo/git`.

### Pitfall 3: Static Library Search Ignored `.a` File Extensions

- **Commit**: `5163058` ("Fix cross-platform linking: recognize .a static libs in register_lib_dirs")
- **Symptom**: Fresh Linux and macOS builds continued to fail with `unable to find library -ldxbc-spirv`.
- **Root Cause**: In `dxil-spirv-sys/build.rs`, the recursive directory walker `register_lib_dirs` checked only for files with the `.lib` extension. On Unix systems, CMake outputs `.a` archives (`libdxbc-spirv.a`), so the walker skipped build output directories and never emitted `cargo:rustc-link-search=native=...`.
- **Fix**: Updated `register_lib_dirs` to inspect both extensions:
  ```rust
  } else if path.extension().is_some_and(|e| e == "lib" || e == "a") {
      has_lib = true;
  }
  ```

### Pitfall 4: Bindgen Enum Signedness Diverged Across Platforms

- **Commit**: `ce156ac` ("Fix bindgen enum signedness breaking Linux/macOS builds")
- **Symptom**: Rust compilation on Linux and macOS failed with:
  ```text
  error[E0308]: mismatched types
     --> dxil-spirv/src/converter.rs:133:43
      |
  133 |             return Err(Error::UnsupportedFeature(option.kind()));
      |                        ------------------------- ^^^^^^^^^^^^^ expected `i32`, found `u32`
  ```
- **Root Cause**: On Windows MSVC, bindgen generated `dxil_spv_option` as `c_int` (`i32`). On Linux and macOS, because the upstream C enum contained no negative values, Clang and bindgen selected an unsigned underlying type (`c_uint` / `u32`).
- **Fix**: Changed `Error::UnsupportedFeature` payload to `u32` in `dxil-spirv/src/error.rs` and added an explicit normalizing cast `option.kind() as u32` in `dxil-spirv/src/converter.rs`.

### Pitfall 5: Missing C++ Runtime Standard Library Link Directives

- **Commit**: `dde03e4` ("Link C++ stdlib on GCC/Clang targets")
- **Symptom**: Linux and macOS linking failed with hundreds of undefined symbol errors:
  ```text
  undefined reference to `operator new(unsigned long)'
  undefined reference to `__cxa_pure_virtual'
  undefined reference to `std::terminate()'
  ```
- **Root Cause**: The upstream core uses C++ standard library features. MSVC links the C++ runtime automatically via the CRT. GCC and Clang do not link the C++ runtime when Rust links static C archives.
- **Fix**: Added explicit link directives in `dxil-spirv-sys/build.rs`:
  ```rust
  match env::var("CARGO_CFG_TARGET_OS").as_deref() {
      Ok("macos") => println!("cargo:rustc-link-lib=dylib=c++"),
      Ok(os) if os != "windows" => println!("cargo:rustc-link-lib=dylib=stdc++"),
      _ => {}
  }
  ```

### Pitfall 6: Windows-Only DXC Binary Broke Unix Test Execution

- **Commit**: `262a41e` ("Skip e2e shader tests gracefully when DXC is not runnable")
- **Symptom**: End-to-end test execution crashed on Linux and macOS runners with `Exec format error` during DXC execution, failing the completeness check.
- **Root Cause**: The test harness downloaded Microsoft official DXC release assets containing Windows PE binaries.
- **Fix**: Added `is_dxc_runnable` detection in `dxil-spirv-tests/build.rs` to conditionally emit `cargo:rustc-cfg=dxc_unavailable`. Guarded test runners in `dxil-spirv-tests/tests/e2e.rs` with `cfg!(dxc_unavailable)`.

### Pitfall 7: Platform-Normalizing Casts Triggered Clippy Lint Failures

- **Commits**: `a923f9b` ("Allow same-type cast for cross-platform bindgen enum") and `5ecba0b` ("Allow platform-normalizing casts crate-wide (clippy::unnecessary_cast)")
- **Symptom**: Clippy matrix jobs on Linux and macOS failed under `-D warnings` with:
  ```text
  error: unnecessary cast to the same type: `u32` as `u32`
     --> dxil-spirv/src/converter.rs:133:43
  ```
- **Root Cause**: Casting `option.kind() as u32` is a real conversion on Windows (`i32` to `u32`), but a redundant same-type cast on Linux and macOS where bindgen emits `u32`.
- **Fix**: Added a crate-level `#![allow(clippy::unnecessary_cast)]` in `dxil-spirv/src/lib.rs` with documentation explaining that cross-platform FFI type stability requires these platform-normalizing casts.

## Adding a Job or Platform

To introduce a new target platform to CI:

1. **Verify Toolchain Availability**: The runner must provide a C++17 compiler, CMake 3.20 or newer, Ninja, and `libclang`.
2. **Expand the Workflow Matrix**: Add the runner identifier to `.github/workflows/ci.yml`:
   ```yaml
   strategy:
     fail-fast: false
     matrix:
       os: [windows-latest, ubuntu-latest, macos-latest, <new-runner-os>]
   ```
3. **Configure System Dependencies**: Add conditional package installation steps for the new runner OS if needed.
4. **Validate Standard Library Linkage**: Ensure `dxil-spirv-sys/build.rs` maps the target OS to the correct C++ runtime library (`libc++` for Darwin/BSD, `libstdc++` for GNU/Linux).

## Local Pre-Push Validation Gate

To verify changes locally before opening a pull request, run the exact command sequence executed by CI:

```sh
# Step 1: Format check (corresponds to 'fmt' job)
cargo fmt --all -- --check

# Step 2: Workspace lint check (corresponds to 'clippy' job)
cargo clippy --workspace --all-targets -- -D warnings

# Step 3: Build and test execution (corresponds to 'build-test' job)
cargo build --workspace --all-targets
cargo test --workspace
```

Running these commands locally ensures the branch will clear all CI matrix gates cleanly across platforms.
