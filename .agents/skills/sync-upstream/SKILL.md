---
description: Synchronize dxil-spirv-rs with upstream dxil-spirv and align binding patterns with reference Rust binding crates (spirv_cross, spirv-to-dxil-rs). Use when upgrading the vendored dxil-spirv submodule, adding newly exposed C API surface to the safe wrapper, or rebuilding the crate from scratch against a fresh upstream. Triggers - sync upstream, update dxil-spirv, upgrade bindings, add new dxil_spv_ functions, refresh bindings.
---

# Sync dxil-spirv-rs with upstream & reference bindings

This skill guides an agent to autonomously update or complete the `dxil-spirv`
Rust bindings by consulting two authoritative sources:

1. **Upstream C library** — `HansKristian-Work/dxil-spirv` (the source of truth
   for the C API surface).
2. **Reference binding crates** — existing, published Rust binding projects
   whose structure and `build.rs` patterns we mirror.

## Reference repositories (git submodules under `references/`)

| Path | Repo | Role |
|---|---|---|
| `references/dxil-spirv` | `HansKristian-Work/dxil-spirv` | Upstream C API (`dxil_spirv_c.h`), CMake targets |
| `references/spirv_cross` | `grovesNL/spirv_cross` | Binding-structure template (cc/bindgen/feature layout) |

These are **reference material only** — they are NOT build inputs and MUST NOT
be linked or compiled. The actual build consumes the vendored copy at
`dxil-spirv-sys/dxil-spirv/`.

## Prerequisites

- `codegraph` CLI on PATH (check with `codegraph --version`). If missing, tell
  the user to install it; do NOT attempt a global install yourself.
- All submodules initialized: `git submodule update --init --recursive`.

## Step 1 — Index with codegraph

Run once per session (idempotent):

```powershell
codegraph init <repo-root>                          # this crate
codegraph init references/dxil-spirv               # upstream C API
codegraph init references/spirv_cross              # reference binding
```

Use `codegraph explore "<symbol>"` to answer cross-repo questions such as
"which `dxil_spv_converter_*` functions exist upstream but are not yet wrapped
in `dxil-spirv/src/converter.rs`".

## Step 2 — Determine the target upstream version

- The vendored submodule `dxil-spirv-sys/dxil-spirv` is pinned to a specific
  commit (see `git submodule status`).
- Upstream dxil-spirv does NOT publish git tags; it follows a rolling-master
  model. The "released version" is therefore the commit recorded in the
  submodule.
- To upgrade: `cd dxil-spirv-sys/dxil-spirv && git fetch && git checkout <new-commit>`,
  then mirror the same commit into `references/dxil-spirv` so the reference
  copy matches what we build against.

## Step 3 — Sync the bindings

1. Diff the upstream C header between the old and new commit:
   `git diff <old>..<new> -- dxil_spirv_c.h`.
2. For every new/changed `dxil_spv_*` function or enum, decide:
   - **sys layer** — bindgen picks it up automatically; verify with
     `cargo build -p dxil-spirv-sys`.
   - **safe layer** — add/extend the RAII wrapper in `dxil-spirv/src/`
     following the existing `ParsedBlob`/`Converter` pattern.
3. Mirror build-system changes from `references/spirv_cross` (e.g. new feature
   flags, MSVC flag tweaks, bindgen config changes).

## Step 4 — Verify

```powershell
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

All three must pass before committing.

## Hard rules

- NEVER commit changes inside `references/` — those are read-only mirrors.
- NEVER point the build at `references/`; the build must only use
  `dxil-spirv-sys/dxil-spirv`.
- Keep the public safe API backward-compatible within a minor version bump.
