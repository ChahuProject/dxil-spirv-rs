# 平台支持

[English](platform-support.md) | [中文](platform-support.zh-CN.md)

`dxil-spirv-rs` 封装了上游 `dxil-spirv` C++ 库（通过 CMake 从源码构建）并暴露安全的 Rust API。由于 C++ 核心在本地编译，该 crate 可在任何具备可用 C++17 工具链和 CMake 的平台上工作。

## 支持的平台

以下平台在每次 push 和 pull request 时都会在 CI 中构建和测试。

| OS      | 架构 | Rust target              | C++ 工具链      | 状态             |
|---------|--------------|--------------------------|--------------------|--------------------|
| Windows | x86_64       | `x86_64-pc-windows-msvc` | MSVC (VS 2022)     | ✅ 已在 CI 中测试     |
| Linux   | x86_64       | `x86_64-unknown-linux-gnu` | GCC 13 / Clang   | ✅ 已在 CI 中测试     |
| macOS   | aarch64 (Apple Silicon) | `aarch64-apple-darwin` | Apple Clang | ✅ 已在 CI 中测试     |

### 各平台说明

- **Windows (MSVC)** — 参考平台。C++ 核心使用动态 CRT（`/MD` 或 `/MDd`）构建以匹配 Rust 的默认链接方式，并通过 `/EHsc` 启用 C++ 异常。DXC（仅测试套件使用的 HLSL 编译器）是 Windows 二进制，因此完整的端到端着色器测试套件在这里最完整。

- **Linux (GNU)** — 使用 GCC 或 Clang 构建。C++ 源码默认启用 C++ 异常，因此无需特殊标志。需要 `cmake`、`ninja-build`、`libclang-dev` 和 `clang`（用于 `bindgen`）。

- **macOS (Apple Silicon)** — 使用 Apple Clang 构建。需要 `cmake`、`ninja` 和 `llvm`（用于 `bindgen`）。Intel macOS（`x86_64-apple-darwin`）应同样工作，但未在 CI 中验证。

## 要求

- **Rust**：见 `Cargo.toml` 中的 `rust-version`（当前为 **1.85**，对应 edition 2024）。
- **CMake**：建议 3.20 或更新（用于构建 vendored 的 C++ 核心）。
- **C++ 工具链**：任何支持 C++17 的编译器（MSVC、GCC 或 Clang）。
- **libclang**（仅构建期，用于 `bindgen` 生成 FFI 绑定）。
- **Ninja**（可选但推荐；加速 CMake 构建）。

## 仅测试套件的注意事项：DXC

`dxil-spirv-tests` crate 使用微软的 **DXC** 编译 HLSL 测试着色器，而 DXC 以 **Windows x64** 二进制形式发布。在非 Windows 平台上，构建脚本无法运行下载的 `dxc.exe`，因此着色器必须预编译（或通过 `DXC_PATH` 环境变量以其他方式提供 DXC）。这**只影响测试框架**，不影响发布的 `dxil-spirv` / `dxil-spirv-sys` 库 — 库本身完全跨平台。

## 添加新平台

只要满足上述要求，新平台即可受支持。验证步骤：

1. 确保安装了 `cmake`、C++17 编译器和 `libclang`。
2. 运行 `cargo build --workspace` — C++ 核心应能配置并构建。
3. 运行 `cargo test --workspace` — 安全包装器和转换测试应通过。

如果你验证了此处未列出的平台（例如 `x86_64-apple-darwin`、`aarch64-unknown-linux-gnu` 或 `x86_64-pc-windows-gnu`），请开 issue 或 PR，以便将其加入支持矩阵，并在可行时加入 CI。
