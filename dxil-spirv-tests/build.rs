//! Build script for dxil-spirv-tests: synchronizes test shaders from the
//! upstream submodule and compiles them with DXC when available.
//!
//! DXC policy:
//! - Preferred version: 1.9.2602.17 (first production release with SM 6.9)
//! - If a suitable DXC is not found in PATH / DXC_PATH, this build script
//!   downloads the official Microsoft release and caches it under
//!   `target/dxc/1.9.2602.17/`.
//! - The downloaded copy is used only for tests; it never affects the
//!   published `dxil-spirv` or `dxil-spirv-sys` crates.

use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Official Microsoft DXC release that supports Shader Model 6.9.
const DXC_VERSION: &str = "1.9.2602.17";
const DXC_RELEASE_TAG: &str = "v1.9.2602";
/// The release asset uses a date-based name, not the version number.
const DXC_ASSET_NAME: &str = "dxc_2026_02_20.zip";

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().expect("workspace root");

    let upstream_shaders = workspace_root.join("dxil-spirv-sys/dxil-spirv/shaders");
    let upstream_reference = workspace_root.join("dxil-spirv-sys/dxil-spirv/reference/shaders");
    let test_shaders = workspace_root.join("tests/shaders");
    let test_reference = workspace_root.join("tests/reference/shaders");

    // Only sync if upstream submodule is initialized
    if !upstream_shaders.exists() {
        println!("cargo:warning=upstream shaders not found, skipping sync");
        return;
    }

    // Sync shader sources and reference outputs.
    // NOTE: .h and .inc files MUST be synced — shaders use
    // `#include "nvHLSLExtns.h"` / `#include "ags_shader_intrinsics_dx12.inc"`
    // and DXC resolves these relative to the source file, so the headers
    // must sit next to the shaders. Sync everything.
    sync_directory(&upstream_shaders, &test_shaders, &[]);
    sync_directory(&upstream_reference, &test_reference, &[]);

    // Find or download DXC
    let dxc_path = match find_dxc() {
        Some(path) => path,
        None => match download_dxc(workspace_root) {
            Ok(path) => path,
            Err(e) => {
                println!("cargo:warning=failed to download DXC {}: {}", DXC_VERSION, e);
                println!("cargo:warning=shader compilation will be incomplete");
                return;
            }
        },
    };

    println!("cargo:warning=Using DXC at {}", dxc_path.display());
    compile_shaders(&dxc_path, &test_shaders);

    // Generate manifest for completeness check
    generate_manifest(&test_shaders, workspace_root);

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", upstream_shaders.display());
    println!("cargo:rerun-if-changed={}", upstream_reference.display());
}

/// Recursively copy files from src to dst, skipping files with given extensions.
fn sync_directory(src: &Path, dst: &Path, skip_extensions: &[&str]) {
    if !src.exists() {
        return;
    }
    fs::create_dir_all(dst).expect("failed to create destination directory");

    for entry in walkdir(src) {
        let path = entry;

        if path.is_dir() {
            continue;
        }

        // Skip certain extensions (headers, includes)
        if let Some(ext) = path.extension() {
            if skip_extensions.iter().any(|e| ext == *e) {
                continue;
            }
        }

        let rel_path = path.strip_prefix(src).expect("strip prefix");
        let dst_path = dst.join(rel_path);

        // Create parent directories
        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent).expect("failed to create parent directory");
        }

        // Copy file if it doesn't exist or is different
        let should_copy = !dst_path.exists() || {
            let src_content = fs::read(&path).unwrap_or_default();
            let dst_content = fs::read(&dst_path).unwrap_or_default();
            src_content != dst_content
        };

        if should_copy {
            fs::copy(&path, &dst_path).expect("failed to copy file");
        }
    }
}

/// Simple directory walker (avoid external dependency)
fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                entries.extend(walkdir(&path));
            } else {
                entries.push(path);
            }
        }
    }
    entries
}

/// Find a suitable DXC executable.
///
/// Order of preference:
/// 1. Cached copy under `target/dxc/<DXC_VERSION>/` (downloaded previously)
/// 2. `DXC_PATH` environment variable (if it points to a compatible version)
/// 3. `dxc` in PATH (if version >= 1.9.2602.17)
/// 4. Windows Kits copy (if version >= 1.9.2602.17)
///
/// Returns `None` if no suitable DXC is found and we should attempt download.
fn find_dxc() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().expect("workspace root");

    // 1. Check our own cache first
    let cached = workspace_root
        .join("target")
        .join("dxc")
        .join(DXC_VERSION)
        .join("dxc.exe");
    if cached.exists() {
        return Some(cached);
    }

    // 2. DXC_PATH override
    if let Ok(path) = env::var("DXC_PATH") {
        let path = PathBuf::from(path);
        if path.exists() && is_dxc_compatible(&path) {
            return Some(path);
        }
    }

    // 3. dxc in PATH
    if let Ok(output) = Command::new("dxc").arg("--version").output() {
        if output.status.success() {
            let path = PathBuf::from("dxc");
            if is_dxc_compatible(&path) {
                return Some(path);
            }
        }
    }

    // 4. Windows Kits fallback
    let windows_kits = PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin");
    if windows_kits.exists() {
        let mut versions: Vec<_> = fs::read_dir(&windows_kits)
            .ok()?
            .flatten()
            .filter(|e| e.path().is_dir())
            .collect();
        versions.sort_by_key(|e| e.file_name());
        if let Some(newest) = versions.last() {
            let dxc = newest.path().join("x64").join("dxc.exe");
            if dxc.exists() && is_dxc_compatible(&dxc) {
                return Some(dxc);
            }
        }
    }

    None
}

/// Check whether the given DXC executable is new enough for SM 6.9.
fn is_dxc_compatible(path: &Path) -> bool {
    let output = match Command::new(path).arg("--version").output() {
        Ok(o) => o,
        Err(_) => return false,
    };
    if !output.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Example: dxcompiler.dll: 1.9 - 1.9.2602.17 (...)
    stdout.contains("1.9.2602.17") || stdout.contains("1.9 - 1.9.2602.17")
}

/// Download the official DXC release and cache it under target/dxc/.
fn download_dxc(workspace_root: &Path) -> Result<PathBuf, String> {
    let cache_dir = workspace_root
        .join("target")
        .join("dxc")
        .join(DXC_VERSION);
    let dxc_path = cache_dir.join("dxc.exe");

    if dxc_path.exists() {
        return Ok(dxc_path);
    }

    fs::create_dir_all(&cache_dir).map_err(|e| format!("create_dir_all failed: {e}"))?;

    // Official GitHub release asset for Windows x64.
    let url = format!(
        "https://github.com/microsoft/DirectXShaderCompiler/releases/download/{}/{}",
        DXC_RELEASE_TAG, DXC_ASSET_NAME
    );

    println!("cargo:warning=Downloading DXC {} from {}", DXC_VERSION, url);

    let response = ureq::get(&url)
        .call()
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if response.status() != 200 {
        return Err(format!("HTTP status {}", response.status()));
    }

    let mut zip_data = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut zip_data)
        .map_err(|e| format!("read_to_end failed: {e}"))?;

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_data))
        .map_err(|e| format!("invalid zip archive: {e}"))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("zip entry error: {e}"))?;
        let name = file.name().to_string();

        // Only extract the files we need
        if name.ends_with("dxc.exe") || name.ends_with("dxil.dll") || name.ends_with("dxcompiler.dll") {
            let out_path = cache_dir.join(Path::new(&name).file_name().unwrap());
            let mut out_file = fs::File::create(&out_path)
                .map_err(|e| format!("create failed for {}: {e}", out_path.display()))?;
            std::io::copy(&mut file, &mut out_file)
                .map_err(|e| format!("extract failed for {}: {e}", name))?;
        }
    }

    if !dxc_path.exists() {
        return Err("dxc.exe not found in downloaded archive".into());
    }

    Ok(dxc_path)
}

/// Compile all HLSL shaders to DXIL using DXC
fn compile_shaders(dxc: &Path, shaders_dir: &Path) {
    let shader_extensions = [
        "vert", "frag", "comp", "geom", "tesc", "tese", "mesh", "task", "rgen", "rmiss", "rclosest",
        "rany", "rint", "rcall",
    ];

    for entry in walkdir(shaders_dir) {
        let path = entry;

        if path.is_dir() {
            continue;
        }

        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e,
            None => continue,
        };

        // Skip non-shader files
        if !shader_extensions.contains(&ext) {
            continue;
        }

        // Skip if .dxil already exists and is newer
        let dxil_path = path.with_extension("dxil");
        if dxil_path.exists() {
            let src_time = fs::metadata(&path).and_then(|m| m.modified()).ok();
            let dst_time = fs::metadata(&dxil_path).and_then(|m| m.modified()).ok();
            if let (Some(src), Some(dst)) = (src_time, dst_time) {
                if dst > src {
                    continue; // .dxil is up to date
                }
            }
        }

        // Determine shader model from filename and extension
        let file_name = path.file_name().unwrap().to_string_lossy();
        let (target, version_minor) = get_shader_target(&file_name, ext);

        // Build DXC command
        let mut cmd = Command::new(dxc);
        cmd.arg("-Qstrip_reflect")
            .arg("-Qstrip_debug")
            .arg("-Vd")
            .arg("-T")
            .arg(&target)
            .arg("-Fo")
            .arg(&dxil_path)
            .arg(&path);

        // Mirror upstream test_shaders.py: -enable-16bit-types for every
        // shader model >= 6.2 (i.e. everything except .sm60./.sm61.).
        // The default version_minor is 5, so unmarked shaders also get it.
        if version_minor >= 2 {
            cmd.arg("-enable-16bit-types");
        }

        if file_name.contains(".denorm-ftz.") {
            cmd.args(["-denorm", "ftz"]);
        }
        if file_name.contains(".denorm-preserve.") {
            cmd.args(["-denorm", "preserve"]);
        }
        if file_name.contains(".no-legacy-cbuf-layout.") {
            cmd.arg("-no-legacy-cbuf-layout");
        }

        // Execute DXC
        match cmd.output() {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!(
                        "cargo:warning=DXC failed for {}: {}",
                        path.display(),
                        stderr
                    );
                }
            }
            Err(e) => {
                println!("cargo:warning=Failed to run DXC for {}: {}", path.display(), e);
            }
        }
    }
}

/// Determine DXC target profile from filename and extension.
///
/// Mirrors upstream `test_shaders.py` `get_sm()`:
/// - version_minor defaults to 5; `.smNN.` markers override it.
/// - `.node.` compute shaders always use `lib_6_8`.
/// - Ray tracing / callable stages use `lib_6_{minor}` (minor clamped to >= 5).
///
/// Returns `(target_profile, version_minor)` — the latter drives the
/// `-enable-16bit-types` decision.
fn get_shader_target(filename: &str, ext: &str) -> (String, u32) {
    let version_minor: u32 = if filename.contains(".sm60.") {
        0
    } else if filename.contains(".sm66.") {
        6
    } else if filename.contains(".sm67.") {
        7
    } else if filename.contains(".sm69.") {
        9
    } else {
        5 // default, same as upstream
    };

    let shader_type = match ext {
        "vert" => format!("vs_6_{version_minor}"),
        "frag" => format!("ps_6_{version_minor}"),
        "comp" => {
            if filename.contains(".node.") {
                "lib_6_8".to_string() // Work Graphs require SM 6.8 lib target
            } else {
                format!("cs_6_{version_minor}")
            }
        }
        "geom" => format!("gs_6_{version_minor}"),
        "tesc" => format!("hs_6_{version_minor}"),
        "tese" => format!("ds_6_{version_minor}"),
        // Mesh/task: upstream uses minor 5 for <= 5, else the marker value
        "mesh" => format!("ms_6_{}", version_minor.max(5)),
        "task" => format!("as_6_{}", version_minor.max(5)),
        // RT stages: upstream clamps minor to >= 5 for lib targets
        "rgen" | "rmiss" | "rclosest" | "rany" | "rint" | "rcall" => {
            format!("lib_6_{}", version_minor.max(5))
        }
        _ => format!("lib_6_{}", version_minor.max(5)),
    };

    (shader_type, version_minor)
}

/// Generate a manifest of all test shaders for the completeness check
fn generate_manifest(shaders_dir: &Path, output_dir: &Path) {
    let mut shaders = Vec::new();

    for entry in walkdir(shaders_dir) {
        let path = entry;

        if path.is_dir() {
            continue;
        }

        let rel_path = path.strip_prefix(shaders_dir).expect("strip prefix");
        let rel_path_str = rel_path.to_string_lossy().replace('\\', "/");

        // Check if .dxil exists
        let has_dxil = path.with_extension("dxil").exists();

        // Check if reference exists
        let ref_path = output_dir
            .join("tests/reference/shaders")
            .join(rel_path);
        let has_reference = ref_path.exists();

        shaders.push(format!(
            "{}|dxil={}|ref={}",
            rel_path_str, has_dxil, has_reference
        ));
    }

    let manifest_path = output_dir.join("tests/shaders_manifest.txt");
    fs::write(&manifest_path, shaders.join("\n")).expect("failed to write manifest");
    println!(
        "cargo:warning=Generated manifest with {} shaders",
        shaders.len()
    );
}
