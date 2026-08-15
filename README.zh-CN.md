# dxil-spirv-rs

[English](README.md) | [中文](README.zh-CN.md)

[dxil-spirv](https://github.com/HansKristian-Work/dxil-spirv) 的安全 Rust 绑定 — 将 D3D11/D3D12 着色器字节码（DXBC 容器或 DXIL 位码）转换为 SPIR-V。

将生成的 SPIR-V 送入交叉编译器（如 [SPIRV-Cross](https://github.com/KhronosGroup/SPIRV-Cross)）即可恢复可读的 **HLSL / GLSL / MSL** 源码，或直接用 Vulkan 工具链消费。典型用途：着色器检查/调试工具、D3D12 着色器逆向工程、D3D→Vulkan 转换研究。

```text
DXBC / DXIL 容器 ──dxil-spirv──▶ SPIR-V ──SPIRV-Cross──▶ HLSL / GLSL / MSL
```

## AI 生成声明

本 crate **完全由 AI 生成**（大型语言模型编码代理），经人类指导和审查。无手写逻辑。

**生成方式：**

1. **底层** — 上游 `dxil-spirv` C++ 库（MIT 许可证，Hans-Kristian Arntzen / Valve）作为 git 子模块 vendored 在 `dxil-spirv-sys/dxil-spirv` 下。代理未重新实现任何转换逻辑；仅进行绑定。
2. **sys 层（`dxil-spirv-sys`）** — 一个 `build.rs`，编译上游 `dxil-spirv-c-static` CMake 目标，并对上游 C 头文件 `dxil_spirv_c.h` 运行 [bindgen](https://github.com/rust-lang/rust-bindgen) 生成原始 FFI 接口。
3. **安全层（`dxil-spirv`）** — RAII 包装器（`ParsedBlob`、`Converter`）、类型化的选项/绑定/重映射结构体，以及 `thiserror` 错误类型，全部由代理基于生成的绑定编写。
4. **参考驱动** — 绑定结构和 `build.rs` 模式以成熟的 [`grovesNL/spirv_cross`](https://github.com/grovesNL/spirv_cross) crate 和积极维护的 [`SnowflakePowered/spirv-cross2-rs`](https://github.com/SnowflakePowered/spirv-cross2-rs)（现代化的 `-sys` + 安全层架构，具有更强的类型/生命周期安全模式）为模板。捆绑的维护技能（`.agents/skills/sync-upstream`）编码了已验证的事实（静态链接闭包、CRT 规则、bindgen 边界、回调蹦床模式），以便未来更新可由代理安全地重新生成。

由于代码是机器生成的，请像对待任何新依赖一样谨慎：生产使用前请审查，发现任何异常请报告。欢迎 Issue 和人类审查。

## 快速开始

### 前置要求

- **Rust**（稳定版；见 `rust-toolchain.toml`）
- **C++ 工具链 + CMake**（sys crate 在构建时编译上游 C++ 库）：
  - Windows：MSVC（Visual Studio Build Tools）+ CMake
  - Linux/macOS：C++14 编译器 + CMake
- **git 子模块**：本仓库 vendored 上游源码，请递归克隆。

### 克隆与构建

```sh
git clone --recursive https://github.com/ChahuProject/dxil-spirv-rs.git
cd dxil-spirv-rs

# 如果已经克隆但没有 --recursive：
git submodule update --init --recursive

cargo build --workspace
cargo test  --workspace
```

### 使用

在 `Cargo.toml` 中添加：

```toml
[dependencies]
dxil-spirv = "0.1"
```

将着色器二进制转换为 SPIR-V：

```rust
fn main() -> dxil_spirv::Result<()> {
    // 完整的 DXBC 容器（SM4/SM5/SM6）或原始 DXIL 位码切片。
    let blob: Vec<u8> = std::fs::read("shader.dxil").expect("读取着色器");

    let spirv_words = dxil_spirv::convert_to_spirv(&blob)?;
    println!("生成了 {} 个 SPIR-V 字", spirv_words.len());

    // 将 `spirv_words` 交给 SPIRV-Cross（例如 `spirv_cross` crate）以
    // 输出 HLSL / GLSL / MSL。
    Ok(())
}
```

如需更精细的控制，显式驱动各阶段：

```rust
use dxil_spirv::{Converter, ParsedBlob};

let parsed = ParsedBlob::parse(&blob)?;
println!("阶段: {:?}, 入口点数量: {}", parsed.shader_stage(), parsed.num_entry_points()?);

let converter = Converter::new(&parsed)?;
converter.run()?;
let spirv_words = converter.compiled_spirv()?;
```

### Crate 布局

| Crate | 路径 | 用途 |
|---|---|---|
| `dxil-spirv` | `dxil-spirv/` | 安全、惯用的包装器 — 你依赖的目标 |
| `dxil-spirv-sys` | `dxil-spirv-sys/` | 原始 bindgen FFI + CMake 构建（传递链接） |

### API 覆盖

安全包装器暴露了上游 C API（`dxil_spv_*`）的**所有**函数，包括：

- 核心转换（`parse` → `convert` → `compiled_spirv`）
- 全部 8 个重映射回调（SRV、UAV、CBV、采样器、顶点输入、阶段 I/O、流输出）
- 根签名 / 描述符映射（本地根常量、描述符表、参数映射）
- Work Graphs / 网格节点内省（SM6.8）
- 资源扫描（转换前内省）
- RDAT 子对象（DXR 状态对象）
- 线程日志回调和分配器上下文管理

编译时测试（`tests/api_coverage.rs`）确保没有上游函数被意外遗漏。
如果上游新增函数，测试会失败，直到被包装或明确记录为有意跳过。

## 许可证

以下许可证任选其一：

- Apache License, Version 2.0（[LICENSE-APACHE](LICENSE-APACHE)）
- MIT 许可证（[LICENSE-MIT](LICENSE-MIT)）

vendored 的上游 `dxil-spirv` 采用 MIT 许可证；见 `dxil-spirv-sys/dxil-spirv/LICENSE.MIT`。
