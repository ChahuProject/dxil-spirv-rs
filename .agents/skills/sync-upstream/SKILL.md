---
description: Synchronize dxil-spirv-rs with upstream dxil-spirv and keep the safe wrapper correct across upstream updates. Use when upgrading the vendored dxil-spirv submodule, adding newly exposed C API surface to the safe wrapper, fixing build/link/bindgen issues, or reviewing the crate for FFI safety. Triggers - sync upstream, update dxil-spirv, upgrade bindings, add new dxil_spv_ functions, refresh bindings, dxil-spirv build broken.
---

# Sync dxil-spirv-rs with upstream & keep it correct

This skill lets an agent autonomously update or complete the `dxil-spirv` Rust
bindings **without breaking the build**, by combining a fixed workflow, a hard
acceptance gate, and the project documentation as the source of truth.

This skill is the **operating manual** (what to do, step by step). The
**knowledge base** (why things are the way they are — FFI rules, link closure,
pitfalls) lives in the project docs and is referenced below. If this skill and
a doc disagree, the doc wins — update the doc.

## Required reading (before any sync work)

Read these before making changes; they encode the verified facts:

1. `docs/architecture.md` — crate topology, build pipeline, static link
   closure (9 libraries, dependent-before-dependency), FFI boundary rules,
   callback trampoline pattern, Send/Sync policy, CRT/exceptions per platform,
   C++ stdlib linking, the **cross-platform pitfall ledger** (paid-for lessons),
   experimental API surface, versioning rule.
2. `docs/testing.md` — the e2e suite: coverage guarantee (829/829 shaders),
   test data flow, regression baseline mechanics, DXC toolchain and the
   `dxc_unavailable` cfg, how to add shaders safely.
3. `docs/ci.md` — CI jobs and the platform strategy (Windows runs the full
   suite; Linux/macOS skip shader tests because DXC is a Windows binary).
4. `docs/contributing.md` — acceptance gate and code conventions (edition
   2024, rustfmt rules incl. the empty guard files, unsafe policy).

## Authoritative sources

1. **Upstream C library** — `HansKristian-Work/dxil-spirv`. Source of truth
   for the C API. (The build consumes the submodule at
   `dxil-spirv-sys/dxil-spirv`.)
2. **Reference binding crates** — `grovesNL/spirv_cross` (classic template for
   build.rs / bindgen / safe-wrapper structure) and
   `SnowflakePowered/spirv-cross2-rs` (actively maintained modern successor:
   `-sys` + safe-layer split, `Handle<T>` instance tagging, Arc-guarded
   context lifetimes, `+SHORTSHA` upstream-pinned versioning). See
   `docs/architecture.md` §2 for how their patterns apply here.

### Reference clones live INSIDE this skill (on demand)

The reference repos are NOT git submodules and are NOT part of a normal
clone — a user who only wants to *use* the crate should not download hundreds
of MB of reference source. They are cloned **on demand by this skill** into
the skill's own directory, which is git-ignored:

```
.agents/skills/sync-upstream/references/dxil-spirv
.agents/skills/sync-upstream/references/spirv_cross
.agents/skills/sync-upstream/references/spirv-cross2-rs
```

These clones are **read-only mirrors**, never build inputs. The build only
consumes `dxil-spirv-sys/dxil-spirv`.

## Autonomy rule — never stop to ask the user mid-run

Resolve design questions by consulting the docs above and the reference repos,
NOT by asking the user. Only pause for genuinely irreversible, unprecedented
public-API breaks. Everything else: pick the doc-aligned option and proceed.

---

## Acceptance gate (Definition of Done)

A change is NOT complete until ALL of the following are green. If any is red,
keep fixing until it passes — do not report "done" with a failing build.

```powershell
cargo fmt --all -- --check                # formatting (rustfmt, edition 2024)
cargo clippy --workspace --all-targets -- -D warnings   # no lint
cargo build --workspace --all-targets     # full build (incl. C++ via CMake)
cargo test --workspace                    # unit + layout + doctest + e2e
```

**`cargo build` passing is NOT enough.** Static-link and CRT errors only
surface at final link time, which `cargo build` of an rlib skips — always run
`cargo test` to force a real link.

**Platform caveat**: the e2e shader suite (`dxil-spirv-tests`) only runs where
DXC is runnable (Windows). On Linux/macOS the suite self-skips via the
`dxc_unavailable` cfg — that is expected, not a failure (see
`docs/ci.md` §Platform Strategy). The unit/layout/doctest legs still run.

---

## Workflow

### Step 0 — Prepare reference clones (first run, or to refresh)

Discover or create the on-demand clones inside this skill directory, pinned to
deterministic commits. Do NOT clone into the crate root and do NOT commit them.

```powershell
$skill = "<repo-root>/.agents/skills/sync-upstream"
$refs  = "$skill/references"
New-Item -ItemType Directory -Force $refs | Out-Null

# Upstream dxil-spirv: rolling master, no tags. Pin to the SAME commit as the
# build submodule so the reference matches what we compile against.
$buildCommit = git -C <repo-root> submodule status dxil-spirv-sys/dxil-spirv | ForEach-Object { ($_ -split '\s+')[1] }
if (-not (Test-Path "$refs/dxil-spirv")) {
    git clone https://github.com/HansKristian-Work/dxil-spirv.git "$refs/dxil-spirv"
}
git -C "$refs/dxil-spirv" fetch --all
git -C "$refs/dxil-spirv" checkout $buildCommit
git -C "$refs/dxil-spirv" submodule update --init --recursive

# spirv_cross binding template: pin to the commit matching the crate version
# we mirror (or a known-good commit).
if (-not (Test-Path "$refs/spirv_cross")) {
    git clone https://github.com/grovesNL/spirv_cross.git "$refs/spirv_cross"
}
git -C "$refs/spirv_cross" submodule update --init --recursive

# spirv-cross2-rs: modern reference for soundness patterns (Handle<T>,
# Arc-guarded context, +SHORTSHA versioning).
if (-not (Test-Path "$refs/spirv-cross2-rs")) {
    git clone https://github.com/SnowflakePowered/spirv-cross2-rs.git "$refs/spirv-cross2-rs"
}
git -C "$refs/spirv-cross2-rs" submodule update --init --recursive
```

If a clone already exists, `fetch` and re-checkout the pinned commit; do not
leave it on an arbitrary newer master.

### Step 1 — Index with codegraph

Idempotent; run once per session (after Step 0):

```powershell
codegraph init <repo-root>
codegraph init "$refs/dxil-spirv"
codegraph init "$refs/spirv_cross"
codegraph init "$refs/spirv-cross2-rs"
```

Use `codegraph explore "<symbol>"` for cross-repo questions (e.g. "which
`dxil_spv_converter_*` are not yet wrapped in `converter.rs`").

### Step 2 — Determine the target upstream version

- Upstream dxil-spirv uses a rolling master with **no git tags**. The
  "released version" is the commit pinned in `dxil-spirv-sys/dxil-spirv`.
- The upstream **C API is explicitly kept ABI/API stable** (see the upstream
  README). The version macros
  `DXIL_SPV_API_VERSION_MAJOR/MINOR/PATCH` (currently `2.72.1`) in
  `dxil_spirv_c.h` are the authoritative compatibility signal.
- To upgrade: checkout the new commit in `dxil-spirv-sys/dxil-spirv`, then
  mirror the SAME commit into the skill's reference clone (Step 0).

### Step 3 — Post-upgrade review checklist

Run this EVERY time the upstream commit changes. Check the diff of
`dxil_spirv_c.h` and the CMake files:

1. **Version macros** — did `DXIL_SPV_API_VERSION_*` or any
   `*_INTERFACE_VERSION` change? (→ update the crate version, Step 4 of
   versioning below)
2. **Enums** — new variants in `dxil_spv_shader_stage` / `dxil_spv_resource_kind`
   / `dxil_spv_result` / option enums? Add them to the Rust enums.
3. **Structs** — new fields in options / binding structs? Bindgen picks them
   up; verify safe-layer constructors still initialize everything.
4. **Functions / callbacks** — new `DXIL_SPV_PUBLIC_API` exports or changed
   remapper prototypes? Wrap them (and keep `tests/api_coverage.rs` green).
5. **CMake targets** — new/removed `add_library` / changed
   `target_link_libraries`? Update the link list in `build.rs` (see
   `docs/architecture.md` §2 Static Link Closure).
6. **Compile flags** — changed `DXIL_SPV_CXX_FLAGS` or feature macros
   (`AMD_EXTENSIONS`, `HAVE_LLVMBC`, `DXBC_SPV_ENABLE_SM5`)? Mirror in
   build.rs (see `docs/architecture.md` §9 for the switch checklist).
7. **Crate version sync** — after any upstream bump, update the version in the
   workspace `Cargo.toml` per the versioning rule below.

### Step 4 — Sync the bindings

- **sys layer**: bindgen allowlists `dxil_spv_.*`, so new functions appear
  automatically; confirm with `cargo build -p dxil-spirv-sys`.
- **safe layer**: extend `dxil-spirv/src/` following the existing
  `ParsedBlob` / `Converter` RAII pattern and `docs/architecture.md`
  (§3 FFI boundaries, §4 callback trampoline, §5 Send/Sync policy).

### Step 5 — Verify against the acceptance gate

Run the full gate (fmt + clippy + build + test). Fix until green. **Do not
mark the task complete, and do not commit, while any leg is red.**

### Step 6 — Update the docs and skill

The project docs are the single source of truth and must not drift:

1. New option/binding/remapper surface → check `docs/usage.md` covers it.
2. Changed build/link/FFI behavior → update `docs/architecture.md`
   (pitfall ledger especially — add a new entry with symptom → root cause →
   fix, and the fixing commit).
3. New/changed shader test infrastructure → update `docs/testing.md`.
4. CI changes → update `docs/ci.md`.
5. User-visible change → add an entry to `docs/changelog.md`.
6. If the knowledge base in `docs/` was wrong or incomplete, fix the DOC —
   do not accumulate facts in this skill file.

---

## Versioning rule

The crate version lives in the workspace `Cargo.toml` (`[workspace.package]`)
and uses semver **build metadata** to advertise the vendored upstream C API
version:

```text
<crate-version>+dxil-spirv.<UPSTREAM_MAJOR.MINOR.PATCH>
e.g.  0.1.0+dxil-spirv.2.72.1
```

- `+dxil-spirv.X.Y.Z` MUST always equal the `DXIL_SPV_API_VERSION_*` macros of
  the pinned upstream submodule. crates.io accepts and ignores the `+...`
  suffix, so it never affects version resolution — it is purely informational.
- When the upstream commit bump changes `DXIL_SPV_API_VERSION_*`, update the
  suffix to match.
- Bump the leading crate version (`0.1.0` part) independently, by normal
  semver rules for the Rust API: breaking safe-API change → minor/major bump;
  backward-compatible addition or upstream-only refresh → patch bump.
- After editing the version, re-run the acceptance gate and confirm
  `cargo publish --dry-run` still packages cleanly (version string validity is
  checked there).

---

## Hard rules

- NEVER commit the skill's `references/` clones (they are git-ignored).
- NEVER point the build at any reference clone; only `dxil-spirv-sys/dxil-spirv`.
- Keep the public safe API backward-compatible within a minor version bump.
- ALWAYS keep the version's `+dxil-spirv.X.Y.Z` suffix in sync with the pinned
  upstream `DXIL_SPV_API_VERSION_*` (see the Versioning rule).
- Do NOT skip the acceptance gate, and do NOT report completion on a red build.
  `cargo test` is mandatory — it is the only leg that forces a real link.
- Do NOT stop to ask the user for decisions the docs or a reference repo can
  answer.
- Do NOT add new knowledge to this file. Knowledge lives in `docs/`; this file
  only points at it. If a doc is missing the fact you need, add it to the doc.
