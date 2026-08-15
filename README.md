# dxil-spirv-rs

Safe Rust bindings to [dxil-spirv](https://github.com/HansKristian-Work/dxil-spirv) — convert D3D11/D3D12 shader bytecode (DXBC container or DXIL bitcode) into SPIR-V.

Feed the resulting SPIR-V into a cross-compiler such as [SPIRV-Cross](https://github.com/KhronosGroup/SPIRV-Cross) to recover readable **HLSL / GLSL / MSL** source, or consume it directly with Vulkan tooling. Typical uses: shader inspection / debugging tools, reverse-engineering D3D12 shaders, D3D→Vulkan translation research.

```text
DXBC / DXIL container ──dxil-spirv──▶ SPIR-V ──SPIRV-Cross──▶ HLSL / GLSL / MSL
```

## AI-generated

This crate is **entirely AI-generated** (by a large-language-model coding agent), with human direction and review. No hand-written logic.

**How it was produced:**

1. **Substrate** — the upstream `dxil-spirv` C++ library (MIT, by Hans-Kristian Arntzen / Valve) is vendored as a git submodule under `dxil-spirv-sys/dxil-spirv`. The agent did not reimplement any of the conversion logic; it only binds to it.
2. **sys layer (`dxil-spirv-sys`)** — a `build.rs` that compiles the upstream `dxil-spirv-c-static` CMake target and runs [bindgen](https://github.com/rust-lang/rust-bindgen) over the upstream C header `dxil_spirv_c.h` to produce the raw FFI surface.
3. **safe layer (`dxil-spirv`)** — RAII wrappers (`ParsedBlob`, `Converter`), typed option/binding/remapper structs, and a `thiserror` error type, all written by the agent against the generated bindings.
4. **Reference-driven** — binding structure and `build.rs` patterns were modelled on the mature [`grovesNL/spirv_cross`](https://github.com/grovesNL/spirv_cross) crate. A bundled maintenance skill (`.agents/skills/sync-upstream`) encodes the verified facts (static-link closure, CRT rules, bindgen boundaries, callback trampoline pattern) so future updates can be re-generated safely by an agent.

Because the code is machine-generated, please treat it with the same care you would any new dependency: review before production use, and report anything that looks off. Issues and human review are very welcome.

## Quick start

### Prerequisites

- **Rust** (stable; see `rust-toolchain.toml`)
- **A C++ toolchain + CMake** (the sys crate compiles the upstream C++ library at build time):
  - Windows: MSVC (Visual Studio Build Tools) + CMake
  - Linux/macOS: a C++14 compiler + CMake
- **git submodules**: this repo vendors upstream source, so clone recursively.

### Clone & build

```sh
git clone --recursive https://github.com/ChahuProject/dxil-spirv-rs.git
cd dxil-spirv-rs

# if you already cloned without --recursive:
git submodule update --init --recursive

cargo build --workspace
cargo test  --workspace
```

### Use it

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

For finer control, drive the stages explicitly:

```rust
use dxil_spirv::{Converter, ParsedBlob};

let parsed = ParsedBlob::parse(&blob)?;
println!("stage: {:?}, entry points: {}", parsed.shader_stage(), parsed.num_entry_points()?);

let converter = Converter::new(&parsed)?;
converter.run()?;
let spirv_words = converter.compiled_spirv()?;
```

### Crate layout

| Crate | Path | Purpose |
|---|---|---|
| `dxil-spirv` | `dxil-spirv/` | Safe, idiomatic wrapper — what you depend on |
| `dxil-spirv-sys` | `dxil-spirv-sys/` | Raw bindgen FFI + CMake build (linked transitively) |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

The vendored upstream `dxil-spirv` is MIT-licensed; see `dxil-spirv-sys/dxil-spirv/LICENSE.MIT`.
