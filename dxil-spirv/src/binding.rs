//! Typed Rust representations of the D3D and Vulkan binding structs used by
//! the resource remapper callbacks.

use crate::stage::ShaderStage;
use dxil_spirv_sys as sys;
use std::ffi::CStr;

/// The kind of a D3D shader resource (texture, buffer, sampler, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    /// Invalid / unknown.
    Invalid,
    /// 1D texture.
    Texture1D,
    /// 2D texture.
    Texture2D,
    /// 2D multisampled texture.
    Texture2DMs,
    /// 3D texture.
    Texture3D,
    /// Cube texture.
    TextureCube,
    /// 1D texture array.
    Texture1DArray,
    /// 2D texture array.
    Texture2DArray,
    /// 2D multisampled texture array.
    Texture2DMsArray,
    /// Cube texture array.
    TextureCubeArray,
    /// Typed buffer.
    TypedBuffer,
    /// Raw (byte-address) buffer.
    RawBuffer,
    /// Structured buffer.
    StructuredBuffer,
    /// Constant buffer.
    ConstantBuffer,
    /// Sampler state.
    Sampler,
    /// Texture buffer.
    TBuffer,
    /// Ray-tracing acceleration structure.
    RtAccelerationStructure,
    /// Feedback texture 2D.
    FeedbackTexture2D,
    /// Feedback texture 2D array.
    FeedbackTexture2DArray,
}

impl From<sys::dxil_spv_resource_kind> for ResourceKind {
    fn from(value: sys::dxil_spv_resource_kind) -> Self {
        match value {
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_TEXTURE_1D => Self::Texture1D,
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_TEXTURE_2D => Self::Texture2D,
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_TEXTURE_2DMS => Self::Texture2DMs,
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_TEXTURE_3D => Self::Texture3D,
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_TEXTURE_CUBE => Self::TextureCube,
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_TEXTURE_1D_ARRAY => {
                Self::Texture1DArray
            }
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_TEXTURE_2D_ARRAY => {
                Self::Texture2DArray
            }
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_TEXTURE_2D_MS_ARRAY => {
                Self::Texture2DMsArray
            }
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_TEXTURE_CUBE_ARRAY => {
                Self::TextureCubeArray
            }
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_TYPED_BUFFER => Self::TypedBuffer,
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_RAW_BUFFER => Self::RawBuffer,
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_STRUCTURED_BUFFER => {
                Self::StructuredBuffer
            }
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_CONSTANT_BUFFER => {
                Self::ConstantBuffer
            }
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_SAMPLER => Self::Sampler,
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_TBUFFER => Self::TBuffer,
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_RT_ACCELERATION_STRUCTURE => {
                Self::RtAccelerationStructure
            }
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_FEEDBACK_TEXTURE_2D => {
                Self::FeedbackTexture2D
            }
            sys::dxil_spv_resource_kind_DXIL_SPV_RESOURCE_KIND_FEEDBACK_TEXTURE_2D_ARRAY => {
                Self::FeedbackTexture2DArray
            }
            _ => Self::Invalid,
        }
    }
}

/// The Vulkan descriptor type to emit for a resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VulkanDescriptorType {
    /// Use the natural descriptor type for the resource.
    Identity,
    /// SSBO (storage buffer).
    Ssbo,
    /// Texel buffer.
    TexelBuffer,
    /// Buffer device address.
    BufferDeviceAddress,
    /// Uniform buffer object.
    Ubo,
    /// Input attachment (tile shaders).
    InputAttachment,
}

impl From<VulkanDescriptorType> for sys::dxil_spv_vulkan_descriptor_type {
    fn from(value: VulkanDescriptorType) -> Self {
        match value {
            VulkanDescriptorType::Identity => sys::dxil_spv_vulkan_descriptor_type_DXIL_SPV_VULKAN_DESCRIPTOR_TYPE_IDENTITY,
            VulkanDescriptorType::Ssbo => sys::dxil_spv_vulkan_descriptor_type_DXIL_SPV_VULKAN_DESCRIPTOR_TYPE_SSBO,
            VulkanDescriptorType::TexelBuffer => sys::dxil_spv_vulkan_descriptor_type_DXIL_SPV_VULKAN_DESCRIPTOR_TYPE_TEXEL_BUFFER,
            VulkanDescriptorType::BufferDeviceAddress => sys::dxil_spv_vulkan_descriptor_type_DXIL_SPV_VULKAN_DESCRIPTOR_TYPE_BUFFER_DEVICE_ADDRESS,
            VulkanDescriptorType::Ubo => sys::dxil_spv_vulkan_descriptor_type_DXIL_SPV_VULKAN_DESCRIPTOR_TYPE_UBO,
            VulkanDescriptorType::InputAttachment => sys::dxil_spv_vulkan_descriptor_type_DXIL_SPV_VULKAN_DESCRIPTOR_TYPE_INPUT_ATTACHMENT,
        }
    }
}

/// D3D binding description passed to the remapper callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct D3dBinding {
    /// Shader stage.
    pub stage: ShaderStage,
    /// Resource kind.
    pub kind: ResourceKind,
    /// Resource index in the shader.
    pub resource_index: u32,
    /// Register space.
    pub register_space: u32,
    /// Register index.
    pub register_index: u32,
    /// Range size.
    pub range_size: u32,
    /// Alignment (16 for raw buffers, element stride for structured buffers,
    /// otherwise 0).
    pub alignment: u32,
}

impl From<&sys::dxil_spv_d3d_binding> for D3dBinding {
    fn from(raw: &sys::dxil_spv_d3d_binding) -> Self {
        Self {
            stage: ShaderStage::from(raw.stage),
            kind: ResourceKind::from(raw.kind),
            resource_index: raw.resource_index,
            register_space: raw.register_space,
            register_index: raw.register_index,
            range_size: raw.range_size,
            alignment: raw.alignment,
        }
    }
}

/// D3D UAV binding description passed to the UAV remapper callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UavD3dBinding {
    /// The base D3D binding.
    pub binding: D3dBinding,
    /// Whether the UAV has a counter.
    pub has_counter: bool,
}

impl From<&sys::dxil_spv_uav_d3d_binding> for UavD3dBinding {
    fn from(raw: &sys::dxil_spv_uav_d3d_binding) -> Self {
        Self {
            binding: D3dBinding::from(&raw.d3d_binding),
            has_counter: raw.has_counter != 0,
        }
    }
}

/// Vulkan binding produced by a remapper callback.
///
/// The `root_constant_index` / `input_attachment_index` union is resolved
/// automatically from the `descriptor_type` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VulkanBinding {
    /// Vulkan descriptor set.
    pub set: u32,
    /// Vulkan binding within the set.
    pub binding: u32,
    /// For bindless: the Nth root constant. For BDA: the Nth root descriptor.
    /// For input attachments: the input attachment index (`u32::MAX` = depth/stencil).
    pub root_constant_index: u32,
    /// Bindless heap parameters.
    pub bindless: Bindless,
    /// The descriptor type to emit.
    pub descriptor_type: VulkanDescriptorType,
}

/// Bindless heap parameters for a Vulkan binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bindless {
    /// Offset into the root descriptor heap.
    pub heap_root_offset: u32,
    /// Whether to use the heap.
    pub use_heap: bool,
}

impl From<VulkanBinding> for sys::dxil_spv_vulkan_binding {
    // bindgen unions cannot be initialized with a struct literal; mutate a
    // `Default::default()` instead, which trips field_reassign_with_default.
    #[allow(clippy::field_reassign_with_default)]
    fn from(value: VulkanBinding) -> Self {
        let mut raw = sys::dxil_spv_vulkan_binding::default();
        raw.set = value.set;
        raw.binding = value.binding;
        raw.bindless.heap_root_offset = value.bindless.heap_root_offset;
        raw.bindless.use_heap = if value.bindless.use_heap { 1 } else { 0 };
        raw.descriptor_type = value.descriptor_type.into();
        raw.__bindgen_anon_1.root_constant_index = value.root_constant_index;
        raw
    }
}

/// SRV Vulkan binding pair produced by the SRV remapper callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SrvVulkanBinding {
    /// The buffer binding.
    pub buffer_binding: VulkanBinding,
    /// The offset binding (used for typed-buffer offsets).
    pub offset_binding: VulkanBinding,
}

impl From<SrvVulkanBinding> for sys::dxil_spv_srv_vulkan_binding {
    fn from(value: SrvVulkanBinding) -> Self {
        Self {
            buffer_binding: value.buffer_binding.into(),
            offset_binding: value.offset_binding.into(),
        }
    }
}

/// UAV Vulkan binding triple produced by the UAV remapper callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UavVulkanBinding {
    /// The buffer binding.
    pub buffer_binding: VulkanBinding,
    /// The counter binding (if the UAV has a counter).
    pub counter_binding: VulkanBinding,
    /// The offset binding (used for typed-buffer offsets).
    pub offset_binding: VulkanBinding,
}

impl From<UavVulkanBinding> for sys::dxil_spv_uav_vulkan_binding {
    fn from(value: UavVulkanBinding) -> Self {
        Self {
            buffer_binding: value.buffer_binding.into(),
            counter_binding: value.counter_binding.into(),
            offset_binding: value.offset_binding.into(),
        }
    }
}

/// CBV Vulkan binding produced by the CBV remapper callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CbvVulkanBinding {
    /// Map to a regular uniform binding.
    Uniform(VulkanBinding),
    /// Map to a push-constant block at the given word offset.
    PushConstant {
        /// Offset in 32-bit words from the start of the push-constant block.
        offset_in_words: u32,
    },
}

impl From<CbvVulkanBinding> for sys::dxil_spv_cbv_vulkan_binding {
    fn from(value: CbvVulkanBinding) -> Self {
        let mut raw = sys::dxil_spv_cbv_vulkan_binding::default();
        match value {
            CbvVulkanBinding::Uniform(b) => {
                raw.vulkan.uniform_binding = b.into();
                raw.push_constant = 0;
            }
            CbvVulkanBinding::PushConstant { offset_in_words } => {
                raw.vulkan.push_constant.offset_in_words = offset_in_words;
                raw.push_constant = 1;
            }
        }
        raw
    }
}

/// D3D vertex input description passed to the vertex-input remapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct D3dVertexInput {
    /// Semantic name (e.g. `"POSITION"`).
    pub semantic: String,
    /// Semantic index.
    pub semantic_index: u32,
    /// Starting row.
    pub start_row: u32,
    /// Number of rows.
    pub rows: u32,
}

impl From<&sys::dxil_spv_d3d_vertex_input> for D3dVertexInput {
    fn from(raw: &sys::dxil_spv_d3d_vertex_input) -> Self {
        let semantic = if raw.semantic.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(raw.semantic) }
                .to_string_lossy()
                .into_owned()
        };
        Self {
            semantic,
            semantic_index: raw.semantic_index,
            start_row: raw.start_row,
            rows: raw.rows,
        }
    }
}

/// Vulkan vertex input produced by the vertex-input remapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VulkanVertexInput {
    /// Vulkan vertex attribute location.
    pub location: u32,
}

impl From<VulkanVertexInput> for sys::dxil_spv_vulkan_vertex_input {
    fn from(value: VulkanVertexInput) -> Self {
        Self {
            location: value.location,
        }
    }
}

/// D3D stream-output description passed to the stream-output remapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct D3dStreamOutput {
    /// Semantic name.
    pub semantic: String,
    /// Semantic index.
    pub semantic_index: u32,
}

impl From<&sys::dxil_spv_d3d_stream_output> for D3dStreamOutput {
    fn from(raw: &sys::dxil_spv_d3d_stream_output) -> Self {
        let semantic = if raw.semantic.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(raw.semantic) }
                .to_string_lossy()
                .into_owned()
        };
        Self {
            semantic,
            semantic_index: raw.semantic_index,
        }
    }
}

/// Vulkan stream output produced by the stream-output remapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VulkanStreamOutput {
    /// Offset within the stream-output buffer.
    pub offset: u32,
    /// Stride between vertices.
    pub stride: u32,
    /// Stream-output buffer index.
    pub buffer_index: u32,
    /// Whether the output is enabled.
    pub enable: bool,
}

impl From<VulkanStreamOutput> for sys::dxil_spv_vulkan_stream_output {
    fn from(value: VulkanStreamOutput) -> Self {
        Self {
            offset: value.offset,
            stride: value.stride,
            buffer_index: value.buffer_index,
            enable: if value.enable { 1 } else { 0 },
        }
    }
}

/// D3D shader-stage I/O description passed to the stage I/O remappers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct D3dShaderStageIo {
    /// Semantic name.
    pub semantic: String,
    /// Semantic index.
    pub semantic_index: u32,
    /// Starting row.
    pub start_row: u32,
    /// Number of rows.
    pub rows: u32,
}

impl From<&sys::dxil_spv_d3d_shader_stage_io> for D3dShaderStageIo {
    fn from(raw: &sys::dxil_spv_d3d_shader_stage_io) -> Self {
        let semantic = if raw.semantic.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(raw.semantic) }
                .to_string_lossy()
                .into_owned()
        };
        Self {
            semantic,
            semantic_index: raw.semantic_index,
            start_row: raw.start_row,
            rows: raw.rows,
        }
    }
}

/// Vulkan shader-stage I/O produced by the stage I/O remappers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VulkanShaderStageIo {
    /// Vulkan location.
    pub location: u32,
    /// Vulkan component.
    pub component: u32,
    /// Flags (e.g. `PER_PRIMITIVE`).
    pub flags: VulkanShaderStageIoFlags,
}

/// Flags for [`VulkanShaderStageIo`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VulkanShaderStageIoFlags {
    /// No flags.
    None,
    /// Per-primitive I/O (mesh shaders).
    PerPrimitive,
}

impl From<VulkanShaderStageIo> for sys::dxil_spv_vulkan_shader_stage_io {
    fn from(value: VulkanShaderStageIo) -> Self {
        let flags = match value.flags {
            VulkanShaderStageIoFlags::None => {
                sys::dxil_spv_vulkan_shader_stage_io_flags_DXIL_SPV_SHADER_STAGE_IO_NONE
            }
            VulkanShaderStageIoFlags::PerPrimitive => {
                sys::dxil_spv_vulkan_shader_stage_io_flags_DXIL_SPV_SHADER_STAGE_IO_PER_PRIMITIVE
            }
        };
        Self {
            location: value.location,
            component: value.component,
            flags: flags as _,
        }
    }
}
