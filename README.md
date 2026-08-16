# dxil-spirv-rs

[English](README.md) | [中文](README.zh-CN.md)

Safe Rust bindings to [dxil-spirv](https://github.com/HansKristian-Work/dxil-spirv) — convert D3D11/D3D12 shader bytecode (DXBC container or DXIL bitcode) into SPIR-V.

Feed the resulting SPIR-V into a cross-compiler such as [SPIRV-Cross](https://github.com/KhronosGroup/SPIRV-Cross) to recover readable **HLSL / GLSL / MSL** source, or consume it directly with Vulkan tooling. Typical uses: shader inspection / debugging tools, reverse-engineering D3D12 shaders, D3D→Vulkan translation research.

```text
DXBC / DXIL container ──dxil-spirv──▶ SPIR-V ──SPIRV-Cross──▶ HLSL / GLSL / MSL
```

**Status**: edition 2024, MSRV 1.85, CI green on Windows / Linux / macOS, **all 829 upstream shader tests pass (100%)**.

## AI-maintained

This project is **AI-maintained**: it was created by the **Kimi K3** model, and
AI-generated and AI-edited code is explicitly welcome and is the normal way
this project evolves. Human direction and review are applied throughout; the
AI follows the same standards as any human contributor. See the
[AI-maintenance policy](docs/contributing.md) for details.

## Using this crate (for users)

Add to your `Cargo.toml`:

```toml
[dependencies]
dxil-spirv = "0.1"
```

Convert a shader blob to SPIR-V:

```rust
fn main() -> dxil_spirv::Result<()> {
    // A full DXBC container (SM4/SM5/SM6) or a raw DXIL bitcode slice.
    let blob: Vec<u8> = std::fs::read("shader.dxil").expect("read shader");

    let spirv_words = dxil_spirv::convert_to_spirv(&blob)?;
    println!("produced {} SPIR-V words", spirv_words.len());

    // Hand `spirv_words` to SPIRV-Cross (e.g. the `spirv_cross` crate) to
    // emit HLSL / GLSL / MSL.
    Ok(())
}
```

For finer control — entry-point selection, converter options, root
signatures, descriptor remapping — drive the stages explicitly:

```rust
use dxil_spirv::{Converter, ParsedBlob};

let parsed = ParsedBlob::parse(&blob)?;
let converter = Converter::new(&parsed)?;
converter.run()?;
let spirv_words = converter.compiled_spirv()?;
```

**Full usage guide**: [docs/usage.md](docs/usage.md) — every converter option,
remapper configuration, error handling, and platform notes.

## Developing this crate (for developers)

This crate is a `-sys` + safe-layer split that compiles the vendored upstream
C++ library via CMake at build time:

| Crate | Path | Role |
|---|---|---|
| `dxil-spirv` | `dxil-spirv/` | Safe, idiomatic wrapper — what you depend on |
| `dxil-spirv-sys` | `dxil-spirv-sys/` | Raw bindgen FFI + CMake build (linked transitively) |
| `dxil-spirv-tests` | `dxil-spirv-tests/` | End-to-end suite against all upstream shaders |

The safe wrapper exposes **all** functions from the upstream C API
(`dxil_spv_*`) — enforced by a compile-time test
(`dxil-spirv/tests/api_coverage.rs`) that fails if upstream adds functions we
haven't wrapped.

**Start here**: [docs/architecture.md](docs/architecture.md) — crate topology,
FFI boundary rules, static link closure, and the cross-platform pitfall ledger
(the paid-for lessons about CMake, bindgen, and C++ linking).

**Testing**: [docs/testing.md](docs/testing.md) — how the 829-shader suite
works, regression baseline mechanics, and how to add tests.

**CI**: [docs/ci.md](docs/ci.md) — job layout, platform strategy, and the
caching pitfalls that shaped it.

**Contribute**: [docs/contributing.md](docs/contributing.md) — contribution
flow, code conventions, and the AI-maintenance policy.

## What we did (project history)

- **Initial bindings** — workspace split, RAII wrappers, FFI trampolines, static link closure.
- **Full API coverage** — all 64 upstream C functions wrapped, zero gaps.
- **End-to-end test suite** — 829 upstream shaders, DXC integration, subprocess isolation, GLSL round-trip validation.
- **Regression baseline** — pass/fail tracking per shader with hard regression detection.
- **100% pass rate** — 76.2% → 98.9% → **829/829 (100%)** by completing the upstream option/remapper surface.
- **Edition 2024 + cross-platform CI** — rustfmt, MSRV 1.85, CI green on Windows / Linux / macOS.

The full story, milestone by milestone with commits: [docs/changelog.md](docs/changelog.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

The vendored upstream `dxil-spirv` is MIT-licensed; see `dxil-spirv-sys/dxil-spirv/LICENSE.MIT`.
