//! Safe Rust bindings to [dxil-spirv](https://github.com/HansKristian-Work/dxil-spirv).
//!
//! Converts D3D11/D3D12 shader bytecode (DXBC container or DXIL bitcode) into
//! SPIR-V, suitable for feeding into cross-compilers such as SPIRV-Cross to
//! obtain HLSL/GLSL/MSL source, or for direct consumption by Vulkan tooling.
//!
//! # Example
//!
//! ```no_run
//! let blob: Vec<u8> = std::fs::read("shader.dxil")?;
//! let spirv = dxil_spirv::convert_to_spirv(&blob)?;
//! println!("produced {} SPIR-V words", spirv.len());
//! # Ok::<(), dxil_spirv::Error>(())
//! ```

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

mod converter;
mod error;
mod parsed_blob;
mod stage;

pub use converter::Converter;
pub use error::{Error, Result};
pub use parsed_blob::ParsedBlob;
pub use stage::ShaderStage;

/// Returns the upstream dxil-spirv version as `(major, minor, patch)`.
pub fn version() -> (u32, u32, u32) {
    let (mut major, mut minor, mut patch) = (0u32, 0u32, 0u32);
    unsafe {
        dxil_spirv_sys::dxil_spv_get_version(&mut major, &mut minor, &mut patch);
    }
    (major, minor, patch)
}

/// One-shot convenience: parse a shader blob and convert it to SPIR-V words.
///
/// `blob` may be a full DXBC container (SM4/SM5/SM6) or a raw DXIL bitcode
/// slice. The returned vector contains little-endian SPIR-V `u32` words.
pub fn convert_to_spirv(blob: &[u8]) -> Result<Vec<u32>> {
    let parsed = ParsedBlob::parse(blob)?;
    let converter = Converter::new(&parsed)?;
    converter.run()?;
    converter.compiled_spirv()
}
