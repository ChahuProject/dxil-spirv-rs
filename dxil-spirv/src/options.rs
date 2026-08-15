//! Typed wrapper for the `dxil_spv_option_*` argument structs.
//!
//! Upstream exposes a single tagged struct per option, all sharing a common
//! `dxil_spv_option_base` header. The safe API exposes a single
//! [`ConverterOption`] enum with one variant per upstream option; the
//! [`Converter::add_option`](crate::Converter::add_option) method converts
//! the enum into the matching raw struct and forwards it.

use dxil_spirv_sys as sys;
use std::ffi::{c_uint, CString};

/// All converter options supported by this version of dxil-spirv.
///
/// The enum variants map 1:1 to `dxil_spv_option` values; each variant
/// carries the fields of the matching `dxil_spv_option_*` struct.
///
/// Note: `supports_option` can be queried with [`Converter::supports_option`]
/// to detect whether a given option is recognized by the linked library.
#[derive(Debug, Clone, PartialEq)]
pub enum ConverterOption {
    /// `DXIL_SPV_OPTION_SHADER_DEMOTE_TO_HELPER` — support demote-to-helper.
    ShaderDemoteToHelper {
        /// Whether the feature is supported.
        supported: bool,
    },
    /// `DXIL_SPV_OPTION_DUAL_SOURCE_BLENDING` — enable dual-source blending.
    DualSourceBlending {
        /// Whether dual-source blending is enabled.
        enabled: bool,
    },
    /// `DXIL_SPV_OPTION_OUTPUT_SWIZZLE` — per-target output component swizzle.
    OutputSwizzle {
        /// One `u32` per render target; each holds 2-bit component indices
        /// packed as `R | G<<2 | B<<4 | A<<6`.
        swizzles: Vec<u32>,
    },
    /// `DXIL_SPV_OPTION_RASTERIZER_SAMPLE_COUNT` — forced sample count.
    RasterizerSampleCount {
        /// The number of samples to declare.
        sample_count: u32,
        /// Emit a specialization constant instead of a literal.
        spec_constant: bool,
    },
    /// `DXIL_SPV_OPTION_ROOT_CONSTANT_INLINE_UNIFORM_BLOCK` — map a root
    /// constant block to an inline uniform block descriptor.
    RootConstantInlineUniformBlock {
        /// Vulkan descriptor set.
        desc_set: u32,
        /// Vulkan binding within the set.
        binding: u32,
        /// Whether the mapping is enabled.
        enable: bool,
    },
    /// `DXIL_SPV_OPTION_BINDLESS_CBV_SSBO_EMULATION` — emulate CBVs as SSBOs.
    BindlessCbvSsboEmulation {
        /// Whether the emulation is enabled.
        enable: bool,
    },
    /// `DXIL_SPV_OPTION_PHYSICAL_STORAGE_BUFFER` — use physical storage buffer.
    PhysicalStorageBuffer {
        /// Whether the feature is enabled.
        enable: bool,
    },
    /// `DXIL_SPV_OPTION_SBT_DESCRIPTOR_SIZE_LOG2` — SBT descriptor sizes.
    SbtDescriptorSizeLog2 {
        /// `log2` of the SRV/UAV/CBV descriptor size.
        size_log2_srv_uav_cbv: u32,
        /// `log2` of the sampler descriptor size.
        size_log2_sampler: u32,
    },
    /// `DXIL_SPV_OPTION_SSBO_ALIGNMENT` — minimum SSBO alignment.
    SsboAlignment {
        /// Alignment in bytes.
        alignment: u32,
    },
    /// `DXIL_SPV_OPTION_TYPED_UAV_READ_WITHOUT_FORMAT` — typed UAV reads
    /// without a format.
    TypedUavReadWithoutFormat {
        /// Whether the feature is supported.
        supported: bool,
    },
    /// `DXIL_SPV_OPTION_SHADER_SOURCE_FILE` — source file name embedded in
    /// debug info.
    ShaderSourceFile {
        /// The file name; copied by the implementation.
        name: CString,
    },
    /// `DXIL_SPV_OPTION_BINDLESS_TYPED_BUFFER_OFFSETS` — typed buffer offsets.
    BindlessTypedBufferOffsets {
        /// Whether the feature is enabled.
        enable: bool,
    },
    /// `DXIL_SPV_OPTION_BINDLESS_OFFSET_BUFFER_LAYOUT` — offset buffer layout.
    BindlessOffsetBufferLayout {
        /// Offset for untyped buffers.
        untyped_offset: u32,
        /// Offset for typed buffers.
        typed_offset: u32,
        /// Stride between entries.
        stride: u32,
    },
    /// `DXIL_SPV_OPTION_STORAGE_INPUT_OUTPUT_16BIT` — 16-bit storage I/O.
    StorageInputOutput16Bit {
        /// Whether the feature is supported.
        supported: bool,
    },
    /// `DXIL_SPV_OPTION_DESCRIPTOR_QA` — descriptor QA instrumentation.
    DescriptorQa {
        /// Whether descriptor QA is enabled.
        enabled: bool,
        /// Interface version.
        version: u32,
        /// Global descriptor set.
        global_desc_set: u32,
        /// Global binding.
        global_binding: u32,
        /// Heap descriptor set.
        heap_desc_set: u32,
        /// Heap binding.
        heap_binding: u32,
        /// Hash identifying the shader.
        shader_hash: u64,
    },
    /// `DXIL_SPV_OPTION_MIN_PRECISION_NATIVE_16BIT` — native 16-bit min-precision.
    MinPrecisionNative16Bit {
        /// Whether the feature is enabled.
        enabled: bool,
    },
    /// `DXIL_SPV_OPTION_SHADER_I8_DOT` — 8-bit integer dot product.
    ShaderI8Dot {
        /// Whether the feature is supported.
        supported: bool,
    },
    /// `DXIL_SPV_OPTION_SHADER_RAY_TRACING_PRIMITIVE_CULLING` — RT primitive culling.
    ShaderRayTracingPrimitiveCulling {
        /// Whether the feature is supported.
        supported: bool,
    },
    /// `DXIL_SPV_OPTION_INVARIANT_POSITION` — invariant position output.
    InvariantPosition {
        /// Whether the feature is enabled.
        enabled: bool,
    },
    /// `DXIL_SPV_OPTION_SCALAR_BLOCK_LAYOUT` — scalar block layout.
    ScalarBlockLayout {
        /// Whether the feature is supported.
        supported: bool,
        /// Whether per-component robustness is supported.
        supports_per_component_robustness: bool,
    },
    /// `DXIL_SPV_OPTION_BARYCENTRIC_KHR` — KHR barycentric coordinates.
    BarycentricKhr {
        /// Whether the feature is supported.
        supported: bool,
    },
    /// `DXIL_SPV_OPTION_ROBUST_PHYSICAL_CBV_LOAD` — robust physical CBV loads
    /// (obsolete; prefer the shader-quirk variant).
    RobustPhysicalCbvLoad {
        /// Whether the feature is enabled.
        enabled: bool,
    },
    /// `DXIL_SPV_OPTION_ARITHMETIC_RELAXED_PRECISION` — relaxed-precision arithmetic.
    ArithmeticRelaxedPrecision {
        /// Whether the feature is enabled.
        enabled: bool,
    },
    /// `DXIL_SPV_OPTION_PHYSICAL_ADDRESS_DESCRIPTOR_INDEXING` — physical-address
    /// descriptor indexing.
    PhysicalAddressDescriptorIndexing {
        /// Stride between descriptor elements.
        element_stride: u32,
        /// Offset to the first element.
        element_offset: u32,
    },
    /// `DXIL_SPV_OPTION_FORCE_SUBGROUP_SIZE` — force a specific subgroup size.
    ForceSubgroupSize {
        /// The subgroup size to force.
        forced_value: u32,
        /// Whether wave-size forcing is enabled.
        wave_size_enable: bool,
    },
    /// `DXIL_SPV_OPTION_DENORM_PRESERVE_SUPPORT` — denorm preservation.
    DenormPreserveSupport {
        /// Whether 16-bit denorm preserve is supported.
        supports_float16_denorm_preserve: bool,
        /// Whether 64-bit denorm preserve is supported.
        supports_float64_denorm_preserve: bool,
    },
    /// `DXIL_SPV_OPTION_STRICT_HELPER_LANE_WAVE_OPS` — strict helper-lane wave ops.
    StrictHelperLaneWaveOps {
        /// Whether the feature is enabled.
        enable: bool,
    },
    /// `DXIL_SPV_OPTION_SUBGROUP_PARTITIONED_NV` — NV partitioned subgroups.
    SubgroupPartitionedNv {
        /// Whether the feature is supported.
        supported: bool,
    },
    /// `DXIL_SPV_OPTION_DEAD_CODE_ELIMINATE` — dead-code elimination.
    DeadCodeEliminate {
        /// Whether the pass is enabled.
        enabled: bool,
    },
    /// `DXIL_SPV_OPTION_PRECISE_CONTROL` — precise value tracking.
    PreciseControl {
        /// Force precise on all values.
        force_precise: bool,
        /// Propagate precise through expressions.
        propagate_precise: bool,
    },
    /// `DXIL_SPV_OPTION_SAMPLE_GRAD_OPTIMIZATION_CONTROL` — sample/grad
    /// optimization control.
    SampleGradOptimizationControl {
        /// Whether the optimization is enabled.
        enabled: bool,
        /// Assume uniform scale for gradients.
        assume_uniform_scale: bool,
    },
    /// `DXIL_SPV_OPTION_OPACITY_MICROMAP` — opacity micromap support.
    OpacityMicromap {
        /// Enable opacity micromap for `TraceRay`.
        trace_ray_enabled: bool,
        /// Force OMM execution mode in legacy shader models.
        ray_query_force_omm_execution_mode_in_legacy_sm: bool,
    },
    /// `DXIL_SPV_OPTION_BRANCH_CONTROL` — branch control hints.
    BranchControl {
        /// Use shader metadata for branch hints.
        use_shader_metadata: bool,
        /// Force unrolling.
        force_unroll: bool,
        /// Force loops.
        force_loop: bool,
        /// Force flattening.
        force_flatten: bool,
        /// Force branching.
        force_branch: bool,
    },
    /// `DXIL_SPV_OPTION_SUBGROUP_PROPERTIES` — subgroup size properties.
    SubgroupProperties {
        /// Minimum subgroup size.
        minimum_size: u32,
        /// Maximum subgroup size.
        maximum_size: u32,
    },
    /// `DXIL_SPV_OPTION_DESCRIPTOR_HEAP_ROBUSTNESS` — descriptor heap robustness.
    DescriptorHeapRobustness {
        /// Whether the feature is enabled.
        enabled: bool,
    },
    /// `DXIL_SPV_OPTION_COMPUTE_SHADER_DERIVATIVES_NV` — NV compute derivatives.
    ComputeShaderDerivativesNv {
        /// Whether the feature is supported.
        supported: bool,
    },
    /// `DXIL_SPV_OPTION_QUAD_CONTROL_RECONVERGENCE` — quad control & reconvergence.
    QuadControlReconvergence {
        /// Whether quad control is supported.
        supports_quad_control: bool,
        /// Whether maximal reconvergence is supported.
        supports_maximal_reconvergence: bool,
        /// Force maximal reconvergence.
        force_maximal_reconvergence: bool,
    },
    /// `DXIL_SPV_OPTION_RAW_ACCESS_CHAINS_NV` — NV raw access chains.
    RawAccessChainsNv {
        /// Whether the feature is supported.
        supported: bool,
    },
    /// `DXIL_SPV_OPTION_DRIVER_VERSION` — driver identification.
    DriverVersion {
        /// Driver ID.
        driver_id: u32,
        /// Driver version.
        driver_version: u32,
    },
    /// `DXIL_SPV_OPTION_COMPUTE_SHADER_DERIVATIVES` — compute derivatives.
    ComputeShaderDerivatives {
        /// Whether NV derivatives are supported.
        supports_nv: bool,
        /// Whether KHR derivatives are supported.
        supports_khr: bool,
    },
    /// `DXIL_SPV_OPTION_INSTRUCTION_INSTRUMENTATION` — instruction-level
    /// instrumentation.
    InstructionInstrumentation {
        /// Whether instrumentation is enabled.
        enabled: bool,
        /// Interface version.
        version: u32,
        /// Control descriptor set.
        control_desc_set: u32,
        /// Control binding.
        control_binding: u32,
        /// Payload descriptor set.
        payload_desc_set: u32,
        /// Payload binding.
        payload_binding: u32,
        /// Hash identifying the shader.
        shader_hash: u64,
        /// Instrumentation type.
        kind: InstructionInstrumentationType,
    },
    /// `DXIL_SPV_OPTION_SHADER_QUIRK` — enable a shader quirk.
    ShaderQuirk {
        /// The quirk to enable.
        quirk: ShaderQuirk,
    },
    /// `DXIL_SPV_OPTION_EXTENDED_ROBUSTNESS` — extended robustness checks.
    ExtendedRobustness {
        /// Robustness for group-shared memory.
        robust_group_shared: bool,
        /// Robustness for `alloca`.
        robust_alloca: bool,
        /// Robustness for constant look-up tables.
        robust_constant_lut: bool,
    },
    /// `DXIL_SPV_OPTION_MAX_TESS_FACTOR` — maximum tessellation factor.
    MaxTessFactor {
        /// The maximum tessellation factor.
        max_tess_factor: u32,
    },
    /// `DXIL_SPV_OPTION_VULKAN_MEMORY_MODEL` — Vulkan memory model.
    VulkanMemoryModel {
        /// Whether the memory model is enabled.
        enabled: bool,
    },
    /// `DXIL_SPV_OPTION_FLOAT8_SUPPORT` — FP8 support.
    Float8Support {
        /// WMMA FP8.
        wmma_fp8: bool,
        /// NV cooperative-matrix-2 conversions.
        nv_cooperative_matrix2_conversions: bool,
    },
    /// `DXIL_SPV_OPTION_NVAPI` — NVAPI integration.
    Nvapi {
        /// Whether NVAPI is enabled.
        enabled: bool,
        /// Register index for the NVAPI UAV.
        register_index: u32,
        /// Register space for the NVAPI UAV.
        register_space: u32,
    },
    /// `DXIL_SPV_OPTION_EXTENDED_NON_SEMANTIC` — extended non-semantic info.
    ExtendedNonSemantic {
        /// Whether the feature is enabled.
        enabled: bool,
    },
    /// `DXIL_SPV_OPTION_MIXED_FLOAT_DOT_PRODUCT` — mixed-precision dot product.
    MixedFloatDotProduct {
        /// FP16 × FP16 → FP32.
        fp16_fp16_fp32: bool,
    },
    /// `DXIL_SPV_OPTION_COMPUTE_SHADER_DERIVATIVES_QUAD` — quad compute derivatives.
    ComputeShaderDerivativesQuad {
        /// Whether quad derivatives are supported.
        supports_quad: bool,
    },
    /// `DXIL_SPV_OPTION_SSBO_ADDRESSING_BEHAVIOR` — SSBO addressing behavior.
    SsboAddressingBehavior {
        /// SSBO wraps 32-bit offset before robustness.
        ssbo_wraps_32bit_offset_before_robustness: bool,
        /// Raw access chains wrap 32-bit offset before robustness.
        raw_access_chain_wraps_32bit_offset_before_robustness: bool,
    },
    /// `DXIL_SPV_OPTION_FLOAT_CONTROLS_2` — extended float controls.
    FloatControls2 {
        /// Whether the feature is supported.
        supported: bool,
    },
}

/// Shader quirks that can be enabled via [`ConverterOption::ShaderQuirk`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderQuirk {
    /// No quirk.
    None,
    /// Promote thread-group coherence barriers to device-scope.
    ForceDeviceMemoryBarriersThreadGroupCoherence,
    /// Assume broken sub-8×8 cube mips.
    AssumeBrokenSub8x8CubeMips,
    /// Robust physical CBV forwarding.
    RobustPhysicalCbvForwarding,
    /// Mesh outputs robustness.
    MeshOutputsRobustness,
    /// Aggressive non-uniform resource indexing.
    AggressiveNonUniform,
    /// Robust physical CBV loads.
    RobustPhysicalCbv,
    /// Promote group barriers to device memory barriers.
    PromoteGroupToDeviceMemoryBarrier,
    /// Auto-insert group-shared barriers.
    GroupSharedAutoBarrier,
    /// Fix up loop-header undef phis.
    FixupLoopHeaderUndefPhis,
    /// Fix up `rsqrt` inf/nan.
    FixupRsqrtInfNan,
    /// Ignore primitive shading rate.
    IgnorePrimitiveShadingRate,
    /// Robust compute quad broadcast.
    RobustComputeQuadBroadcast,
    /// Force precise FMA.
    PreciseFma,
    /// Clamp wave size to thread-group 32.
    ClampWaveSizeToThreadGroup32,
    /// Non-semantic signal for concurrent workgroups.
    NonSemanticSignalConcurrentWorkgroup,
    /// Force non-uniform indexing everywhere.
    ForceNonUniform,
}

impl From<ShaderQuirk> for sys::dxil_spv_shader_quirk {
    fn from(value: ShaderQuirk) -> Self {
        match value {
            ShaderQuirk::None => sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_NONE,
            ShaderQuirk::ForceDeviceMemoryBarriersThreadGroupCoherence => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_FORCE_DEVICE_MEMORY_BARRIERS_THREAD_GROUP_COHERENCE
            }
            ShaderQuirk::AssumeBrokenSub8x8CubeMips => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_ASSUME_BROKEN_SUB_8x8_CUBE_MIPS
            }
            ShaderQuirk::RobustPhysicalCbvForwarding => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_ROBUST_PHYSICAL_CBV_FORWARDING
            }
            ShaderQuirk::MeshOutputsRobustness => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_MESH_OUTPUTS_ROBUSTNESS
            }
            ShaderQuirk::AggressiveNonUniform => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_AGGRESSIVE_NONUNIFORM
            }
            ShaderQuirk::RobustPhysicalCbv => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_ROBUST_PHYSICAL_CBV
            }
            ShaderQuirk::PromoteGroupToDeviceMemoryBarrier => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_PROMOTE_GROUP_TO_DEVICE_MEMORY_BARRIER
            }
            ShaderQuirk::GroupSharedAutoBarrier => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_GROUP_SHARED_AUTO_BARRIER
            }
            ShaderQuirk::FixupLoopHeaderUndefPhis => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_FIXUP_LOOP_HEADER_UNDEF_PHIS
            }
            ShaderQuirk::FixupRsqrtInfNan => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_FIXUP_RSQRT_INF_NAN
            }
            ShaderQuirk::IgnorePrimitiveShadingRate => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_IGNORE_PRIMITIVE_SHADING_RATE
            }
            ShaderQuirk::RobustComputeQuadBroadcast => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_ROBUST_COMPUTE_QUAD_BROADCAST
            }
            ShaderQuirk::PreciseFma => sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_PRECISE_FMA,
            ShaderQuirk::ClampWaveSizeToThreadGroup32 => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_CLAMP_WAVE_SIZE_TO_THREAD_GROUP_32
            }
            ShaderQuirk::NonSemanticSignalConcurrentWorkgroup => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_NON_SEMANTIC_SIGNAL_CONCURRENT_WORKGROUP
            }
            ShaderQuirk::ForceNonUniform => {
                sys::dxil_spv_shader_quirk_DXIL_SPV_SHADER_QUIRK_FORCE_NONUNIFORM
            }
        }
    }
}

/// Instruction instrumentation types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstructionInstrumentationType {
    /// Full NaN/Inf instrumentation.
    FullNanInf,
    /// Instrument externally-visible writes only.
    ExternallyVisibleWriteNanInf,
    /// Flush NaN to zero.
    FlushNanToZero,
    /// `expect` / `assume` instrumentation.
    ExpectAssume,
    /// Buffer synchronization validation.
    BufferSynchronizationValidation,
}

impl From<InstructionInstrumentationType> for sys::dxil_spv_instruction_instrumentation_type {
    fn from(value: InstructionInstrumentationType) -> Self {
        match value {
            InstructionInstrumentationType::FullNanInf => {
                sys::dxil_spv_instruction_instrumentation_type_DXIL_SPV_INSTRUCTION_INSTRUMENTATION_TYPE_FULL_NAN_INF
            }
            InstructionInstrumentationType::ExternallyVisibleWriteNanInf => {
                sys::dxil_spv_instruction_instrumentation_type_DXIL_SPV_INSTRUCTION_INSTRUMENTATION_TYPE_EXTERNALLY_VISIBLE_WRITE_NAN_INF
            }
            InstructionInstrumentationType::FlushNanToZero => {
                sys::dxil_spv_instruction_instrumentation_type_DXIL_SPV_INSTRUCTION_INSTRUMENTATION_TYPE_FLUSH_NAN_TO_ZERO
            }
            InstructionInstrumentationType::ExpectAssume => {
                sys::dxil_spv_instruction_instrumentation_type_DXIL_SPV_INSTRUCTION_INSTRUMENTATION_TYPE_EXPECT_ASSUME
            }
            InstructionInstrumentationType::BufferSynchronizationValidation => {
                sys::dxil_spv_instruction_instrumentation_type_DXIL_SPV_INSTRUCTION_INSTRUMENTATION_TYPE_BUFFER_SYNCHRONIZATION_VALIDATION
            }
        }
    }
}

impl ConverterOption {
    /// The `dxil_spv_option` tag for this option.
    pub fn kind(&self) -> sys::dxil_spv_option {
        match self {
            Self::ShaderDemoteToHelper { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_SHADER_DEMOTE_TO_HELPER
            }
            Self::DualSourceBlending { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_DUAL_SOURCE_BLENDING
            }
            Self::OutputSwizzle { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_OUTPUT_SWIZZLE,
            Self::RasterizerSampleCount { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_RASTERIZER_SAMPLE_COUNT
            }
            Self::RootConstantInlineUniformBlock { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_ROOT_CONSTANT_INLINE_UNIFORM_BLOCK
            }
            Self::BindlessCbvSsboEmulation { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_BINDLESS_CBV_SSBO_EMULATION
            }
            Self::PhysicalStorageBuffer { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_PHYSICAL_STORAGE_BUFFER
            }
            Self::SbtDescriptorSizeLog2 { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_SBT_DESCRIPTOR_SIZE_LOG2
            }
            Self::SsboAlignment { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_SSBO_ALIGNMENT,
            Self::TypedUavReadWithoutFormat { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_TYPED_UAV_READ_WITHOUT_FORMAT
            }
            Self::ShaderSourceFile { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_SHADER_SOURCE_FILE
            }
            Self::BindlessTypedBufferOffsets { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_BINDLESS_TYPED_BUFFER_OFFSETS
            }
            Self::BindlessOffsetBufferLayout { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_BINDLESS_OFFSET_BUFFER_LAYOUT
            }
            Self::StorageInputOutput16Bit { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_STORAGE_INPUT_OUTPUT_16BIT
            }
            Self::DescriptorQa { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_DESCRIPTOR_QA,
            Self::MinPrecisionNative16Bit { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_MIN_PRECISION_NATIVE_16BIT
            }
            Self::ShaderI8Dot { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_SHADER_I8_DOT,
            Self::ShaderRayTracingPrimitiveCulling { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_SHADER_RAY_TRACING_PRIMITIVE_CULLING
            }
            Self::InvariantPosition { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_INVARIANT_POSITION
            }
            Self::ScalarBlockLayout { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_SCALAR_BLOCK_LAYOUT
            }
            Self::BarycentricKhr { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_BARYCENTRIC_KHR,
            Self::RobustPhysicalCbvLoad { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_ROBUST_PHYSICAL_CBV_LOAD
            }
            Self::ArithmeticRelaxedPrecision { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_ARITHMETIC_RELAXED_PRECISION
            }
            Self::PhysicalAddressDescriptorIndexing { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_PHYSICAL_ADDRESS_DESCRIPTOR_INDEXING
            }
            Self::ForceSubgroupSize { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_FORCE_SUBGROUP_SIZE
            }
            Self::DenormPreserveSupport { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_DENORM_PRESERVE_SUPPORT
            }
            Self::StrictHelperLaneWaveOps { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_STRICT_HELPER_LANE_WAVE_OPS
            }
            Self::SubgroupPartitionedNv { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_SUBGROUP_PARTITIONED_NV
            }
            Self::DeadCodeEliminate { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_DEAD_CODE_ELIMINATE
            }
            Self::PreciseControl { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_PRECISE_CONTROL,
            Self::SampleGradOptimizationControl { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_SAMPLE_GRAD_OPTIMIZATION_CONTROL
            }
            Self::OpacityMicromap { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_OPACITY_MICROMAP,
            Self::BranchControl { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_BRANCH_CONTROL,
            Self::SubgroupProperties { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_SUBGROUP_PROPERTIES
            }
            Self::DescriptorHeapRobustness { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_DESCRIPTOR_HEAP_ROBUSTNESS
            }
            Self::ComputeShaderDerivativesNv { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_COMPUTE_SHADER_DERIVATIVES_NV
            }
            Self::QuadControlReconvergence { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_QUAD_CONTROL_RECONVERGENCE
            }
            Self::RawAccessChainsNv { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_RAW_ACCESS_CHAINS_NV
            }
            Self::DriverVersion { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_DRIVER_VERSION,
            Self::ComputeShaderDerivatives { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_COMPUTE_SHADER_DERIVATIVES
            }
            Self::InstructionInstrumentation { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_INSTRUCTION_INSTRUMENTATION
            }
            Self::ShaderQuirk { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_SHADER_QUIRK,
            Self::ExtendedRobustness { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_EXTENDED_ROBUSTNESS
            }
            Self::MaxTessFactor { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_MAX_TESS_FACTOR,
            Self::VulkanMemoryModel { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_VULKAN_MEMORY_MODEL
            }
            Self::Float8Support { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_FLOAT8_SUPPORT,
            Self::Nvapi { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_NVAPI,
            Self::ExtendedNonSemantic { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_EXTENDED_NON_SEMANTIC
            }
            Self::MixedFloatDotProduct { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_MIXED_FLOAT_DOT_PRODUCT
            }
            Self::ComputeShaderDerivativesQuad { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_COMPUTE_SHADER_DERIVATIVES_QUAD
            }
            Self::SsboAddressingBehavior { .. } => {
                sys::dxil_spv_option_DXIL_SPV_OPTION_SSBO_ADDRESSING_BEHAVIOR
            }
            Self::FloatControls2 { .. } => sys::dxil_spv_option_DXIL_SPV_OPTION_FLOAT_CONTROLS_2,
        }
    }

    /// Returns `true` if the linked dxil-spirv library recognizes this option.
    pub fn is_supported(&self) -> bool {
        let raw = self.kind();
        unsafe { sys::dxil_spv_converter_supports_option(raw) == 1 }
    }

    /// Convert to the raw `dxil_spv_option_base`-derived struct.
    ///
    /// The returned tuple owns the struct plus any heap data (e.g. `Vec<u32>`
    /// or `CString`) that the raw struct points into.
    pub(crate) fn to_raw(&self) -> (sys::dxil_spv_option_base, RawOptionData) {
        let kind = self.kind();
        let base = sys::dxil_spv_option_base { type_: kind };
        match self {
            Self::ShaderDemoteToHelper { supported } => {
                let raw = sys::dxil_spv_option_shader_demote_to_helper {
                    base,
                    supported: bool_to_dxil(*supported),
                };
                (raw.base, RawOptionData::ShaderDemoteToHelper(raw))
            }
            Self::DualSourceBlending { enabled } => {
                let raw = sys::dxil_spv_option_dual_source_blending {
                    base,
                    enabled: bool_to_dxil(*enabled),
                };
                (raw.base, RawOptionData::DualSourceBlending(raw))
            }
            Self::OutputSwizzle { swizzles } => {
                let raw = sys::dxil_spv_option_output_swizzle {
                    base,
                    swizzles: swizzles.as_ptr(),
                    swizzle_count: swizzles.len() as c_uint,
                };
                (
                    raw.base,
                    RawOptionData::OutputSwizzle(raw, swizzles.clone()),
                )
            }
            Self::RasterizerSampleCount {
                sample_count,
                spec_constant,
            } => {
                let raw = sys::dxil_spv_option_rasterizer_sample_count {
                    base,
                    sample_count: *sample_count,
                    spec_constant: bool_to_dxil(*spec_constant),
                };
                (raw.base, RawOptionData::RasterizerSampleCount(raw))
            }
            Self::RootConstantInlineUniformBlock {
                desc_set,
                binding,
                enable,
            } => {
                let raw = sys::dxil_spv_option_root_constant_inline_uniform_block {
                    base,
                    desc_set: *desc_set,
                    binding: *binding,
                    enable: bool_to_dxil(*enable),
                };
                (raw.base, RawOptionData::RootConstantInlineUniformBlock(raw))
            }
            Self::BindlessCbvSsboEmulation { enable } => {
                let raw = sys::dxil_spv_option_bindless_cbv_ssbo_emulation {
                    base,
                    enable: bool_to_dxil(*enable),
                };
                (raw.base, RawOptionData::BindlessCbvSsboEmulation(raw))
            }
            Self::PhysicalStorageBuffer { enable } => {
                let raw = sys::dxil_spv_option_physical_storage_buffer {
                    base,
                    enable: bool_to_dxil(*enable),
                };
                (raw.base, RawOptionData::PhysicalStorageBuffer(raw))
            }
            Self::SbtDescriptorSizeLog2 {
                size_log2_srv_uav_cbv,
                size_log2_sampler,
            } => {
                let raw = sys::dxil_spv_option_sbt_descriptor_size_log2 {
                    base,
                    size_log2_srv_uav_cbv: *size_log2_srv_uav_cbv,
                    size_log2_sampler: *size_log2_sampler,
                };
                (raw.base, RawOptionData::SbtDescriptorSizeLog2(raw))
            }
            Self::SsboAlignment { alignment } => {
                let raw = sys::dxil_spv_option_ssbo_alignment {
                    base,
                    alignment: *alignment,
                };
                (raw.base, RawOptionData::SsboAlignment(raw))
            }
            Self::TypedUavReadWithoutFormat { supported } => {
                let raw = sys::dxil_spv_option_typed_uav_read_without_format {
                    base,
                    supported: bool_to_dxil(*supported),
                };
                (raw.base, RawOptionData::TypedUavReadWithoutFormat(raw))
            }
            Self::ShaderSourceFile { name } => {
                let raw = sys::dxil_spv_option_shader_source_file {
                    base,
                    name: name.as_ptr(),
                };
                (raw.base, RawOptionData::ShaderSourceFile(raw, name.clone()))
            }
            Self::BindlessTypedBufferOffsets { enable } => {
                let raw = sys::dxil_spv_option_bindless_typed_buffer_offsets {
                    base,
                    enable: bool_to_dxil(*enable),
                };
                (raw.base, RawOptionData::BindlessTypedBufferOffsets(raw))
            }
            Self::BindlessOffsetBufferLayout {
                untyped_offset,
                typed_offset,
                stride,
            } => {
                let raw = sys::dxil_spv_option_bindless_offset_buffer_layout {
                    base,
                    untyped_offset: *untyped_offset,
                    typed_offset: *typed_offset,
                    stride: *stride,
                };
                (raw.base, RawOptionData::BindlessOffsetBufferLayout(raw))
            }
            Self::StorageInputOutput16Bit { supported } => {
                let raw = sys::dxil_spv_option_storage_input_output_16bit {
                    base,
                    supported: bool_to_dxil(*supported),
                };
                (raw.base, RawOptionData::StorageInputOutput16Bit(raw))
            }
            Self::DescriptorQa {
                enabled,
                version,
                global_desc_set,
                global_binding,
                heap_desc_set,
                heap_binding,
                shader_hash,
            } => {
                let raw = sys::dxil_spv_option_descriptor_qa {
                    base,
                    enabled: bool_to_dxil(*enabled),
                    version: *version,
                    global_desc_set: *global_desc_set,
                    global_binding: *global_binding,
                    heap_desc_set: *heap_desc_set,
                    heap_binding: *heap_binding,
                    shader_hash: *shader_hash,
                };
                (raw.base, RawOptionData::DescriptorQa(raw))
            }
            Self::MinPrecisionNative16Bit { enabled } => {
                let raw = sys::dxil_spv_option_min_precision_native_16bit {
                    base,
                    enabled: bool_to_dxil(*enabled),
                };
                (raw.base, RawOptionData::MinPrecisionNative16Bit(raw))
            }
            Self::ShaderI8Dot { supported } => {
                let raw = sys::dxil_spv_option_shader_i8_dot {
                    base,
                    supported: bool_to_dxil(*supported),
                };
                (raw.base, RawOptionData::ShaderI8Dot(raw))
            }
            Self::ShaderRayTracingPrimitiveCulling { supported } => {
                let raw = sys::dxil_spv_option_shader_ray_tracing_primitive_culling {
                    base,
                    supported: bool_to_dxil(*supported),
                };
                (
                    raw.base,
                    RawOptionData::ShaderRayTracingPrimitiveCulling(raw),
                )
            }
            Self::InvariantPosition { enabled } => {
                let raw = sys::dxil_spv_option_invariant_position {
                    base,
                    enabled: bool_to_dxil(*enabled),
                };
                (raw.base, RawOptionData::InvariantPosition(raw))
            }
            Self::ScalarBlockLayout {
                supported,
                supports_per_component_robustness,
            } => {
                let raw = sys::dxil_spv_option_scalar_block_layout {
                    base,
                    supported: bool_to_dxil(*supported),
                    supports_per_component_robustness: bool_to_dxil(
                        *supports_per_component_robustness,
                    ),
                };
                (raw.base, RawOptionData::ScalarBlockLayout(raw))
            }
            Self::BarycentricKhr { supported } => {
                let raw = sys::dxil_spv_option_barycentric_khr {
                    base,
                    supported: bool_to_dxil(*supported),
                };
                (raw.base, RawOptionData::BarycentricKhr(raw))
            }
            Self::RobustPhysicalCbvLoad { enabled } => {
                let raw = sys::dxil_spv_option_robust_physical_cbv_load {
                    base,
                    enabled: bool_to_dxil(*enabled),
                };
                (raw.base, RawOptionData::RobustPhysicalCbvLoad(raw))
            }
            Self::ArithmeticRelaxedPrecision { enabled } => {
                let raw = sys::dxil_spv_option_arithmetic_relaxed_precision {
                    base,
                    enabled: bool_to_dxil(*enabled),
                };
                (raw.base, RawOptionData::ArithmeticRelaxedPrecision(raw))
            }
            Self::PhysicalAddressDescriptorIndexing {
                element_stride,
                element_offset,
            } => {
                let raw = sys::dxil_spv_option_physical_address_descriptor_indexing {
                    base,
                    element_stride: *element_stride,
                    element_offset: *element_offset,
                };
                (
                    raw.base,
                    RawOptionData::PhysicalAddressDescriptorIndexing(raw),
                )
            }
            Self::ForceSubgroupSize {
                forced_value,
                wave_size_enable,
            } => {
                let raw = sys::dxil_spv_option_force_subgroup_size {
                    base,
                    forced_value: *forced_value,
                    wave_size_enable: bool_to_dxil(*wave_size_enable),
                };
                (raw.base, RawOptionData::ForceSubgroupSize(raw))
            }
            Self::DenormPreserveSupport {
                supports_float16_denorm_preserve,
                supports_float64_denorm_preserve,
            } => {
                let raw = sys::dxil_spv_option_denorm_preserve_support {
                    base,
                    supports_float16_denorm_preserve: bool_to_dxil(
                        *supports_float16_denorm_preserve,
                    ),
                    supports_float64_denorm_preserve: bool_to_dxil(
                        *supports_float64_denorm_preserve,
                    ),
                };
                (raw.base, RawOptionData::DenormPreserveSupport(raw))
            }
            Self::StrictHelperLaneWaveOps { enable } => {
                let raw = sys::dxil_spv_option_strict_helper_lane_wave_ops {
                    base,
                    enable: bool_to_dxil(*enable),
                };
                (raw.base, RawOptionData::StrictHelperLaneWaveOps(raw))
            }
            Self::SubgroupPartitionedNv { supported } => {
                let raw = sys::dxil_spv_option_subgroup_partitioned_nv {
                    base,
                    supported: bool_to_dxil(*supported),
                };
                (raw.base, RawOptionData::SubgroupPartitionedNv(raw))
            }
            Self::DeadCodeEliminate { enabled } => {
                let raw = sys::dxil_spv_option_dead_code_eliminate {
                    base,
                    enabled: bool_to_dxil(*enabled),
                };
                (raw.base, RawOptionData::DeadCodeEliminate(raw))
            }
            Self::PreciseControl {
                force_precise,
                propagate_precise,
            } => {
                let raw = sys::dxil_spv_option_precise_control {
                    base,
                    force_precise: bool_to_dxil(*force_precise),
                    propagate_precise: bool_to_dxil(*propagate_precise),
                };
                (raw.base, RawOptionData::PreciseControl(raw))
            }
            Self::SampleGradOptimizationControl {
                enabled,
                assume_uniform_scale,
            } => {
                let raw = sys::dxil_spv_option_sample_grad_optimization_control {
                    base,
                    enabled: bool_to_dxil(*enabled),
                    assume_uniform_scale: bool_to_dxil(*assume_uniform_scale),
                };
                (raw.base, RawOptionData::SampleGradOptimizationControl(raw))
            }
            Self::OpacityMicromap {
                trace_ray_enabled,
                ray_query_force_omm_execution_mode_in_legacy_sm,
            } => {
                let raw = sys::dxil_spv_option_opacity_micromap {
                    base,
                    trace_ray_enabled: bool_to_dxil(*trace_ray_enabled),
                    ray_query_force_omm_execution_mode_in_legacy_sm: bool_to_dxil(
                        *ray_query_force_omm_execution_mode_in_legacy_sm,
                    ),
                };
                (raw.base, RawOptionData::OpacityMicromap(raw))
            }
            Self::BranchControl {
                use_shader_metadata,
                force_unroll,
                force_loop,
                force_flatten,
                force_branch,
            } => {
                let raw = sys::dxil_spv_option_branch_control {
                    base,
                    use_shader_metadata: bool_to_dxil(*use_shader_metadata),
                    force_unroll: bool_to_dxil(*force_unroll),
                    force_loop: bool_to_dxil(*force_loop),
                    force_flatten: bool_to_dxil(*force_flatten),
                    force_branch: bool_to_dxil(*force_branch),
                };
                (raw.base, RawOptionData::BranchControl(raw))
            }
            Self::SubgroupProperties {
                minimum_size,
                maximum_size,
            } => {
                let raw = sys::dxil_spv_option_subgroup_properties {
                    base,
                    minimum_size: *minimum_size,
                    maximum_size: *maximum_size,
                };
                (raw.base, RawOptionData::SubgroupProperties(raw))
            }
            Self::DescriptorHeapRobustness { enabled } => {
                let raw = sys::dxil_spv_option_descriptor_heap_robustness {
                    base,
                    enabled: bool_to_dxil(*enabled),
                };
                (raw.base, RawOptionData::DescriptorHeapRobustness(raw))
            }
            Self::ComputeShaderDerivativesNv { supported } => {
                let raw = sys::dxil_spv_option_compute_shader_derivatives_nv {
                    base,
                    supported: bool_to_dxil(*supported),
                };
                (raw.base, RawOptionData::ComputeShaderDerivativesNv(raw))
            }
            Self::QuadControlReconvergence {
                supports_quad_control,
                supports_maximal_reconvergence,
                force_maximal_reconvergence,
            } => {
                let raw = sys::dxil_spv_option_quad_control_reconvergence {
                    base,
                    supports_quad_control: bool_to_dxil(*supports_quad_control),
                    supports_maximal_reconvergence: bool_to_dxil(*supports_maximal_reconvergence),
                    force_maximal_reconvergence: bool_to_dxil(*force_maximal_reconvergence),
                };
                (raw.base, RawOptionData::QuadControlReconvergence(raw))
            }
            Self::RawAccessChainsNv { supported } => {
                let raw = sys::dxil_spv_option_raw_access_chains_nv {
                    base,
                    supported: bool_to_dxil(*supported),
                };
                (raw.base, RawOptionData::RawAccessChainsNv(raw))
            }
            Self::DriverVersion {
                driver_id,
                driver_version,
            } => {
                let raw = sys::dxil_spv_option_driver_version {
                    base,
                    driver_id: *driver_id,
                    driver_version: *driver_version,
                };
                (raw.base, RawOptionData::DriverVersion(raw))
            }
            Self::ComputeShaderDerivatives {
                supports_nv,
                supports_khr,
            } => {
                let raw = sys::dxil_spv_option_compute_shader_derivatives {
                    base,
                    supports_nv: bool_to_dxil(*supports_nv),
                    supports_khr: bool_to_dxil(*supports_khr),
                };
                (raw.base, RawOptionData::ComputeShaderDerivatives(raw))
            }
            Self::InstructionInstrumentation {
                enabled,
                version,
                control_desc_set,
                control_binding,
                payload_desc_set,
                payload_binding,
                shader_hash,
                kind,
            } => {
                let raw = sys::dxil_spv_option_instruction_instrumentation {
                    base,
                    enabled: bool_to_dxil(*enabled),
                    version: *version,
                    control_desc_set: *control_desc_set,
                    control_binding: *control_binding,
                    payload_desc_set: *payload_desc_set,
                    payload_binding: *payload_binding,
                    shader_hash: *shader_hash,
                    type_: (*kind).into(),
                };
                (raw.base, RawOptionData::InstructionInstrumentation(raw))
            }
            Self::ShaderQuirk { quirk } => {
                let raw = sys::dxil_spv_option_shader_quirk {
                    base,
                    quirk: (*quirk).into(),
                };
                (raw.base, RawOptionData::ShaderQuirk(raw))
            }
            Self::ExtendedRobustness {
                robust_group_shared,
                robust_alloca,
                robust_constant_lut,
            } => {
                let raw = sys::dxil_spv_option_extended_robustness {
                    base,
                    robust_group_shared: bool_to_dxil(*robust_group_shared),
                    robust_alloca: bool_to_dxil(*robust_alloca),
                    robust_constant_lut: bool_to_dxil(*robust_constant_lut),
                };
                (raw.base, RawOptionData::ExtendedRobustness(raw))
            }
            Self::MaxTessFactor { max_tess_factor } => {
                let raw = sys::dxil_spv_option_max_tess_factor {
                    base,
                    max_tess_factor: *max_tess_factor,
                };
                (raw.base, RawOptionData::MaxTessFactor(raw))
            }
            Self::VulkanMemoryModel { enabled } => {
                let raw = sys::dxil_spv_option_vulkan_memory_model {
                    base,
                    enabled: bool_to_dxil(*enabled),
                };
                (raw.base, RawOptionData::VulkanMemoryModel(raw))
            }
            Self::Float8Support {
                wmma_fp8,
                nv_cooperative_matrix2_conversions,
            } => {
                let raw = sys::dxil_spv_option_float8_support {
                    base,
                    wmma_fp8: bool_to_dxil(*wmma_fp8),
                    nv_cooperative_matrix2_conversions: bool_to_dxil(
                        *nv_cooperative_matrix2_conversions,
                    ),
                };
                (raw.base, RawOptionData::Float8Support(raw))
            }
            Self::Nvapi {
                enabled,
                register_index,
                register_space,
            } => {
                let raw = sys::dxil_spv_option_nvapi {
                    base,
                    enabled: bool_to_dxil(*enabled),
                    register_index: *register_index,
                    register_space: *register_space,
                };
                (raw.base, RawOptionData::Nvapi(raw))
            }
            Self::ExtendedNonSemantic { enabled } => {
                let raw = sys::dxil_spv_option_extended_non_semantic {
                    base,
                    enabled: bool_to_dxil(*enabled),
                };
                (raw.base, RawOptionData::ExtendedNonSemantic(raw))
            }
            Self::MixedFloatDotProduct { fp16_fp16_fp32 } => {
                let raw = sys::dxil_spv_option_mixed_float_dot_product {
                    base,
                    fp16_fp16_fp32: bool_to_dxil(*fp16_fp16_fp32),
                    reserved: [0; 4],
                };
                (raw.base, RawOptionData::MixedFloatDotProduct(raw))
            }
            Self::ComputeShaderDerivativesQuad { supports_quad } => {
                let raw = sys::dxil_spv_option_compute_shader_derivatives_quad {
                    base,
                    supports_quad: bool_to_dxil(*supports_quad),
                };
                (raw.base, RawOptionData::ComputeShaderDerivativesQuad(raw))
            }
            Self::SsboAddressingBehavior {
                ssbo_wraps_32bit_offset_before_robustness,
                raw_access_chain_wraps_32bit_offset_before_robustness,
            } => {
                let raw = sys::dxil_spv_option_ssbo_addressing_behavior {
                    base,
                    ssbo_wraps_32bit_offset_before_robustness: bool_to_dxil(
                        *ssbo_wraps_32bit_offset_before_robustness,
                    ),
                    raw_access_chain_wraps_32bit_offset_before_robustness: bool_to_dxil(
                        *raw_access_chain_wraps_32bit_offset_before_robustness,
                    ),
                };
                (raw.base, RawOptionData::SsboAddressingBehavior(raw))
            }
            Self::FloatControls2 { supported } => {
                let raw = sys::dxil_spv_options_float_controls_2 {
                    base,
                    supported: bool_to_dxil(*supported),
                };
                (raw.base, RawOptionData::FloatControls2(raw))
            }
        }
    }
}

/// Owned data that a raw option struct may point into.
///
/// Kept alive alongside the raw struct for the duration of the
/// `dxil_spv_converter_add_option` call.
///
/// Some variants carry owned backing data (`Vec<u32>` swizzle table,
/// `CString` path) whose only job is to keep the pointer stored inside the
/// raw struct valid; that data is never read back, hence `dead_code`.
#[allow(dead_code)]
pub(crate) enum RawOptionData {
    ShaderDemoteToHelper(sys::dxil_spv_option_shader_demote_to_helper),
    DualSourceBlending(sys::dxil_spv_option_dual_source_blending),
    OutputSwizzle(sys::dxil_spv_option_output_swizzle, Vec<u32>),
    RasterizerSampleCount(sys::dxil_spv_option_rasterizer_sample_count),
    RootConstantInlineUniformBlock(sys::dxil_spv_option_root_constant_inline_uniform_block),
    BindlessCbvSsboEmulation(sys::dxil_spv_option_bindless_cbv_ssbo_emulation),
    PhysicalStorageBuffer(sys::dxil_spv_option_physical_storage_buffer),
    SbtDescriptorSizeLog2(sys::dxil_spv_option_sbt_descriptor_size_log2),
    SsboAlignment(sys::dxil_spv_option_ssbo_alignment),
    TypedUavReadWithoutFormat(sys::dxil_spv_option_typed_uav_read_without_format),
    ShaderSourceFile(sys::dxil_spv_option_shader_source_file, CString),
    BindlessTypedBufferOffsets(sys::dxil_spv_option_bindless_typed_buffer_offsets),
    BindlessOffsetBufferLayout(sys::dxil_spv_option_bindless_offset_buffer_layout),
    StorageInputOutput16Bit(sys::dxil_spv_option_storage_input_output_16bit),
    DescriptorQa(sys::dxil_spv_option_descriptor_qa),
    MinPrecisionNative16Bit(sys::dxil_spv_option_min_precision_native_16bit),
    ShaderI8Dot(sys::dxil_spv_option_shader_i8_dot),
    ShaderRayTracingPrimitiveCulling(sys::dxil_spv_option_shader_ray_tracing_primitive_culling),
    InvariantPosition(sys::dxil_spv_option_invariant_position),
    ScalarBlockLayout(sys::dxil_spv_option_scalar_block_layout),
    BarycentricKhr(sys::dxil_spv_option_barycentric_khr),
    RobustPhysicalCbvLoad(sys::dxil_spv_option_robust_physical_cbv_load),
    ArithmeticRelaxedPrecision(sys::dxil_spv_option_arithmetic_relaxed_precision),
    PhysicalAddressDescriptorIndexing(sys::dxil_spv_option_physical_address_descriptor_indexing),
    ForceSubgroupSize(sys::dxil_spv_option_force_subgroup_size),
    DenormPreserveSupport(sys::dxil_spv_option_denorm_preserve_support),
    StrictHelperLaneWaveOps(sys::dxil_spv_option_strict_helper_lane_wave_ops),
    SubgroupPartitionedNv(sys::dxil_spv_option_subgroup_partitioned_nv),
    DeadCodeEliminate(sys::dxil_spv_option_dead_code_eliminate),
    PreciseControl(sys::dxil_spv_option_precise_control),
    SampleGradOptimizationControl(sys::dxil_spv_option_sample_grad_optimization_control),
    OpacityMicromap(sys::dxil_spv_option_opacity_micromap),
    BranchControl(sys::dxil_spv_option_branch_control),
    SubgroupProperties(sys::dxil_spv_option_subgroup_properties),
    DescriptorHeapRobustness(sys::dxil_spv_option_descriptor_heap_robustness),
    ComputeShaderDerivativesNv(sys::dxil_spv_option_compute_shader_derivatives_nv),
    QuadControlReconvergence(sys::dxil_spv_option_quad_control_reconvergence),
    RawAccessChainsNv(sys::dxil_spv_option_raw_access_chains_nv),
    DriverVersion(sys::dxil_spv_option_driver_version),
    ComputeShaderDerivatives(sys::dxil_spv_option_compute_shader_derivatives),
    InstructionInstrumentation(sys::dxil_spv_option_instruction_instrumentation),
    ShaderQuirk(sys::dxil_spv_option_shader_quirk),
    ExtendedRobustness(sys::dxil_spv_option_extended_robustness),
    MaxTessFactor(sys::dxil_spv_option_max_tess_factor),
    VulkanMemoryModel(sys::dxil_spv_option_vulkan_memory_model),
    Float8Support(sys::dxil_spv_option_float8_support),
    Nvapi(sys::dxil_spv_option_nvapi),
    ExtendedNonSemantic(sys::dxil_spv_option_extended_non_semantic),
    MixedFloatDotProduct(sys::dxil_spv_option_mixed_float_dot_product),
    ComputeShaderDerivativesQuad(sys::dxil_spv_option_compute_shader_derivatives_quad),
    SsboAddressingBehavior(sys::dxil_spv_option_ssbo_addressing_behavior),
    FloatControls2(sys::dxil_spv_options_float_controls_2),
}

impl RawOptionData {
    /// Get the `dxil_spv_option_base` pointer for the contained struct.
    pub(crate) fn as_base(&self) -> &sys::dxil_spv_option_base {
        match self {
            Self::ShaderDemoteToHelper(s) => &s.base,
            Self::DualSourceBlending(s) => &s.base,
            Self::OutputSwizzle(s, _) => &s.base,
            Self::RasterizerSampleCount(s) => &s.base,
            Self::RootConstantInlineUniformBlock(s) => &s.base,
            Self::BindlessCbvSsboEmulation(s) => &s.base,
            Self::PhysicalStorageBuffer(s) => &s.base,
            Self::SbtDescriptorSizeLog2(s) => &s.base,
            Self::SsboAlignment(s) => &s.base,
            Self::TypedUavReadWithoutFormat(s) => &s.base,
            Self::ShaderSourceFile(s, _) => &s.base,
            Self::BindlessTypedBufferOffsets(s) => &s.base,
            Self::BindlessOffsetBufferLayout(s) => &s.base,
            Self::StorageInputOutput16Bit(s) => &s.base,
            Self::DescriptorQa(s) => &s.base,
            Self::MinPrecisionNative16Bit(s) => &s.base,
            Self::ShaderI8Dot(s) => &s.base,
            Self::ShaderRayTracingPrimitiveCulling(s) => &s.base,
            Self::InvariantPosition(s) => &s.base,
            Self::ScalarBlockLayout(s) => &s.base,
            Self::BarycentricKhr(s) => &s.base,
            Self::RobustPhysicalCbvLoad(s) => &s.base,
            Self::ArithmeticRelaxedPrecision(s) => &s.base,
            Self::PhysicalAddressDescriptorIndexing(s) => &s.base,
            Self::ForceSubgroupSize(s) => &s.base,
            Self::DenormPreserveSupport(s) => &s.base,
            Self::StrictHelperLaneWaveOps(s) => &s.base,
            Self::SubgroupPartitionedNv(s) => &s.base,
            Self::DeadCodeEliminate(s) => &s.base,
            Self::PreciseControl(s) => &s.base,
            Self::SampleGradOptimizationControl(s) => &s.base,
            Self::OpacityMicromap(s) => &s.base,
            Self::BranchControl(s) => &s.base,
            Self::SubgroupProperties(s) => &s.base,
            Self::DescriptorHeapRobustness(s) => &s.base,
            Self::ComputeShaderDerivativesNv(s) => &s.base,
            Self::QuadControlReconvergence(s) => &s.base,
            Self::RawAccessChainsNv(s) => &s.base,
            Self::DriverVersion(s) => &s.base,
            Self::ComputeShaderDerivatives(s) => &s.base,
            Self::InstructionInstrumentation(s) => &s.base,
            Self::ShaderQuirk(s) => &s.base,
            Self::ExtendedRobustness(s) => &s.base,
            Self::MaxTessFactor(s) => &s.base,
            Self::VulkanMemoryModel(s) => &s.base,
            Self::Float8Support(s) => &s.base,
            Self::Nvapi(s) => &s.base,
            Self::ExtendedNonSemantic(s) => &s.base,
            Self::MixedFloatDotProduct(s) => &s.base,
            Self::ComputeShaderDerivativesQuad(s) => &s.base,
            Self::SsboAddressingBehavior(s) => &s.base,
            Self::FloatControls2(s) => &s.base,
        }
    }
}

fn bool_to_dxil(b: bool) -> sys::dxil_spv_bool {
    if b {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_kind_round_trip() {
        let opt = ConverterOption::ShaderDemoteToHelper { supported: true };
        assert_eq!(
            opt.kind(),
            sys::dxil_spv_option_DXIL_SPV_OPTION_SHADER_DEMOTE_TO_HELPER
        );

        let opt = ConverterOption::DualSourceBlending { enabled: false };
        assert_eq!(
            opt.kind(),
            sys::dxil_spv_option_DXIL_SPV_OPTION_DUAL_SOURCE_BLENDING
        );

        let opt = ConverterOption::SsboAlignment { alignment: 16 };
        assert_eq!(
            opt.kind(),
            sys::dxil_spv_option_DXIL_SPV_OPTION_SSBO_ALIGNMENT
        );
    }

    #[test]
    fn supports_option_matches_linked_library() {
        // The linked library should recognize at least the core options.
        let opt = ConverterOption::ShaderDemoteToHelper { supported: true };
        assert!(opt.is_supported());
    }

    #[test]
    fn option_layout_sanity() {
        // Verify that a simple option struct has the expected size.
        let opt = ConverterOption::ShaderDemoteToHelper { supported: true };
        let (_base, data) = opt.to_raw();
        let base = data.as_base();
        assert_eq!(
            base.type_,
            sys::dxil_spv_option_DXIL_SPV_OPTION_SHADER_DEMOTE_TO_HELPER
        );
    }
}
