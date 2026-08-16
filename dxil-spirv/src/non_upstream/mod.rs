//! NON-UPSTREAM EXTENSION
//!
//! Everything under this module is functionality **added by dxil-spirv-rs** on
//! top of the vendored upstream dxil-spirv C++ library. None of it exists in,
//! or is endorsed by, the upstream project.
//!
//! This module is only compiled when the crate feature
//! `non-upstream-hlsl-compat` is enabled. It is disabled by default; users who
//! do not opt in never see these APIs.
//!
//! The design rationale, reproduction steps, and validation for each
//! extension live in [`docs/non-upstream/`](https://github.com/ChahuProject/dxil-spirv-rs/tree/main/docs/non-upstream).

pub mod hlsl_compat;
