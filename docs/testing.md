# Testing Architecture

This document describes the testing architecture for `dxil-spirv-rs`, including coverage guarantees, test harness design, shader configuration markers, and regression baseline mechanics.

## Coverage Guarantee

The safe Rust wrapper provides complete, verified parity with the upstream C++ implementation.

```
Upstream Test Suite Coverage: 829 / 829 shaders (100.0% passing)
Known Failures: 0 (0.0%)
Skipped Shaders: 0 (0.0%)
```

The upstream `dxil-spirv` C++ shader test suite is 100% covered and 100% passing. This pass rate is enforced by three interlocking validation mechanisms:

1. **Completeness Check (`test_completeness_check` in `dxil-spirv-tests/tests/e2e.rs`)**: Discovers all shader source files in `dxil-spirv-sys/dxil-spirv/shaders/` and compares them against the test set in `tests/shaders/`. Any missing or extra shader causes a hard test failure. The check also verifies that neither set is empty, preventing false-positive passes when git submodules are uninitialized.
2. **Build-Time Shader Sync (`dxil-spirv-tests/build.rs`)**: Synchronizes HLSL sources, C header includes, and reference GLSL files directly from the vendored submodule on every build. It invokes the DirectX Shader Compiler (DXC) to compile all shaders into DXIL bytecode.
3. **Regression Baseline Guard (`tests/regression_baseline.json`)**: Records the pass/fail status of every shader. Any regression where a shader transitions from `pass` to any other state triggers an immediate test panic in `test_metrics_report`.

## Test Execution Model

The test harness runs each shader through a multi-stage validation pipeline:

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

Every shader conversion executes in a dedicated child process spawned by `test_shader()` in `harness.rs`. The parent process passes the shader path via the `DXIL_SPIRV_TEST_CHILD_SHADER` environment variable and invokes the test binary with `--exact __child_noop__`.

Child processes communicate results back using a structured stdout protocol (`__DXIL_SPIRV_RESULT__|status|spirv_len|error`). This isolation prevents upstream C++ assertion failures (such as glslang `SpvBuilder.cpp:754` or `ir.hpp:113`) from terminating the test suite runner. Each child process runs under a 30-second watchdog timer to catch any infinite loops in the converter.

### Validation Layers

1. **DXIL Parsing**: Parses compiled DXIL containers via `dxil_spirv::ParsedBlob::parse()`. For raw LLVM bitcode files (`asm/*.bc.dxil`), it uses `dxil_spirv::parse_dxil()`.
2. **Converter Configuration**: Maps filename markers to typed options via `configure_converter()`.
3. **SPIR-V Header Check**: Verifies that the emitted SPIR-V stream begins with the valid SPIR-V magic word `0x07230203` and contains non-empty bytecode.
4. **GLSL Decompilation**: Uses `spirv-cross2` to decompile SPIR-V back into GLSL 460 with Vulkan semantics. This ensures the output is structurally sound and consumable by downstream tools.
5. **Reference Comparison**: When `DXIL_SPIRV_STRICT_GLSL=1` is set, computes the MD5 hash of normalized GLSL output and verifies exact equality against reference files in `tests/reference/shaders/`.

## Test Categories

The 829 test shaders are organized into 22 primary categories. Each category corresponds to an end-to-end test function in `dxil-spirv-tests/tests/e2e.rs`.

| Category | Shaders | Test Function | Purpose |
|---|---|---|---|
| `ags` | 28 | `test_ags` | AMD AGS library functions and SM6.6 WMMA matrix operations |
| `alloca-opts` | 16 | `test_alloca_opts` | Dynamic alloca allocations and stack memory optimizations |
| `auto-barrier` | 6 | `test_auto_barrier` | Automatic barrier insertion for group shared memory |
| `control-flow` | 26 | `test_control_flow` | Loops, switch tables, and complex branching control flow |
| `descriptor_qa` | 13 | `test_descriptor_qa` | Descriptor QA validation and ray tracing acceleration structures |
| `dxil-builtin` | 275 | `test_dxil_builtin` | Intrinsic DXIL operations, wave votes, math builtins |
| `fp16` | 3 | `test_fp16` | Native 16-bit half-precision floating point operations |
| `heap-robustness` | 5 | `test_heap_robustness` | Descriptor heap bounds checking and out-of-bounds safety |
| `instrumentation` | 14 | `test_instrumentation` | Buffer Device Address (BDA) instruction instrumentation |
| `llvm-builtin` | 44 | `test_llvm_builtin` | Bit manipulation intrinsics and LLVM lowering operations |
| `memory-model` | 8 | `test_memory_model` | Vulkan memory model synchronization and UAV coherence |
| `nvapi` | 6 | `test_nvapi` | NVIDIA NVAPI driver extensions and custom registers |
| `opts` | 15 | `test_opts` | Compiler optimization passes and dead code elimination |
| `raw-access` | 23 | `test_raw_access` | Raw access chains and byte address buffer operations |
| `resources` | 159 | `test_resources` | CBV, SRV, UAV, sampler bindings, and bindless heaps |
| `rov` | 29 | `test_rov` | Rasterizer Ordered Views across textures and raw buffers |
| `sampler-feedback`| 2 | `test_sampler_feedback`| Minification and mip-level sampler feedback maps |
| `semantics` | 29 | `test_semantics` | SV_Position, SV_ClipDistance, SV_CullDistance, and SV_ViewID |
| `stages` | 48 | `test_stages` | Full pipeline stages: vertex, fragment, geometry, tessellation, mesh, task, ray tracing |
| `vectorization` | 21 | `test_vectorization` | Vector load/store packing and scalarization paths |
| `view-instancing` | 41 | `test_view_instancing` | Multiview instancing, viewport offsets, and render instance masks |
| `vkmm` | 18 | `test_vkmm` | Vulkan Memory Model acquire/release memory semantics |

In addition to the 829 HLSL shaders, precompiled raw LLVM bitcode shaders in `asm/*.bc.dxil` are tested by `test_asm` using `dxil_spirv::parse_dxil`.

Specialized entry points in `e2e.rs` include:
- `test_smoke`: Quick validation across standard vertex shaders (`simple.invariant.vert`, `boolean-io.vert`, `vertex-array-input.vert`).
- `test_dxbc_detection`: Validates that DXBC container headers are recognized and malformed buffers are rejected cleanly without crashes.
- `test_metrics_report`: Runs the complete suite across all 829 shaders and enforces regression baseline rules.

## Shader Naming Markers and Configuration

Upstream shaders embed test options in their file names. The test harness inspects these tokens in `configure_converter()` and `setup_remappers()` (`dxil-spirv-tests/tests/harness.rs`), translating them into safe Rust API calls.

```
<test-name>.<marker1>.<marker2>.<... >.<stage>
```

### Resource Binding Markers

| Marker | CLI Equivalent | Safe Wrapper API | Description |
|---|---|---|---|
| `.bindless.` | `--bindless` | `set_root_constant_word_count(8)` / `add_root_parameter_mapping()` | BDA bindless heap mapping with descriptor table offsets |
| `.nobda.` | `--no-bda` | `PhysicalStorageBuffer { enable: false }` | Disables PhysicalStorageBuffer for heap addressing |
| `.cbv-as-ssbo.` | `--bindless-cbv-as-ssbo` | `BindlessCbvSsboEmulation { enable: true }` | Emulates bindless CBVs as storage buffers |
| `.inline-ubo.` | `--root-constant-inline-uniform-block` | `RootConstantInlineUniformBlock` | Remaps root constants to inline uniform blocks (set 6, binding 1) |
| `.bindless-typed-buffer-offsets.` | `--bindless-typed-buffer-offsets` | `BindlessTypedBufferOffsets { enable: true }` | Enables offset buffers for typed buffer descriptors |
| `.offset-layout.` | `--bindless-offset-buffer-layout` | `BindlessOffsetBufferLayout` | Defines untyped, typed, and stride layout for offset buffers |
| `.ssbo.` | `--ssbo-uav` `--ssbo-srv` | `SsboAlignment { alignment: 1 }` / Remapper | Treats raw and structured buffers as storage buffers |
| `.ssbo-align.` | `--ssbo-alignment 64` | `SsboAlignment { alignment: 64 }` | Sets storage buffer alignment requirement to 64 bytes |
| `.ssbo-rtas.` | `--ssbo-rtas` | `VulkanDescriptorType::Ssbo` in SRV remapper | Treats ray tracing acceleration structures as SSBOs |
| `.input-attachment.` | `--input-attachments` | `VulkanDescriptorType::InputAttachment` | Binds textures in spaces 1000/1001 as subpass inputs |
| `.root-descriptor.` | `--root-descriptor` | `add_root_descriptor_mapping()` | Configures BDA root buffer pointers for CBV/SRV/UAV |
| `.root-constant.` | `--root-constant` | `set_root_constant_word_count(16)` / CBV remapper | Maps CBVs at space 0/1 to push constant word offsets 4/0 |
| `.local-root-signature.` | `--local-root-signature` | `add_local_root_constants()` / descriptors | Configures DXR local root arguments at register space 15 |
| `.stream-out.` | `--stream-output` | `set_stream_output_remapper()` | Configures vertex stream output strides and buffer indices |

### Feature and Instruction Markers

| Marker | Safe Wrapper API | Effect |
|---|---|---|
| `.native-fp16.` | `ConverterOption::MinPrecisionNative16Bit` | Emits native 16-bit floating point SPIR-V instructions |
| `.16bit-io.` | `ConverterOption::StorageInputOutput16Bit` | Enables 16-bit stage input and output interfaces |
| `.demote-to-helper.` | `ConverterOption::ShaderDemoteToHelper` | Maps discard operations to `OpDemoteToHelperInvocation` |
| `.i8dot.` | `ConverterOption::ShaderI8Dot` | Enables 8-bit integer dot product extensions |
| `.dual-source-blending.` | `ConverterOption::DualSourceBlending` | Emits secondary color output bindings for dual-source blends |
| `.noderivs.` | `ConverterOption::ComputeShaderDerivatives` | Disables quad derivative support in compute shaders |
| `.partitioned.` | `ConverterOption::SubgroupPartitionedNv` | Enables partitioned subgroup operations |
| `.quad-maximal-reconvergence.` | `ConverterOption::QuadControlReconvergence` | Enforces maximal quad control reconvergence |
| `.raw-access-chains.` | `ConverterOption::RawAccessChainsNv` | Emits raw access chain pointer math instructions |
| `.extended-robustness.` | `ConverterOption::ExtendedRobustness` | Enables bounds checks on groupshared, alloca, and LUT buffers |
| `.heap-robustness.` | `ConverterOption::DescriptorHeapRobustness` | Emits robustness checks for descriptor heap indexing |
| `.full-wmma.` | `ConverterOption::Float8Support` | Enables FP8 matrix arithmetic and cooperative conversions |
| `.assume-32bit-wrap.` | `ConverterOption::SsboAddressingBehavior` | Assumes 32-bit wrap behavior before robustness clamping |
| `.auto-group-shared-barrier.` | `ShaderQuirk::GroupSharedAutoBarrier` | Inserts memory barriers before shared memory access |
| `.mixed-float-dot-product.` | `ConverterOption::MixedFloatDotProduct` | Enables FP16 input with FP32 accumulation dot products |
| `.rt-swizzle.` | `ConverterOption::OutputSwizzle` | Swizzles render target output components |
| `.invariant.` | `ConverterOption::InvariantPosition` | Marks `SV_Position` outputs as invariant |
| `.omm.` / `.rq-omm.` | `ConverterOption::OpacityMicromap` | Enables ray tracing Opacity Micromap extensions |

### Instrumentation and Meta Descriptors

| Marker | Safe Wrapper API | Configuration |
|---|---|---|
| `.descriptor-qa.` | `ConverterOption::DescriptorQa` | Version 2, descriptor sets 10/10 and 10/11, hash `0xdeadbeef` |
| `.bda-instrumentation.` | `ConverterOption::InstructionInstrumentation` | Control set 0 binding 2, payload set 0 binding 3, hash `0xabcd` |
| `.vkmm.` | `ConverterOption::VulkanMemoryModel` | Emits Vulkan Memory Model capability and synchronization |
| `.nvapi.` | `ConverterOption::Nvapi` | Enables NVAPI driver support on register 127, space 0 |
| `.heap-robustness-cbv.` | `set_meta_descriptor()` | `ResourceDescriptorHeapSize` bound as UBO constant at set 10 binding 20 |
| `.heap-raw-va-cbv.` | `set_meta_descriptor()` | `RawDescriptorHeapView` bound as UBO BDA at set 10 binding 21 |
| `.view-instancing.` | `set_meta_descriptor()` | `DynamicViewInstancingOffsets` bound as push constant at set 10 binding 22 |
| `.view-instance-mask.` | `set_meta_descriptor()` | `DynamicViewInstancingMask` bound as push constant at set 10 binding 23 |

### Compilation Profile Markers

The build script selects target profiles based on file extensions and marker tags:

- `.sm60.`, `.sm66.`, `.sm67.`, `.sm69.`: Override the default Shader Model (minor version 5).
- `.node.`: Compute shaders targeting Work Graphs compile using the `lib_6_8` library profile.
- `.denorm-ftz.` / `.denorm-preserve.`: Control floating-point denormal flushing modes.
- `.no-legacy-cbuf-layout.`: Disables legacy DirectX constant buffer packing rules.
- `.noglsl.`: Skips GLSL cross-compilation validation when SPIRV-Cross does not support specific shader features.

## Remapper Architecture in the Test Harness

In `harness.rs`, `setup_remappers()` establishes callback closures matching upstream CLI behavior:

- **SRV Remapper**: Checks root descriptor precedence, assigns descriptor set 0 binding 0 for global heaps, sets 0 and 1 for bindless non-buffer and buffer descriptors, and space/index for non-bindless resources. Translates RTAS resources to SSBO descriptors when `.ssbo-rtas.` is active.
- **Sampler Remapper**: Directs bindless samplers to set 2 binding 0 with root constant index 2, or sets matching register space and index.
- **CBV Remapper**: Evaluates root descriptors and push constants. For `.root-constant.` shaders, maps space 0 register 0 to word offset 4, and space 1 register 0 to offset 0. Uniform CBVs route to set 5 binding 0 under bindless configurations.
- **UAV Remapper**: Handles UAV buffer bindings and counter bindings at set 7. Counter descriptor types support TexelBuffer or SSBO depending on marker tags.
- **Vertex Input & Stream Output Remappers**: Maps semantic names (like `ATTR` to location 0) and defines output strides for geometry stream-out stages.

## Regression Baseline Mechanics

The file `tests/regression_baseline.json` records the expected state of each shader. When running `test_metrics_report`, the test driver compares actual execution results against this baseline:

- **Pass to Non-Pass**: Hard test failure. Any previously passing shader that fails indicates a regression in wrapper logic or upstream bindings.
- **Non-Pass to Pass**: Reported in test stdout as a fix, prompting the developer to update the baseline file.
- **New Shader**: Reported when upstream adds test cases, ensuring every new shader is accounted for.

To update the baseline after making intentional improvements, set the environment variable:

```bash
DXIL_SPIRV_UPDATE_BASELINE=1 cargo test -p dxil-spirv-tests test_metrics_report
```

## DXC Toolchain and the `dxc_unavailable` CFG

The test harness requires DXC to compile HLSL sources into DXIL bitcode:

1. `build.rs` searches for DXC in `target/dxc/1.9.2602.17/`, the `DXC_PATH` variable, system `PATH`, and Windows Kits directories.
2. If DXC is not present, `build.rs` downloads the official Microsoft release (`v1.9.2602`, asset `dxc_2026_02_20.zip`) and unpacks x64 binaries into `target/dxc/1.9.2602.17/`.
3. Because the official release asset is a Windows x64 binary, non-Windows hosts cannot run `dxc.exe` directly. The build script tests binary execution via `is_dxc_runnable()`. If DXC cannot run, it sets the compile configuration flag `dxc_unavailable`.
4. In `e2e.rs`, all test functions check `if cfg!(dxc_unavailable)` and skip execution gracefully. This allows the safe wrapper and sys crate to build and pass unit tests on macOS and Linux without failing on missing DXIL artifacts.

## Failure Classification Architecture

The `requires_complex_remapper()` function in `harness.rs` serves as a classifier for known complex remapper patterns. When full per-shader remapping was first developed, this function identified shaders requiring custom descriptor heap tables, BDA root descriptors, or push constants.

Today, because the safe wrapper API and harness support all remapper callbacks, root constant tables, and BDA configurations, **all 829 shaders pass completely (0 known failures)**. The classification function remains in place as an active safety net to categorize any potential future regressions if upstream introduces new remapper syntax.

## Adding a New Test

To add a new shader or test case to the suite:

1. **Place HLSL Source**: Save the shader into the appropriate category folder under `dxil-spirv-sys/dxil-spirv/shaders/<category>/`.
2. **Apply Marker Tokens**: Name the file using the standard marker conventions (for example, `custom_test.bindless.sm66.frag`).
3. **Sync and Compile**: Run `cargo build -p dxil-spirv-tests`. The build script will copy the file to `tests/shaders/` and compile it with DXC.
4. **Extend Remappers If Needed**: If the shader introduces novel CLI flags or binding semantics, add matching logic to `configure_converter()` and `setup_remappers()` in `harness.rs`.
5. **Verify Execution**:
   ```bash
   cargo test -p dxil-spirv-tests test_completeness_check
   cargo test -p dxil-spirv-tests test_<category> -- --nocapture
   ```
6. **Update Baseline**: Update `tests/regression_baseline.json` by running:
   ```bash
   DXIL_SPIRV_UPDATE_BASELINE=1 cargo test -p dxil-spirv-tests test_metrics_report
   ```

## Debugging Test Failures

To isolate and inspect individual shader conversion issues:

```bash
# Run a specific category with stdout output enabled
cargo test -p dxil-spirv-tests test_descriptor_qa -- --nocapture

# Run the lightweight smoke test
cargo test -p dxil-spirv-tests test_smoke -- --nocapture

# Verify DXBC container parser robustness
cargo test -p dxil-spirv-tests test_dxbc_detection -- --nocapture

# Run the full suite with strict GLSL MD5 comparison against upstream reference outputs
# On Linux / macOS / Git Bash:
DXIL_SPIRV_STRICT_GLSL=1 cargo test -p dxil-spirv-tests -- --nocapture

# On Windows PowerShell:
$env:DXIL_SPIRV_STRICT_GLSL='1'; cargo test -p dxil-spirv-tests -- --nocapture

# Filter specific failure messages from a category run
cargo test -p dxil-spirv-tests test_resources -- --nocapture 2>&1 | Select-String "FAIL:"
```
