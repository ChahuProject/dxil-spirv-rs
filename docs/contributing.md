# Contributing to dxil-spirv-rs

[English](contributing.md) | [中文](contributing.zh-CN.md)

Welcome to dxil-spirv-rs. This project provides safe Rust bindings for the upstream dxil-spirv C++ library, converting DXBC container and DXIL bitcode into SPIR-V. We welcome contributions from human developers and AI coding agents alike.

## AI-Maintenance Policy

This project is AI-maintained. Machine-generated and machine-edited code, tests, and documentation are explicitly welcome and represent the standard workflow here. The project was created by the **Kimi K3** model and continues to be maintained with AI assistance under human direction.

Whether you write code by hand, prompt an AI agent, or build an autonomous workflow, the exact same standards apply to all contributions:

1. Every pull request must pass the complete acceptance gate. This means correct formatting via `cargo fmt`, zero linter warnings under `cargo clippy --workspace --all-targets -- -D warnings`, and passing test suites.
2. State the factual basis for claims. When updating documentation or reporting benchmark numbers, include concrete evidence, such as commit hashes, test pass counts, or command output logs.
3. Never silently skip verification. If a test cannot run on your local machine (such as Windows-only DXC shader compilation when working on Linux or macOS), state this clearly in your pull request description instead of guessing.

## How to Contribute

### Reporting Issues

When you find a bug or unexpected behavior, open an issue on GitHub. Please include:

- A minimal reproducible example, such as a standalone shader bytecode blob or a short Rust test case.
- Your target triple and host platform (for example `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, or `aarch64-apple-darwin`).
- The observed output, error codes, or panic stack traces.
- The expected behavior or output.

If you want to suggest a new feature, describe the graphics use case and identify the upstream C API functions (`dxil_spv_*`) needed to support it.

### Pull Request Workflow

1. Fork the repository on GitHub and clone it locally with all submodules:
   ```sh
   git clone --recursive https://github.com/ChahuProject/dxil-spirv-rs.git
   ```
   If you already cloned without submodules, fetch them immediately:
   ```sh
   git submodule update --init --recursive
   ```
2. Create a focused branch for your work:
   ```sh
   git checkout -b feat/remapper-resource-types
   ```
3. Make your changes following the code and documentation conventions detailed below.
4. Run the acceptance gate locally to verify formatting, linter checks, and tests.
5. Commit your changes with clear, imperative commit messages:
   - `feat: expose mesh shader node constants in safe wrapper`
   - `fix: correct root descriptor table offset calculation`
   - `docs: update platform support matrix for FreeBSD`
6. Push your branch to your fork and submit a pull request against `main`.

## Local Development Setup

The workspace compiles the vendored C++ core from source via CMake during `cargo build`. Ensure your system has the required build tools installed:

- **Windows:** Visual Studio 2022 (MSVC with C++ tools) and CMake.
- **Linux:** GCC 13 or Clang, CMake, Ninja, and `libclang-dev` (for `bindgen`).
  ```sh
  sudo apt-get update
  sudo apt-get install -y cmake ninja-build libclang-dev clang
  ```
- **macOS:** Apple Clang, CMake, Ninja, and LLVM.
  ```sh
  brew install cmake ninja llvm
  ```

For additional details on compiler requirements, consult [platform-support.md](platform-support.md).

## The Acceptance Gate

Continuous integration runs on every pull request across Windows, Linux, and macOS. All jobs must be green before merging.

Run these commands locally in order before pushing:

1. Format check:
   ```sh
   cargo fmt --all -- --check
   ```
2. Linter check (all warnings are treated as hard errors):
   ```sh
   cargo clippy --workspace --all-targets -- -D warnings
   ```
3. Build all workspace targets:
   ```sh
   cargo build --workspace --all-targets
   ```
4. Run all workspace tests:
   ```sh
   cargo test --workspace
   ```

### Windows End-to-End Test Nuance

The end-to-end test suite in `dxil-spirv-tests` compiles HLSL shaders into DXIL bitcode with Microsoft DXC (`dxc.exe`). Because the official prebuilt DXC compiler runs as a Windows x64 binary, the full 829-shader suite executes during local Windows runs and on Windows CI runners.

On Linux and macOS, the test harness runs unit tests and safe wrapper integration tests, but skips HLSL compilation unless `DXC_PATH` points to a working DXC binary. If you develop on Linux or macOS, make sure `cargo test --workspace` passes cleanly for all cross-platform tests. Windows CI will validate the complete shader suite on your pull request.

## Code Conventions

### Rust Edition and Toolchain

This workspace targets Rust edition 2024. It requires the stable compiler version specified by `rust-version = "1.85"` in `Cargo.toml` (and pinned in `rust-toolchain.toml`).

### Formatting Rules and Guard Files

The workspace root contains `rustfmt.toml`, which configures code formatting for all first-party crates (`dxil-spirv`, `dxil-spirv-sys`, and `dxil-spirv-tests`).

Never format generated code or vendored upstream submodules. This repository uses empty `.rustfmt.toml` guard files to exempt specific directory trees:

- `dxil-spirv-sys/generated/.rustfmt.toml` protects the generated bindgen bindings (`bindings.rs`).
- `dxil-spirv-sys/dxil-spirv/.rustfmt.toml` protects the vendored upstream C++ repository.

Do not delete these guard files, and don't reformat bindgen output.

### Error Handling

- Define all safe wrapper error variants in `dxil-spirv/src/error.rs` using `thiserror`.
- Do not call `unwrap()` or `expect()` inside the safe library crates (`dxil-spirv` and `dxil-spirv-sys`). Always return structured `Result<T, Error>` values.
- `unwrap()` and `expect()` are permitted in unit tests, integration test assertions, and build scripts where panicking indicates immediate failure.

### Unsafe and Thread Safety Policy

The safe wrapper encapsulates raw pointers returned by the upstream C API.

- `ParsedBlob` and `Converter` implement `Send` because transferring ownership of the underlying C data structures between threads is safe.
- `Converter` does not implement `Sync`. Upstream conversion functions mutate internal converter state during execution, so concurrent access across threads without external synchronization is unsound.
- Keep all `unsafe` blocks strictly isolated within `dxil-spirv/src/`.
- Every `unsafe` block must include a `// SAFETY:` comment explaining why pointer dereferences, raw buffer casts, or lifetime extensions satisfy Rust safety invariants.

### Naming Conventions

- Rust types, traits, and enum variants use `UpperCamelCase` (such as `ParsedBlob`, `ShaderStage`, `RootConstantMapping`).
- Functions, methods, and variables use `snake_case` (such as `convert_to_spirv`, `num_entry_points`).
- Constants use `SCREAMING_SNAKE_CASE` (such as `KNOWN_MISSING`).
- Mirror upstream concept names when wrapping C API structures so developers can transition smoothly between upstream documentation and Rust docs.

## Documentation Standards

All contributors must adhere to the documentation standards defined in [README.md](README.md).

Key documentation requirements:

- **Location:** All project documentation belongs in `docs/`. Never add top-level markdown files other than the root `README.md` and `README.zh-CN.md`.
- **One topic, one file:** Organize documents by concern and audience. When a document grows beyond roughly 400 lines, split it into smaller focused files.
- **Naming:** Use kebab-case filenames without version suffixes (for example `platform-support.md`, never `platform_support.md` or `platform_v2.md`).
- **Index discoverability:** Every markdown file in `docs/` must be registered in the document map within [README.md](README.md).
- **Relative links:** Write relative links between documents (like `[platform-support.md](platform-support.md)`) so links work on GitHub, crates.io, and in offline clones.
- **Evidence-backed claims:** Back up all coverage numbers and performance metrics with verifiable test references (such as `tests/api_coverage.rs` or [testing.md](testing.md)).
- **Doc updates with code:** If your pull request introduces a new public API feature, update [usage.md](usage.md). If your change alters behavior or fixes a bug, add a descriptive entry to [changelog.md](changelog.md).

## Testing Requirements

Every code modification must maintain or improve existing test coverage.

### API Coverage Guard

The integration test in `dxil-spirv/tests/api_coverage.rs` ensures that every public C function (`dxil_spv_*`) exported in `dxil_spirv_c.h` is either:

1. Wrapped by the safe `dxil-spirv` Rust layer, or
2. Listed in `KNOWN_MISSING` with an explicit reason explaining why it is deferred.

When upstream updates introduce new C API exports, `api_coverage.rs` fails immediately. If you expose a previously unwrapped function, remove it from `KNOWN_MISSING`.

### Running Specific Tests

To run targeted test suites during development:

- Safe wrapper API coverage guard:
  ```sh
  cargo test -p dxil-spirv --test api_coverage
  ```
- Safe wrapper unit and doc tests:
  ```sh
  cargo test -p dxil-spirv
  ```
- Shader conversion end-to-end suite (Windows):
  ```sh
  cargo test -p dxil-spirv-tests
  ```

### Adding Shader and Wrapper Tests

- Place unit tests for new safe wrapper methods in their respective module under `dxil-spirv/src/` or as integration tests in `dxil-spirv/tests/`.
- For shader translation tests, follow the naming marker conventions outlined in [testing.md](testing.md).
- Run tests in isolated child processes when handling unknown or invalid shader inputs to prevent upstream C++ assertions from aborting the test runner.

## Common Build Pitfalls

1. **Missing Git Submodules:**
   If you encounter compilation errors indicating missing `dxil_spirv_c.h` or CMake build directory errors, verify that submodules are populated with `git submodule update --init --recursive`.
2. **Stale Target Directory After CMake Changes:**
   Cargo does not track internal CMake dependency artifacts automatically. If you modify C++ source files or CMake configuration, run `cargo clean` and rebuild fresh.
3. **Clang Not Found During Bindgen:**
   If `dxil-spirv-sys` fails to locate `libclang`, ensure `LIBCLANG_PATH` points to your LLVM bin or lib directory.

## Versioning Policy

This workspace follows Semantic Versioning with build metadata:

- Package version strings take the form `MAJOR.MINOR.PATCH+dxil-spirv.X.Y.Z`.
- The `+dxil-spirv.X.Y.Z` build metadata tracks the vendored upstream C API version (`DXIL_SPV_API_VERSION_*` from `dxil_spirv_c.h`).
- Cargo and crates.io ignore the `+...` build metadata suffix during dependency resolution, but the tag provides unambiguous traceability for users and downstream tools.

## License

Contributions submitted to this repository are dual-licensed under the workspace terms:

- MIT License ([LICENSE-MIT](../LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE))

The vendored upstream `dxil-spirv` C++ codebase is licensed under the MIT License (`dxil-spirv-sys/dxil-spirv/LICENSE.MIT`).

Submitting a pull request confirms your agreement to license your work under these terms.
