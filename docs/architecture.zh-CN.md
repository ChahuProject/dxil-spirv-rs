# 开发者架构

[English](architecture.md) | [中文](architecture.zh-CN.md)

本文档介绍 `dxil-spirv-rs` 的内部架构，包括 crate 拓扑、原生构建与静态链接流程、FFI 边界规则、生命周期管理模式、线程安全约定、测试框架基础设施以及记录的跨平台经验。

## 1. Crate 拓扑

工作区将关注点拆分到三个 crate 中，遵循由 `spirv-cross2-rs` 和 `spirv_cross` 等现代 Rust 原生绑定生态系统建立的模式：

```text
dxil-spirv (safe idiomatic Rust wrapper)
    │
    ▼ (depends on)
dxil-spirv-sys (build.rs + CMake compilation + raw bindgen FFI)
    │
    ▼ (compiles and links)
dxil-spirv (vendored C++ submodule at dxil-spirv-sys/dxil-spirv)

dxil-spirv-tests (out-of-tree end-to-end test suite + DXC compilation harness)
```

### `dxil-spirv-sys`

`dxil-spirv-sys` 是底层 FFI 基石：
- 子模块：在 `dxil-spirv-sys/dxil-spirv` 处引入上游 `HansKristian-Work/dxil-spirv`。
- 构建脚本：`build.rs` 编排 `dxil-spirv-c-static` 目标的 CMake 编译，发出所有组成静态库的链接搜索标志，链接平台 C++ 标准库，并对 `dxil_spirv_c.h` 执行 `bindgen`。
- 输出：将原始绑定生成到 `OUT_DIR/bindings.rs` 中，并将副本复制到 `dxil-spirv-sys/generated/bindings.rs` 以供离线检查。

### `dxil-spirv`

`dxil-spirv` 是安全、符合 Rust 习惯的公开接口：
- RAII 封装：将原始 C 句柄封装在 `ParsedBlob`（`dxil_spv_parsed_blob`）和 `Converter`（`dxil_spv_converter`）中，确保在 drop 时释放内存。
- 类型化转换：将 C 枚举和带标签的选项结构体映射为 Rust 枚举，例如 `ShaderStage` 和 `ConverterOption`。
- Trampoline 层：使用双重 Box 分配模式并配合 panic 捕获，将安全的 Rust `FnMut` 闭包桥接到 C remapper 回调。
- 错误处理：通过 `thiserror` 将 `dxil_spv_result` 状态码转换为类型化的 `Result<T, Error>`。

### `dxil-spirv-tests`

`dxil-spirv-tests` 是验证测试框架：
- 测试数据集：从上游子模块同步 829 个测试着色器。
- DXC 集成：在 Windows 上自动下载并运行 DirectX Shader Compiler（`dxc.exe`），将 HLSL 源码编译为 DXIL bitcode blob。
- 子进程隔离：在单独的子进程中执行每个着色器转换测试，避免上游 C++ 断言导致整个测试运行器终止。

## 2. 原生构建流程与静态链接闭包

上游 `dxil-spirv` 是用 C++17 编写的 CMake 项目。`dxil-spirv-sys/build.rs` 使用 `cmake` crate 编译静态 C API 目标。

### 与参考绑定 Crate 的对比

该绑定架构融合了两个成熟前辈的设计思路：
- `grovesNL/spirv_cross`：使用经典构建脚本和原始 FFI 层。与使用 `cc` crate 构建第三方源码的 `spirv_cross` 不同，`dxil-spirv-sys` 使用 `cmake` crate，因为上游依赖 CMake 目标图和子项目定义。
- `SnowflakePowered/spirv-cross2-rs`：展示了现代健全性模式、严格的 `-sys` 分离、Arc 保护的上下文生命周期以及固定上游版本的 semver 元数据。

### 静态链接闭包

`dxil-spirv-c-static` 目标依赖多个内部静态库。静态链接器按顺序解析符号，因此各个库必须严格按照依赖方在前、被依赖方在后的顺序声明。

精确的 9 个库链接闭包如下：

```text
1. dxil-spirv-c-static      (C API export surface)
2. dxil-converter           (Core DXIL to SPIR-V translation logic)
3. spirv-module             (SPIR-V binary instruction builder)
4. dxil-utils               (Shared utilities and container parsers)
5. dxil-debug               (Disassembly and debug printing)
6. dxbc-spirv               (Legacy DXBC SM4/SM5 translation fallback)
7. glslang-spirv-builder    (SPIR-V AST construction backend)
8. llvm-bc                  (LLVM bitcode container reader)
9. bc-decoder               (Low-level bitstream decoder)
```

如果链接器报告未解析的 `LLVMBC::*` 或 `spv::Builder::*` 符号，说明列表中缺少某个库或顺序有误。

此闭包中特意省略了两个上游目标：
- `dxil-spirv-headers`：仅头文件的 CMake 接口目标，不生成归档文件。
- `spirv-cross` / `spirv-tools`：可选的上游 CLI 工具依赖，不属于库链接闭包的一部分。

### 跨配置的库查找

CMake 会在不同的子目录中构建静态目标。在 Windows（MSVC）上，输出位于按配置划分的目录中，例如 `Release/` 或 `Debug/`。在 Unix 系统上，各个静态归档文件位于按目标划分的构建子树中。

`build.rs` 实现了 `register_lib_dirs()`，该函数递归遍历 CMake 构建输出目录。每当找到包含 `.lib`（Windows）或 `.a`（Linux/macOS）归档文件的目录时，就会发出 `cargo:rustc-link-search=native=<dir>` 指令。

### Bindgen 工作流与布局验证

`build.rs` 在编译时生成 Rust 绑定：
- 头文件：`dxil-spirv-sys/dxil-spirv/dxil_spirv_c.h`。
- 白名单：匹配 `dxil_spv_.*` 的函数、匹配 `dxil_spv_.*` 的类型以及匹配 `DXIL_SPV_.*` 的变量。
- 布局测试：保持启用 bindgen 结构体布局测试。与禁用布局测试的 `spirv_cross` 不同，`dxil-spirv-sys` 保留了这些测试，因为上游结构体集合很稳定。如果上游修改了字段对齐或填充，这些测试会在 `cargo test` 期间立即失败。

## 3. FFI 边界规则

将 Rust 与上游 C API 对接需要遵守特定的边界处理规则：

### 带类型转换的宏常量

带有类型转换定义的宏（例如 `#define DXIL_SPV_TRUE ((dxil_spv_bool)1)`）会被 bindgen 跳过。Rust 代码不得尝试引用 `sys::DXIL_SPV_TRUE` 或 `sys::DXIL_SPV_FALSE`。相反，应传递类型为 `sys::dxil_spv_bool`（`u8`）的整数字面量 `1` 或 `0`。

### 匿名 Union

Bindgen 将 C 匿名 union 建模为嵌套私有 union 类型，无法通过结构体字面量直接构造。在将安全结构体转换为原始 FFI 结构体时：
1. 通过 `Default::default()` 初始化结构体。
2. 显式为目标字段赋值。
3. 为 `From` 实现添加 `#[allow(clippy::field_reassign_with_default)]` 注解。

### 枚举符号性归一化

根据目标平台头文件，Clang 和 bindgen 会将 C 枚举生成为带符号整数（`c_int` / `i32`）或无符号整数（`c_uint` / `u32`）。例如，包含负值哨兵的枚举会变成带符号类型，而纯正数集合则变成无符号类型。

安全封装层将所有枚举判别值和选项标签归一化为 `u32`。像 `raw as u32` 或 `tag as sys::dxil_spv_option` 这样的显式类型转换在某些平台上是必需的，而在其他平台上则是空操作。为了避免在同构目标上产生编译器警告，`dxil-spirv/src/lib.rs` 在整个 crate 范围内允许 `#![allow(clippy::unnecessary_cast)]`。

## 4. FFI 回调 Trampoline 模式

上游 `dxil-spirv` 在 `Converter` 上提供了 8 个 remapper 回调（SRV、UAV、CBV、sampler、顶点输入、阶段输入、阶段输出、流输出），并在 `ParsedBlob::scan_resources` 上提供了 1 个回调。

`spirv_cross` 缺少回调 API，因此 `dxil-spirv-rs` 在 `dxil-spirv/src/remapper.rs` 中建立了一套自定义的双重 Box trampoline 模式。

```text
Rust Closure: Box<dyn FnMut(&D3d) -> Option<Vulkan> + Send>
                           │
                 Box::new (outer box)
                           │
                           ▼
                 *mut Box<dyn FnMut...>  <─── Thin Pointer (*mut c_void userdata)
                           │
                 C Library Converter
                           │
                 (invokes trampoline during dxil_spv_converter_run)
                           │
                           ▼
                  extern "C" trampoline
                           │
       ┌───────────────────┴───────────────────┐
       ▼                                       ▼
&mut **(userdata as *mut Box<...>)   catch_unwind block
(dereference twice to get &mut dyn)  (returns 0 / DXIL_SPV_FALSE on panic)
```

### 用于瘦指针的双重 Box

特征对象 `Box<dyn FnMut...>` 是由两个字组成的胖指针（数据指针和虚表指针）。C API 仅接受单个 `*mut c_void` userdata 指针。在 Rust 中直接将胖指针强制转换为 `*mut c_void` 是非法的。

解决方案是将胖指针包装在外部堆 Box 中：
1. 构造 `Box<Box<dyn FnMut...>>`。
2. 获取指向外部 Box 的瘦裸指针：`(&mut *holder.closure) as *mut Box<dyn FnMut...> as *mut c_void`。
3. 在 trampoline 中，将 `userdata` 转回 `*mut Box<dyn FnMut...>` 并解引用两次（`&mut **ptr`）以访问内部闭包。

### 生命周期与所有权模型

`Converter` 实例在内部 `Option<Box<RemapperHolder>>` 中持有 remapper 闭包的所有权：
- 转换运行期间，外部 Box 的内存在堆上保持固定在稳定地址。
- 在 `Converter::drop` 中，`self._remappers.take()` 首先释放 Rust 闭包，随后调用 `dxil_spv_converter_free`。这保证了 C 代码绝不会持有悬垂的 userdata 指针。
- 避免使用 `Box::into_raw`，因为同时持有活跃的裸指针和受管 `Box` 会带来重复释放的风险。

### Panic 边界安全性

Rust panic 绝不能跨越 `extern "C"` ABI 边界展开。否则会导致未定义行为或直接中止进程。

每个回调 trampoline 都将闭包调用封装在 `std::panic::catch_unwind(std::panic::AssertUnwindSafe(...))` 中。如果发生 panic：
- Panic 会在 trampoline 内部被捕获。
- Trampoline 返回 `0`（`DXIL_SPV_FALSE`），向上游 C++ 核心报告失败。

### 存活伴随对象

传递给 `Converter::add_option` 的选项可能包含引用堆缓冲区的裸指针（例如 `Vec<u32>` 输出 swizzle 或 `CString` 文件路径）。`dxil-spirv/src/options.rs` 中的 `RawOptionData` 枚举在 FFI 调用期间将这些底层内存分配与 C 结构体一同保存。由于保留它们纯粹是为了维持生命周期，而不会被 Rust 读回，因此该枚举添加了 `#[allow(dead_code)]` 注解。

## 5. 线程安全约定

`dxil-spirv-rs` 强制执行显式并发不变性：

```rust
// In dxil-spirv/src/converter.rs:
unsafe impl Send for Converter {}
// Sync is deliberately omitted.

// In dxil-spirv/src/parsed_blob.rs:
unsafe impl Send for ParsedBlob {}
// Sync is deliberately omitted.
```

### 为什么实现 `Send`

上游转换过程是完全同步且自包含的。Remapper 回调仅在调用该函数的线程上执行 `dxil_spv_converter_run` 期间触发。转换过程中没有后台工作线程，没有跨运行保留的隐式线程局部状态，也没有共享全局状态。在线程之间移动 `Converter` 或 `ParsedBlob` 是安全的。

### 为什么不实现 `Sync`

从多个线程对同一个句柄并发调用 `dxil_spv_converter_run` 或修改选项设置器会导致 C++ 对象中的数据竞争。因此，`Converter` 和 `ParsedBlob` 均未实现 `Sync`。跨线程共享访问需要通过 `Mutex` 或 `RwLock` 进行外部同步。

Remapper 闭包仅需满足 `Send + 'static`，不需要 `Sync`。

## 6. C 运行时（CRT）与异常处理

Windows MSVC 构建要求 Rust 和 C++ 运行时配置保持严格一致。

### MSVC CRT 匹配

`dxil-spirv-sys/build.rs` 将 CMake 配置为：

```rust
cfg.define("CMAKE_MSVC_RUNTIME_LIBRARY", "MultiThreaded$<$<CONFIG:Debug>:Debug>DLL");
```

这指示 MSVC 链接动态 C 运行时：
- Debug Rust 构建（`PROFILE=debug`）通过 `Debug` CMake 配置链接 `MSVCRTD.lib`。
- Release Rust 构建通过 `Release` CMake 配置链接 `MSVCRT.lib`。

配置不匹配会导致在最终链接时找不到如 `_CrtDbgReport` 或 `_calloc_dbg` 等函数的未解析符号错误。

### 异常标志与 RTTI

上游 GCC 和 Clang 构建标志指定了 `-fno-exceptions -fno-rtti`，但库代码中没有使用 `try`/`catch`/`throw` 语句，并用自定义的 LLVM 风格 `isa<>` 和 `dyn_cast<>` 模板替换了原生 RTTI。
- 在 MSVC 上，`build.rs` 传递 `/EHsc` 以启用结构化 C++ 异常处理并消除 STL 警告 `C4530`。
- 在 GCC 和 Clang（Linux/macOS）上，对 `.cpp` 文件默认启用异常处理。向 GCC 或 Clang 传递 `/EHsc` 会导致 CMake 中的编译器配置检查失败并报错 `no such file or directory: '/EHsc'`。该标志严格限制在 `target_env == "msvc"` 时生效。

## 7. C++ 标准库链接

Rust 编译器会自动链接 C 运行时库，但在链接静态 C++ 归档时不会自动链接 C++ 标准库。

上游 `dxil-spirv` 依赖 `operator new`、`operator delete`、标准容器（`std::vector`、`std::string`、`std::unordered_map`）以及 RTTI 结构。

为了在非 MSVC 工具链上解析这些符号，`dxil-spirv-sys/build.rs` 发出显式动态链接指令：
- macOS：`println!("cargo:rustc-link-lib=dylib=c++");`（libc++）
- Linux、BSD 与 Unix 目标：`println!("cargo:rustc-link-lib=dylib=stdc++");`（libstdc++）
- Windows MSVC：由默认 CRT 配置自动处理。

有关完整的平台矩阵详情，请参阅 [platform-support.md](platform-support.zh-CN.md)。

## 8. 跨平台踩坑实录

下表记录了具体的集成问题、根本原因以及解决它们的提交：

| 领域 | 现象 | 根本原因 | 修复方案 | 提交 |
|---|---|---|---|---|
| Library Discovery | 链接失败：在 Linux/macOS 上找不到 `-ldxbc-spirv` | `register_lib_dirs` 仅检查 `.lib` 文件，忽略了 Unix 上的 `.a` 归档 | 在递归遍历中同时搜索 `.lib` 和 `.a` 扩展名 | `5163058` |
| Bindgen Types | 编译器错误 `E0308`：Linux 上枚举匹配类型不一致 | 对于没有负值的枚举，Clang 在 Linux 上生成 `c_uint`，而在 Windows 上生成 `c_int` | 将枚举归一化为 `u32` 并在 FFI 边界显式转换 | `ce156ac` |
| Clippy Lint | Windows 目标上的 lint 警告 `clippy::unnecessary_cast` | 归一化转换在 Linux 上是必需的，但在 Windows 上是多余的 | 在安全层整个 crate 范围内应用 `#![allow(clippy::unnecessary_cast)]` | `5ecba0b` |
| CMake Compiler Check | CMake 配置失败：`no such file or directory: '/EHsc'` | `/EHsc` 是 MSVC 特有标志，会破坏 GCC 和 Clang 命令行解析 | 将 `/EHsc` 限制在目标检查 `CARGO_CFG_TARGET_ENV == "msvc"` 之后 | `ed19ee0` |
| Linker Symbols | Linux 和 macOS 上未解析的 `std::__cxx11` / `operator new` | rustc 链接 C 运行时，但不会自动拉取 C++ 标准库 | 在 Linux 上发出 `cargo:rustc-link-lib=dylib=stdc++`，在 macOS 上发出 `c++` | `dde03e4` |
| Test Harness | 因缺少 DXC 二进制文件导致 Linux/macOS 上的端到端测试失败 | 微软 DXC 发布归档仅提供 Windows `dxc.exe` x64 二进制文件 | 在非 Windows 平台或未设置 `DXC_PATH` 时平滑跳过 DXC 编译 | `262a41e` |
| CI Cache | CI 中子模块更新后仍链接了陈旧的 C++ 目标文件 | 缓存完整的 `target/` 目录保留了过期的原生归档 | 从 CI 工作流中剔除脆弱的 `target/` 缓存，仅依赖 Cargo 依赖缓存 | `6d7757f` |
| Directory Recursion | 链接失败：缺少来自嵌套子项目（`llvm-bc`）的符号 | CMake 将静态目标嵌套在不同的子目录中 | 在 `register_lib_dirs` 中递归遍历所有子文件夹 | `5163058` |

## 9. 实验性与条件编译 API 接口

上游通过 `dxil_spirv_c.h` 中的预处理器定义控制某些 C API 声明：

| 宏 | 暴露的函数 | 上游默认状态 | Rust 封装层处理 |
|---|---|---|---|
| `DXIL_SPV_ENABLE_EXPERIMENTAL_WORKGRAPHS` | `dxil_spv_parsed_blob_get_entry_point_node_input`, `dxil_spv_parsed_blob_get_entry_point_num_node_outputs`, `dxil_spv_parsed_blob_get_entry_point_node_output` | 在 `dxil_spirv_c.cpp` 中启用 | 向 bindgen 传递 `-DDXIL_SPV_ENABLE_EXPERIMENTAL_WORKGRAPHS`；安全层始终封装 |
| `DXIL_SPV_ENABLE_EXPERIMENTAL_MULTIVIEW` | `dxil_spv_converter_is_multiview_compatible` | 在 `dxil_spirv_c.cpp` 中启用 | 向 bindgen 传递 `-DDXIL_SPV_ENABLE_EXPERIMENTAL_MULTIVIEW`；安全层始终封装 |

因为 `dxil_spirv_c.cpp` 硬编码了这些宏定义，编译后的静态库始终包含这些符号。在 `dxil-spirv-sys/build.rs` 中传递匹配的 `-D` 参数可确保 bindgen 生成对应的函数签名。

### 开关维护核对清单

如果上游修改了现有的特性标志或引入了新的条件开关：
1. 更新 `dxil-spirv-sys/build.rs` 中 `generate_bindings()` 内部的 bindgen `-D` 定义。
2. 更新 `dxil-spirv-sys/build.rs` 中 `build_with_cmake()` 内部的 CMake 编译选项。
3. 在 `dxil-spirv/src/` 中添加或调整对应的 `#[cfg(feature = "...")]` 条件门控。
4. 更新 `dxil-spirv/Cargo.toml` 中的特性声明。
5. 更新 `docs/usage.md` 和 `docs/architecture.md` 中的文档。
6. 更新 `dxil-spirv/tests/api_coverage.rs` 以验证新函数已被追踪。

## 10. 上游版本控制与同步

`dxil-spirv-rs` 跟踪上游 `HansKristian-Work/dxil-spirv`，上游在滚动 master 分支上运作，不发布打标签的 release。

### Semver 构建元数据规则

工作区 `Cargo.toml`（`[workspace.package]`）中的 crate 版本使用 semver 构建元数据记录锁定的上游 C API 版本：

```text
<crate-version>+dxil-spirv.<UPSTREAM_MAJOR.MINOR.PATCH>
Example: 0.1.0+dxil-spirv.2.72.1
```

- `+dxil-spirv.X.Y.Z` 后缀与 `dxil_spirv_c.h` 中的 `DXIL_SPV_API_VERSION_MAJOR`、`DXIL_SPV_API_VERSION_MINOR` 和 `DXIL_SPV_API_VERSION_PATCH` 定义保持一致。
- Crates.io 会解析构建元数据，但不会将其用于版本优先级判定，从而保持标准的依赖解析行为。
- 基础版本（`0.1.0`）遵循标准 semver：安全 Rust 封装层的破坏性变更触发次版本或主版本升级，而内部更新或向后兼容的增补则触发修订版本升级。

## 11. 端到端测试基础设施架构

`dxil-spirv-tests` crate 提供了针对完整上游着色器测试套件的自动化验证。

### 测试数据流

```text
dxil-spirv-sys/dxil-spirv/shaders/   ──sync──▶  tests/shaders/   (git-ignored)
dxil-spirv-sys/dxil-spirv/reference/ ──sync──▶  tests/reference/ (git-ignored)
                                               │
                                               ▼ DXC 1.9.2602.17
                                           tests/shaders/*.dxil
```

### 核心架构组件

测试基础设施由三个主要模块组成：
- `dxil-spirv-tests/build.rs`：同步着色器目录，下载微软 DXC 发布资产，并将 HLSL 测试着色器编译为二进制 DXIL 容器。
- `dxil-spirv-tests/tests/harness.rs`：驱动着色器转换，配置 remapper 状态，并归一化输出。
- `dxil-spirv-tests/tests/e2e.rs`：实现测试套件，涵盖套件完整性、分类冒烟测试、回归基线和转换指标。

### 子进程隔离模式

上游 C++ 转换代码包含严格的调试断言（例如 `SpvBuilder.cpp:754`）。当不受支持或格式错误的指令序列触发 C++ 断言时，abort 会立即终止宿主进程。

`dxil-spirv-tests` 使用 `std::process::Command` 在独立的子进程中执行每个着色器转换。如果触发断言：
- 子进程退出并返回错误状态码。
- 父测试框架捕获失败，不会终止其余测试套件。

### DXC 版本锁定

测试框架在 `dxil-spirv-tests/build.rs` 中将 `DXC_VERSION` 锁定为 `1.9.2602.17`。该版本提供了针对 Shader Model 6.9（SM6.9）的首个稳定生产编译器。降级 DXC 会导致 SM6.9 着色器在编译时失败。

### 已知失败追踪与完整性门禁

上游测试套件中的某些着色器需要针对单个着色器的自定义 remapper 回调。测试框架通过 `harness.rs` 中的 `requires_complex_remapper()` 对这些情况进行分类，并将其标记为 `KnownFailure`。

该设计实现了两个目标：
1. `test_completeness_check` 强制跟踪并核对所有 829 个上游着色器。
2. 持续测量确切的已知失败率（约 33.7%，即 279/829 个着色器），而不会破坏自动化构建门禁。
