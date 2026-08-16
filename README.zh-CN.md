# dxil-spirv-rs

[English](README.md) | [中文](README.zh-CN.md)

[dxil-spirv](https://github.com/HansKristian-Work/dxil-spirv) 的安全 Rust 绑定 — 将 D3D11/D3D12 着色器字节码（DXBC 容器或 DXIL 位码）转换为 SPIR-V。

将生成的 SPIR-V 送入交叉编译器（如 [SPIRV-Cross](https://github.com/KhronosGroup/SPIRV-Cross)）即可恢复可读的 **HLSL / GLSL / MSL** 源码，或直接用 Vulkan 工具链消费。典型用途：着色器检查/调试工具、D3D12 着色器逆向工程、D3D→Vulkan 转换研究。

```text
DXBC / DXIL 容器 ──dxil-spirv──▶ SPIR-V ──SPIRV-Cross──▶ HLSL / GLSL / MSL
```

**当前状态**：edition 2024，MSRV 1.85，CI 在 Windows / Linux / macOS 全绿，**上游 829 个着色器测试全部通过（100%）**。

## AI 维护声明

本项目由 **AI 维护**：它由 **Kimi K3** 模型创建，AI 生成与 AI 编辑的代码是被明确欢迎的，也是本项目演进的常态。全程有人类方向的把控与审查；AI 遵守与人类贡献者完全相同的标准。详见 [AI 维护政策](docs/contributing.md)。

## 使用本 crate（给使用者）

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

如需更精细的控制 — 入口点选择、转换器选项、根签名、描述符重映射 — 显式驱动各阶段：

```rust
use dxil_spirv::{Converter, ParsedBlob};

let parsed = ParsedBlob::parse(&blob)?;
let converter = Converter::new(&parsed)?;
converter.run()?;
let spirv_words = converter.compiled_spirv()?;
```

**完整使用指南**：[docs/usage.md](docs/usage.md) — 每个转换器选项、重映射配置、错误处理与平台注意事项。

## 开发本 crate（给开发者）

本 crate 采用 `-sys` + 安全层拆分，构建时通过 CMake 编译 vendored 的上游 C++ 库：

| Crate | 路径 | 角色 |
|---|---|---|
| `dxil-spirv` | `dxil-spirv/` | 安全、惯用的包装器 — 你依赖的目标 |
| `dxil-spirv-sys` | `dxil-spirv-sys/` | 原始 bindgen FFI + CMake 构建（传递链接） |
| `dxil-spirv-tests` | `dxil-spirv-tests/` | 针对全部上游着色器的端到端测试套件 |

安全包装器暴露了上游 C API（`dxil_spv_*`）的**所有**函数 — 由编译期测试（`dxil-spirv/tests/api_coverage.rs`）强制保证，上游新增函数未包装即失败。

**从这里开始**：[docs/architecture.md](docs/architecture.md) — crate 拓扑、FFI 边界规则、静态链接闭包，以及跨平台踩坑实录（CMake、bindgen、C++ 链接的付费教训）。

**测试**：[docs/testing.md](docs/testing.md) — 829 个着色器套件如何工作、回归基线机制、如何添加测试。

**CI**：[docs/ci.md](docs/ci.md) — job 布局、平台策略、以及塑造它的缓存教训。

**贡献**：[docs/contributing.md](docs/contributing.md) — 贡献流程、代码规范、AI 维护政策。

## 我们做了什么（项目历程）

- **初始绑定** — workspace 拆分、RAII 包装器、FFI 蹦床、静态链接闭包。
- **完整 API 覆盖** — 上游 64 个 C 函数全部包装，零缺口。
- **端到端测试套件** — 829 个上游着色器、DXC 集成、子进程隔离、GLSL 往返验证。
- **回归基线** — 逐着色器的 pass/fail 跟踪，带硬性回归检测。
- **100% 通过率** — 76.2% → 98.9% → **829/829（100%）**，通过补全上游选项/重映射表面达成。
- **Edition 2024 + 跨平台 CI** — rustfmt、MSRV 1.85、CI 在 Windows / Linux / macOS 全绿。

完整故事，逐里程碑带提交记录：[docs/changelog.md](docs/changelog.md)。

## 许可证

以下许可证任选其一：

- Apache License, Version 2.0（[LICENSE-APACHE](LICENSE-APACHE)）
- MIT 许可证（[LICENSE-MIT](LICENSE-MIT)）

vendored 的上游 `dxil-spirv` 采用 MIT 许可证；见 `dxil-spirv-sys/dxil-spirv/LICENSE.MIT`。
