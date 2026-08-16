# 变更日志

[English](changelog.md) | [中文](changelog.zh-CN.md)

当前状态：版本 0.1.0+dxil-spirv.2.72.1 | Rust Edition 2024 | 829/829 个着色器测试通过 (100.0%) | Windows、Linux、macOS 平台 CI 全部通过

本文档记录了 dxil-spirv-rs 按时间顺序的开发历程，直接根据 git 历史和提交说明重构而成。

## 里程碑

### 1. 初始绑定与构建基础设施
- 日期：2026-08-15
- 关键提交：`b794e5f`, `670be82`, `8b36488`, `c465d4a`, `72b8ed5`, `5f991f6`, `01ef4c4`, `b02dd90`, `76d0456`, `218e11a`, `e6c51ce`

项目初期确立了包含两个 crate 的 workspace 拓扑结构。`dxil-spirv-sys` 负责原生 CMake 编译与底层的 bindgen FFI 绑定，而 `dxil-spirv` 则提供安全、符合 Rust 习惯的 API。上游来自 `HansKristian-Work/dxil-spirv` 的 C++ 代码库被固定为 git submodule。

早期实现工作确立了核心 RAII 类型 `ParsedBlob` 和 `Converter`，以及单次转换辅助函数 `convert_to_spirv()`。包装层随后迅速扩充，覆盖了 `ConverterOption` 中的全部 50 余个选项、计算着色器工作组维度、入口点选择器、wave 尺寸启发式规则以及 LLVM IR 反汇编访问器。

将 Rust 闭包对接到 C 回调需要解决 Rust 中的胖指针限制。诸如 `Box<dyn FnMut...>` 等 trait 对象无法直接转换为瘦指针 `*mut c_void` userdata。双重装箱模式（`Box<Box<dyn FnMut...>>`）通过为外部指针创建稳定的堆内存地址解决了这一问题。蹦床函数（trampoline）使用 `std::panic::catch_unwind` 包装闭包调用，防止 panic 跨越 `extern "C"` 边界发生展开。该模式已应用于全部 8 种回调重映射器：
- SRV 重映射器（Shader Resource Views）
- UAV 重映射器（Unordered Access Views）
- CBV 重映射器（Constant Buffer Views）
- Sampler 重映射器
- Vertex Input 重映射器
- Stage Input 重映射器
- Stage Output 重映射器
- Stream Output 重映射器

静态链接需要按严格的依赖顺序确定完整的 9 个库闭包：
1. `dxil-spirv-c-static`（C API 导出接口）
2. `dxil-converter`（核心 DXIL 到 SPIR-V 转换逻辑）
3. `spirv-module`（SPIR-V 二进制指令构建器）
4. `dxil-utils`（通用工具与容器解析器）
5. `dxil-debug`（反汇编与调试打印）
6. `dxbc-spirv`（旧版 DXBC SM4/SM5 转换回退）
7. `glslang-spirv-builder`（SPIR-V AST 构建后端）
8. `llvm-bc`（LLVM bitcode 容器读取器）
9. `bc-decoder`（底层位流解码器）

在 Windows 上，`CMAKE_MSVC_RUNTIME_LIBRARY` 经过动态配置以匹配 Rust 的 debug 和 release CRT 配置（`MultiThreaded$<$<CONFIG:Debug>:Debug>DLL`）。这解决了 `_CrtDbgReport` 与 `_calloc_dbg` 的未解析链接器符号问题。项目采用了 Semver 构建元数据（`0.1.0+dxil-spirv.2.72.1`）来直接追踪上游 C API 版本字符串。

项目研究了 `grovesNL/spirv_cross` 和 `SnowflakePowered/spirv-cross2-rs` 的参考架构，确立了合理的绑定规范。参考仓库在 `.agents/skills/sync-upstream/` 下配置为按需克隆，下游 crate 使用者无需拉取不必要的源码树。

### 2. 补齐 API 覆盖
- 日期：2026-08-15
- 关键提交：`b7fc828`, `3310b19`

依赖项升级到了 bindgen 0.72 与 thiserror 2.0。项目中引入了专门的编译期测试（`tests/api_coverage.rs`），用于追踪安全层是否遗漏了任何上游 C 函数。

安全包装层新增了 23 个函数绑定，完整暴露了上游全部 C 接口：
- 根签名参数映射与描述符表（`add_root_descriptor_mapping`、`set_root_constant_word_count`、`add_local_root_constants`、`begin_local_root_descriptor_table`、`add_local_root_descriptor_table`、`end_local_root_descriptor_table`）
- Work Graphs 入口点（针对 SM6.8 网格节点的 `node_input`、`num_node_outputs`、`node_output`）
- 面向 DirectX Raytracing 状态对象的 RDAT 子对象解析（`get_num_rdat_subobjects`、`get_rdat_subobject`）
- 转换前自省所需的资源扫描（`scan_resources`）
- 线程分配器内存 arena 管理（`ThreadAllocatorContext`）
- 线程局部日志回调注册（`set_thread_log_callback`）
- 无需 DXBC 容器开销的直接 DXIL bitcode 解析（`parse_dxil`）

引入了新的类型化数据结构，用于安全表示原始 C 结构体：`ResourceClass`、`MetaDescriptor`、`MetaDescriptorKind`、`RdatSubobject`、`RdatSubobjectKind`、`NodeInputData`、`NodeOutputData` 以及 `LogLevel`。

在 `build.rs` 的 bindgen 编译参数中添加了实验性特性标志（`-DDXIL_SPV_ENABLE_EXPERIMENTAL_WORKGRAPHS` 和 `-DDXIL_SPV_ENABLE_EXPERIMENTAL_MULTIVIEW`）。由于上游 C++ 实现始终编译这些路径，Rust 包装层现已暴露 C API 中的全部 64 个函数（`KNOWN_MISSING` 降至 0）。

### 3. 端到端测试基础设施
- 日期：2026-08-16
- 关键提交：`41260c8`, `3558baf`, `5e18ce6`, `38f713a`

新增了第三个 crate `dxil-spirv-tests`，用于针对真实着色器验证转换功能。构建脚本在每次构建时都会从 `dxil-spirv-sys/dxil-spirv/shaders/` 和 `reference/` 同步 1,550 个着色器源文件和 839 个参考文件。

测试框架实现了自动下载 Microsoft DXC 1.9.2602.17，将 HLSL 源文件编译为 DXIL 字节码。选用此版本 DXC 是因为它提供了对 Shader Model 6.9 的首个生产级支持。

由于上游 C++ 断言在遇到无效输入时会调用 `abort()`，在同进程内运行转换会导致整个测试运行器异常退出。测试框架通过 `std::process::Command` 派生的专属子进程隔离每个着色器转换。子进程通过 `DXIL_SPIRV_TEST_CHILD_SHADER` 接收目标着色器，并通过结构化的标准输出消息（`__DXIL_SPIRV_RESULT__|status|spirv_len|error`）返回结果。

集成了基于 `spirv-cross2` 0.7.1 的 GLSL 往返校验，确保生成的 SPIR-V 可以干净地反编译为 GLSL 460。可选择启用的严格比对模式（`DXIL_SPIRV_STRICT_GLSL=1`）会针对上游参考文件校验 MD5 哈希。创建了 `test_metrics_report` 套件，设置了强硬断言，要求零未预期失败和零跳过着色器。初始测试运行通过了 47/48 个阶段和 67/159 种资源。

### 4. 回归基线与已知失败分类
- 日期：2026-08-16
- 关键提交：`e5187dd`, `488afa3`, `568dda3`, `2aa1e90`, `ca2598e`

测试套件覆盖面扩展到上游全部 24 种着色器类别，消除了测试盲区：`ags`、`alloca-opts`、`auto-barrier`、`control-flow`、`descriptor_qa`、`dxil-builtin`、`fp16`、`heap-robustness`、`instrumentation`、`llvm-builtin`、`memory-model`、`nvapi`、`opts`、`raw-access`、`resources`、`rov`、`sampler-feedback`、`semantics`、`stages`、`vectorization`、`view-instancing`、`vkmm`、`asm` 以及根着色器。`asm/*.bc.dxil` 中的原始 LLVM bitcode 文件已接入 `dxil_spirv::parse_dxil`。

更新了 `requires_complex_remapper` 中的已知失败分类逻辑，使其仅在转换尝试失败后执行。成功转换的着色器不再被隐匿为已知失败。这一调整立即使通过率从 66.3% 提升至 76.2%（632 个通过，197 个已知失败）。

生成了回归基线文件（`tests/regression_baseline.json`），用于捕获代码变更之间的任何由通过变为失败的退步。更新流程通过环境变量 `DXIL_SPIRV_UPDATE_BASELINE=1` 进行。子进程增加了 30 秒看门狗定时器以终止死循环。`test_completeness_check` 增加了针对着色器目录为空的保护措施，防止在 submodule 缺失时产生误报通过。

通过将 `zip` 升级到支持 deflate 的 8.6.0 版本加固了压缩包解压逻辑。解压逻辑还严格限定在 `bin/x64/` 范围内，避免误用 32 位 DXC 二进制文件。

### 5. 达成 100% 着色器通过率
- 日期：2026-08-16
- 关键提交：`4e77570`, `47daf64`, `5b89e37`

解决剩余的测试失败需要在三个渐进阶段中与上游 CLI 选项和重映射配置完全对齐。

阶段 1 在 `configure_converter()` 中补齐了 9 个缺失的选项映射：
- 针对 `.descriptor-qa.` 的 `DescriptorQa`（版本 2，描述符集 10/11，哈希 `0xdeadbeef`）
- 针对 `.bda-instrumentation.` 的 `InstructionInstrumentation`（缓冲区同步校验）
- 针对 `.vkmm.` 的 `VulkanMemoryModel`
- 针对 `.nvapi.` 的 `Nvapi`（寄存器 127，空间 0）
- 针对 `.full-wmma.` 的 `Float8Support`（FP8 算术与协同矩阵）
- 针对 `.auto-group-shared-barrier.` 的 `ShaderQuirk::GroupSharedAutoBarrier`
- 针对 `.mixed-float-dot-product.` 的 `MixedFloatDotProduct`
- 针对 `.rt-swizzle.` 的 `OutputSwizzle`
- 针对 `.raw-access-chains.` 的 `RawAccessChainsNv`

这些选项映射使已知失败数从 197 减少到 194。

阶段 2 解决了底层绑定默认值问题：
- SSBO 对齐：上游库默认对齐为 16 字节，但 CLI 工具默认对齐为 1 字节。添加 `SsboAlignment { alignment: 1 }` 作为基础选项，解决了所有非 bindless SSBO 着色器的偏移错误。
- Bindless push constants：为 `.bindless.` 分配至少 8 个 word，为描述符 QA 额外分配 4 个 word，同时配置 64 个根参数映射。
- 根描述符：为 `.root-descriptor.` 着色器启用了 `set_root_descriptor_count(4)` 和 Buffer Device Address (BDA)。

这些变更将已知失败数从 194 降低至 9，通过率达到 98.9%。

阶段 3 通过三项针对性修复解决了最后 9 个失败：
1. BDA 插桩：根描述符 BDA 此前将 RTAS 堆强制设置为 BufferDeviceAddress，但插桩需要 SSBO 自省缓冲区。添加了跳过 RTAS 堆 BDA 覆盖的标志（6 个着色器）。
2. 局部根签名：用上游等价调用 `add_local_root_constants(15, 0, 5)`、`add_local_root_constants(15, 1, 6)` 和 `add_local_root_descriptor()` 替换手动描述符表调用，并配合 `PhysicalStorageBuffer`（2 个着色器）。
3. 堆健壮性：修正了 meta 描述符类型，将 `ResourceDescriptorHeapSize` 作为描述符集 10 绑定 20 的 UBO 常量，并将 `RawDescriptorHeapView` 作为描述符集 10 绑定 21 的 UBO BDA（1 个着色器）。

经过这些修复，上游测试套件中的全部 829 个着色器完全通过（100.0% 通过率，0 个已知失败，0 个回归）。

### 6. Edition 2024 迁移与代码格式化
- 日期：2026-08-16
- 关键提交：`ed19ee0`, `55fa6fa`

整个 workspace 迁移到了 Rust edition 2024，在 `Cargo.toml` 中设置了 `rust-version = "1.85"`。全部 Rust 源文件均已重新格式化，以符合 edition 2024 风格规范和模块导入顺序。

添加了根目录 `rustfmt.toml` 文件以强制执行 workspace 级别的格式化标准。在 `dxil-spirv-sys/generated/` 中放置了一个空的 `.rustfmt.toml` 文件，防止 rustfmt 修改生成的 `bindings.rs` 文件（遵循 rust-lang/rustfmt#4264）。

CI 工作流被重构为三个具有独立缓存范围的专用 job：
- `fmt`：快速格式化检查门禁（约 30 秒），无需检出 submodule。
- `clippy`：check 级别的代码分析与 lint 验证。
- `build-test`：完整的 C++ 编译与测试执行。

配置了构建产物共享，使 `cargo build` 与 `cargo test` 共享已编译的依赖项，无需多次重复构建原生 C++ 库。

### 7. 跨平台 CI 修复与链接修正
- 日期：2026-08-16
- 关键提交：`ed19ee0`, `6d7757f`, `5163058`, `ce156ac`, `dde03e4`, `262a41e`, `a923f9b`, `5ecba0b`

在 Windows、Linux 和 macOS 运行器上排查并解决了多个平台特定的链接与编译问题。

将 MSVC 特有的 `/EHsc` 编译器标志限制为仅在 `target_env == "msvc"` 时使用。GCC 和 Clang 在 CMake 编译器特性检查期间会拒绝该标志，报错如 `no such file or directory: '/EHsc'`。

优化了 CI 缓存配置，移除了 `target/` 缓存目录。恢复过期的 target 目录会导致 CMake 静态库在跨次运行中移动或失效时出现链接失败。

更新了 `build.rs` 中的 `register_lib_dirs()`，使其同时搜索 `.a` 归档文件与 `.lib` 文件。没有此更改，Unix 链接器无法找到嵌套在 CMake 子目录中的静态库（`libdxbc-spirv.a`、`libglslang-spirv-builder.a`、`libllvm-bc.a`、`libbc-decoder.a`）。

Bindgen 在 Windows 上将 `dxil_spv_option` 生成为 `c_int`（有符号），而在 Linux 和 macOS 上生成为 `c_uint`（无符号），因为 C 头文件不包含负值枚举项。安全包装层将所有枚举字段统一标准化为 `u32`。添加了 crate 级别的 `#![allow(clippy::unnecessary_cast)]` 属性，使跨平台类型转换在 `-D warnings` 下能够干净编译。

为 C++ 标准库添加了显式的动态链接指令（macOS 上为 `c++`，Linux 上为 `stdc++`）。在无法运行 DXC 二进制文件的非 Windows 平台上，`build.rs` 输出 `cargo:rustc-cfg=dxc_unavailable`，允许测试套件平稳跳过着色器执行，同时仍然验证库本身的构建。

### 8. 文档重构
- 日期：2026-08-16
- 关键提交：`4e77570`, `38f713a`, `55fa6fa`, `ed19ee0`

项目文档在 `docs/` 下被重新组织为多个专用文件，为使用者和贡献者清晰分离关注点。

`docs/README.zh-CN.md` 作为中心入口并定义了维护政策。`docs/usage.zh-CN.md` 为 crate 使用者提供端到端指南，详述了转换、重映射器回调、根布局、日志记录和内存 arena。

`docs/architecture.zh-CN.md` 涵盖了面向开发者的内部设计主题，包括 crate 拓扑、9 库静态链接闭包、FFI 安全边界以及跨平台踩坑实录。`docs/testing.zh-CN.md` 记录了测试框架架构、着色器标记和基线机制。

`docs/platform-support.zh-CN.md` 详细列出了支持的操作系统与架构目标。新增了此变更日志，用于追踪项目在各个主要里程碑中的演进。

### 9. 非上游 hlsl-compat 扩展
- 日期:2026-08-16

新增首个**非上游扩展**:cargo feature `non-upstream-hlsl-compat`(默认关闭)启用`dxil_spirv::non_upstream::hlsl_compat::vec4_align_cbuffers` —— 一个纯 SPIR-V 后处理 pass,修复上游输出中 spirv-cross2 的 HLSL 后端无法表达的情况。

**为什么做**:上游 dxbc-spirv 对经由局部数组拷贝(标量粒度动态索引)访问的 cbuffer,输出 stride-4 标量数组(`struct { float[N] ArrayStride 4 }`)。这是合法 std140,但 spirv-cross2 的 HLSL 后端把 cbuffer 建模为 vec4 寄存器并拒绝。实测 Unity URP 着色器:HLSL 反编译失败率 74%(D3D12)/ 92%(D3D11),全部来自这一个错误类别;同一批着色器的 GLSL/MSL 全部成功。

**做了什么**:pass 把 stride-4 cbuffer 视图重写为 `float4[N/4]`(stride 16),并重写所有访问链(`[member, i]` -> `[member, i/4, i%4]`;动态索引变为 `OpUDiv`/`OpUMod`)。当同一 cbuffer 也存在 vec4 视图(同 binding)时,标量视图被合并进 vec4 视图并删除重复变量。变换保持布局与语义不变。

**隔离**:独立模块(`non_upstream`)、独立错误类型、独立测试;feature 关闭时模块不存在。vendored 上游 C++ 零改动。

**验证**:全量 810 个着色器扫描 —— GLSL 零回归,修复 10 个 HLSL 失败(全部为 cbuffer 布局类)。新增单元测试(4)与 e2e 测试(3),均在 feature 门控下运行。完整分析、复现步骤与验证见 docs/non-upstream/hlsl-compat-rationale.md。

## 如何更新

任何引入用户可见的 API 变更、修改内部构建脚本或改动着色器测试覆盖范围的拉取请求（Pull Request），都必须在此变更日志中添加条目。请将新条目放在最新里程碑下方，或者在变更代表一个独立开发阶段时创建新的小节。引用相关的 commit hash，描述改动了什么，解释变更的必要性，并在测试结果受影响时包含更新后的验证指标。
