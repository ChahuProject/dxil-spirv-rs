//! RAII wrapper around `dxil_spv_parsed_blob`.

use crate::error::{check, Error, Result};
use crate::stage::ShaderStage;
use dxil_spirv_sys as sys;

/// A parsed DXIL/DXBC shader blob.
///
/// Owns the underlying `dxil_spv_parsed_blob` handle and frees it on drop.
/// Construct via [`ParsedBlob::parse`] (full container or raw bitcode) or
/// [`ParsedBlob::parse_reflection`].
#[derive(Debug)]
pub struct ParsedBlob {
    pub(crate) handle: sys::dxil_spv_parsed_blob,
}

impl ParsedBlob {
    /// Parse a shader blob (DXBC container or raw DXIL bitcode).
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::EmptyInput);
        }
        let mut handle: sys::dxil_spv_parsed_blob = std::ptr::null_mut();
        let result =
            unsafe { sys::dxil_spv_parse_dxil_blob(data.as_ptr().cast(), data.len(), &mut handle) };
        check(result)?;
        if handle.is_null() {
            return Err(Error::NoOutput);
        }
        Ok(Self { handle })
    }

    /// Parse a shader blob with reflection information preserved.
    ///
    /// Use this when the blob will later be passed to
    /// [`crate::Converter::new_with_reflection`].
    pub fn parse_reflection(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::EmptyInput);
        }
        let mut handle: sys::dxil_spv_parsed_blob = std::ptr::null_mut();
        let result = unsafe {
            sys::dxil_spv_parse_reflection_dxil_blob(data.as_ptr().cast(), data.len(), &mut handle)
        };
        check(result)?;
        if handle.is_null() {
            return Err(Error::NoOutput);
        }
        Ok(Self { handle })
    }

    /// Returns the shader stage of the (first) entry point.
    pub fn shader_stage(&self) -> ShaderStage {
        let raw = unsafe { sys::dxil_spv_parsed_blob_get_shader_stage(self.handle) };
        ShaderStage::from(raw)
    }

    /// Returns the number of entry points in this blob.
    pub fn num_entry_points(&self) -> Result<u32> {
        let mut count = 0u32;
        let result =
            unsafe { sys::dxil_spv_parsed_blob_get_num_entry_points(self.handle, &mut count) };
        check(result)?;
        Ok(count)
    }
}

impl Drop for ParsedBlob {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { sys::dxil_spv_parsed_blob_free(self.handle) };
        }
    }
}

// The upstream handle is reference-counted / owned per-instance; it is safe to
// move across threads but not to access concurrently without external sync.
unsafe impl Send for ParsedBlob {}
