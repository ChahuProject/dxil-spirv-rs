# Platform Support

[English](platform-support.md) | [中文](platform-support.zh-CN.md)

`dxil-spirv-rs` wraps the upstream `dxil-spirv` C++ library (built from source via
CMake) and exposes a safe Rust API. Because the C++ core is compiled locally, the
crate works on any platform that has a working C++17 toolchain and CMake.

## Supported platforms

These platforms are built and tested in CI on every push and pull request.

| OS      | Architecture | Rust target              | C++ toolchain      | Status             |
|---------|--------------|--------------------------|--------------------|--------------------|
| Windows | x86_64       | `x86_64-pc-windows-msvc` | MSVC (VS 2022)     | ✅ Tested in CI     |
| Linux   | x86_64       | `x86_64-unknown-linux-gnu` | GCC 13 / Clang   | ✅ Tested in CI     |
| macOS   | aarch64 (Apple Silicon) | `aarch64-apple-darwin` | Apple Clang | ✅ Tested in CI     |

### Notes per platform

- **Windows (MSVC)** — the reference platform. The C++ core is built with the
  dynamic CRT (`/MD` or `/MDd`) to match Rust's default linkage, and C++
  exceptions are enabled with `/EHsc`. DXC (the HLSL compiler used only by the
  test suite) is a Windows binary, so the full end-to-end shader test suite is
  most complete here.

- **Linux (GNU)** — built with GCC or Clang. C++ exceptions are enabled by
  default for C++ sources, so no special flags are needed. Requires `cmake`,
  `ninja-build`, `libclang-dev` and `clang` (for `bindgen`).

- **macOS (Apple Silicon)** — built with Apple Clang. Requires `cmake`,
  `ninja` and `llvm` (for `bindgen`). Intel macOS (`x86_64-apple-darwin`)
  should work identically but is not exercised in CI.

## Requirements

- **Rust**: see `rust-version` in `Cargo.toml` (currently **1.85**, for edition 2024).
- **CMake**: 3.20 or newer recommended (used to build the vendored C++ core).
- **C++ toolchain**: any C++17-capable compiler (MSVC, GCC, or Clang).
- **libclang** (build-time only, for `bindgen` to generate FFI bindings).
- **Ninja** (optional but recommended; speeds up the CMake build).

## Test-suite-only caveat: DXC

The `dxil-spirv-tests` crate compiles HLSL test shaders with Microsoft's **DXC**,
which ships as a **Windows x64** binary. On non-Windows platforms the build script
cannot run the downloaded `dxc.exe`, so shaders must be precompiled (or DXC
provided another way via the `DXC_PATH` environment variable). This affects **only
the test harness**, not the published `dxil-spirv` / `dxil-spirv-sys` libraries —
the libraries themselves are fully cross-platform.

## Adding a new platform

A new platform is supported if it satisfies the requirements above. To validate:

1. Ensure `cmake`, a C++17 compiler, and `libclang` are installed.
2. Run `cargo build --workspace` — the C++ core should configure and build.
3. Run `cargo test --workspace` — the safe-wrapper and conversion tests should pass.

If you validate a platform not listed here (e.g. `x86_64-apple-darwin`,
`aarch64-unknown-linux-gnu`, or `x86_64-pc-windows-gnu`), please open an issue or
PR so it can be added to the support matrix and, if feasible, to CI.
