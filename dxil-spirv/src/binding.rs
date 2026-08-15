//! Typed Rust representations of the D3D and Vulkan binding structs used by
//! the resource remapper callbacks.

use crate::stage::ShaderStage;
use dxil_spirv_sys as sys;
use std::ffi::CStr;

/// The kind of a D3D shader resource (texture, buffer, sampler, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {    /// Invalid / unknown.
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

/// D3D12 resource class for root signature mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceClass {
    /// Shader resource view (SRV).
    Srv,
    /// Unordered access view (UAV).
    Uav,
    /// Constant buffer view (CBV).
    Cbv,
    /// Sampler state.
    Sampler,
}

impl From<ResourceClass> for sys::dxil_spv_resource_class {
    fn from(value: ResourceClass) -> Self {
        match value {
            ResourceClass::Srv => sys::dxil_spv_resource_class_DXIL_SPV_RESOURCE_CLASS_SRV,
            ResourceClass::Uav => sys::dxil_spv_resource_class_DXIL_SPV_RESOURCE_CLASS_UAV,
            ResourceClass::Cbv => sys::dxil_spv_resource_class_DXIL_SPV_RESOURCE_CLASS_CBV,
            ResourceClass::Sampler => {
                sys::dxil_spv_resource_class_DXIL_SPV_RESOURCE_CLASS_SAMPLER
            }
        }
    }
}

/// Meta descriptor identifier for advanced Vulkan features.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetaDescriptor {
    /// Size of the resource descriptor heap.
    ResourceDescriptorHeapSize,
    /// Raw view into the descriptor heap.
    RawDescriptorHeapView,
    /// Dynamic view instancing offsets.
    DynamicViewInstancingOffsets,
    /// Dynamic view instancing mask.
    DynamicViewInstancingMask,
}

impl From<MetaDescriptor> for sys::dxil_spv_meta_descriptor {
    fn from(value: MetaDescriptor) -> Self {
        match value {
            MetaDescriptor::ResourceDescriptorHeapSize => {
                sys::dxil_spv_meta_descriptor_DXIL_SPV_META_DESCRIPTOR_RESOURCE_DESCRIPTOR_HEAP_SIZE
            }
            MetaDescriptor::RawDescriptorHeapView => {
                sys::dxil_spv_meta_descriptor_DXIL_SPV_META_DESCRIPTOR_RAW_DESCRIPTOR_HEAP_VIEW
            }
            MetaDescriptor::DynamicViewInstancingOffsets => {
                sys::dxil_spv_meta_descriptor_DXIL_SPV_META_DESCRIPTOR_DYNAMIC_VIEW_INSTANCING_OFFSETS
            }
            MetaDescriptor::DynamicViewInstancingMask => {
                sys::dxil_spv_meta_descriptor_DXIL_SPV_META_DESCRIPTOR_DYNAMIC_VIEW_INSTANCING_MASK
            }
        }
    }
}

/// How a meta descriptor is exposed to Vulkan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetaDescriptorKind {
    /// Invalid / unused.
    Invalid,
    /// As a push constant.
    PushConstant,
    /// As a push constant buffer device address.
    PushBda,
    /// As a UBO containing a constant.
    UboContainingConstant,
    /// As a UBO containing a buffer device address.
    UboContainingBda,
    /// As a read-only SSBO.
    ReadonlySsbo,
}

impl From<MetaDescriptorKind> for sys::dxil_spv_meta_descriptor_kind {
    fn from(value: MetaDescriptorKind) -> Self {
        match value {
            MetaDescriptorKind::Invalid => {
                sys::dxil_spv_meta_descriptor_kind_DXIL_SPV_META_DESCRIPTOR_KIND_INVALID
            }
            MetaDescriptorKind::PushConstant => {
                sys::dxil_spv_meta_descriptor_kind_DXIL_SPV_META_DESCRIPTOR_KIND_PUSH_CONSTANT
            }
            MetaDescriptorKind::PushBda => {
                sys::dxil_spv_meta_descriptor_kind_DXIL_SPV_META_DESCRIPTOR_KIND_PUSH_BDA
            }
            MetaDescriptorKind::UboContainingConstant => {
                sys::dxil_spv_meta_descriptor_kind_DXIL_SPV_META_DESCRIPTOR_KIND_UBO_CONTAINING_CONSTANT
            }
            MetaDescriptorKind::UboContainingBda => {
                sys::dxil_spv_meta_descriptor_kind_DXIL_SPV_META_DESCRIPTOR_KIND_UBO_CONTAINING_BDA
            }
            MetaDescriptorKind::ReadonlySsbo => {
                sys::dxil_spv_meta_descriptor_kind_DXIL_SPV_META_DESCRIPTOR_KIND_READONLY_SSBO
            }
        }
    }
}

/// RDAT (Runtime Data) subobject kind for DXR state objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RdatSubobjectKind {
    /// State object configuration.
    StateObjectConfig,
    /// Global root signature.
    GlobalRootSignature,
    /// Local root signature.
    LocalRootSignature,
    /// Subobject to exports association.
    SubobjectToExportsAssociation,
    /// Raytracing shader configuration.
    RaytracingShaderConfig,
    /// Raytracing pipeline configuration.
    RaytracingPipelineConfig,
    /// Hit group.
    HitGroup,
    /// Raytracing pipeline configuration (DXIL 1.1).
    RaytracingPipelineConfig1,
    /// Unknown / invalid.
    Unknown,
}

impl From<sys::dxil_spv_rdat_subobject_kind> for RdatSubobjectKind {
    fn from(value: sys::dxil_spv_rdat_subobject_kind) -> Self {
        match value {
            sys::dxil_spv_rdat_subobject_kind_DXIL_SPV_RDAT_SUBOBJECT_KIND_STATE_OBJECT_CONFIG => {
                Self::StateObjectConfig
            }
            sys::dxil_spv_rdat_subobject_kind_DXIL_SPV_RDAT_SUBOBJECT_KIND_GLOBAL_ROOT_SIGNATURE => {
                Self::GlobalRootSignature
            }
            sys::dxil_spv_rdat_subobject_kind_DXIL_SPV_RDAT_SUBOBJECT_KIND_LOCAL_ROOT_SIGNATURE => {
                Self::LocalRootSignature
            }
            sys::dxil_spv_rdat_subobject_kind_DXIL_SPV_RDAT_SUBOBJECT_KIND_SUBOBJECT_TO_EXPORTS_ASSOCIATION => {
                Self::SubobjectToExportsAssociation
            }
            sys::dxil_spv_rdat_subobject_kind_DXIL_SPV_RDAT_SUBOBJECT_KIND_RAYTRACING_SHADER_CONFIG => {
                Self::RaytracingShaderConfig
            }
            sys::dxil_spv_rdat_subobject_kind_DXIL_SPV_RDAT_SUBOBJECT_KIND_RAYTRACING_PIPELINE_CONFIG => {
                Self::RaytracingPipelineConfig
            }
            sys::dxil_spv_rdat_subobject_kind_DXIL_SPV_RDAT_SUBOBJECT_KIND_HIT_GROUP => {
                Self::HitGroup
            }
            sys::dxil_spv_rdat_subobject_kind_DXIL_SPV_RDAT_SUBOBJECT_KIND_RAYTRACING_PIPELINE_CONFIG1 => {
                Self::RaytracingPipelineConfig1
            }
            _ => Self::Unknown,
        }
    }
}

/// An RDAT subobject extracted from a parsed blob.
///
/// RDAT (Runtime Data) subobjects are used in DXR (DirectX Raytracing)
/// state objects to describe pipeline configuration, root signatures,
/// hit groups, and associations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdatSubobject {
    /// The subobject kind.
    pub kind: RdatSubobjectKind,
    /// The subobject name (e.g. hit group name, export name).
    pub name: String,
    /// Hit group type (only valid for `HitGroup` kind).
    pub hit_group_type: u32,
    /// Export names referenced by this subobject.
    pub exports: Vec<String>,
    /// Raw payload data.
    pub payload: Vec<u8>,
}

/// Log level for the dxil-spirv thread log callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    /// Debug message.
    Debug,
    /// Warning message.
    Warn,
    /// Error message.
    Error,
}

impl From<sys::dxil_spv_log_level> for LogLevel {
    fn from(value: sys::dxil_spv_log_level) -> Self {
        match value {
            sys::dxil_spv_log_level_DXIL_SPV_LOG_LEVEL_DEBUG => Self::Debug,
            sys::dxil_spv_log_level_DXIL_SPV_LOG_LEVEL_WARN => Self::Warn,
            sys::dxil_spv_log_level_DXIL_SPV_LOG_LEVEL_ERROR => Self::Error,
            _ => Self::Debug,
        }
    }
}

/// Work Graphs node input data (SM6.8+).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeInputData {
    /// Node ID (often same as entry point name).
    pub node_id: String,
    /// Payload stride; 0 means EmptyNode.
    pub payload_stride: u32,
    /// Launch type.
    pub launch_type: u32,
    /// Node array index.
    pub node_array_index: u32,
    /// Dispatch grid offset.
    pub dispatch_grid_offset: u32,
    /// Dispatch grid type bits.
    pub dispatch_grid_type_bits: u32,
    /// Dispatch grid components.
    pub dispatch_grid_components: u32,
    /// Broadcast grid dimensions.
    pub broadcast_grid: [u32; 3],
    /// Thread group size spec IDs.
    pub thread_group_size_spec_id: [u32; 3],
    /// Max broadcast grid spec IDs.
    pub max_broadcast_grid_spec_id: [u32; 3],
    /// Recursion factor.
    pub recursion_factor: u32,
    /// Coalesce factor.
    pub coalesce_factor: u32,
    /// Node share input ID.
    pub node_share_input_id: String,
    /// Node share input array index.
    pub node_share_input_array_index: u32,
    /// Local root arguments table index.
    pub local_root_arguments_table_index: u32,
    /// Is indirect BDA stride program entry spec ID.
    pub is_indirect_bda_stride_program_entry_spec_id: u32,
    /// Is entry point spec ID.
    pub is_entry_point_spec_id: u32,
    /// Dispatch grid is upper bound spec ID.
    pub dispatch_grid_is_upper_bound_spec_id: u32,
    /// Is static broadcast node spec ID.
    pub is_static_broadcast_node_spec_id: u32,
    /// Dispatch grid is upper bound.
    pub dispatch_grid_is_upper_bound: bool,
    /// Node track RW input sharing.
    pub node_track_rw_input_sharing: bool,
    /// Is program entry.
    pub is_program_entry: bool,
}

impl From<sys::dxil_spv_node_input_data> for NodeInputData {
    fn from(raw: sys::dxil_spv_node_input_data) -> Self {
        Self {
            node_id: if raw.node_id.is_null() {
                String::new()
            } else {
                unsafe { std::ffi::CStr::from_ptr(raw.node_id) }
                    .to_string_lossy()
                    .into_owned()
            },
            payload_stride: raw.payload_stride,
            launch_type: raw.launch_type as u32,
            node_array_index: raw.node_array_index,
            dispatch_grid_offset: raw.dispatch_grid_offset,
            dispatch_grid_type_bits: raw.dispatch_grid_type_bits,
            dispatch_grid_components: raw.dispatch_grid_components,
            broadcast_grid: raw.broadcast_grid,
            thread_group_size_spec_id: raw.thread_group_size_spec_id,
            max_broadcast_grid_spec_id: raw.max_broadcast_grid_spec_id,
            recursion_factor: raw.recursion_factor,
            coalesce_factor: raw.coalesce_factor,
            node_share_input_id: if raw.node_share_input_id.is_null() {
                String::new()
            } else {
                unsafe { std::ffi::CStr::from_ptr(raw.node_share_input_id) }
                    .to_string_lossy()
                    .into_owned()
            },
            node_share_input_array_index: raw.node_share_input_array_index,
            local_root_arguments_table_index: raw.local_root_arguments_table_index,
            is_indirect_bda_stride_program_entry_spec_id: raw
                .is_indirect_bda_stride_program_entry_spec_id,
            is_entry_point_spec_id: raw.is_entry_point_spec_id,
            dispatch_grid_is_upper_bound_spec_id: raw.dispatch_grid_is_upper_bound_spec_id,
            is_static_broadcast_node_spec_id: raw.is_static_broadcast_node_spec_id,
            dispatch_grid_is_upper_bound: raw.dispatch_grid_is_upper_bound != 0,
            node_track_rw_input_sharing: raw.node_track_rw_input_sharing != 0,
            is_program_entry: raw.is_program_entry != 0,
        }
    }
}

/// Work Graphs node output data (SM6.8+).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeOutputData {
    /// Node ID.
    pub node_id: String,
    /// Node array index.
    pub node_array_index: u32,
    /// Node array size (`u32::MAX` = unbounded).
    pub node_array_size: u32,
    /// Node index spec constant ID.
    pub node_index_spec_constant_id: u32,
    /// Max records.
    pub max_records: u32,
    /// Sparse array flag.
    pub sparse_array: bool,
}

impl From<sys::dxil_spv_node_output_data> for NodeOutputData {
    fn from(raw: sys::dxil_spv_node_output_data) -> Self {
        Self {
            node_id: if raw.node_id.is_null() {
                String::new()
            } else {
                unsafe { std::ffi::CStr::from_ptr(raw.node_id) }
                    .to_string_lossy()
                    .into_owned()
            },
            node_array_index: raw.node_array_index,
            node_array_size: raw.node_array_size,
            node_index_spec_constant_id: raw.node_index_spec_constant_id,
            max_records: raw.max_records,
            sparse_array: raw.sparse_array != 0,
        }
    }
}
