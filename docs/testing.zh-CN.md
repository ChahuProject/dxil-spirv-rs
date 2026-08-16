# 测试架构

[English](testing.md) | [中文](testing.zh-CN.md)

本文档描述 `dxil-spirv-rs` 的测试架构，包括覆盖率保证、测试 harness 设计、shader 配置 marker 以及 regression baseline 机制。

## 覆盖率保证

安全 Rust wrapper 提供了与上游 C++ 实现完全一致且经过验证的功能对齐。

```
Upstream Test Suite Coverage: 829 / 829 shaders (100.0% passing)
Known Failures: 0 (0.0%)
Skipped Shaders: 0 (0.0%)
```

上游 `dxil-spirv` C++ shader 测试套件实现了 100% 覆盖且 100% 通过。此通过率由三个相互锁定的验证机制共同保证：

1. **完整性检查（`dxil-spirv-tests/tests/e2e.rs` 中的 `test_completeness_check`）**：发现 `dxil-spirv-sys/dxil-spirv/shaders/` 中的所有 shader 源文件，并将其与 `tests/shaders/` 中的测试集进行比对。任何缺失或多出的 shader 都会导致测试硬失败（hard failure）。该检查还会验证两个集合均不为空，防止在 git submodule 未初始化时产生假阳性（false-positive）通过。
2. **构建期 Shader 同步（`dxil-spirv-tests/build.rs`）**：在每次构建时直接从 vendored submodule 同步 HLSL 源码、C 头文件 include 以及参考 GLSL 文件。它调用 DirectX Shader Compiler (DXC) 将所有 shader 编译为 DXIL 字节码。
3. **Regression Baseline 防护（`tests/regression_baseline.json`）**：记录每个 shader 的 pass/fail 状态。任何 shader 从 `pass` 变为任何其他状态的 regression 都会在 `test_metrics_report` 中立即触发测试 panic。

## 测试执行模型

测试 harness 通过多阶段验证流水线运行每个 shader：

```
+-------------------+      +--------------------+      +-------------------+
|    HLSL Source    | ---> |    DXC Compiler    | ---> |   DXIL Bitcode    |
|   (.vert/.frag)   |      |  (v1.9.2602.17)    |      |      (.dxil)      |
+-------------------+      +--------------------+      +---------+---------+
                                                                 |
                               +---------------------------------+
                               v
                    +--------------------+
                    |   dxil-spirv-rs    |
                    |   (Safe Wrapper)   |
                    +----------+---------+
                               |
                               v
                    +--------------------+
                    |    SPIR-V Words    |
                    |  (Magic 0x07230203)|
                    +----------+---------+
                               |
                               v
                    +--------------------+
                    |    spirv-cross2    |
                    |    (GLSL Output)   |
                    +----------+---------+
                               |
                               v
                    +--------------------+
                    |     Validation     |
                    | (MD5 vs Reference) |
                    +--------------------+
```

### Subprocess Isolation

每个 shader 转换都在 `harness.rs` 中的 `test_shader()` 启动的独立子进程中执行。父进程通过 `DXIL_SPIRV_TEST_CHILD_SHADER` 环境变量传递 shader 路径，并以 `--exact __child_noop__` 参数调用测试二进制文件。

子进程使用结构化的 stdout 协议（`__DXIL_SPIRV_RESULT__|status|spirv_len|error`）将结果传回。这种隔离机制可防止上游 C++ 断言失败（如 glslang 的 `SpvBuilder.cpp:754` 或 `ir.hpp:113`）导致整个测试套件 runner 终止。每个子进程都在 30 秒看门狗定时器下运行，以捕获转换器中的任何死循环。

### 验证层级

1. **DXIL 解析**：通过 `dxil_spirv::ParsedBlob::parse()` 解析编译后的 DXIL 容器。对于原始 LLVM bitcode 文件（`asm/*.bc.dxil`），使用 `dxil_spirv::parse_dxil()`。
2. **转换器配置**：通过 `configure_converter()` 将文件名 marker 映射为类型化选项。
3. **SPIR-V 头部检查**：验证生成的 SPIR-V 流以有效的 SPIR-V magic word `0x07230203` 开头且包含非空字节码。
4. **GLSL 反编译**：使用 `spirv-cross2` 将 SPIR-V 反编译回具有 Vulkan 语义的 GLSL 460。这确保了输出在结构上健全，并能被下游工具消费。
5. **参考文件比对**：当设置 `DXIL_SPIRV_STRICT_GLSL=1` 时，计算规范化后的 GLSL 输出的 MD5 哈希值，并验证其与 `tests/reference/shaders/` 中的参考文件完全一致。

## 测试类别

829 个测试 shader 被组织到 22 个主要类别中。每个类别对应 `dxil-spirv-tests/tests/e2e.rs` 中的一个端到端测试函数。

| 类别 | Shader 数量 | 测试函数 | 用途 |
|---|---|---|---|
| `ags` | 28 | `test_ags` | AMD AGS 库函数与 SM6.6 WMMA 矩阵操作 |
| `alloca-opts` | 16 | `test_alloca_opts` | 动态 alloca 分配与栈内存优化 |
| `auto-barrier` | 6 | `test_auto_barrier` | group shared 内存的自动屏障插入 |
| `control-flow` | 26 | `test_control_flow` | 循环、switch 表以及复杂分支控制流 |
| `descriptor_qa` | 13 | `test_descriptor_qa` | Descriptor QA 验证与光线追踪加速结构 |
| `dxil-builtin` | 275 | `test_dxil_builtin` | 固有 DXIL 操作、wave vote、数学 builtin |
| `fp16` | 3 | `test_fp16` | 原生 16 位半精度浮点操作 |
| `heap-robustness` | 5 | `test_heap_robustness` | Descriptor heap 越界检查与越界安全 |
| `instrumentation` | 14 | `test_instrumentation` | Buffer Device Address (BDA) 指令插桩 |
| `llvm-builtin` | 44 | `test_llvm_builtin` | 位操作 intrinsic 与 LLVM lowering 操作 |
| `memory-model` | 8 | `test_memory_model` | Vulkan 内存模型同步与 UAV 一致性 |
| `nvapi` | 6 | `test_nvapi` | NVIDIA NVAPI 驱动扩展与自定义寄存器 |
| `opts` | 15 | `test_opts` | 编译器优化 pass 与死代码消除 |
| `raw-access` | 23 | `test_raw_access` | Raw access chain 与 byte address buffer 操作 |
| `resources` | 159 | `test_resources` | CBV、SRV、UAV、sampler 绑定以及 bindless heap |
| `rov` | 29 | `test_rov` | 纹理与原始 buffer 上的光栅化有序视图（Rasterizer Ordered Views） |
| `sampler-feedback`| 2 | `test_sampler_feedback`| 缩小（minification）与 mip 级别采样器反馈图 |
| `semantics` | 29 | `test_semantics` | SV_Position、SV_ClipDistance、SV_CullDistance 与 SV_ViewID |
| `stages` | 48 | `test_stages` | 完整管线阶段：vertex、fragment、geometry、tessellation、mesh、task、光线追踪 |
| `vectorization` | 21 | `test_vectorization` | 向量 load/store 打包与标量化路径 |
| `view-instancing` | 41 | `test_view_instancing` | 多视口实例化（multiview instancing）、视口偏移与渲染实例掩码 |
| `vkmm` | 18 | `test_vkmm` | Vulkan Memory Model acquire/release 内存语义 |

除 829 个 HLSL shader 外，`asm/*.bc.dxil` 中预编译的原始 LLVM bitcode shader 由 `test_asm` 通过 `dxil_spirv::parse_dxil` 进行测试。

`e2e.rs` 中的专用入口点包括：
- `test_smoke`：跨标准顶点着色器（`simple.invariant.vert`、`boolean-io.vert`、`vertex-array-input.vert`）的快速验证。
- `test_dxbc_detection`：验证 DXBC 容器头部是否被正确识别，且格式错误的 buffer 能被干净拒绝而不会崩溃。
- `test_metrics_report`：在所有 829 个 shader 上运行完整套件并强制执行 regression baseline 规则。

## Shader 命名 Marker 与配置

上游 shader 在其文件名中嵌入了测试选项。测试 harness 在 `configure_converter()` 和 `setup_remappers()`（`dxil-spirv-tests/tests/harness.rs`）中检查这些 token，将其转换为安全 Rust API 调用。

```
<test-name>.<marker1>.<marker2>.<... >.<stage>
```

### 资源绑定 Marker

| Marker | CLI 等效项 | 安全 Wrapper API | 描述 |
|---|---|---|---|
| `.bindless.` | `--bindless` | `set_root_constant_word_count(8)` / `add_root_parameter_mapping()` | 带有 descriptor table 偏移的 BDA bindless heap 映射 |
| `.nobda.` | `--no-bda` | `PhysicalStorageBuffer { enable: false }` | 禁用用于 heap 寻址的 PhysicalStorageBuffer |
| `.cbv-as-ssbo.` | `--bindless-cbv-as-ssbo` | `BindlessCbvSsboEmulation { enable: true }` | 将 bindless CBV 模拟为 storage buffer |
| `.inline-ubo.` | `--root-constant-inline-uniform-block` | `RootConstantInlineUniformBlock` | 将 root constant 重映射为 inline uniform block（set 6, binding 1） |
| `.bindless-typed-buffer-offsets.` | `--bindless-typed-buffer-offsets` | `BindlessTypedBufferOffsets { enable: true }` | 为 typed buffer descriptor 启用 offset buffer |
| `.offset-layout.` | `--bindless-offset-buffer-layout` | `BindlessOffsetBufferLayout` | 定义 offset buffer 的 untyped、typed 和 stride 布局 |
| `.ssbo.` | `--ssbo-uav` `--ssbo-srv` | `SsboAlignment { alignment: 1 }` / Remapper | 将 raw 和 structured buffer 视为 storage buffer |
| `.ssbo-align.` | `--ssbo-alignment 64` | `SsboAlignment { alignment: 64 }` | 将 storage buffer 对齐要求设置为 64 字节 |
| `.ssbo-rtas.` | `--ssbo-rtas` | SRV remapper 中的 `VulkanDescriptorType::Ssbo` | 将光线追踪加速结构视为 SSBO |
| `.input-attachment.` | `--input-attachments` | `VulkanDescriptorType::InputAttachment` | 将 space 1000/1001 中的纹理绑定为 subpass input |
| `.root-descriptor.` | `--root-descriptor` | `add_root_descriptor_mapping()` | 为 CBV/SRV/UAV 配置 BDA root buffer 指针 |
| `.root-constant.` | `--root-constant` | `set_root_constant_word_count(16)` / CBV remapper | 将 space 0/1 的 CBV 映射到 push constant word 偏移 4/0 |
| `.local-root-signature.` | `--local-root-signature` | `add_local_root_constants()` / descriptors | 在寄存器 space 15 配置 DXR local root 参数 |
| `.stream-out.` | `--stream-output` | `set_stream_output_remapper()` | 配置顶点 stream output stride 和 buffer 索引 |

### 功能与指令 Marker

| Marker | 安全 Wrapper API | 效果 |
|---|---|---|
| `.native-fp16.` | `ConverterOption::MinPrecisionNative16Bit` | 发射原生 16 位浮点 SPIR-V 指令 |
| `.16bit-io.` | `ConverterOption::StorageInputOutput16Bit` | 启用 16 位 stage 输入与输出接口 |
| `.demote-to-helper.` | `ConverterOption::ShaderDemoteToHelper` | 将 discard 操作映射为 `OpDemoteToHelperInvocation` |
| `.i8dot.` | `ConverterOption::ShaderI8Dot` | 启用 8 位整数点积扩展 |
| `.dual-source-blending.` | `ConverterOption::DualSourceBlending` | 为双源混合发射次要颜色输出绑定 |
| `.noderivs.` | `ConverterOption::ComputeShaderDerivatives` | 在 compute shader 中禁用 quad 导数支持 |
| `.partitioned.` | `ConverterOption::SubgroupPartitionedNv` | 启用分区的 subgroup 操作 |
| `.quad-maximal-reconvergence.` | `ConverterOption::QuadControlReconvergence` | 强制执行最大 quad control 重汇聚（reconvergence） |
| `.raw-access-chains.` | `ConverterOption::RawAccessChainsNv` | 发射 raw access chain 指针运算指令 |
| `.extended-robustness.` | `ConverterOption::ExtendedRobustness` | 在 groupshared、alloca 和 LUT buffer 上启用越界检查 |
| `.heap-robustness.` | `ConverterOption::DescriptorHeapRobustness` | 发射 descriptor heap 索引的 robustness 检查 |
| `.full-wmma.` | `ConverterOption::Float8Support` | 启用 FP8 矩阵算术与 cooperative 转换 |
| `.assume-32bit-wrap.` | `ConverterOption::SsboAddressingBehavior` | 在 robustness 截断前假定 32 位溢出回绕（wrap）行为 |
| `.auto-group-shared-barrier.` | `ShaderQuirk::GroupSharedAutoBarrier` | 在访问共享内存前插入内存屏障 |
| `.mixed-float-dot-product.` | `ConverterOption::MixedFloatDotProduct` | 启用 FP16 输入、FP32 累加的点积 |
| `.rt-swizzle.` | `ConverterOption::OutputSwizzle` | 对 render target 输出分量进行 swizzle |
| `.invariant.` | `ConverterOption::InvariantPosition` | 将 `SV_Position` 输出标记为 invariant |
| `.omm.` / `.rq-omm.` | `ConverterOption::OpacityMicromap` | 启用光线追踪 Opacity Micromap 扩展 |

### 插桩与元描述符（Meta Descriptor）

| Marker | 安全 Wrapper API | 配置 |
|---|---|---|
| `.descriptor-qa.` | `ConverterOption::DescriptorQa` | 版本 2，descriptor set 10/10 与 10/11，哈希值 `0xdeadbeef` |
| `.bda-instrumentation.` | `ConverterOption::InstructionInstrumentation` | control set 0 binding 2，payload set 0 binding 3，哈希值 `0xabcd` |
| `.vkmm.` | `ConverterOption::VulkanMemoryModel` | 发射 Vulkan Memory Model capability 与同步 |
| `.nvapi.` | `ConverterOption::Nvapi` | 在寄存器 127、space 0 上启用 NVAPI 驱动支持 |
| `.heap-robustness-cbv.` | `set_meta_descriptor()` | `ResourceDescriptorHeapSize` 在 set 10 binding 20 绑定为 UBO 常量 |
| `.heap-raw-va-cbv.` | `set_meta_descriptor()` | `RawDescriptorHeapView` 在 set 10 binding 21 绑定为 UBO BDA |
| `.view-instancing.` | `set_meta_descriptor()` | `DynamicViewInstancingOffsets` 在 set 10 binding 22 绑定为 push constant |
| `.view-instance-mask.` | `set_meta_descriptor()` | `DynamicViewInstancingMask` 在 set 10 binding 23 绑定为 push constant |

### 编译 Profile Marker

构建脚本根据文件扩展名与 marker 标签选择目标 profile：

- `.sm60.`、`.sm66.`、`.sm67.`、`.sm69.`：覆盖默认 Shader Model（次版本号 5）。
- `.node.`：面向 Work Graph 的 compute shader 使用 `lib_6_8` 库 profile 编译。
- `.denorm-ftz.` / `.denorm-preserve.`：控制浮点非规范化数（denormal）刷新模式。
- `.no-legacy-cbuf-layout.`：禁用传统 DirectX 常量缓冲区打包规则。
- `.noglsl.`：当 SPIRV-Cross 不支持特定 shader 特性时跳过 GLSL 交叉编译验证。

## 测试 Harness 中的 Remapper 架构

在 `harness.rs` 中，`setup_remappers()` 建立与上游 CLI 行为一致的回调闭包：

- **SRV Remapper**：检查 root descriptor 优先级，为全局 heap 分配 descriptor set 0 binding 0，为 bindless 非 buffer 和 buffer descriptor 分配 set 0 与 1，并为非 bindless 资源分配 space/index。当 `.ssbo-rtas.` 处于激活状态时，将 RTAS 资源转换为 SSBO descriptor。
- **Sampler Remapper**：将 bindless sampler 指向 set 2 binding 0，且 root constant 索引为 2，或设置匹配的寄存器 space 与 index。
- **CBV Remapper**：计算 root descriptor 与 push constant。对于 `.root-constant.` shader，将 space 0 register 0 映射到 word 偏移 4，将 space 1 register 0 映射到偏移 0。在 bindless 配置下，Uniform CBV 路由到 set 5 binding 0。
- **UAV Remapper**：处理 UAV buffer 绑定以及 set 7 处的 counter 绑定。根据 marker 标签，counter descriptor 类型支持 TexelBuffer 或 SSBO。
- **顶点输入与 Stream Output Remapper**：映射语义名称（例如将 `ATTR` 映射到 location 0）并为 geometry stream-out 阶段定义输出 stride。

## Regression Baseline 机制

文件 `tests/regression_baseline.json` 记录了每个 shader 的预期状态。运行 `test_metrics_report` 时，测试驱动会将实际执行结果与该 baseline 进行比对：

- **Pass 变为 Non-Pass**：测试硬失败。任何先前通过但现在失败的 shader 都表明 wrapper 逻辑或上游绑定中存在 regression。
- **Non-Pass 变为 Pass**：在测试 stdout 中报告为修复（fix），提示开发者更新 baseline 文件。
- **新增 Shader**：在上游添加测试用例时报告，确保每个新增 shader 都被纳入考量。

在进行有意的改进后若需更新 baseline，请设置环境变量：

```bash
DXIL_SPIRV_UPDATE_BASELINE=1 cargo test -p dxil-spirv-tests test_metrics_report
```

## DXC 工具链与 `dxc_unavailable` CFG

测试 harness 需要 DXC 将 HLSL 源码编译为 DXIL bitcode：

1. `build.rs` 在 `target/dxc/1.9.2602.17/`、`DXC_PATH` 变量、系统 `PATH` 以及 Windows Kits 目录中查找 DXC。
2. 若未找到 DXC，`build.rs` 会下载官方 Microsoft 发布版本（`v1.9.2602`，asset 为 `dxc_2026_02_20.zip`），并将 x64 二进制文件解压到 `target/dxc/1.9.2602.17/`。
3. 由于官方发布 asset 是 Windows x64 二进制文件，非 Windows 主机无法直接运行 `dxc.exe`。构建脚本通过 `is_dxc_runnable()` 测试二进制执行。如果 DXC 无法运行，则设置编译配置标志 `dxc_unavailable`。
4. 在 `e2e.rs` 中，所有测试函数都会检查 `if cfg!(dxc_unavailable)` 并优雅跳过执行。这使得安全 wrapper 和 sys crate 能够在 macOS 和 Linux 上构建并通过单元测试，而不会因为缺失 DXIL 制品而失败。

## 失败分类架构

`harness.rs` 中的 `requires_complex_remapper()` 函数充当已知复杂 remapper 模式的分类器。在最初开发完整的逐 shader 重映射功能时，该函数用于识别需要自定义 descriptor heap 表、BDA root descriptor 或 push constant 的 shader。

如今，由于安全 wrapper API 和 harness 已支持所有 remapper 回调、root constant 表以及 BDA 配置，**所有 829 个 shader 均已完全通过（0 个已知失败）**。该分类函数依然作为主动安全网保留，以便在上游引入新的 remapper 语法时对任何潜在的未来 regression 进行分类。

## 添加新测试

若要向套件中添加新的 shader 或测试用例：

1. **放置 HLSL 源码**：将 shader 保存到 `dxil-spirv-sys/dxil-spirv/shaders/<category>/` 下相应的类别文件夹中。
2. **应用 Marker Token**：使用标准 marker 命名约定命名文件（例如 `custom_test.bindless.sm66.frag`）。
3. **同步并编译**：运行 `cargo build -p dxil-spirv-tests`。构建脚本会将文件复制到 `tests/shaders/` 并使用 DXC 对其进行编译。
4. **按需扩展 Remapper**：如果 shader 引入了新的 CLI 标志或绑定语义，请在 `harness.rs` 中的 `configure_converter()` 和 `setup_remappers()` 中添加对应的逻辑。
5. **验证执行**：
   ```bash
   cargo test -p dxil-spirv-tests test_completeness_check
   cargo test -p dxil-spirv-tests test_<category> -- --nocapture
   ```
6. **更新 Baseline**：通过运行以下命令更新 `tests/regression_baseline.json`：
   ```bash
   DXIL_SPIRV_UPDATE_BASELINE=1 cargo test -p dxil-spirv-tests test_metrics_report
   ```

## 调试测试失败

若要隔离并检查单个 shader 转换问题：

```bash
# 运行特定类别并启用 stdout 输出
cargo test -p dxil-spirv-tests test_descriptor_qa -- --nocapture

# 运行轻量级 smoke 测试
cargo test -p dxil-spirv-tests test_smoke -- --nocapture

# 验证 DXBC 容器解析器的健壮性
cargo test -p dxil-spirv-tests test_dxbc_detection -- --nocapture

# 运行完整套件，并对上游参考输出执行严格的 GLSL MD5 比对
# 在 Linux / macOS / Git Bash 上：
DXIL_SPIRV_STRICT_GLSL=1 cargo test -p dxil-spirv-tests -- --nocapture

# 在 Windows PowerShell 上：
$env:DXIL_SPIRV_STRICT_GLSL='1'; cargo test -p dxil-spirv-tests -- --nocapture

# 从类别运行中过滤特定的失败信息
cargo test -p dxil-spirv-tests test_resources -- --nocapture 2>&1 | Select-String "FAIL:"
```
