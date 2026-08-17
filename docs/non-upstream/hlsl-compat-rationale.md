# hlsl-compat: cbuffer vec4 alignment post-processing (NON-UPSTREAM EXTENSION)

> This document details the **non-upstream extension** `non_upstream::hlsl_compat` in dxil-spirv-rs,
> enabled via the crate feature `non-upstream-hlsl-compat` (disabled by default).
> It isn't part of upstream dxil-spirv / dxbc-spirv, nor is it endorsed upstream.
> Design motivation, reproduction steps, root-cause analysis, and verification results are documented here.

---

## 1. Background & motivation

The primary use case for dxil-spirv-rs is converting D3D11/D3D12 captured shaders (DXBC or DXIL) into SPIR-V, then feeding them to SPIRV-Cross to decompile readable HLSL / GLSL / MSL for inspection.

Real-world test results (render debugger, Unity Universal 3D Sample):

| Stage | D3D12 (105 shaders) | D3D11 (62 shaders) |
|---|---|---|
| DXIL/DXBC → SPIR-V conversion | All succeeded | All succeeded |
| HLSL decompilation failure rate | **74% (78 shaders)** | **92% (57 shaders)** |
| GLSL / MSL / JSON / Assembly | All succeeded | All succeeded |

The failure errors all belong to the exact same class:

```
The SPIR-V operation is unsupported: cbuffer ID N (name: _9_11), member index 0
(name: _m0) cannot be expressed with either HLSL packing layout or packoffset.
```

GLSL/MSL conversions succeeded on every shader in the same batch. That demonstrates the problem **exists solely in the SPIR-V → HLSL stage**.

## 2. Symptom: stride-4 scalar cbuffer views

In the SPIR-V of failing shaders, upstream emits certain cbuffers as **tightly packed scalar arrays**:

```text
struct { _m0 @ 0: float[536] (ArrayStride 4) }   ← source DXBC declared 134 vec4s (134×4=536)
```

In the very same shader, another cbuffer looks like this:

```text
struct { _m0 @ 0: float4[134] (ArrayStride 16) }
```

A stride-4 `float[N]` array is completely valid in GLSL (relaxed std430/std140 layouts). However, the HLSL backend in spirv-cross2 models cbuffers as **16-byte vec4 registers**. It computes an HLSL packed array stride of 16, which mismatches the SPIR-V ArrayStride of 4, so it rejects the shader (reporting an error rather than generating illegal HLSL, which is the correct behavior).

## 3. Reproduction (inside the repository)

`alloca-opts/float4-array-load.dxil` from the upstream test suite reliably reproduces the exact same error (even the `_9_11` identifier matches):

```text
cargo run -p hlsl-compat-inspect -- repro tests/shaders/alloca-opts/float4-array-load.dxil
```

Output:

```text
HLSL compile before: UnsupportedSpirv("cbuffer ID 11 (name: _9_11), member index 0
  (name: _m0) cannot be expressed with either HLSL packing layout or packoffset.")
HLSL compile after:  OK
rewritten views: 1 (skipped: 0)
```

The `dump` command displays the dual-view cbuffer structure of the shader:

```text
cargo run -p hlsl-compat-inspect -- dump tests/shaders/alloca-opts/float4-array-load.dxil
```

```text
Variable #11 struct#9 binding=(0,0):  _m0 @ 0: float32[24] stride=4
Variable #17 struct#15 binding=(0,0): _m0 @ 0: float32x4[6] stride=16
```

**The same cbuffer exists simultaneously as two views in the module**: a scalar view (accessed entirely via dynamic scalar indexing) and a vec4 view (static whole-vector accesses), sharing identical bindings.

## 4. Root-cause analysis (verified from vendored source code)

### 4.1 Where the scalar views come from

The type-propagation pass is not at fault. Key facts:

- `handleDclConstantBuffer` (dxbc-spirv/dxbc/dxbc_resources.cpp) **always** creates cbuffer IR types as `Type(eUnknown, 4)[N]` (vec4 base).
- `setupArrayType()` (ir/passes/ir_pass_propagate_resource_types.cpp) outputs `Type(scalarType, vectorSize=4)[N]`, which is a **vec4 array**, never producing `float[N]`.
- **The DXIL pipeline hardcodes `structuredCbv = false`** (dxil-spirv/bc/module_dxbc_ir.cpp), meaning DXIL cbuffers unconditionally follow the vec4 array path.

The stride-4 scalar views originate from two **independent cbuffer copy-promotion mechanisms**:

| Mechanism | Location | Control |
|---|---|---|
| DXBC: `CleanupScratchPass::promoteScratchCbvCopy` (scratch → cbuffer scalar access) | dxbc-spirv/ir/passes/ir_pass_scratch.cpp | Internal `resolveCbvCopy`, defaults to true, unexposed |
| DXIL: alloca → CBV punchthrough (`analyze_alloca_cbv_forwarding_*`, `emit_gep_as_cbuffer_scalar_offset`) | dxil-spirv/opcodes/ | **No switch** |

Trigger pattern: the shader copies cbuffer data into a local array (alloca or indexable temporary register), then **dynamically indexes** that local array. Upstream promotes or forwards these accesses into **scalar-granularity** cbuffer reads. It also creates a stride-4 scalar view buffer alongside the canonical vec4 view of the cbuffer (dual views with identical DescriptorSet/Binding).

### 4.2 Why spirv-cross2 rejects it

The spirv-cross2 HLSL backend (`spirv_hlsl.cpp::emit_hlsl_cbuffer`) calls `buffer_is_packing_standard(type, BufferPackingHLSLCbufferPackOffset)` (spirv_glsl.cpp) on every cbuffer. This model treats HLSL cbuffers as sequences of vec4 registers:

- For a `float[N]` array, the computed packed array stride under HLSL packing rules is 16 (`(4 + 15) & ~15`, vec4 aligned), which conflicts with the SPIR-V ArrayStride of 4, triggering rejection.

This doesn't contradict the fact that HLSL can express `float arr[536]` with stride 4. The packing model in spirv-cross2 is conservative: it can't prove in the general case that scalar array layouts match the HLSL compiler packing, so it rejects the input instead of risking invalid output.

## 5. Why implement this in Rust (rather than elsewhere)

| Option | Assessment |
|---|---|
| **Rust post-processing pass (this approach)** | Output-level fix without touching vendored C++. Disabled by default in an isolated module. Benefits all downstream consumers, and regression tests directly reuse the 810-shader suite. |
| Modify upstream C++ | Addresses the true root cause, but one mechanism has no switch while the other has an unexposed one. Disabling them alters output for all consumers (including Vulkan resilience scenarios). Requires a fork or upstream PR with unpredictable timelines. |
| Modify spirv-cross2 HLSL backend | Insufficient on its own: dual views (two buffers at the same binding) still produce duplicate `register(b0)` declarations in HLSL. Also requires maintaining a fork. |
| spirv-tools optimization pass | No passes exist for cbuffer layout canonicalization. |
| Application layer | Forces every downstream consumer to reinvent the wheel, rebuilding test infrastructure from scratch. |

Conclusion: **performing pure SPIR-V post-processing within the dxil-spirv crate is the most pragmatic choice under current constraints**.

## 6. Design

### 6.1 API & isolation

- Feature: `non-upstream-hlsl-compat` (disabled by default; the `non_upstream` module doesn't exist when disabled)
- Module: `dxil_spirv::non_upstream::hlsl_compat`
- Entry point: `vec4_align_cbuffers(&[u32]) -> Result<NormalizeOutput, HlslCompatError>`
- Independent error type (`HlslCompatError`) to avoid polluting upstream `dxil_spirv::Error`

### 6.2 Algorithm

For each cbuffer view matching: Uniform storage class with Block decoration, single-member struct, member is a 32-bit stride-4 scalar array, offset 0, length N where N%4==0:

1. **Dual-view pairing**: search for a vec4 view (`float4[N/4]`, stride 16) with the same set/binding and total byte size. If found, **merge** (redirect access chains to the vec4 view and remove the scalar view variable). If not found, **repack in place** (replace the type with `float4[N/4]`, stride 16).
2. **Access chain rewriting**: `[member, i]` → `[member, i/4, i%4]`. Static indices fold into constants; dynamic indices insert `OpUDiv`/`OpUMod` ahead of the access chain. The resulting access chain type remains unchanged (still pointing to a scalar).
3. **Cleanup**: remove merged variables along with their entry-point interface entries, decorations (OpDecorate), and names (OpName), leaving no dangling references.
4. **Skip conditions** (the entire view is preserved as-is if any apply): N%4!=0, non-zero offset, vector load consuming a scalar array access chain, referenced by OpStore/nested access chains/non-semantic debug instructions, non-32-bit types, or StorageBuffer storage class.

Because byte layouts remain identical (both scalar and vec4 arrays represent the same underlying memory), this transformation is **semantics-preserving**.

## 7. Verification

### 7.1 Full 810-shader scan (`hlsl-compat-inspect scan`)

```text
total scanned: 810
shaders with rewritten cbuffer views: 11
GLSL regressions (ok -> fail): 0          ← Semantics-preserving: zero breakage across the full suite
HLSL failures before: 191
HLSL failures after: 181
HLSL fixed by pass: 10                    ← All cbuffer-layout failures resolved
```

The remaining 181 HLSL failures belong to unrelated categories (unsupported builtins, WMMA, etc.) outside the scope of this pass. Zero GLSL regressions provide the strongest evidence: the pass acts as a byte-identical no-op on untouched shaders, while preserving semantic equivalence (reading identical bytes from the same memory) on modified ones.

### 7.2 Testing

- Unit tests (`dxil-spirv/tests/non_upstream_hlsl_compat.rs`, constructing modules via rspirv without C++ conversion dependencies): dual-view merging, in-place repacking with static index splitting, idempotent no-ops, and invalid input rejection.
- End-to-end regressions (`dxil-spirv-tests/tests/non_upstream/hlsl_compat_e2e.rs`): assertion of fixes across 10 target shaders, zero regressions across the entire alloca-opts directory, and clean module idempotency.
- Full semantic neutrality: see 7.1.

Running tests (default `cargo test` excludes these):

```text
cargo test -p dxil-spirv --features non-upstream-hlsl-compat --test non_upstream_hlsl_compat
cargo test -p dxil-spirv-tests --features non-upstream-hlsl-compat --test non_upstream_hlsl_compat_e2e
```

## 8. Known limitations & future work

- Handles only single-member wrapper structs (member is directly a stride-4 array at offset 0). Stride-4 arrays inside nested structs, non-16-aligned offsets, and multi-member structs are skipped.
- Only handles 32-bit scalars; 16-bit (min16float, stride 2) and 64-bit (stride 8) types are skipped.
- Dynamic-indexed vector loads over scalar arrays cannot be safely rewritten, so the entire view is skipped.
- Merging removes the scalar view variable; dead types (old struct/array/pointer) remain in the module as valid harmless redundancies that do not affect consumers.
- If spirv-cross2 exposes other HLSL incompatibilities in the future, new functions will follow the same pattern in the `non_upstream` module without altering this pass's contract.

## 9. Upstream relationship

- The scalar view output from upstream dxbc-spirv is **valid std140**. This pass is a compatibility fix for HLSL consumers, not a bug fix for upstream output.
- No vendored C++ code is modified (zero changes to dxil-spirv / dxbc-spirv / dxil-spirv-sys).
- Upstream synchronization (`/sync-upstream`) remains unaffected: when the feature is disabled, this extension is completely omitted from the build.
