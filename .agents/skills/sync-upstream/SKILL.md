---
description: Synchronize dxil-spirv-rs with upstream dxil-spirv and keep the safe wrapper correct across upstream updates. Use when upgrading the vendored dxil-spirv submodule, adding newly exposed C API surface to the safe wrapper, fixing build/link/bindgen issues, or reviewing the crate for FFI safety. Triggers - sync upstream, update dxil-spirv, upgrade bindings, add new dxil_spv_ functions, refresh bindings, dxil-spirv build broken.
---

# Sync dxil-spirv-rs with upstream & keep it correct

This skill lets an agent autonomously update or complete the `dxil-spirv` Rust
bindings **without breaking the build**, by combining a fixed workflow, a hard
acceptance gate, and a built-in knowledge base of verified facts about the
upstream C library and mature binding patterns.

## Authoritative sources

1. **Upstream C library** — `references/dxil-spirv` (mirrors the build
   submodule `dxil-spirv-sys/dxil-spirv`). Source of truth for the C API.
2. **Reference binding crate** — `references/spirv_cross` (`grovesNL/spirv_cross`).
   Template for build.rs / bindgen / safe-wrapper structure.

`references/*` are **read-only mirrors**, never build inputs. The build only
consumes `dxil-spirv-sys/dxil-spirv`.

## Autonomy rule — never stop to ask the user mid-run

Resolve design questions by consulting the reference repos and the knowledge
base below, NOT by asking the user. Only pause for genuinely irreversible,
unprecedented public-API breaks. Everything else: pick the reference-aligned
option and proceed.

---

## Acceptance gate (Definition of Done)

A change is NOT complete until ALL of the following are green. If any is red,
keep fixing until it passes — do not report "done" with a failing build.

```powershell
cargo build --workspace                      # compiles
cargo clippy --workspace -- -D warnings      # no lint (warnings = errors)
cargo test --workspace                       # unit + layout + doctest pass
```

**`cargo build` passing is NOT enough.** Static-link and CRT errors only
surface at final link time, which `cargo build` of an rlib skips — always run
`cargo test` to force a real link.

---

## Workflow

### Step 1 — Index with codegraph

Idempotent; run once per session:

```powershell
codegraph init <repo-root>
codegraph init references/dxil-spirv
codegraph init references/spirv_cross
```

Use `codegraph explore "<symbol>"` for cross-repo questions (e.g. "which
`dxil_spv_converter_*` are not yet wrapped in `converter.rs`").

### Step 2 — Determine the target upstream version

- Upstream dxil-spirv uses a rolling master with **no git tags**. The
  "released version" is the commit pinned in `dxil-spirv-sys/dxil-spirv`.
- The upstream **C API is explicitly kept ABI/API stable** (see
  `references/dxil-spirv/README.md`: "Only the C API is installed and is
  expected to be kept ABI/API stable when it releases."). The version macros
  `DXIL_SPV_API_VERSION_MAJOR/MINOR/PATCH` (currently `2.72.1`) in
  `dxil_spirv_c.h` are the authoritative compatibility signal.
- To upgrade: checkout the new commit in `dxil-spirv-sys/dxil-spirv`, then
  mirror the SAME commit into `references/dxil-spirv`.

### Step 3 — Post-upgrade review checklist

Run this EVERY time the upstream commit changes. Check the diff of
`dxil_spirv_c.h` and the CMake files:

1. **Version macros** — did `DXIL_SPV_API_VERSION_*` or any
   `*_INTERFACE_VERSION` change?
2. **Enums** — new variants in `dxil_spv_shader_stage` / `dxil_spv_resource_kind`
   / `dxil_spv_result` / option enums? Add them to the Rust enums.
3. **Structs** — new fields in options / binding structs? Bindgen picks them
   up; verify safe-layer constructors still initialize everything.
4. **Functions / callbacks** — new `DXIL_SPV_PUBLIC_API` exports or changed
   remapper prototypes? Wrap them.
5. **CMake targets** — new/removed `add_library` / changed
   `target_link_libraries`? Update the link list in `build.rs` (see KB-4).
6. **Compile flags** — changed `DXIL_SPV_CXX_FLAGS` or feature macros
   (`AMD_EXTENSIONS`, `HAVE_LLVMBC`, `DXBC_SPV_ENABLE_SM5`)? Mirror in build.rs.

### Step 4 — Sync the bindings

- **sys layer**: bindgen allowlists `dxil_spv_.*`, so new functions appear
  automatically; confirm with `cargo build -p dxil-spirv-sys`.
- **safe layer**: extend `dxil-spirv/src/` following the existing
  `ParsedBlob` / `Converter` RAII pattern and the knowledge base below.

### Step 5 — Verify against the acceptance gate

Run the full gate. Fix until green.

---

## Knowledge base (verified facts — do not rediscover)

### KB-1 · bindgen boundaries

- **Casted `#define` macros are NOT emitted.** `DXIL_SPV_TRUE`/`DXIL_SPV_FALSE`
  are `((dxil_spv_bool)1)`/`((dxil_spv_bool)0)`; bindgen skips them. Use the
  literal `1` / `0` (type is `dxil_spv_bool = c_uchar`) — never reference
  `sys::DXIL_SPV_TRUE`.
- **Anonymous unions cannot be built with a struct literal.** Use
  `Default::default()` then assign fields; annotate the `From` impl with
  `#[allow(clippy::field_reassign_with_default)]` and a comment.
- spirv_cross disables layout tests (`layout_tests(false)`) to avoid fragile
  cross-platform assertions; we keep bindgen's generated layout tests because
  our struct set is stable — if an upstream struct changes ABI, those tests
  are the early warning. Do not blindly disable them.

### KB-2 · FFI callback / userdata pattern (no spirv_cross precedent)

`spirv_cross` has NO callback API, so do not look there for trampoline help.
Use the pattern already proven in `dxil-spirv/src/remapper.rs`:

- A `Box<dyn FnMut…>` is a **fat pointer** and cannot be cast to `*mut c_void`.
  **Double-box**: store `Box<Box<dyn FnMut…>>`, hand C a thin pointer to the
  outer box, deref twice in the trampoline.
- The converter **owns** the closure (stored in a holder struct); `userdata`
  is a re-borrow valid while the holder is alive. Never `Box::into_raw` and
  then also keep a Rust `Box` to the same pointer (double free).
- Wrap the call in `std::panic::catch_unwind`; on panic return the C "failure"
  value so unwinding never crosses the FFI boundary.
- Keep-alive companions (e.g. a `Vec<u32>` swizzle table or `CString` path
  whose pointer is stored in the raw option struct) live in the same enum and
  are never read back — mark the enum `#[allow(dead_code)]` with a comment.

### KB-3 · unsafe / Send / Sync policy

- Upstream conversion is **single-threaded and synchronous**: remapper
  callbacks fire only during `dxil_spv_converter_run`, on the calling thread,
  with no background worker threads and no concurrent re-entry.
- Therefore `unsafe impl Send for Converter` / `ParsedBlob` is sound (the
  handle may move across threads), but do NOT add `Sync` (concurrent `run()`
  on one converter is not safe). Callbacks only need `Send`, not `Sync`.
- spirv_cross deliberately leaves its raw-pointer compiler `!Send`; our
  `Send` is a deliberate, justified divergence — keep the rationale comment.

### KB-4 · Static link closure (MSVC)

`dxil-spirv-c-static` needs exactly these 9 static libs, in this
dependent-before-dependency order (SPIRV-Tools / SPIRV-Cross are CLI-only and
NOT in the closure; `dxil-spirv-headers` is INTERFACE-only, no `.lib`):

```text
dxil-spirv-c-static  dxil-converter  spirv-module  glslang-spirv-builder
llvm-bc  bc-decoder  dxbc-spirv  dxil-utils  dxil-debug
```

If the linker reports unresolved `LLVMBC::*` / `spv::Builder::*` symbols, a
library is missing or mis-ordered here.

### KB-5 · CRT / build-type (MSVC)

- Upstream does NOT set `CMAKE_MSVC_RUNTIME_LIBRARY` or hardcode `/MT`/`/MD`.
- `build.rs` MUST set `CMAKE_MSVC_RUNTIME_LIBRARY` to
  `MultiThreaded$<$<CONFIG:Debug>:Debug>DLL` and align `CMAKE_BUILD_TYPE` /
  `.profile()` with the Rust `PROFILE` (debug→Debug, else Release).
- Mixing a Release C++ lib with a Debug Rust link yields unresolved
  `_CrtDbgReport` / `_calloc_dbg`. If those appear, this is the cause.

### KB-6 · Exceptions / RTTI

- Upstream GCC/Clang flags use `-fno-exceptions -fno-rtti`, but the core
  library uses no `try`/`catch`/`throw` and no native RTTI (it has its own
  LLVM-style `isa<>`/`dyn_cast<>`).
- On MSVC we pass `/EHsc` to silence STL `C4530` and match Rust C++ build
  convention. It is benign; no upstream conflict.

### KB-7 · build.rs structure

- We compile upstream via the `cmake` crate (upstream is a CMake project),
  unlike spirv_cross which uses `cc` on vendored sources. Keep `cmake`.
- spirv_cross generates bindings OFFLINE and commits them (avoiding a libclang
  runtime dependency for downstream builds). We currently run bindgen in
  build.rs; if downstream build ergonomics become a concern, consider switching
  to the offline+committed model as a future improvement (not required now).

---

## Hard rules

- NEVER commit changes inside `references/`.
- NEVER point the build at `references/`; only `dxil-spirv-sys/dxil-spirv`.
- Keep the public safe API backward-compatible within a minor version bump.
- Do NOT skip the acceptance gate, and do NOT report completion on a red build.
- Do NOT stop to ask the user for decisions the knowledge base or a reference
  repo can answer.
