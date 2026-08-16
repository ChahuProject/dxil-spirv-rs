# CI 架构

[English](ci.md) | [中文](ci.zh-CN.md)

本文档介绍 `dxil-spirv-rs` 的持续集成（CI）架构，定义于 `.github/workflows/ci.yml`。内容涵盖 job 拓扑结构、跨平台验证策略、缓存策略、本地推送前验证，以及 CI 踩坑实录与修复历史。

关于平台要求和支持的目标三元组，参见 [platform-support.md](platform-support.zh-CN.md)。关于测试套件机制与着色器覆盖率指标，参见 [testing.md](testing.zh-CN.md)。

## Job 概览与 Workflow 拓扑

每次向 `main` 分支推送代码或提交 Pull Request 时，CI 都会在 GitHub Actions 上执行。Workflow 被划分为三个 job，将快速健全性检查与耗时较重的原生编译分离。

| `ci.yml` 中的 Job ID | 步骤名称 | Runner OS | 超时时间 | 执行命令 |
|---|---|---|---|---|
| `fmt` | `rustfmt` | `ubuntu-latest` | 5 min | `cargo fmt --all -- --check` |
| `clippy` | `clippy (${{ matrix.os }})` | `windows-latest`<br>`ubuntu-latest`<br>`macos-latest` | 30 min | `cargo clippy --workspace --all-targets -- -D warnings` |
| `build-test` | `build & test (${{ matrix.os }})` | `windows-latest`<br>`ubuntu-latest`<br>`macos-latest` | 45 min | `cargo build --workspace --all-targets --verbose`<br>`cargo test --workspace --verbose` |

### Pipeline 结构与设计考量

Pipeline 强制执行三个阶段的验证：

1. **快速格式检查关卡（`fmt`）**：
   在单台 `ubuntu-latest` runner 上运行。不拉取 submodule 也不安装 C++ 依赖，耗时在 30 秒以内。若代码风格或 import 顺序违反 `rustfmt.toml`（edition 2024），流水线会立即终止，避免启动更耗资源的矩阵 runner。

2. **跨平台 Lint 检查关卡（`clippy`）**：
   在所有三个操作系统上运行，设置 `strategy.fail-fast: false`。Clippy 主要基于 check 阶段的产物工作。将其作为独立的矩阵 job 运行，能够在不阻塞完整测试执行的情况下，及早暴露特定平台的 lint 回归（例如类型转换 lint 或 target 配置警告）。

3. **统一构建与测试关卡（`build-test`）**：
   在所有三个操作系统上运行。通过 CMake 编译上游 C++ 静态库（`dxil-spirv-c-static`），并执行单元测试、安全包装器测试以及集成测试套件。依次运行 `cargo build --workspace --all-targets` 与 `cargo test --workspace`，确保 C++ 核心和测试二进制文件在同一个 target 工作区内只编译一次。

### Runner 环境配置与宿主依赖

在 Linux（`ubuntu-latest`）上，CI 通过 `apt-get` 安装构建依赖：
- `cmake` 与 `ninja-build`：供 `cmake-rs` 配置和编译上游 C++ 源码使用。
- `libclang-dev` 与 `clang`：编译 `dxil-spirv-sys` 期间供 `bindgen` 解析 `dxil_spirv_c.h` 所需。

在 macOS（`macos-latest`，Apple Silicon `aarch64-apple-darwin`）上，CI 通过 Homebrew 安装依赖：
- `cmake` 与 `ninja`：C++ 静态核心的构建工具。
- `llvm`：为 `bindgen` 提供 `libclang`。

在 Windows（`windows-latest`，`x86_64-pc-windows-msvc`）上，runner 镜像原生内置了 MSVC 与 CMake。

## 平台策略与测试框架差异

仓库包含三个 workspace crate：
- `dxil-spirv-sys`：编译 vendored 的 C++ 核心并提供原始 FFI 绑定。
- `dxil-spirv`：安全、惯用的 Rust API 包装器。
- `dxil-spirv-tests`：集成测试套件与 829 个着色器的回归测试框架。

核心库（`dxil-spirv` 与 `dxil-spirv-sys`）在 Windows、Linux 和 macOS 上具有 100% 的功能对等性并能正常编译运行。然而，端到端测试框架因外部编译器可用性存在差异。

### DXC 二进制限制与 `dxc_unavailable`

将 HLSL 测试着色器编译为 DXIL 字节码需要 Microsoft DirectX Shader Compiler (DXC)。在 `dxil-spirv-tests/build.rs` 中，构建脚本按以下顺序查找 DXC：
1. `target/dxc/1.9.2602.17/dxc.exe` 下的缓存可执行文件。
2. `DXC_PATH` 环境变量中指定的路径。
3. 系统 `PATH`（`dxc --version`）。
4. Windows Kits 目录回退路径（`C:\Program Files (x86)\Windows Kits\10\bin`）。
5. 自动下载微软官方发布的 `v1.9.2602`（`dxc_2026_02_20.zip`）。

由于微软发布的 DXC release 压缩包仅包含 Windows x64 二进制文件，下载的 `dxc.exe` 无法在 Linux 或 macOS 内核上执行（会报错 `Permission denied` 或 `Exec format error`）。

为了在不禁用集成测试编译的前提下保持 Unix runner CI 状态正常：

1. `dxil-spirv-tests/build.rs` 调用 `is_dxc_runnable(&dxc_path)`，通过 `--version` 测试进程能否执行。
2. 当不可执行时，`build.rs` 输出 `cargo:rustc-cfg=dxc_unavailable`。
3. 为符合编译器 lint 要求，`build.rs` 无条件输出 `cargo:rustc-check-cfg=cfg(dxc_unavailable)`.
4. 在 `dxil-spirv-tests/tests/e2e.rs` 中，测试入口函数（`test_smoke`、`run_category`、`test_metrics_report`）会评估 `if cfg!(dxc_unavailable)` 并提前退出且带有跳过提示。
5. Windows CI runner 执行全部 829 个着色器的完整端到端测试。Linux 和 macOS CI runner 则编译所有测试框架、执行所有单元测试并验证安全 API 包装器，不会触发完整性检查失败。

## 缓存策略

CI 使用 GitHub Actions 缓存（`actions/cache@v4`），并设定了严格的缓存边界。

### 缓存内容

- `~/.cargo/registry`：已下载的 crate 索引和包归档。
- `~/.cargo/git`：已克隆的 git 依赖仓库。

缓存 key 按 runner 操作系统隔离，并使用 lockfile 哈希作为后缀：
```yaml
key: ${{ runner.os }}-build-crates-${{ hashFiles('**/Cargo.lock') }}
restore-keys: ${{ runner.os }}-build-crates-
```

### 为什么不缓存 `target/`（陈旧 CMake 产物的教训）

早期的 CI 配置曾尝试缓存 `target/` 目录，以避免在没有变更时重新构建 C++ 核心。这导致了持续出现的链接失败：

1. **不可见的构建产物**：上游 C++ 静态库（`libdxbc-spirv.a`、`libglslang-spirv-builder.a`、`libllvm-bc.a`、`libbc-decoder.a`，或 Windows 上的 `.lib`）由 CMake 在 `target/debug/build/dxil-spirv-sys-*/out/build/` 下生成。
2. **指纹断联**：Cargo 会跟踪 Rust 源码和 `build.rs` 脚本，但不会监控中间的 CMake 构建产物。
3. **陈旧链接引用**：从缓存恢复 `target/` 会带回预编译的 Rust `.rlib` 归档文件，其中包含指向已丢失或已重定向的 CMake 构建目录的绝对链接路径。
4. **失败表现**：编译测试二进制文件时，链接器报错 `unable to find library -ldxbc-spirv`，因为所引用的静态库在恢复后的路径上并不存在。

从缓存中移除 `target/` 可确保 C++ 核心在每次 CI 运行中均通过 CMake 全新构建。尽管这增加了一定的编译开销，但彻底消除了链接器路径漂移问题。

## 踩坑实录（CI 修复历史）

以下记录详细梳理了在提交 `ed19ee0..5ecba0b` 期间出现的各类 CI 失败现象、根本原因以及具体的修复方案。

### 坑 1：无条件向 GCC 和 Clang 传递 MSVC `/EHsc` 参数

- **Commit**：`ed19ee0` ("Migrate to edition 2024, fix cross-platform CI, add rustfmt config")
- **症状**：Linux 和 macOS CI runner 上的 CMake 配置失败，报错如下：
  ```text
  c++: error: no such file or directory: '/EHsc'
  ```
- **根本原因**：`dxil-spirv-sys/build.rs` 无条件向 `cmake::Config` 传入了 `.cxxflag("/EHsc")`。MSVC 需要 `/EHsc` 来启用 C++ 结构化异常处理，但 GCC 和 Clang 会将 `/EHsc` 视为无法识别的文件参数。
- **修复方案**：在 `dxil-spirv-sys/build.rs` 中将该参数限制为仅在 MSVC target 上传递：
  ```rust
  if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
      cfg.cxxflag("/EHsc");
  }
  ```
  GCC 和 Clang 在编译 C++ 源码时默认启用 C++ 异常。

### 坑 2：陈旧 `target/` 缓存导致静态库链接丢失

- **Commit**：`6d7757f` ("Fix CI: drop fragile target/ cache causing stale C++ lib links")
- **症状**：Linux 和 macOS 构建在最终链接阶段失败，报错如下：
  ```text
  error: linking with `cc` failed: exit status: 1
  = note: /usr/bin/ld: cannot find -ldxbc-spirv: No such file or directory
  ```
- **根本原因**：CI workflow 缓存了 `target/`。缓存的 `.rlib` 文件包含了指向 CMake 静态库的硬编码路径，而在全新创建的 runner 环境中这些文件并不存在。
- **修复方案**：从 `.github/workflows/ci.yml` 的缓存路径中移除了 `target`。仅缓存 `~/.cargo/registry` 与 `~/.cargo/git`。

### 坑 3：静态库搜索忽略了 `.a` 文件扩展名

- **Commit**：`5163058` ("Fix cross-platform linking: recognize .a static libs in register_lib_dirs")
- **症状**：全新安装的 Linux 和 macOS 构建依然因 `unable to find library -ldxbc-spirv` 报错失败。
- **根本原因**：在 `dxil-spirv-sys/build.rs` 中，递归目录遍历函数 `register_lib_dirs` 仅检查了 `.lib` 扩展名的文件。在 Unix 系统上，CMake 输出的是 `.a` 归档文件（`libdxbc-spirv.a`），因此遍历跳过了构建输出目录，未曾输出 `cargo:rustc-link-search=native=...`。
- **修复方案**：更新 `register_lib_dirs` 以同时检查这两种扩展名：
  ```rust
  } else if path.extension().is_some_and(|e| e == "lib" || e == "a") {
      has_lib = true;
  }
  ```

### 坑 4：Bindgen 枚举有符号性在不同平台上不一致

- **Commit**：`ce156ac` ("Fix bindgen enum signedness breaking Linux/macOS builds")
- **症状**：Linux 和 macOS 上的 Rust 编译失败，报错如下：
  ```text
  error[E0308]: mismatched types
     --> dxil-spirv/src/converter.rs:133:43
      |
  133 |             return Err(Error::UnsupportedFeature(option.kind()));
      |                        ------------------------- ^^^^^^^^^^^^^ expected `i32`, found `u32`
  ```
- **根本原因**：在 Windows MSVC 上，bindgen 将 `dxil_spv_option` 生成为 `c_int`（`i32`）。在 Linux 和 macOS 上，由于上游 C 枚举中不含负值，Clang 和 bindgen 选择了无符号底层类型（`c_uint` / `u32`）。
- **修复方案**：在 `dxil-spirv/src/error.rs` 中将 `Error::UnsupportedFeature` 的负载类型更改为 `u32`，并在 `dxil-spirv/src/converter.rs` 中添加了显式的规范化类型转换 `option.kind() as u32`。

### 坑 5：缺少 C++ 运行时标准库链接指令

- **Commit**：`dde03e4` ("Link C++ stdlib on GCC/Clang targets")
- **症状**：Linux 和 macOS 链接失败，产生数百个未定义符号错误：
  ```text
  undefined reference to `operator new(unsigned long)'
  undefined reference to `__cxa_pure_virtual'
  undefined reference to `std::terminate()'
  ```
- **根本原因**：上游核心使用了 C++ 标准库特性。MSVC 会通过 CRT 自动链接 C++ 运行时。而当 Rust 链接静态 C/C++ 库时，GCC 和 Clang 不会自动链接 C++ 运行时。
- **修复方案**：在 `dxil-spirv-sys/build.rs` 中添加显式链接指令：
  ```rust
  match env::var("CARGO_CFG_TARGET_OS").as_deref() {
      Ok("macos") => println!("cargo:rustc-link-lib=dylib=c++"),
      Ok(os) if os != "windows" => println!("cargo:rustc-link-lib=dylib=stdc++"),
      _ => {}
  }
  ```

### 坑 6：仅支持 Windows 的 DXC 二进制导致 Unix 测试执行中断

- **Commit**：`262a41e` ("Skip e2e shader tests gracefully when DXC is not runnable")
- **症状**：端到端测试在 Linux 和 macOS runner 上执行时发生崩溃，在执行 DXC 时报错 `Exec format error`，导致完整性检查未通过。
- **根本原因**：测试框架下载的微软官方 DXC release 资产中仅包含 Windows PE 二进制文件。
- **修复方案**：在 `dxil-spirv-tests/build.rs` 中添加了 `is_dxc_runnable` 检测，以便条件性输出 `cargo:rustc-cfg=dxc_unavailable`。在 `dxil-spirv-tests/tests/e2e.rs` 中使用 `cfg!(dxc_unavailable)` 保护测试执行逻辑。

### 坑 7：平台规范化类型转换触发 Clippy Lint 报错

- **Commits**：`a923f9b` ("Allow same-type cast for cross-platform bindgen enum") 与 `5ecba0b` ("Allow platform-normalizing casts crate-wide (clippy::unnecessary_cast)")
- **症状**：Linux 和 macOS 上的 Clippy 矩阵 job 在 `-D warnings` 下报错失败：
  ```text
  error: unnecessary cast to the same type: `u32` as `u32`
     --> dxil-spirv/src/converter.rs:133:43
  ```
- **根本原因**：转换 `option.kind() as u32` 在 Windows 上是真实的类型转换（`i32` 到 `u32`），但在 bindgen 生成 `u32` 的 Linux 和 macOS 上属于多余的同类型转换。
- **修复方案**：在 `dxil-spirv/src/lib.rs` 中添加了 crate 级别的 `#![allow(clippy::unnecessary_cast)]`，并附带文档说明跨平台 FFI 类型的稳定性需要这些平台规范化转换。

## 添加 Job 或平台

要为 CI 引入新的目标平台：

1. **验证工具链可用性**：Runner 必须提供 C++17 编译器、CMake 3.20 或更高版本、Ninja 以及 `libclang`。
2. **扩展 Workflow 矩阵**：在 `.github/workflows/ci.yml` 中添加 runner 标识符：
   ```yaml
   strategy:
     fail-fast: false
     matrix:
       os: [windows-latest, ubuntu-latest, macos-latest, <new-runner-os>]
   ```
3. **配置系统依赖**：如有需要，为新的 runner OS 添加条件性软件包安装步骤。
4. **验证标准库链接**：确保 `dxil-spirv-sys/build.rs` 将目标操作系统映射到正确的 C++ 运行时库（Darwin/BSD 对应 `libc++`，GNU/Linux 对应 `libstdc++`）。

## 本地推送前验证关卡

要在发起 Pull Request 之前在本地验证更改，请运行与 CI 完全一致的命令序列：

```sh
# 步骤 1: 代码格式检查（对应 'fmt' job）
cargo fmt --all -- --check

# 步骤 2: Workspace lint 检查（对应 'clippy' job）
cargo clippy --workspace --all-targets -- -D warnings

# 步骤 3: 构建与测试执行（对应 'build-test' job）
cargo build --workspace --all-targets
cargo test --workspace
```

在本地运行这些命令可确保分支在所有平台上都能顺利通过 CI 矩阵检查。
