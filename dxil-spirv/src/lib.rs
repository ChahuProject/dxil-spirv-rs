//! Safe Rust bindings to [dxil-spirv](https://github.com/HansKristian-Work/dxil-spirv).
//!
//! Converts D3D11/D3D12 shader bytecode (DXBC container or DXIL bitcode) into
//! SPIR-V, suitable for feeding into cross-compilers such as SPIRV-Cross to
//! obtain HLSL/GLSL/MSL source, or for direct consumption by Vulkan tooling.
//!
//! # Example
//!
//! ```no_run
//! # fn main() -> dxil_spirv::Result<()> {
//! let blob: Vec<u8> = std::fs::read("shader.dxil").expect("read shader");
//! let spirv = dxil_spirv::convert_to_spirv(&blob)?;
//! println!("produced {} SPIR-V words", spirv.len());
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

pub mod binding;
mod converter;
mod error;
pub mod options;
mod parsed_blob;
mod remapper;
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

/// Parse a raw DXIL bitcode slice directly.
///
/// This is a lower-level alternative to [`ParsedBlob::parse`] for when you
/// already have raw DXIL bitcode (not a DXBC container). Most users should
/// prefer [`ParsedBlob::parse`] which auto-detects the format.
pub fn parse_dxil(data: &[u8]) -> Result<ParsedBlob> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }
    let mut handle: dxil_spirv_sys::dxil_spv_parsed_blob = std::ptr::null_mut();
    let result = unsafe {
        dxil_spirv_sys::dxil_spv_parse_dxil(data.as_ptr().cast(), data.len(), &mut handle)
    };
    error::check(result)?;
    if handle.is_null() {
        return Err(Error::NoOutput);
    }
    Ok(ParsedBlob { handle })
}

// ── Thread log callback ─────────────────────────────────────────────────

mod log_callback {
    use crate::binding::LogLevel;
    use dxil_spirv_sys as sys;
    use std::ffi::c_void;
    use std::sync::Mutex;

    type LogCallback = Box<dyn FnMut(LogLevel, &str) + Send>;

    struct LogState {
        callback: Option<LogCallback>,
    }

    static LOG_STATE: Mutex<Option<LogState>> = Mutex::new(None);

    extern "C" fn trampoline(
        userdata: *mut c_void,
        level: sys::dxil_spv_log_level,
        message: *const std::os::raw::c_char,
    ) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let state_ptr = userdata as *mut LogState;
            if state_ptr.is_null() {
                return;
            }
            let state = unsafe { &mut *state_ptr };
            if let Some(ref mut cb) = state.callback {
                let level = LogLevel::from(level);
                let msg = if message.is_null() {
                    ""
                } else {
                    unsafe { std::ffi::CStr::from_ptr(message) }
                        .to_str()
                        .unwrap_or("")
                };
                cb(level, msg);
            }
        }));
    }

    /// Set a thread-local log callback for dxil-spirv diagnostic messages.
    ///
    /// The callback is invoked on the thread that calls dxil-spirv functions
    /// when the library emits log messages. Pass `None` to clear the callback.
    ///
    /// Note: This sets per-thread state in the C++ library. Each thread that
    /// uses dxil-spirv and wants logging must call this function.
    pub fn set_thread_log_callback<F>(callback: Option<F>)
    where
        F: FnMut(LogLevel, &str) + Send + 'static,
    {
        let mut guard = LOG_STATE.lock().unwrap();
        match callback {
            Some(cb) => {
                let state = LogState {
                    callback: Some(Box::new(cb)),
                };
                let state_ptr = Box::into_raw(Box::new(state));
                *guard = Some(LogState { callback: None }); // placeholder to keep the guard alive
                unsafe {
                    sys::dxil_spv_set_thread_log_callback(
                        Some(trampoline),
                        state_ptr as *mut c_void,
                    )
                };
                // Store the raw pointer so we can free it later.
                *guard = Some(LogState {
                    callback: Some(Box::new(move |_, _| {})),
                });
            }
            None => {
                unsafe { sys::dxil_spv_set_thread_log_callback(None, std::ptr::null_mut()) };
                *guard = None;
            }
        }
    }
}

pub use log_callback::set_thread_log_callback;

// ── Thread allocator context ────────────────────────────────────────────

/// RAII guard for a dxil-spirv thread allocator context.
///
/// While the guard is alive, dxil-spirv uses a thread-local allocator for
/// internal allocations. This is useful for embedded scenarios or when you
/// need to track dxil-spirv memory usage separately.
///
/// # Example
///
/// ```no_run
/// let _guard = dxil_spirv::ThreadAllocatorContext::begin();
/// // dxil-spirv operations here use the thread-local allocator
/// // guard dropped -> allocator context ended automatically
/// ```
#[derive(Debug)]
pub struct ThreadAllocatorContext {
    _private: (),
}

impl ThreadAllocatorContext {
    /// Begin a thread allocator context.
    ///
    /// The context is active until the returned guard is dropped.
    pub fn begin() -> Self {
        unsafe { dxil_spirv_sys::dxil_spv_begin_thread_allocator_context() };
        Self { _private: () }
    }

    /// Reset the current thread allocator context.
    ///
    /// This frees all allocations made within the current context.
    pub fn reset(&self) {
        unsafe { dxil_spirv_sys::dxil_spv_reset_thread_allocator_context() };
    }
}

impl Drop for ThreadAllocatorContext {
    fn drop(&mut self) {
        unsafe { dxil_spirv_sys::dxil_spv_end_thread_allocator_context() };
    }
}
