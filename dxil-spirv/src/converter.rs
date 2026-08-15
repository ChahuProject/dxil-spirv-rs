//! RAII wrapper around `dxil_spv_converter`.

use crate::error::{check, Error, Result};
use crate::options::ConverterOption;
use crate::parsed_blob::ParsedBlob;
use crate::remapper::{
    CbvRemapper, RemapperHolder, SamplerRemapper, SrvRemapper, StageInputRemapper,
    StageOutputRemapper, StreamOutputRemapper, UavRemapper, VertexInputRemapper,
};
use dxil_spirv_sys as sys;
use std::ffi::{CStr, CString};

/// A DXIL/DXBC → SPIR-V converter.
///
/// Owns the underlying `dxil_spv_converter` handle and frees it on drop.
/// Create from a [`ParsedBlob`], call [`Converter::run`], then retrieve the
/// result with [`Converter::compiled_spirv`].
///
/// The converter owns any remapper closures that have been registered;
/// dropping the converter also drops the closures.
pub struct Converter {
    handle: sys::dxil_spv_converter,
    /// Owns the boxed remapper closures so they outlive the converter.
    /// Never read directly; only kept for lifetime purposes.
    _remappers: Option<Box<RemapperHolder>>,
}

impl std::fmt::Debug for Converter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Converter").field("handle", &self.handle).finish()
    }
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
        Ok(Self {
            handle,
            _remappers: None,
        })
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
        Ok(Self {
            handle,
            _remappers: None,
        })
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

    /// Set the entry point to convert.
    ///
    /// The string is copied by the implementation; the `CString` may be
    /// dropped after this call.
    pub fn set_entry_point(&mut self, entry_point: &str) -> Result<()> {
        let c_entry = CString::new(entry_point).map_err(|_| Error::InvalidString)?;
        unsafe { sys::dxil_spv_converter_set_entry_point(self.handle, c_entry.as_ptr()) };
        Ok(())
    }

    /// Returns the name of the compiled entry point, if conversion has run.
    ///
    /// Returns `None` when no entry point has been compiled yet.
    pub fn compiled_entry_point(&self) -> Result<Option<String>> {
        let mut ptr: *const std::os::raw::c_char = std::ptr::null();
        let result = unsafe { sys::dxil_spv_converter_get_compiled_entry_point(self.handle, &mut ptr) };
        check(result)?;
        if ptr.is_null() {
            return Ok(None);
        }
        let s = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
        Ok(Some(s))
    }

    /// Add a converter option.
    ///
    /// See [`ConverterOption`] for the full list of supported options.
    /// Returns [`Error::UnsupportedFeature`] if the option is not recognized
    /// by the linked library.
    pub fn add_option(&mut self, option: &ConverterOption) -> Result<()> {
        if !option.is_supported() {
            return Err(Error::UnsupportedFeature(option.kind()));
        }
        let (_base, data) = option.to_raw();
        let base = data.as_base();
        let result = unsafe { sys::dxil_spv_converter_add_option(self.handle, base) };
        check(result)
    }

    /// Returns `true` if the converter emits the `SubgroupSize` builtin.
    ///
    /// Must be called after [`Converter::run`].
    pub fn uses_subgroup_size(&self) -> bool {
        unsafe { sys::dxil_spv_converter_uses_subgroup_size(self.handle) == 1 }
    }

    /// Returns the compute workgroup dimensions `(x, y, z)`.
    ///
    /// Must be called after [`Converter::run`].
    pub fn compute_workgroup_dimensions(&self) -> Result<(u32, u32, u32)> {
        let (mut x, mut y, mut z) = (0u32, 0u32, 0u32);
        let result = unsafe {
            sys::dxil_spv_converter_get_compute_workgroup_dimensions(self.handle, &mut x, &mut y, &mut z)
        };
        check(result)?;
        Ok((x, y, z))
    }

    /// Returns the required wave size, or `None` if not required.
    ///
    /// A non-zero value maps to `requiredSubgroupSize`; zero means
    /// `VARYING_SUBGROUP_SIZE`.
    pub fn compute_required_wave_size(&self) -> Result<Option<u32>> {
        let mut size = 0u32;
        let result = unsafe {
            sys::dxil_spv_converter_get_compute_required_wave_size(self.handle, &mut size)
        };
        check(result)?;
        Ok(if size == 0 { None } else { Some(size) })
    }

    /// Returns the wave-size range `(min, max, preferred)`.
    pub fn compute_wave_size_range(&self) -> Result<(u32, u32, u32)> {
        let (mut min, mut max, mut preferred) = (0u32, 0u32, 0u32);
        let result = unsafe {
            sys::dxil_spv_converter_get_compute_wave_size_range(self.handle, &mut min, &mut max, &mut preferred)
        };
        check(result)?;
        Ok((min, max, preferred))
    }

    /// Returns the heuristic minimum wave size suggested by the analysis.
    pub fn compute_heuristic_min_wave_size(&self) -> Result<u32> {
        let mut size = 0u32;
        let result = unsafe {
            sys::dxil_spv_converter_get_compute_heuristic_min_wave_size(self.handle, &mut size)
        };
        check(result)?;
        Ok(size)
    }

    /// Returns the heuristic maximum wave size suggested by the analysis.
    pub fn compute_heuristic_max_wave_size(&self) -> Result<u32> {
        let mut size = 0u32;
        let result = unsafe {
            sys::dxil_spv_converter_get_compute_heuristic_max_wave_size(self.handle, &mut size)
        };
        check(result)?;
        Ok(size)
    }

    /// Returns the number of patch vertices for hull shaders.
    pub fn patch_vertex_count(&self) -> Result<u32> {
        let mut count = 0u32;
        let result = unsafe { sys::dxil_spv_converter_get_patch_vertex_count(self.handle, &mut count) };
        check(result)?;
        Ok(count)
    }

    /// Returns the patch-location offset set via [`Converter::set_patch_location_offset`].
    pub fn patch_location_offset(&self) -> Result<u32> {
        let mut offset = 0u32;
        let result = unsafe {
            sys::dxil_spv_converter_get_patch_location_offset(self.handle, &mut offset)
        };
        check(result)?;
        Ok(offset)
    }

    /// Set the patch-location offset for domain shaders linked with hull shaders.
    pub fn set_patch_location_offset(&mut self, offset: u32) {
        unsafe { sys::dxil_spv_converter_set_patch_location_offset(self.handle, offset) };
    }

    /// Returns `true` if the shader uses the given feature.
    ///
    /// Must be called after [`Converter::run`].
    pub fn uses_shader_feature(&self, feature: ShaderFeature) -> bool {
        unsafe {
            sys::dxil_spv_converter_uses_shader_feature(self.handle, feature.into()) == 1
        }
    }

    /// Returns analysis warnings produced during compilation, if any.
    ///
    /// The string is owned by the converter; a copy is returned.
    pub fn analysis_warnings(&self) -> Option<String> {
        let ptr = unsafe { sys::dxil_spv_converter_get_analysis_warnings(self.handle) };
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned())
    }

    /// Register a callback that remaps SRV bindings.
    ///
    /// The closure receives the D3D binding description and returns the
    /// desired Vulkan binding. Return `None` to skip remapping.
    ///
    /// The closure is stored inside the converter and dropped with it.
    pub fn set_srv_remapper<F>(&mut self, callback: F)
    where
        F: FnMut(&crate::binding::D3dBinding) -> Option<crate::binding::SrvVulkanBinding> + Send + 'static,
    {
        let boxed = Box::new(callback);
        let (remapper, cb, userdata) = SrvRemapper::register(boxed);
        let holder = self._remappers.get_or_insert_with(|| Box::new(RemapperHolder::default()));
        holder.srv = Some(remapper);
        unsafe { sys::dxil_spv_converter_set_srv_remapper(self.handle, cb, userdata) };
    }

    /// Register a callback that remaps UAV bindings.
    pub fn set_uav_remapper<F>(&mut self, callback: F)
    where
        F: FnMut(&crate::binding::UavD3dBinding) -> Option<crate::binding::UavVulkanBinding> + Send + 'static,
    {
        let boxed = Box::new(callback);
        let (remapper, cb, userdata) = UavRemapper::register(boxed);
        let holder = self._remappers.get_or_insert_with(|| Box::new(RemapperHolder::default()));
        holder.uav = Some(remapper);
        unsafe { sys::dxil_spv_converter_set_uav_remapper(self.handle, cb, userdata) };
    }

    /// Register a callback that remaps CBV bindings.
    pub fn set_cbv_remapper<F>(&mut self, callback: F)
    where
        F: FnMut(&crate::binding::D3dBinding) -> Option<crate::binding::CbvVulkanBinding> + Send + 'static,
    {
        let boxed = Box::new(callback);
        let (remapper, cb, userdata) = CbvRemapper::register(boxed);
        let holder = self._remappers.get_or_insert_with(|| Box::new(RemapperHolder::default()));
        holder.cbv = Some(remapper);
        unsafe { sys::dxil_spv_converter_set_cbv_remapper(self.handle, cb, userdata) };
    }

    /// Register a callback that remaps sampler bindings.
    pub fn set_sampler_remapper<F>(&mut self, callback: F)
    where
        F: FnMut(&crate::binding::D3dBinding) -> Option<crate::binding::VulkanBinding> + Send + 'static,
    {
        let boxed = Box::new(callback);
        let (remapper, cb, userdata) = SamplerRemapper::register(boxed);
        let holder = self._remappers.get_or_insert_with(|| Box::new(RemapperHolder::default()));
        holder.sampler = Some(remapper);
        unsafe { sys::dxil_spv_converter_set_sampler_remapper(self.handle, cb, userdata) };
    }

    /// Register a callback that remaps vertex input attributes.
    pub fn set_vertex_input_remapper<F>(&mut self, callback: F)
    where
        F: FnMut(&crate::binding::D3dVertexInput) -> Option<crate::binding::VulkanVertexInput> + Send + 'static,
    {
        let boxed = Box::new(callback);
        let (remapper, cb, userdata) = VertexInputRemapper::register(boxed);
        let holder = self._remappers.get_or_insert_with(|| Box::new(RemapperHolder::default()));
        holder.vertex_input = Some(remapper);
        unsafe { sys::dxil_spv_converter_set_vertex_input_remapper(self.handle, cb, userdata) };
    }

    /// Register a callback that remaps stage-input variables.
    pub fn set_stage_input_remapper<F>(&mut self, callback: F)
    where
        F: FnMut(&crate::binding::D3dShaderStageIo) -> Option<crate::binding::VulkanShaderStageIo> + Send + 'static,
    {
        let boxed = Box::new(callback);
        let (remapper, cb, userdata) = StageInputRemapper::register(boxed);
        let holder = self._remappers.get_or_insert_with(|| Box::new(RemapperHolder::default()));
        holder.stage_input = Some(remapper);
        unsafe { sys::dxil_spv_converter_set_stage_input_remapper(self.handle, cb, userdata) };
    }

    /// Register a callback that remaps stage-output variables.
    pub fn set_stage_output_remapper<F>(&mut self, callback: F)
    where
        F: FnMut(&crate::binding::D3dShaderStageIo) -> Option<crate::binding::VulkanShaderStageIo> + Send + 'static,
    {
        let boxed = Box::new(callback);
        let (remapper, cb, userdata) = StageOutputRemapper::register(boxed);
        let holder = self._remappers.get_or_insert_with(|| Box::new(RemapperHolder::default()));
        holder.stage_output = Some(remapper);
        unsafe { sys::dxil_spv_converter_set_stage_output_remapper(self.handle, cb, userdata) };
    }

    /// Register a callback that remaps stream-output variables.
    pub fn set_stream_output_remapper<F>(&mut self, callback: F)
    where
        F: FnMut(&crate::binding::D3dStreamOutput) -> Option<crate::binding::VulkanStreamOutput> + Send + 'static,
    {
        let boxed = Box::new(callback);
        let (remapper, cb, userdata) = StreamOutputRemapper::register(boxed);
        let holder = self._remappers.get_or_insert_with(|| Box::new(RemapperHolder::default()));
        holder.stream_output = Some(remapper);
        unsafe { sys::dxil_spv_converter_set_stream_output_remapper(self.handle, cb, userdata) };
    }
}

impl Drop for Converter {
    fn drop(&mut self) {
        // Drop remapper boxes before freeing the converter handle so that
        // any in-flight C callback never sees a dangling userdata pointer.
        drop(self._remappers.take());
        if !self.handle.is_null() {
            unsafe { sys::dxil_spv_converter_free(self.handle) };
        }
    }
}

unsafe impl Send for Converter {}

/// Shader features that can be queried with [`Converter::uses_shader_feature`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderFeature {
    /// Native 16-bit arithmetic operations.
    Native16BitOperations,
}

impl From<ShaderFeature> for sys::dxil_spv_shader_feature {
    fn from(value: ShaderFeature) -> Self {
        match value {
            ShaderFeature::Native16BitOperations => {
                sys::dxil_spv_shader_feature_DXIL_SPV_SHADER_FEATURE_NATIVE_16BIT_OPERATIONS
            }
        }
    }
}
