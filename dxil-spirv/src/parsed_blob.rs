//! RAII wrapper around `dxil_spv_parsed_blob`.

use crate::error::{Error, Result, check};
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
        let raw = unsafe {
            sys::dxil_spv_parsed_blob_get_shader_stage_for_entry(self.handle, c_entry.as_ptr())
        };
        Ok(ShaderStage::from(raw))
    }

    /// Returns the index of the entry point with the given demangled name,
    /// or `None` if not found.
    pub fn entry_index_by_name(&self, entry: &str) -> Result<Option<u32>> {
        let c_entry = CString::new(entry).map_err(|_| Error::InvalidString)?;
        let mut index = 0u32;
        let result = unsafe {
            sys::dxil_spv_parsed_blob_get_entry_index_by_name(
                self.handle,
                c_entry.as_ptr(),
                &mut index,
            )
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
        let result =
            unsafe { sys::dxil_spv_parsed_blob_get_entry_point_name(self.handle, index, &mut ptr) };
        check(result)?;
        if ptr.is_null() {
            return Err(Error::NoOutput);
        }
        Ok(unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned())
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
        Ok(unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned())
    }

    /// Returns the disassembled LLVM IR as a UTF-8 string.
    ///
    /// Only available for DXIL blobs; legacy DXBC blobs do not carry IR.
    pub fn disassembled_ir(&self) -> Result<String> {
        let mut ptr: *const std::os::raw::c_char = std::ptr::null();
        let result =
            unsafe { sys::dxil_spv_parsed_blob_get_disassembled_ir(self.handle, &mut ptr) };
        check(result)?;
        if ptr.is_null() {
            return Err(Error::NoOutput);
        }
        Ok(unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned())
    }

    /// Returns the raw LLVM bitcode for the blob.
    pub fn raw_ir(&self) -> Result<Vec<u8>> {
        let mut data: *const std::os::raw::c_void = std::ptr::null();
        let mut size = 0usize;
        let result =
            unsafe { sys::dxil_spv_parsed_blob_get_raw_ir(self.handle, &mut data, &mut size) };
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

    // ── Resource scanning ───────────────────────────────────────────────

    /// Scan all resources in the blob, invoking the provided callbacks for
    /// each SRV, sampler, CBV, and UAV found.
    ///
    /// This is a convenience API for introspection before conversion. The
    /// callbacks receive the D3D binding description and may return `Some`
    /// with a Vulkan binding to remap, or `None` to leave unchanged.
    ///
    /// The callbacks are called synchronously on the current thread.
    pub fn scan_resources<S, M, C, U>(
        &self,
        mut srv: S,
        mut sampler: M,
        mut cbv: C,
        mut uav: U,
    ) -> Result<()>
    where
        S: FnMut(&crate::binding::D3dBinding) -> Option<crate::binding::SrvVulkanBinding>
            + Send
            + 'static,
        M: FnMut(&crate::binding::D3dBinding) -> Option<crate::binding::VulkanBinding>
            + Send
            + 'static,
        C: FnMut(&crate::binding::D3dBinding) -> Option<crate::binding::CbvVulkanBinding>
            + Send
            + 'static,
        U: FnMut(&crate::binding::UavD3dBinding) -> Option<crate::binding::UavVulkanBinding>
            + Send
            + 'static,
    {
        // We reuse the same double-boxing trampoline pattern as the
        // converter remappers, but with a simpler scope: the callbacks are
        // only alive for the duration of this call.
        use crate::remapper::{CbvRemapper, SamplerRemapper, SrvRemapper, UavRemapper};

        type SrvBox = Box<
            dyn FnMut(&crate::binding::D3dBinding) -> Option<crate::binding::SrvVulkanBinding>
                + Send,
        >;
        type SamplerBox = Box<
            dyn FnMut(&crate::binding::D3dBinding) -> Option<crate::binding::VulkanBinding> + Send,
        >;
        type CbvBox = Box<
            dyn FnMut(&crate::binding::D3dBinding) -> Option<crate::binding::CbvVulkanBinding>
                + Send,
        >;
        type UavBox = Box<
            dyn FnMut(&crate::binding::UavD3dBinding) -> Option<crate::binding::UavVulkanBinding>
                + Send,
        >;

        let srv_boxed: SrvBox = Box::new(move |b| srv(b));
        let sampler_boxed: SamplerBox = Box::new(move |b| sampler(b));
        let cbv_boxed: CbvBox = Box::new(move |b| cbv(b));
        let uav_boxed: UavBox = Box::new(move |b| uav(b));

        let (_srv_holder, srv_cb, srv_ud) = SrvRemapper::register(srv_boxed);
        let (_sampler_holder, sampler_cb, sampler_ud) = SamplerRemapper::register(sampler_boxed);
        let (_cbv_holder, cbv_cb, cbv_ud) = CbvRemapper::register(cbv_boxed);
        let (_uav_holder, uav_cb, uav_ud) = UavRemapper::register(uav_boxed);

        // The C API takes four separate userdata pointers, but our
        // trampoline uses a single userdata. We need to pack all four into
        // a single struct.
        #[allow(dead_code)] // Fields are read by C code, not Rust
        struct ScanUserdata {
            srv: *mut std::ffi::c_void,
            sampler: *mut std::ffi::c_void,
            cbv: *mut std::ffi::c_void,
            uav: *mut std::ffi::c_void,
        }

        let mut ud = ScanUserdata {
            srv: srv_ud,
            sampler: sampler_ud,
            cbv: cbv_ud,
            uav: uav_ud,
        };

        let result = unsafe {
            sys::dxil_spv_parsed_blob_scan_resources(
                self.handle,
                srv_cb,
                sampler_cb,
                cbv_cb,
                uav_cb,
                &mut ud as *mut ScanUserdata as *mut std::ffi::c_void,
            )
        };

        // Keep holders alive until the C call completes.
        drop(_srv_holder);
        drop(_sampler_holder);
        drop(_cbv_holder);
        drop(_uav_holder);

        check(result)
    }

    // ── RDAT subobjects (DXR) ───────────────────────────────────────────

    /// Returns the number of RDAT subobjects embedded in this blob.
    ///
    /// RDAT (Runtime Data) is used for DXR (raytracing) state objects.
    /// Returns 0 for non-raytracing blobs.
    pub fn num_rdat_subobjects(&self) -> u32 {
        unsafe { sys::dxil_spv_parsed_blob_get_num_rdat_subobjects(self.handle) }
    }

    /// Extract the RDAT subobject at `index`.
    ///
    /// Returns `None` if the index is out of range.
    pub fn rdat_subobject(&self, index: u32) -> Option<crate::binding::RdatSubobject> {
        if index >= self.num_rdat_subobjects() {
            return None;
        }
        let mut raw = sys::dxil_spv_rdat_subobject {
            kind: 0,
            subobject_name: std::ptr::null(),
            hit_group_type: 0,
            exports: std::ptr::null(),
            num_exports: 0,
            args: [0; 2],
            payload: std::ptr::null(),
            payload_size: 0,
        };
        unsafe { sys::dxil_spv_parsed_blob_get_rdat_subobject(self.handle, index, &mut raw) };

        let name = if raw.subobject_name.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(raw.subobject_name) }
                .to_string_lossy()
                .into_owned()
        };

        let exports = if raw.exports.is_null() || raw.num_exports == 0 {
            Vec::new()
        } else {
            (0..raw.num_exports as usize)
                .map(|i| {
                    let ptr = unsafe { *raw.exports.add(i) };
                    if ptr.is_null() {
                        String::new()
                    } else {
                        unsafe { CStr::from_ptr(ptr) }
                            .to_string_lossy()
                            .into_owned()
                    }
                })
                .collect()
        };

        let payload = if raw.payload.is_null() || raw.payload_size == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(raw.payload.cast::<u8>(), raw.payload_size) }
                .to_vec()
        };

        Some(crate::binding::RdatSubobject {
            kind: crate::binding::RdatSubobjectKind::from(raw.kind),
            name,
            hit_group_type: raw.hit_group_type as u32,
            exports,
            payload,
        })
    }

    // ── Work Graphs / mesh node (SM6.8) ─────────────────────────────────

    /// Get the node input data for a Work Graphs entry point.
    ///
    /// Only valid for SM6.8+ Work Graphs shaders.
    pub fn entry_point_node_input(&self, index: u32) -> Result<crate::binding::NodeInputData> {
        let mut raw = sys::dxil_spv_node_input_data::default();
        let result = unsafe {
            sys::dxil_spv_parsed_blob_get_entry_point_node_input(self.handle, index, &mut raw)
        };
        check(result)?;
        Ok(crate::binding::NodeInputData::from(raw))
    }

    /// Get the number of node outputs for a Work Graphs entry point.
    pub fn entry_point_num_node_outputs(&self, index: u32) -> Result<u32> {
        let mut count = 0u32;
        let result = unsafe {
            sys::dxil_spv_parsed_blob_get_entry_point_num_node_outputs(
                self.handle,
                index,
                &mut count,
            )
        };
        check(result)?;
        Ok(count)
    }

    /// Get the node output data for a Work Graphs entry point.
    pub fn entry_point_node_output(
        &self,
        index: u32,
        output_index: u32,
    ) -> Result<crate::binding::NodeOutputData> {
        let mut raw = sys::dxil_spv_node_output_data::default();
        let result = unsafe {
            sys::dxil_spv_parsed_blob_get_entry_point_node_output(
                self.handle,
                index,
                output_index,
                &mut raw,
            )
        };
        check(result)?;
        Ok(crate::binding::NodeOutputData::from(raw))
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
