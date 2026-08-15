//! RAII wrapper around `dxil_spv_converter`.

use crate::error::{check, Error, Result};
use crate::parsed_blob::ParsedBlob;
use dxil_spirv_sys as sys;

/// A DXIL/DXBC → SPIR-V converter.
///
/// Owns the underlying `dxil_spv_converter` handle and frees it on drop.
/// Create from a [`ParsedBlob`], call [`Converter::run`], then retrieve the
/// result with [`Converter::compiled_spirv`].
#[derive(Debug)]
pub struct Converter {
    handle: sys::dxil_spv_converter,
}

impl Converter {
    /// Create a converter from a parsed blob.
    pub fn new(blob: &ParsedBlob) -> Result<Self> {
        let mut handle: sys::dxil_spv_converter = std::ptr::null_mut();
        let result = unsafe { sys::dxil_spv_create_converter(blob.handle, &mut handle) };
        check(result)?;
        if handle.is_null() {
            return Err(Error::NoOutput);
        }
        Ok(Self { handle })
    }

    /// Create a converter with a separate reflection blob.
    ///
    /// Useful when the shader blob has been stripped of reflection data but a
    /// companion blob (e.g. from `ParsedBlob::parse_reflection`) is available.
    pub fn new_with_reflection(blob: &ParsedBlob, reflection: &ParsedBlob) -> Result<Self> {
        let mut handle: sys::dxil_spv_converter = std::ptr::null_mut();
        let result = unsafe {
            sys::dxil_spv_create_converter_with_reflection(blob.handle, reflection.handle, &mut handle)
        };
        check(result)?;
        if handle.is_null() {
            return Err(Error::NoOutput);
        }
        Ok(Self { handle })
    }

    /// Run the conversion.
    pub fn run(&self) -> Result<()> {
        let result = unsafe { sys::dxil_spv_converter_run(self.handle) };
        check(result)
    }

    /// Retrieve the compiled SPIR-V as a vector of little-endian `u32` words.
    ///
    /// Must be called after [`Converter::run`].
    pub fn compiled_spirv(&self) -> Result<Vec<u32>> {
        let mut compiled = sys::dxil_spv_compiled_spirv {
            data: std::ptr::null(),
            size: 0,
        };
        let result = unsafe { sys::dxil_spv_converter_get_compiled_spirv(self.handle, &mut compiled) };
        check(result)?;
        if compiled.data.is_null() || compiled.size == 0 {
            return Err(Error::NoOutput);
        }
        let words = unsafe {
            std::slice::from_raw_parts(compiled.data.cast::<u32>(), compiled.size / 4)
        };
        Ok(words.to_vec())
    }
}

impl Drop for Converter {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { sys::dxil_spv_converter_free(self.handle) };
        }
    }
}

unsafe impl Send for Converter {}
