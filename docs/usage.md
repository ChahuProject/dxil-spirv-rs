# Usage Guide

`dxil-spirv` provides safe Rust bindings to `dxil-spirv`. It converts Direct3D shader bytecode (DXBC containers and raw DXIL bitcode) into SPIR-V words for Vulkan tooling or downstream cross-compilation with SPIRV-Cross.

## Adding the Dependency

Add `dxil-spirv` to your `Cargo.toml`:

```toml
[dependencies]
dxil-spirv = "0.1"
```

The crate builds the vendored C++ core during compilation. You'll need a C++17 compiler and CMake available in your build environment. See [platform-support.md](platform-support.md) for full toolchain requirements across Windows, Linux, and macOS.

## Quick Start

For basic conversions where default settings are sufficient, use `convert_to_spirv`.

```rust
use dxil_spirv::Result;

fn main() -> Result<()> {
    // Read DXBC or DXIL bytecode from disk
    let blob = std::fs::read("shader.dxil").expect("failed to read shader file");

    // Convert directly into SPIR-V binary words
    let spirv_words: Vec<u32> = dxil_spirv::convert_to_spirv(&blob)?;
    println!("Generated {} SPIR-V words", spirv_words.len());

    // You can now pass spirv_words to Vulkan or SPIRV-Cross
    Ok(())
}
```

The one-shot function auto-detects whether the input buffer is a DXBC container (Shader Model 4, 5, or 6) or raw DXIL bitcode. It produces little-endian `u32` words matching standard SPIR-V layout.

## The Staged Conversion Pipeline

When you need to inspect entry points, tune translation options, or remap bindings, drive the stages explicitly.

```text
Bytecode Slice -> ParsedBlob -> Converter -> SPIR-V Words
```

### 1. Parsing and Introspection (`ParsedBlob`)

`ParsedBlob` parses the binary blob and holds the decoded shader model representation.

```rust
use dxil_spirv::{ParsedBlob, Result, ShaderStage};

fn main() -> Result<()> {
    let blob = std::fs::read("shader.dxbc").expect("failed to read shader file");
    let parsed = ParsedBlob::parse(&blob)?;

    let stage: ShaderStage = parsed.shader_stage();
    let count = parsed.num_entry_points()?;
    println!("Primary stage: {:?}, total entry points: {}", stage, count);

    for i in 0..count {
        let name = parsed.entry_point_name(i)?;
        let demangled = parsed.entry_point_demangled_name(i)?;
        println!("  [{}] {} (demangled: {})", i, name, demangled);
    }

    // Inspect LLVM IR for DXIL shaders
    if let Ok(disassembly) = parsed.disassembled_ir() {
        println!("LLVM IR Preview:\n{}", &disassembly[..disassembly.len().min(200)]);
    }

    Ok(())
}
```

If your shader blob had reflection metadata stripped into a companion file, use `ParsedBlob::parse_reflection` to load the companion metadata.

### 2. Configuring the Converter (`Converter`)

Create a `Converter` from the `ParsedBlob`. Configure target options, register remapping callbacks, and choose the entry point before conversion.

```rust
use dxil_spirv::options::ConverterOption;
use dxil_spirv::{Converter, ParsedBlob, Result};

fn main() -> Result<()> {
    let blob = std::fs::read("compute.dxil").expect("failed to read shader");
    let parsed = ParsedBlob::parse(&blob)?;

    let mut converter = Converter::new(&parsed)?;

    // Select entry point if the blob contains multiple entries
    converter.set_entry_point("CSMain")?;

    // Enable target-specific features
    converter.add_option(&ConverterOption::ShaderDemoteToHelper { supported: true })?;
    converter.add_option(&ConverterOption::MinPrecisionNative16Bit { enabled: true })?;

    // Execute translation
    converter.run()?;

    // Retrieve generated SPIR-V binary words
    let spirv: Vec<u32> = converter.compiled_spirv()?;
    println!("Compiled SPIR-V size: {} words", spirv.len());

    // Query compute layout determined during translation
    let (x, y, z) = converter.compute_workgroup_dimensions()?;
    println!("Workgroup dimensions: ({}, {}, {})", x, y, z);

    Ok(())
}
```

## Configuration and Remapping

Upstream Direct3D shaders organize resources into register spaces (`t0`, `u0`, `b0`, `s0`), while Vulkan uses descriptor sets and binding indices (`set = X, binding = Y`). `dxil-spirv` provides comprehensive controls to customize this mapping.

### Converter Options

`ConverterOption` covers all code generation toggles supported by the compiler. Check feature availability at runtime using `ConverterOption::is_supported`.

```rust
use dxil_spirv::options::{ConverterOption, ShaderQuirk};
use dxil_spirv::{Converter, ParsedBlob, Result};

fn configure_options(converter: &mut Converter) -> Result<()> {
    // Configure dual-source blending for fragment shaders
    let dual_blend = ConverterOption::DualSourceBlending { enabled: true };
    if dual_blend.is_supported() {
        converter.add_option(&dual_blend)?;
    }

    // Configure scalar block layout
    converter.add_option(&ConverterOption::ScalarBlockLayout {
        supported: true,
        supports_per_component_robustness: true,
    })?;

    // Apply hardware-specific quirks when needed
    converter.add_option(&ConverterOption::ShaderQuirk {
        quirk: ShaderQuirk::FixupRsqrtInfNan,
    })?;

    Ok(())
}
```

### Resource Remapping Callbacks

Register closures on `Converter` to map Direct3D bindings to Vulkan locations on the fly. Remappers run synchronously during `converter.run()`.

```rust
use dxil_spirv::binding::{
    Bindless, D3dBinding, SrvVulkanBinding, VulkanBinding, VulkanDescriptorType,
};
use dxil_spirv::{Converter, ParsedBlob, Result};

fn setup_remappers(converter: &mut Converter) {
    // Remap SRV (texture/buffer) bindings
    converter.set_srv_remapper(|d3d: &D3dBinding| -> Option<SrvVulkanBinding> {
        let vulkan = VulkanBinding {
            set: d3d.register_space,
            binding: d3d.register_index,
            root_constant_index: 0,
            bindless: Bindless {
                heap_root_offset: 0,
                use_heap: false,
            },
            descriptor_type: VulkanDescriptorType::Identity,
        };

        Some(SrvVulkanBinding {
            buffer_binding: vulkan,
            offset_binding: vulkan,
        })
    });

    // Remap Sampler bindings
    converter.set_sampler_remapper(|d3d: &D3dBinding| -> Option<VulkanBinding> {
        Some(VulkanBinding {
            set: 1,
            binding: d3d.register_index,
            root_constant_index: 0,
            bindless: Bindless {
                heap_root_offset: 0,
                use_heap: false,
            },
            descriptor_type: VulkanDescriptorType::Identity,
        })
    });
}
```

Other available remapper registration methods include:
- `set_uav_remapper`: Unordered Access Views (storage buffers and storage images)
- `set_cbv_remapper`: Constant Buffer Views (UBOs or push constant blocks)
- `set_vertex_input_remapper`: Vertex attribute input locations
- `set_stage_input_remapper`: Inter-stage input interface matching
- `set_stage_output_remapper`: Inter-stage output interface matching
- `set_stream_output_remapper`: Geometry shader transform feedback locations

### Root Constants and Descriptor Tables

Configure push constants and D3D12 root signatures directly on `Converter`:

```rust
use dxil_spirv::binding::ResourceClass;
use dxil_spirv::Converter;

fn configure_root_layout(converter: &mut Converter) {
    // Reserve 16 words (64 bytes) for root push constants
    converter.set_root_constant_word_count(16);

    // Map a local root constant register
    converter.add_local_root_constants(0, 0, 4);

    // Map root parameters to descriptor set and binding indices
    converter.add_root_descriptor_mapping(0, 0, 0);

    // Define local root descriptor tables for raytracing pipelines
    converter.begin_local_root_descriptor_table().ok();
    converter.add_local_root_descriptor_table(ResourceClass::Srv, 0, 0, 8, 0);
    converter.end_local_root_descriptor_table().ok();
}
```

## Error Handling

All functions return `dxil_spirv::Result<T>`, wrapping `dxil_spirv::Error`.

```rust
use dxil_spirv::{Error, Result};

fn run_conversion(bytes: &[u8]) {
    match dxil_spirv::convert_to_spirv(bytes) {
        Ok(words) => println!("Success: {} words", words.len()),
        Err(Error::EmptyInput) => eprintln!("Error: Input buffer was empty"),
        Err(Error::UnsupportedFeature(tag)) => {
            eprintln!("Error: Converter option {} is unsupported", tag);
        }
        Err(Error::InvalidString) => eprintln!("Error: String argument contained interior NUL"),
        Err(Error::NoOutput) => eprintln!("Error: Converter yielded no output"),
        Err(Error::DxilSpirv(code)) => eprintln!("Error: C++ library returned error code {}", code),
    }
}
```

The error enum implements `std::error::Error` and `Display` via `thiserror`.

## Thread Safety and Diagnostics

`ParsedBlob` and `Converter` implement `Send`, but deliberately omit `Sync`.

- You can create a `ParsedBlob` on one thread and transfer it to another.
- Running `Converter::run` concurrently on the same converter handle is unsafe.
- For parallel shader compilation, construct separate `Converter` instances per thread.

### Thread Logging Callback

Receive internal compiler diagnostics and error logs with `set_thread_log_callback`:

```rust
use dxil_spirv::binding::LogLevel;

fn setup_logging() {
    dxil_spirv::set_thread_log_callback(Some(|level: LogLevel, msg: &str| {
        match level {
            LogLevel::Error => eprintln!("[dxil-spirv ERROR] {}", msg),
            LogLevel::Warn => eprintln!("[dxil-spirv WARN] {}", msg),
            LogLevel::Debug => println!("[dxil-spirv DEBUG] {}", msg),
        }
    }));
}
```

Logging state is thread-local. Call `set_thread_log_callback` on each worker thread that needs diagnostic logs.

### Thread Allocator Context

For batch conversions or tight memory environments, manage memory with `ThreadAllocatorContext`:

```rust
use dxil_spirv::ThreadAllocatorContext;

fn batch_compile(blobs: &[Vec<u8>]) {
    // Activates thread-local allocation arena until dropped
    let arena = ThreadAllocatorContext::begin();

    for blob in blobs {
        if let Ok(spirv) = dxil_spirv::convert_to_spirv(blob) {
            println!("Compiled shader ({} words)", spirv.len());
        }
        // Optional: clear memory between iterations
        arena.reset();
    }
}
```

## Limitations and Platform Notes

- **Input Formats**: Accepts DXBC (SM4/SM5/SM6) and DXIL bitcode. Legacy DX9 SM3 and earlier bytecodes are not supported.
- **Platform Matrix**: Supported on Windows x86_64, Linux x86_64, and macOS Apple Silicon. Detailed platform notes are documented in [platform-support.md](platform-support.md).
- **Subprocess Safety**: Upstream C++ assert failures cannot be caught within the same process. Test suites translating arbitrary untrusted shaders should run conversions in child processes.

## Where to Go Next

- Learn how the test suite verifies translation against 829 test shaders in [testing.md](testing.md).
- Explore project architecture and FFI design in [architecture.md](architecture.md).
- Check the top-level [README.md](../README.md) for quick orientation and project layout.
