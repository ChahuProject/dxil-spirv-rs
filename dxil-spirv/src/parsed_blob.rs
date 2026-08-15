//! RAII wrapper around `dxil_spv_parsed_blob`.

use crate::error::{check, Error, Result};
use crate::stage::ShaderStage;
use dxil_spirv_sys as sys;
use std::ffi::{CStr, CString};

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

    /// Returns the shader stage for the entry point with the given demangled
    /// name. Returns [`ShaderStage::Unknown`] if the entry is not found.
    pub fn shader_stage_for_entry(&self, entry: &str) -> Result<ShaderStage> {
        let c_entry = CString::new(entry).map_err(|_| Error::InvalidString)?;
        let raw = unsafe { sys::dxil_spv_parsed_blob_get_shader_stage_for_entry(self.handle, c_entry.as_ptr()) };
        Ok(ShaderStage::from(raw))
    }

    /// Returns the index of the entry point with the given demangled name,
    /// or `None` if not found.
    pub fn entry_index_by_name(&self, entry: &str) -> Result<Option<u32>> {
        let c_entry = CString::new(entry).map_err(|_| Error::InvalidString)?;
        let mut index = 0u32;
        let result = unsafe {
            sys::dxil_spv_parsed_blob_get_entry_index_by_name(self.handle, c_entry.as_ptr(), &mut index)
        };
        match result {
            sys::dxil_spv_result_DXIL_SPV_SUCCESS => Ok(Some(index)),
            sys::dxil_spv_result_DXIL_SPV_ERROR_NO_DATA => Ok(None),
            other => Err(Error::DxilSpirv(other)),
        }
    }

    /// Returns the mangled name of the entry point at `index`.
    pub fn entry_point_name(&self, index: u32) -> Result<String> {
        let mut ptr: *const std::os::raw::c_char = std::ptr::null();
        let result = unsafe {
            sys::dxil_spv_parsed_blob_get_entry_point_name(self.handle, index, &mut ptr)
        };
        check(result)?;
        if ptr.is_null() {
            return Err(Error::NoOutput);
        }
        Ok(unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned())
    }

    /// Returns the demangled name of the entry point at `index`.
    pub fn entry_point_demangled_name(&self, index: u32) -> Result<String> {
        let mut ptr: *const std::os::raw::c_char = std::ptr::null();
        let result = unsafe {
            sys::dxil_spv_parsed_blob_get_entry_point_demangled_name(self.handle, index, &mut ptr)
        };
        check(result)?;
        if ptr.is_null() {
            return Err(Error::NoOutput);
        }
        Ok(unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned())
    }

    /// Returns the disassembled LLVM IR as a UTF-8 string.
    ///
    /// Only available for DXIL blobs; legacy DXBC blobs do not carry IR.
    pub fn disassembled_ir(&self) -> Result<String> {
        let mut ptr: *const std::os::raw::c_char = std::ptr::null();
        let result = unsafe { sys::dxil_spv_parsed_blob_get_disassembled_ir(self.handle, &mut ptr) };
        check(result)?;
        if ptr.is_null() {
            return Err(Error::NoOutput);
        }
        Ok(unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned())
    }

    /// Returns the raw LLVM bitcode for the blob.
    pub fn raw_ir(&self) -> Result<Vec<u8>> {
        let mut data: *const std::os::raw::c_void = std::ptr::null();
        let mut size = 0usize;
        let result = unsafe { sys::dxil_spv_parsed_blob_get_raw_ir(self.handle, &mut data, &mut size) };
        check(result)?;
        if data.is_null() || size == 0 {
            return Err(Error::NoOutput);
        }
        let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) };
        Ok(bytes.to_vec())
    }

    /// Dump the LLVM IR to stdout. For debugging.
    pub fn dump_llvm_ir(&self) {
        unsafe { sys::dxil_spv_parsed_blob_dump_llvm_ir(self.handle) };
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
