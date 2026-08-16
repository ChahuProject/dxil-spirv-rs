//! Error type for the non-upstream HLSL compatibility extension.
//!
//! Deliberately separate from the upstream-facing `dxil_spirv::Error` so the
//! two never mix.

use std::fmt;

/// Errors produced by [`super::vec4_align_cbuffers`].
#[derive(Debug)]
pub enum HlslCompatError {
    /// The input words do not form a valid SPIR-V module.
    InvalidSpirv(String),
    /// A construct required for the rewrite is missing or unsupported.
    Unsupported(String),
}

impl fmt::Display for HlslCompatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HlslCompatError::InvalidSpirv(msg) => write!(f, "invalid SPIR-V input: {msg}"),
            HlslCompatError::Unsupported(msg) => write!(f, "unsupported construct: {msg}"),
        }
    }
}

impl std::error::Error for HlslCompatError {}
