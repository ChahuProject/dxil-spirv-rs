//! Trampoline layer that bridges C remapper callbacks to safe Rust closures.
//!
//! Each `*Remapper` type wraps a boxed `FnMut` closure and exposes:
//! - [`into_raw_parts`] — converts the box into a `(fn pointer, userdata)`
//!   pair suitable for the C API;
//! - [`from_raw_parts`] — reconstructs the box when the converter is dropped,
//!   ensuring the closure is freed exactly once.
//!
//! All trampolines catch panics with [`std::panic::catch_unwind`] and return
//! `DXIL_SPV_FALSE` to the C side on panic, preventing unwinding across the
//! FFI boundary.

use crate::binding::{
    CbvVulkanBinding, D3dBinding, D3dShaderStageIo, D3dStreamOutput, D3dVertexInput,
    SrvVulkanBinding, UavD3dBinding, UavVulkanBinding, VulkanBinding, VulkanShaderStageIo,
    VulkanStreamOutput, VulkanVertexInput,
};
use dxil_spirv_sys as sys;
use std::ffi::c_void;

macro_rules! define_remapper {
    (
        $name:ident,
        closure = $closure:ty,
        cb_ty = $cb_ty:ty,
        d3d_ty = $d3d_raw:ty,
        vulkan_ty = $vulkan_raw:ty,
        d3d_safe = $d3d_safe:ty,
        vulkan_safe = $vulkan_safe:ty
    ) => {
        /// Type-erased holder for a boxed remapper closure.
        ///
        /// Keeps the closure alive until the converter is dropped.
        pub(crate) struct $name {
            /// Double-boxed closure. The outer box gives us a thin, stable
            /// `*mut c_void` userdata pointer (a `Box<dyn FnMut>` is a fat
            /// pointer and cannot be cast to `*mut c_void` directly).
            closure: Box<Box<$closure>>,
        }

        impl $name {
            /// Wrap a boxed closure and produce the `(callback, userdata)`
            /// pair to hand to the C API.
            ///
            /// The holder retains ownership of the closure; `userdata` points
            /// at the (stable) outer box. Store the returned holder in the
            /// converter before registering the callback.
            pub(crate) fn register(closure: Box<$closure>) -> (Self, $cb_ty, *mut c_void) {
                let mut holder = Self {
                    closure: Box::new(closure),
                };
                // Thin pointer to the outer box's heap allocation.
                let userdata = (&mut *holder.closure) as *mut Box<$closure> as *mut c_void;
                (holder, Some(Self::trampoline), userdata)
            }

            /// The `extern "C"` trampoline that C calls.
            extern "C" fn trampoline(
                userdata: *mut c_void,
                d3d: *const $d3d_raw,
                vulkan: *mut $vulkan_raw,
            ) -> sys::dxil_spv_bool {
                // Re-borrow the closure without taking ownership: userdata is
                // a thin pointer to `Box<$closure>`; deref twice to reach the
                // inner `dyn FnMut`.
                let closure: &mut $closure = unsafe { &mut **(userdata as *mut Box<$closure>) };
                let d3d = unsafe { &*d3d };
                let vulkan = unsafe { &mut *vulkan };

                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let d3d_safe = <$d3d_safe>::from(d3d);
                    closure(&d3d_safe)
                }));

                match result {
                    Ok(Some(vulkan_safe)) => {
                        *vulkan = <$vulkan_raw>::from(vulkan_safe);
                        1
                    }
                    Ok(None) | Err(_) => 0,
                }
            }
        }
    };
}

define_remapper!(
    SrvRemapper,
    closure = dyn FnMut(&D3dBinding) -> Option<SrvVulkanBinding> + Send,
    cb_ty = sys::dxil_spv_srv_remapper_cb,
    d3d_ty = sys::dxil_spv_d3d_binding,
    vulkan_ty = sys::dxil_spv_srv_vulkan_binding,
    d3d_safe = D3dBinding,
    vulkan_safe = SrvVulkanBinding
);

define_remapper!(
    UavRemapper,
    closure = dyn FnMut(&UavD3dBinding) -> Option<UavVulkanBinding> + Send,
    cb_ty = sys::dxil_spv_uav_remapper_cb,
    d3d_ty = sys::dxil_spv_uav_d3d_binding,
    vulkan_ty = sys::dxil_spv_uav_vulkan_binding,
    d3d_safe = UavD3dBinding,
    vulkan_safe = UavVulkanBinding
);

define_remapper!(
    CbvRemapper,
    closure = dyn FnMut(&D3dBinding) -> Option<CbvVulkanBinding> + Send,
    cb_ty = sys::dxil_spv_cbv_remapper_cb,
    d3d_ty = sys::dxil_spv_d3d_binding,
    vulkan_ty = sys::dxil_spv_cbv_vulkan_binding,
    d3d_safe = D3dBinding,
    vulkan_safe = CbvVulkanBinding
);

define_remapper!(
    SamplerRemapper,
    closure = dyn FnMut(&D3dBinding) -> Option<VulkanBinding> + Send,
    cb_ty = sys::dxil_spv_sampler_remapper_cb,
    d3d_ty = sys::dxil_spv_d3d_binding,
    vulkan_ty = sys::dxil_spv_vulkan_binding,
    d3d_safe = D3dBinding,
    vulkan_safe = VulkanBinding
);

define_remapper!(
    VertexInputRemapper,
    closure = dyn FnMut(&D3dVertexInput) -> Option<VulkanVertexInput> + Send,
    cb_ty = sys::dxil_spv_vertex_input_remapper_cb,
    d3d_ty = sys::dxil_spv_d3d_vertex_input,
    vulkan_ty = sys::dxil_spv_vulkan_vertex_input,
    d3d_safe = D3dVertexInput,
    vulkan_safe = VulkanVertexInput
);

define_remapper!(
    StageInputRemapper,
    closure = dyn FnMut(&D3dShaderStageIo) -> Option<VulkanShaderStageIo> + Send,
    cb_ty = sys::dxil_spv_shader_stage_io_remapper_cb,
    d3d_ty = sys::dxil_spv_d3d_shader_stage_io,
    vulkan_ty = sys::dxil_spv_vulkan_shader_stage_io,
    d3d_safe = D3dShaderStageIo,
    vulkan_safe = VulkanShaderStageIo
);

define_remapper!(
    StageOutputRemapper,
    closure = dyn FnMut(&D3dShaderStageIo) -> Option<VulkanShaderStageIo> + Send,
    cb_ty = sys::dxil_spv_shader_stage_io_remapper_cb,
    d3d_ty = sys::dxil_spv_d3d_shader_stage_io,
    vulkan_ty = sys::dxil_spv_vulkan_shader_stage_io,
    d3d_safe = D3dShaderStageIo,
    vulkan_safe = VulkanShaderStageIo
);

define_remapper!(
    StreamOutputRemapper,
    closure = dyn FnMut(&D3dStreamOutput) -> Option<VulkanStreamOutput> + Send,
    cb_ty = sys::dxil_spv_stream_output_remapper_cb,
    d3d_ty = sys::dxil_spv_d3d_stream_output,
    vulkan_ty = sys::dxil_spv_vulkan_stream_output,
    d3d_safe = D3dStreamOutput,
    vulkan_safe = VulkanStreamOutput
);

/// Owns all remapper closures registered on a [`Converter`](crate::Converter).
///
/// Stored as an `Option<Box<…>>` inside the converter so that the closures
/// are dropped before the underlying C converter handle is freed.
#[derive(Default)]
pub(crate) struct RemapperHolder {
    pub srv: Option<SrvRemapper>,
    pub uav: Option<UavRemapper>,
    pub cbv: Option<CbvRemapper>,
    pub sampler: Option<SamplerRemapper>,
    pub vertex_input: Option<VertexInputRemapper>,
    pub stage_input: Option<StageInputRemapper>,
    pub stage_output: Option<StageOutputRemapper>,
    pub stream_output: Option<StreamOutputRemapper>,
}
