# 贡献指南

[English](contributing.md) | [中文](contributing.zh-CN.md)

欢迎参与 dxil-spirv-rs 项目。本项目为上游 dxil-spirv C++ 库提供安全的 Rust 绑定，将 DXBC 容器和 DXIL bitcode 转换为 SPIR-V。我们同样欢迎人类开发者与 AI 编码智能体的贡献。

## AI 维护政策

本项目由 AI 维护。机器生成与机器编辑的代码、测试和文档均被明确欢迎，并且是这里的标准工作流。本项目由 **Kimi K3** 模型创建，并继续在人类指导下通过 AI 辅助进行维护。

无论你是手写代码、向 AI 智能体提提示词，还是构建自动化工作流，所有贡献均适用完全相同的标准：

1. 每个 pull request 都必须通过完整的 acceptance gate。这意味着通过 `cargo fmt` 进行正确的格式化，在 `cargo clippy --workspace --all-targets -- -D warnings` 下保持零 linter 警告，并通过全部测试套件。
2. 阐明断言的事实依据。在更新文档或报告 benchmark 数据时，应包含具体证据，例如 commit hash、测试通过数量或命令输出日志。
3. 切勿静默跳过验证。如果某项测试无法在你的本地机器上运行（例如在 Linux 或 macOS 上开发时遇到的 Windows 专有 DXC 着色器编译），应在 pull request 描述中明确说明，而不是进行猜测。

## 如何贡献

### 报告问题

当你发现 bug 或异常行为时，请在 GitHub 上创建 issue。请附带以下信息：

- 最小可复现示例，例如独立的着色器字节码 blob 或简短的 Rust 测试用例。
- 你的 target triple 和宿主平台（例如 `x86_64-pc-windows-msvc`、`x86_64-unknown-linux-gnu` 或 `aarch64-apple-darwin`）。
- 观察到的输出、错误代码或 panic 调用栈追踪。
- 期望的行为或输出。

如果你想建议新功能，请描述图形使用场景并指出支持该场景所需的上游 C API 函数（`dxil_spv_*`）。

### Pull Request 工作流

1. 在 GitHub 上 fork 本仓库，并递归克隆所有 submodule 到本地：
   ```sh
   git clone --recursive https://github.com/ChahuProject/dxil-spirv-rs.git
   ```
   如果克隆时未包含 submodule，请立即拉取它们：
   ```sh
   git submodule update --init --recursive
   ```
2. 为你的工作创建一个专用的分支：
   ```sh
   git checkout -b feat/remapper-resource-types
   ```
3. 遵循下文详述的代码与文档规范进行修改。
4. 在本地运行 acceptance gate 以验证格式化、linter 检查和测试。
5. 使用清晰且祈使语气的提交信息提交你的修改：
   - `feat: expose mesh shader node constants in safe wrapper`
   - `fix: correct root descriptor table offset calculation`
   - `docs: update platform support matrix for FreeBSD`
6. 将你的分支推送至 fork 仓库，并向 `main` 分支提交 pull request。

## 本地开发环境搭建

Workspace 在 `cargo build` 期间通过 CMake 从源码编译 vendored 的 C++ 核心。请确保系统已安装所需的构建工具：

- **Windows：** Visual Studio 2022（MSVC 及 C++ 工具）和 CMake。
- **Linux：** GCC 13 或 Clang、CMake、Ninja 以及 `libclang-dev`（供 `bindgen` 使用）。
   ```sh
   sudo apt-get update
   sudo apt-get install -y cmake ninja-build libclang-dev clang
   ```
- **macOS：** Apple Clang、CMake、Ninja 和 LLVM。
   ```sh
   brew install cmake ninja llvm
   ```

有关编译器要求的更多细节，请查阅 [platform-support.md](platform-support.zh-CN.md)。

## Acceptance Gate

持续集成（CI）会在 Windows、Linux 和 macOS 上对每个 pull request 运行。在合并之前，所有任务都必须为绿色。

在推送前，请在本地按顺序运行以下命令：

1. 格式检查：
   ```sh
   cargo fmt --all -- --check
   ```
2. Linter 检查（所有警告均视为硬错误）：
   ```sh
   cargo clippy --workspace --all-targets -- -D warnings
   ```
3. 构建所有 workspace target：
   ```sh
   cargo build --workspace --all-targets
   ```
4. 运行所有 workspace 测试：
   ```sh
   cargo test --workspace
   ```

### Windows 端到端测试细节

`dxil-spirv-tests` 中的端到端测试套件使用 Microsoft DXC（`dxc.exe`）将 HLSL 着色器编译为 DXIL bitcode。由于官方预编译的 DXC 编译器以 Windows x64 二进制形式运行，因此完整的 829 个着色器测试套件会在本地 Windows 运行以及 Windows CI runner 上执行。

在 Linux 和 macOS 上，测试套件会运行单元测试和安全包装器集成测试，但会跳过 HLSL 编译，除非 `DXC_PATH` 指向可用的 DXC 二进制文件。如果你在 Linux 或 macOS 上开发，请确保 `cargo test --workspace` 能顺利通过所有跨平台测试。Windows CI 会在你的 pull request 上验证完整的着色器套件。

## 代码规范

### Rust Edition 与工具链

本 workspace 面向 Rust edition 2024。它要求 `Cargo.toml` 中 `rust-version = "1.85"` 所指定的稳定版编译器（并在 `rust-toolchain.toml` 中固定）。

### 格式化规则与保护文件

Workspace 根目录包含 `rustfmt.toml`，用于配置所有第一方 crate（`dxil-spirv`、`dxil-spirv-sys` 和 `dxil-spirv-tests`）的代码格式化。

切勿格式化生成的代码或 vendored 的上游 submodule。本仓库使用空的 `.rustfmt.toml` 保护文件来豁免特定的目录树：

- `dxil-spirv-sys/generated/.rustfmt.toml` 保护生成的 bindgen 绑定（`bindings.rs`）。
- `dxil-spirv-sys/dxil-spirv/.rustfmt.toml` 保护 vendored 的上游 C++ 仓库。

请勿删除这些保护文件，也不要重新格式化 bindgen 输出。

### 错误处理

- 在 `dxil-spirv/src/error.rs` 中使用 `thiserror` 定义所有安全包装器错误变体。
- 不要在安全库 crate（`dxil-spirv` 和 `dxil-spirv-sys`）内部调用 `unwrap()` 或 `expect()`。始终返回结构化的 `Result<T, Error>` 值。
- 在单元测试、集成测试断言和构建脚本中允许使用 `unwrap()` 与 `expect()`，在这些场景下 panic 代表直接失败。

### Unsafe 与线程安全策略

安全包装器封装了上游 C API 返回的裸指针。

- `ParsedBlob` 和 `Converter` 实现了 `Send`，因为在线程之间转移底层 C 数据结构的所有权是安全的。
- `Converter` 没有实现 `Sync`。上游转换函数在执行期间会修改转换器的内部状态，因此在没有外部同步的情况下跨线程并发访问是不健全的。
- 将所有 `unsafe` 代码块严格隔离在 `dxil-spirv/src/` 内部。
- 每个 `unsafe` 块都必须包含 `// SAFETY:` 注释，说明为什么指针解引用、裸缓冲区转换或生命周期延长满足 Rust 安全不变性。

### 命名规范

- Rust 类型、trait 和枚举变体使用 `UpperCamelCase`（例如 `ParsedBlob`、`ShaderStage`、`RootConstantMapping`）。
- 函数、方法和变量使用 `snake_case`（例如 `convert_to_spirv`、`num_entry_points`）。
- 常量使用 `SCREAMING_SNAKE_CASE`（例如 `KNOWN_MISSING`）。
- 包装 C API 结构时应沿用上游概念名称，以便开发者在上游文档与 Rust 文档之间平滑过渡。

## 文档标准

所有贡献者都必须遵守 [README.md](README.zh-CN.md) 中定义的文档标准。

关键文档要求：

- **位置：** 所有项目文档都存放在 `docs/` 中。除根目录的 `README.md` 和 `README.zh-CN.md` 之外，绝不添加顶层 markdown 文件。
- **一题一文件：** 按关注点和受众组织文档。当单个文档超过约 400 行时，应将其拆分为更小且专注的文件。
- **命名：** 使用 kebab-case 文件名，不带版本后缀（例如 `platform-support.md`，绝不要使用 `platform_support.md` 或 `platform_v2.md`）。
- **索引可发现性：** `docs/` 中的每个 markdown 文件都必须注册在 [README.md](README.zh-CN.md) 的文档地图中。
- **相对链接：** 在文档之间使用相对链接（如 `[platform-support.md](platform-support.zh-CN.md)`），以便链接在 GitHub、crates.io 以及离线克隆环境中均可正常工作。
- **证据支持的断言：** 所有的覆盖率数字和性能指标都必须有可验证的测试依据作为支撑（例如 `tests/api_coverage.rs` 或 [testing.md](testing.zh-CN.md)）。
- **文档随代码更新：** 如果你的 pull request 引入了新的公开 API 功能，请更新 [usage.md](usage.zh-CN.md)。如果你的修改改变了行为或修复了 bug，请在 [changelog.md](changelog.zh-CN.md) 中添加描述性条目。

## 测试要求

每次代码修改都必须保持或提高现有的测试覆盖率。

### API 覆盖守卫

`dxil-spirv/tests/api_coverage.rs` 中的集成测试确保 `dxil_spirv_c.h` 中导出的每个公开 C 函数（`dxil_spv_*`）都满足以下条件之一：

1. 由安全的 `dxil-spirv` Rust 层进行包装，或
2. 列在 `KNOWN_MISSING` 中，并附带明确的推迟原因。

当上游更新引入新的 C API 导出时，`api_coverage.rs` 会立即失败。如果你公开了之前未包装的函数，请将其从 `KNOWN_MISSING` 中移除。

### 运行特定测试

在开发过程中运行目标测试套件：

- 安全包装器 API 覆盖守卫：
   ```sh
   cargo test -p dxil-spirv --test api_coverage
   ```
- 安全包装器单元测试和文档测试：
   ```sh
   cargo test -p dxil-spirv
   ```
- 着色器转换端到端套件（Windows）：
   ```sh
   cargo test -p dxil-spirv-tests
   ```

### 添加着色器与包装器测试

- 将新安全包装器方法的单元测试放在 `dxil-spirv/src/` 下各自的模块中，或作为集成测试放在 `dxil-spirv/tests/` 中。
- 对于着色器转换测试，请遵循 [testing.md](testing.zh-CN.md) 中概述的命名标记规范。
- 处理未知或无效的着色器输入时，请在隔离的子进程中运行测试，以防止上游 C++ assertion 导致测试运行器直接中止。

## 常见构建陷阱

1. **缺少 Git Submodule：**
   如果遇到提示缺少 `dxil_spirv_c.h` 的编译错误或 CMake 构建目录错误，请使用 `git submodule update --init --recursive` 验证 submodule 是否已拉取完整。
2. **修改 CMake 后 target 目录未更新：**
   Cargo 不会自动跟踪内部 CMake 依赖项产物。如果你修改了 C++ 源文件或 CMake 配置，请运行 `cargo clean` 并重新构建。
3. **Bindgen 期间未找到 Clang：**
   如果 `dxil-spirv-sys` 无法找到 `libclang`，请确保 `LIBCLANG_PATH` 指向你的 LLVM bin 或 lib 目录。

## 版本管理策略

本 workspace 遵循带有构建元数据的语义化版本控制（Semantic Versioning）：

- Package 版本字符串格式为 `MAJOR.MINOR.PATCH+dxil-spirv.X.Y.Z`。
- `+dxil-spirv.X.Y.Z` 构建元数据跟踪 vendored 的上游 C API 版本（来自 `dxil_spirv_c.h` 的 `DXIL_SPV_API_VERSION_*`）。
- Cargo 和 crates.io 在依赖解析期间会忽略 `+...` 构建元数据后缀，但该标签为用户和下游工具提供了清晰明确的可追溯性。

## 许可证

提交至本仓库的贡献在 workspace 条款下双重授权：

- MIT License（[LICENSE-MIT](../LICENSE-MIT)）
- Apache License, Version 2.0（[LICENSE-APACHE](../LICENSE-APACHE)）

vendored 的上游 `dxil-spirv` C++ 代码库采用 MIT 许可证（`dxil-spirv-sys/dxil-spirv/LICENSE.MIT`）。

提交 pull request 即代表你同意在这些条款下授权你的工作。
