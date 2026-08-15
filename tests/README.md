# End-to-End Test Suite

This directory contains the end-to-end test infrastructure for `dxil-spirv-rs`.
It validates that our safe Rust wrapper produces correct SPIR-V output for all
upstream test shaders.

## Architecture

```
tests/
├── shaders/              # HLSL sources + DXC-compiled .dxil (git-ignored)
├── reference/shaders/    # Upstream reference GLSL outputs (git-ignored)
└── shaders_manifest.txt  # Auto-generated completeness manifest (git-ignored)

dxil-spirv-tests/
├── build.rs              # Syncs shaders, downloads DXC, compiles DXIL
├── tests/
│   ├── e2e.rs            # Test entry points (completeness, smoke, categories)
│   └── harness.rs        # Test driver + converter configuration + remappers
└── Cargo.toml            # dev-dependencies: spirv-cross2, md5
```

## How It Works

1. **Build time** (`dxil-spirv-tests/build.rs`):
   - Syncs all 842 shader sources from `dxil-spirv-sys/dxil-spirv/shaders/`
   - Downloads DXC 1.9.2602.17 (production SM6.9 support) to `target/dxc/`
   - Compiles all `.vert`/`.frag`/`.comp`/... to `.dxil` using DXC
   - Generates `tests/shaders_manifest.txt` for the completeness check

2. **Test time** (`cargo test`):
   - Each shader runs in a **fresh subprocess** (crash isolation)
   - DXIL → `dxil_spirv::convert_to_spirv()` → SPIR-V validation
   - SPIR-V → `spirv-cross2` → GLSL compilation check
   - Results classified: `Pass` / `Fail` / `KnownFailure` / `Skip`

## Running Tests

```bash
# All tests (includes completeness check + categories)
cargo test -p dxil-spirv-tests

# Just the completeness check (fast)
cargo test -p dxil-spirv-tests test_completeness_check

# Specific category
cargo test -p dxil-spirv-tests test_stages

# Smoke test (3 simple shaders)
cargo test -p dxil-spirv-tests test_smoke

# Global metrics report (all 829 shaders)
cargo test -p dxil-spirv-tests test_metrics_report

# Strict GLSL MD5 comparison against upstream reference
DXIL_SPIRV_STRICT_GLSL=1 cargo test -p dxil-spirv-tests
```

## Detection Mechanisms

| Layer | What it detects | How |
|-------|----------------|-----|
| **Completeness** | Missing tests for upstream shaders | `test_completeness_check` hard-fails |
| **Crash isolation** | C++ assertion/abort in converter | Subprocess per shader, crash = `Fail` |
| **SPIR-V validity** | Corrupt/empty output | Magic number + length check |
| **GLSL compilability** | SPIR-V that SPIRV-Cross can't consume | `spirv-cross2` compile check |
| **Regression** | Previously-passing shader now fails | `test_metrics_report` hard-fails on `Fail` |
| **Known failures** | Tracked but allowed gaps | `requires_complex_remapper()` classification |

## Known Failures

Currently **279/829 (33.7%)** shaders are classified as `KnownFailure` because
they need complex per-shader remapper configuration that the upstream CLI
provides via command-line arguments:

| Category | Count | Reason |
|----------|-------|--------|
| `.bindless.` | ~60 | Needs per-shader heap mapping |
| `.root-descriptor.` | ~40 | Needs BDA root descriptor table |
| `.root-constant.` | ~30 | Needs push constant mapping |
| `.local-root-signature.` | ~40 | Needs local root descriptor table |
| `.ssbo.` (non-sm66) | ~20 | Needs SSBO default descriptor type |
| `.sm66.` + `.ssbo.` | ~89 | Needs SM6.6 descriptor heap config |

These are **not bugs in our wrapper** — they require the full remapper
configuration that upstream `test_shaders.py` passes as CLI arguments. Our
safe API exposes all the necessary remapper callbacks; the test harness just
needs a config-file-driven approach to populate them per-shader.

### Reducing Known Failures

To convert a `KnownFailure` to `Pass`:

1. Identify the required remapper configuration from upstream `test_shaders.py`
2. Add the configuration to `setup_remappers()` in `harness.rs`
3. Remove the marker from `requires_complex_remapper()`
4. Verify the shader passes

## Upstream Sync

When upstream `dxil-spirv` updates its test shaders:

1. `test_completeness_check` will fail (new shader detected, no test)
2. Run `cargo build -p dxil-spirv-tests` — build.rs syncs new shaders
3. New shaders are compiled to DXIL automatically
4. Run tests — new shaders may pass, fail, or need `KnownFailure` classification
5. Commit updated `harness.rs` if remapper logic needs adjustment

## Environment

| Dependency | Version | Purpose |
|-----------|---------|---------|
| DXC | 1.9.2602.17 | HLSL → DXIL compilation |
| spirv-cross2 | 0.7.1 | SPIR-V → GLSL validation |
| Rust | 1.70+ | Test harness |

DXC is auto-downloaded to `target/dxc/` if not found in PATH. The download
is cached and only fetched once per workspace.

## Troubleshooting

**"No precompiled .dxil available"**
→ Run `cargo build -p dxil-spirv-tests` first. The build script compiles shaders.

**"DXC not found"**
→ Install DXC 1.9+ or let build.rs download it automatically.

**"child process crashed"**
→ A shader triggered a C++ assertion. This is recorded as `Fail`, not a test abort.

**"GLSL MD5 mismatch" (strict mode)**
→ Our GLSL output differs from upstream reference. Usually benign formatting;
check if the SPIR-V is functionally correct first.
