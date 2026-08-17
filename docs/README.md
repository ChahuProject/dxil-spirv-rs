# Documentation

This is the documentation hub for **dxil-spirv-rs** — safe Rust bindings to
[dxil-spirv](https://github.com/HansKristian-Work/dxil-spirv) (DXIL/DXBC → SPIR-V).

[English](README.md) | [中文](README.zh-CN.md)

## Document map

| Document | Audience | What it covers |
|---|---|---|
| [usage.md](usage.md) | **Users** — people who consume the crate | Install, quick start, full API walkthrough, remapper configuration, error handling, limitations |
| [platform-support.md](platform-support.md) | Users + maintainers | Supported OS/arch matrix, build requirements, the DXC test-harness caveat |
| [architecture.md](architecture.md) | **Developers** — people who modify the crate | `-sys`/safe-layer split, FFI boundaries, build.rs link closure, bindgen rules, cross-platform gotchas (a.k.a. the "pitfall ledger") |
| [testing.md](testing.md) | Developers | Test suite architecture, the 829-shader coverage statement, category breakdown, regression baseline mechanics |
| [ci.md](ci.md) | Developers | CI jobs, platform strategy, DXC handling, caching policy and the pitfalls that shaped it |
| [contributing.md](contributing.md) | Contributors | How to contribute, code conventions, the AI-maintenance policy, the documentation standards defined below |
| [changelog.md](changelog.md) | Everyone | What changed and why — the project's evolution from first commit to 100% test coverage |
| [non-upstream/hlsl-compat-rationale.md](non-upstream/hlsl-compat-rationale.md) | Developers | Design rationale for the `non-upstream-hlsl-compat` extension: why upstream's stride-4 scalar cbuffer output is rejected by the spirv-cross2 HLSL backend, and how the vec4-alignment post-processing pass fixes it |

## Documentation standards (how to share docs)

Everyone — human or AI — must follow these rules when adding or editing
project documentation. They keep the docs findable, reviewable, and
non-conflicting.

### Location & naming

1. **All project documentation lives in `docs/`.** No top-level `*.md` files
   other than `README.md` (the front door). Keep documentation out of source
   directories — code comments describe code, `docs/` describes the project.
2. **One topic, one file.** Split by audience and concern (see the map above).
   If a new topic appears, add a new `docs/<topic>.md`; if a file grows beyond
   ~400 lines, split it.
3. **Kebab-case filenames, no trailing numbers** — `platform-support.md`, not
   `platform_support.md` or `support_v2.md`. Old files are *deleted* or
   renamed, never duplicated "v2".
4. **Images and assets go in `docs/assets/`**, referenced with relative paths.
   No binary files in the repo root, no absolute paths in links.
5. **Every `docs/*.md` must be reachable from this index.** The index is the
   entry point; if a document isn't listed here, it isn't discoverable.
6. **Cross-references use relative paths** (`[testing.md](testing.md)`), not
   absolute URLs, so the docs work on GitHub, crates.io, and offline.

### Bilingual conventions

7. **Every human-facing document ships in both English and Chinese**: English `<name>.md`, Chinese `<name>.zh-CN.md`. Skill files (`.agents/skills/`) are exempt.
8. **Both versions carry a language switcher at the top**: a line shaped like `[English](...) | [中文](...)`, with the English file pointing at the matching `.zh-CN.md` file and the Chinese file pointing at the matching English file. Translations must stay in sync with the English version — change the English, change the Chinese, and vice versa.
9. **Translation rules**: code blocks, commands, paths, commit hashes, and API names stay verbatim; technical terms (DXIL, SPIR-V, bindgen, CMake, Cargo, FFI, RAII) remain in English; prose is translated into Simplified Chinese.

### Content rules

10. **Know your audience.** A doc is either *for users* or *for developers* —
    write to one, and link to the other. `usage.md` never explains how the C++
    core is linked; `architecture.md` never re-explains how to add the
    dependency.
11. **State coverage facts with numbers.** Claims like "all upstream tests are
    covered" must carry the evidence: *829/829 shaders, enforced by
    `test_completeness_check`*. If a number goes stale, update it — stale
    numbers are worse than no numbers.
12. **Code examples must be runnable.** Every snippet should be copy-pasteable
    from a fresh crate. Prefer `no_run` doctests in the crate over prose-only
    snippets.
13. **Document pitfalls where they bite.** Cross-platform gotchas (link flags,
    bindgen signedness, DXC availability) belong in the doc that explains the
    affected subsystem — see the pitfall ledger in
    [architecture.md](architecture.md) and the CI history in
    [ci.md](ci.md). Never delete a pitfall entry without a reason; they are
    paid-for lessons.
14. **Changelog entries are factual, not promotional.** State what changed and
    why; reference the commit that introduced it. No AI-flavored filler.

### AI-maintenance policy (applies to docs too)

15. This project is **AI-maintained**: AI-generated and AI-edited code and
    documentation are explicitly welcome (see
    [contributing.md](contributing.md) for the full policy). AI contributors
    must follow the exact same standards as humans — including this document.
16. AI-edited docs must state the factual basis for numbers and claims
    (e.g. "verified by running `cargo test` on commit X"). A claim without a
    verification path is a defect, not an opinion.

## Quick orientation

```
README.md                 ← front door (30-second overview + links)
docs/                     ← everything else, organized by audience
  usage.md                ← for users
  platform-support.md     ← for users + maintainers
  architecture.md         ← for developers
  testing.md              ← for developers
  ci.md                   ← for developers
  contributing.md         ← for contributors
  changelog.md            ← for everyone
  non-upstream/           ← non-upstream extension rationale
    hlsl-compat-rationale.md
.agents/skills/           ← AI maintenance skills (reference these docs)
```
