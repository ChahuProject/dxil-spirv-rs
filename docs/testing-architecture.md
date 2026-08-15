# Testing Architecture

This document describes the dxil-spirv-rs test suite architecture, test categories, and known-failure classification system.

## Test Suite Overview

The test suite validates that the safe Rust wrapper produces identical output to the upstream dxil-spirv CLI for all 829 test shaders.

### Test Execution Model

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   HLSL Shader   │────▶│  DXC Compiler    │────▶│  DXIL Bitcode   │
│   (.frag/.comp) │     │  (v1.9.2602.17)  │     │  (.dxil)        │
└─────────────────┘     └──────────────────┘     └────────┬────────┘
                                                          │
                           ┌──────────────────────────────┘
                           ▼
                    ┌──────────────────┐
                    │  dxil-spirv-rs   │
                    │  (Safe Wrapper)  │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │  SPIR-V Words    │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │  spirv-cross2    │
                    │  (GLSL Output)   │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │  MD5 Compare     │
                    │  (vs Reference)  │
                    └──────────────────┘
```

### Subprocess Isolation

Each shader conversion runs in a **fresh child process** (`test_shader()` in `harness.rs`). This is critical because upstream C++ can hit hard asserts (`SpvBuilder.cpp:754`, `ir.hpp:113`) that would kill the entire test run.

## Test Categories (24 total)

| Category | Shaders | Purpose | Complexity |
|----------|---------|---------|------------|
| `stages` | ~50 | Vertex/fragment/geometry/etc. stage coverage | Low |
| `resources` | ~150 | Resource binding (CBV/SRV/UAV/sampler) | Medium-High |
| `dxil-builtin` | ~50 | DXIL intrinsic functions | Medium |
| `vectorization` | ~30 | Vector load/store operations | Medium |
| `instrumentation` | ~15 | BDA instrumentation | High |
| `descriptor_qa` | ~15 | Descriptor QA validation | High |
| `rov` | ~15 | Rasterizer ordered views | Medium |
| `raw-access` | ~15 | Raw access chains | Medium |
| `vkmm` | ~10 | Vulkan memory model | High |
| `memory-model` | ~10 | Memory coherence | Medium |
| `opts` | ~10 | Optimization hints | Medium |
| `heap-robustness` | ~10 | Descriptor heap robustness | High |
| `alloca-opts` | ~10 | Alloca optimizations | Medium |
| `nvapi` | ~10 | NVAPI integration | High |
| `ags` | ~10 | AGS library functions | High |
| `asm` | ~5 | Assembly-level tests | Low |
| `auto-barrier` | ~5 | Automatic barrier insertion | Medium |
| `control-flow` | ~20 | Control flow graphs | Low |
| `fp16` | ~10 | Half-precision float | Medium |
| `llvm-builtin` | ~10 | LLVM intrinsic mapping | Low |
| `sampler-feedback` | ~5 | Sampler feedback | Medium |
| `semantics` | ~20 | Semantic validation | Low |
| `smoke` | ~5 | Basic smoke tests | Low |
| `view-instancing` | ~10 | View instancing | High |

## Shader Naming Convention

Shaders use filename markers to indicate required configuration:

```
<name>.<marker1>.<marker2>.<...>.<stage>
```

### Markers by Category

#### Resource Binding Markers
| Marker | CLI Equivalent | Safe API | Description |
|--------|---------------|----------|-------------|
| `.bindless.` | `--bindless` | `BindlessCbvSsboEmulation` | Use descriptor heap |
| `.root-descriptor.` | `--root-descriptor` | `add_root_descriptor_mapping()` | BDA root descriptors |
| `.root-constant.` | `--root-constant` | `add_local_root_constants()` | Push constants |
| `.local-root-signature.` | `--local-root-signature` | `begin/end_local_root_descriptor_table()` | DXR local root sig |
| `.ssbo.` | `--ssbo-uav` `--ssbo-srv` | Remapper descriptor type | SSBO for raw/structured |
| `.ssbo-rtas.` | `--ssbo-rtas` | Remapper descriptor type | RTAS as SSBO |
| `.input-attachment.` | `--input-attachments` | Remapper descriptor type | Input attachments |
| `.cbv-as-ssbo.` | `--bindless-cbv-as-ssbo` | `BindlessCbvSsboEmulation` | CBV as SSBO |

#### Feature Markers
| Marker | CLI Equivalent | Safe API | Description |
|--------|---------------|----------|-------------|
| `.native-fp16.` | `--min-precision-native-16bit` | `MinPrecisionNative16Bit` | Native FP16 |
| `.16bit-io.` | `--storage-input-output-16bit` | `StorageInputOutput16Bit` | 16-bit I/O |
| `.demote-to-helper.` | `--enable-shader-demote` | `ShaderDemoteToHelper` | Demote to helper |
| `.i8dot.` | `--enable-shader-i8-dot` | `ShaderI8Dot` | Int8 dot product |
| `.dual-source-blending.` | `--enable-dual-source-blending` | `DualSourceBlending` | Dual source blend |
| `.noderivs.` | `--no-compute-shader-derivatives` | `ComputeShaderDerivatives` | No derivatives |
| `.partitioned.` | `--subgroup-partitioned-nv` | `SubgroupPartitionedNv` | Partitioned subgroup |
| `.quad-maximal-reconvergence.` | `--quad-control-maximal-reconvergence` | `QuadControlReconvergence` | Quad control |
| `.raw-access-chains.` | `--raw-access-chains-nv` | `RawAccessChainsNv` | Raw access chains |
| `.extended-robustness.` | `--extended-robustness` | `ExtendedRobustness` | Extended robustness |
| `.heap-robustness.` | `--descriptor-heap-robustness` | `DescriptorHeapRobustness` | Heap robustness |

#### Instrumentation Markers
| Marker | CLI Equivalent | Safe API | Description |
|--------|---------------|----------|-------------|
| `.descriptor-qa.` | `--descriptor-qa 10 10 deadbeef` | `DescriptorQa` | Descriptor QA |
| `.bda-instrumentation.` | `--instruction-instrumentation 4 0 2 abcd` | `InstructionInstrumentation` | BDA instrumentation |
| `.vkmm.` | `--vkmm` | `VulkanMemoryModel` | Vulkan memory model |
| `.nvapi.` | `--nvapi 127 0` | `Nvapi` | NVAPI integration |

#### Shader Model Markers
| Marker | Description |
|--------|-------------|
| `.sm60.` | Shader Model 6.0 |
| `.sm66.` | Shader Model 6.6 |
| `.sm67.` | Shader Model 6.7 |
| `.sm69.` | Shader Model 6.9 |

#### Misc Markers
| Marker | Description |
|--------|-------------|
| `.noglsl.` | Skip GLSL comparison |
| `.noderivs.` | No compute derivatives |
| `.invariant.` | Invariant position |
| `.omm.` | Opacity micromap |
| `.rq-omm.` | Ray query OMM |

## Known-Failure Classification

The `requires_complex_remapper()` function classifies failures into categories:

### Classification Priority (evaluated in order)

1. **`.bindless.`** (96 shaders) - Needs per-shader heap mapping configuration
2. **`.root-descriptor.`** (28 shaders) - Needs BDA root descriptor table
3. **`.root-constant.`** (7 shaders) - Needs push constant mapping
4. **`.local-root-signature.`** (1 shader) - Needs local root descriptor table
5. **`.ssbo.`** (65 shaders) - Needs SSBO default descriptor type
6. **`.bda-instrumentation.`** (0 shaders) - All caught by earlier branches

### Current Status (2026-08-16)

```
Total: 829 shaders
Passed: 635 (76.6%)
Known failures: 194 (23.4%)
Skipped: 0
```

### Failure Root Causes

| Root Cause | Count | Fix Strategy |
|------------|-------|--------------|
| Missing CLI option mapping | 3 | Add option to `configure_converter()` |
| Push constant range insufficient | 12 | Increase `root_constant_word_count` |
| Upstream C++ internal error (-2) | ~30 | Requires upstream fix |
| "Dummy SSBO must be an SSBO" | ~10 | Fix SSBO descriptor type in remapper |
| Complex per-shader config | ~120 | Phase 3: config file driven remapper |

## Phase 3: Config-Driven Remapper (Deferred)

The remaining ~124 complex failures need a per-shader configuration table, similar to upstream's `test_shaders.py` but in Rust:

```rust
// Planned: per-shader config
struct ShaderConfig {
    name_pattern: String,
    root_constant_word_count: Option<u32>,
    ssbo_descriptor_type: Option<VulkanDescriptorType>,
    bindless_heap_offset: Option<u32>,
    // ...
}
```

This is deferred to keep Phase 1+2 scope manageable.

## Regression Baseline

The `tests/regression_baseline.json` tracks expected pass/fail status per shader:

- **pass** → **non-pass**: Hard failure (regression detected)
- **non-pass** → **pass**: Printed as fix (suggest baseline update)
- **New shader**: Printed as new

Update baseline: `DXIL_SPIRV_UPDATE_BASELINE=1 cargo test -p dxil-spirv-tests test_metrics_report`

## Adding New Tests

1. Add HLSL file to appropriate category in `dxil-spirv-sys/dxil-spirv/shaders/`
2. Use appropriate filename markers for required configuration
3. Run `cargo test -p dxil-spirv-tests` to verify
4. If new marker needed, add to `configure_converter()`
5. Update baseline if intentional change

## Debugging Failed Tests

```bash
# Run single category
cargo test -p dxil-spirv-tests --test e2e test_descriptor_qa -- --nocapture

# Run with strict GLSL comparison
$env:DXIL_SPIRV_STRICT_GLSL='1'; cargo test -p dxil-spirv-tests -- --nocapture

# Check specific shader error
cargo test -p dxil-spirv-tests --test e2e test_descriptor_qa -- --nocapture 2>&1 | Select-String "FAIL:"
```
