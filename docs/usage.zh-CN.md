# 使用指南

[English](usage.md) | [中文](usage.zh-CN.md)

`dxil-spirv` 提供对 `dxil-spirv` 的安全 Rust 绑定。它将 Direct3D 着色器字节码（DXBC 容器和原始 DXIL bitcode）转换为 SPIR-V word，供 Vulkan 工具链或下游使用 SPIRV-Cross 进行交叉编译。

## 添加依赖

在 `Cargo.toml` 中添加 `dxil-spirv`：

```toml
[dependencies]
dxil-spirv = "0.1"
```

该 crate 在编译期间构建内置的 C++ 核心。你的构建环境中需要具备 C++17 编译器和 CMake。关于 Windows、Linux 和 macOS 上的完整工具链要求，参见 [platform-support.md](platform-support.zh-CN.md)。

## 快速开始

对于默认设置即可满足需求的基础转换，使用 `convert_to_spirv`。

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

该单次转换函数会自动检测输入缓冲区是 DXBC 容器（Shader Model 4、5 或 6）还是原始 DXIL bitcode。它生成符合标准 SPIR-V 布局的小端序 `u32` word。

## 分阶段转换管线

当需要检查 entry point、调整翻译选项或重映射 binding 时，可以显式分阶段驱动。

```text
Bytecode Slice -> ParsedBlob -> Converter -> SPIR-V Words
```

### 1. 解析与内省（`ParsedBlob`）

`ParsedBlob` 用于解析二进制 blob 并持有解码后的 shader model 表示。

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

如果着色器 blob 的反射元数据被分离到了附属文件中，可使用 `ParsedBlob::parse_reflection` 加载附属元数据。

### 2. 配置转换器（`Converter`）

从 `ParsedBlob` 创建 `Converter`。在转换前配置目标选项、注册重映射回调并选择 entry point。

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

## 配置与重映射

上游 Direct3D 着色器将资源组织在寄存器空间（`t0`、`u0`、`b0`、`s0`）中，而 Vulkan 使用 descriptor set 和 binding 索引（`set = X, binding = Y`）。`dxil-spirv` 提供了完善的控制机制来自定义这一映射。

### 转换器选项

`ConverterOption` 涵盖了编译器支持的所有代码生成开关。可以在运行时通过 `ConverterOption::is_supported` 检查特性可用性。

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

### 资源重映射回调

在 `Converter` 上注册闭包，以动态将 Direct3D binding 映射到 Vulkan 位置。Remapper 在 `converter.run()` 执行期间同步运行。

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

其他可用的 remapper 注册方法包括：
- `set_uav_remapper`：无序访问视图（Unordered Access Views，即 storage buffer 和 storage image）
- `set_cbv_remapper`：常量缓冲区视图（Constant Buffer Views，即 UBO 或 push constant 块）
- `set_vertex_input_remapper`：顶点属性输入位置
- `set_stage_input_remapper`：跨阶段输入接口匹配
- `set_stage_output_remapper`：跨阶段输出接口匹配
- `set_stream_output_remapper`：几何着色器变换反馈（transform feedback）位置

### Root Constant 与 Descriptor Table

直接在 `Converter` 上配置 push constant 和 D3D12 root signature：

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

## 错误处理

所有函数均返回 `dxil_spirv::Result<T>`，它包装了 `dxil_spirv::Error`。

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

错误枚举通过 `thiserror` 实现了 `std::error::Error` 和 `Display`。

## 线程安全与诊断

`ParsedBlob` 和 `Converter` 实现了 `Send`，但特意未实现 `Sync`。

- 可以在一个线程上创建 `ParsedBlob` 并将其传递到另一个线程。
- 在同一个 converter 句柄上并发运行 `Converter::run` 是不安全的。
- 如需并行编译着色器，请在每个线程中分别构建独立的 `Converter` 实例。

### 线程日志回调

使用 `set_thread_log_callback` 接收内部编译器诊断和错误日志：

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

日志状态是线程局部的。在每个需要诊断日志的工作线程中调用 `set_thread_log_callback` 即可。

### 线程分配器上下文

对于批量转换或内存紧张的环境，可以使用 `ThreadAllocatorContext` 管理内存：

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

## 限制与平台注意事项

- **输入格式**：支持 DXBC（SM4/SM5/SM6）和 DXIL bitcode。不支持旧版 DX9 SM3 及更早的字节码。
- **平台矩阵**：支持 Windows x86_64、Linux x86_64 和 macOS Apple Silicon。详细平台说明记录在 [platform-support.md](platform-support.zh-CN.md) 中。
- **子进程安全性**：上游 C++ assert 失败无法在同一进程内捕获。转换任意不受信任着色器的测试套件应在子进程中运行转换。

## 下一步

- 在 [testing.md](testing.zh-CN.md) 中了解测试套件如何针对 829 个测试着色器验证转换。
- 在 [architecture.md](architecture.zh-CN.md) 中探索项目架构与 FFI 设计。
- 查看顶层 [README.md](../README.zh-CN.md) 获取快速入门指引和项目结构。
