//! Shader stage enumeration mirroring `dxil_spv_shader_stage`.

use dxil_spirv_sys as sys;

/// The pipeline stage a shader belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    /// Unknown / not specified.
    Unknown,
    /// Vertex shader.
    Vertex,
    /// Hull (tessellation control) shader.
    Hull,
    /// Domain (tessellation evaluation) shader.
    Domain,
    /// Geometry shader.
    Geometry,
    /// Pixel (fragment) shader.
    Pixel,
    /// Compute shader.
    Compute,
    /// Ray tracing intersection shader.
    Intersection,
    /// Ray tracing closest-hit shader.
    ClosestHit,
    /// Ray tracing miss shader.
    Miss,
    /// Ray tracing any-hit shader.
    AnyHit,
    /// Ray generation shader.
    RayGeneration,
    /// Callable shader.
    Callable,
    /// Amplification (task) shader.
    Amplification,
    /// Mesh shader.
    Mesh,
}

impl From<sys::dxil_spv_shader_stage> for ShaderStage {
    fn from(value: sys::dxil_spv_shader_stage) -> Self {
        match value {
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_VERTEX => Self::Vertex,
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_HULL => Self::Hull,
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_DOMAIN => Self::Domain,
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_GEOMETRY => Self::Geometry,
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_PIXEL => Self::Pixel,
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_COMPUTE => Self::Compute,
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_INTERSECTION => Self::Intersection,
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_CLOSEST_HIT => Self::ClosestHit,
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_MISS => Self::Miss,
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_ANY_HIT => Self::AnyHit,
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_RAY_GENERATION => Self::RayGeneration,
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_CALLABLE => Self::Callable,
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_AMPLIFICATION => Self::Amplification,
            sys::dxil_spv_shader_stage_DXIL_SPV_STAGE_MESH => Self::Mesh,
            _ => Self::Unknown,
        }
    }
}
