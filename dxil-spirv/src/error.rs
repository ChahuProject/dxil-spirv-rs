//! Error type for the dxil-spirv safe wrapper.

use dxil_spirv_sys as sys;
use thiserror::Error;

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during parsing or conversion.
#[derive(Debug, Error)]
pub enum Error {
    /// The input buffer was empty.
    #[error("input blob is empty")]
    EmptyInput,

    /// dxil-spirv returned a non-success result code.
    #[error("dxil-spirv error: {0}")]
    DxilSpirv(i32),

    /// The converter produced no SPIR-V output.
    #[error("converter produced no SPIR-V output")]
    NoOutput,

    /// A string passed to the API contained an interior NUL byte.
    #[error("string contains an interior NUL byte")]
    InvalidString,

    /// The requested option is not supported by the linked dxil-spirv library.
    #[error("unsupported dxil-spirv option: {0}")]
    UnsupportedFeature(i32),
}

pub(crate) fn check(result: sys::dxil_spv_result) -> Result<()> {
    if result == sys::dxil_spv_result_DXIL_SPV_SUCCESS {
        Ok(())
    } else {
        Err(Error::DxilSpirv(result))
    }
}
