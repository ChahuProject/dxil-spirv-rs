//! Experimental / reproduction tool for the non-upstream hlsl-compat
//! extension. Not part of the published crates and not run in CI.
//!
//! Commands:
//!
//! * `dump <shader.dxbc|shader.dxil>` — convert and print every Uniform block
//!   type tree (shows the scalar view / vec4 alias duality).
//! * `repro <shader.dxbc|shader.dxil>` — convert, run `vec4_align_cbuffers`,
//!   and report spirv-cross2 HLSL compile success before/after.
//!
//! See `docs/non-upstream/hlsl-compat-rationale.md` for how these commands
//! support the reproduction steps documented there.

use dxil_spirv::non_upstream::hlsl_compat::ir as hc_ir;
use rspirv::spirv::{Op, StorageClass};
use spirv_cross2::compile::CompilableTarget;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Subprocess mode for `scan`: the parent sets this env var per shader.
    if let Ok(rel) = std::env::var("HLCI_SCAN_SHADER") {
        return scan_child(&rel);
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, path) = match args.as_slice() {
        [cmd] if cmd == "scan" => (cmd.as_str(), ""),
        [cmd, path] => (cmd.as_str(), path.as_str()),
        _ => {
            eprintln!("usage: hlsl-compat-inspect <dump|repro|scan> [shader.dxbc|shader.dxil]");
            return ExitCode::from(2);
        }
    };

    if cmd == "scan" {
        return scan();
    }

    let blob = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("failed to read {path}: {e}");
            return ExitCode::from(2);
        }
    };

    let spirv = match dxil_spirv::convert_to_spirv(&blob) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("conversion failed: {e}");
            return ExitCode::from(1);
        }
    };

    match cmd {
        "dump" => dump(&spirv),
        "repro" => repro(&spirv),
        "debug" => debug(&spirv),
        "scan" => scan(),
        _ => {
            eprintln!("unknown command '{cmd}'");
            ExitCode::from(2)
        }
    }
}

/// Full-suite scan (subprocess-isolated per shader).
///
/// Parent walks `tests/shaders` and spawns a child per `.dxil` that converts,
/// runs the pass, and reports before/after compile status. Summarizes:
/// rewritten count, GLSL regressions, HLSL before/after failure counts.
fn scan() -> ExitCode {
    const CHILD_ENV: &str = "HLCI_SCAN_SHADER";
    if let Ok(rel) = std::env::var(CHILD_ENV) {
        return scan_child(&rel);
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .join("tests/shaders");
    let exe = std::env::current_exe().unwrap();

    let mut total = 0usize;
    let mut rewritten = 0usize;
    let mut glsl_regress = 0usize;
    let mut hlsl_before_fail = 0usize;
    let mut hlsl_after_fail = 0usize;
    let mut converted_fail = 0usize;
    let mut crashed = 0usize;
    let mut hlsl_fixed: Vec<String> = vec![];

    let mut files = walk_dir(&root);
    files.sort();
    for rel in files {
        if !rel.ends_with(".dxil") {
            continue;
        }
        total += 1;
        let out = std::process::Command::new(&exe)
            .env(CHILD_ENV, &rel)
            .args(["--exact-scan"])
            .output();
        let Ok(out) = out else { continue };
        let stdout = String::from_utf8_lossy(&out.stdout);
        let line = stdout.lines().find(|l| l.starts_with("__SCAN__|")).unwrap_or("");
        if line.is_empty() {
            crashed += 1;
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        // __SCAN__|rel|conv=ok|rew=N|gb=ok|ga=ok|hb=ok|ha=ok
        // or __SCAN__|rel|conv=fail|...
        if parts[2].starts_with("conv=fail") {
            converted_fail += 1;
            continue;
        }
        if parts.len() < 8 {
            continue;
        }
        if parts[2] != "conv=ok" {
            converted_fail += 1;
            continue;
        }
        let rew: usize = parts[3].trim_start_matches("rew=").parse().unwrap_or(0);
        if rew > 0 {
            rewritten += 1;
        }
        let gb = parts[4] == "gb=true";
        let ga = parts[5] == "ga=true";
        let hb = parts[6] == "hb=true";
        let ha = parts[7] == "ha=true";
        if gb && !ga {
            glsl_regress += 1;
            println!("  GLSL REGRESSION: {rel}");
        }
        if !hb {
            hlsl_before_fail += 1;
            if ha {
                hlsl_fixed.push(rel);
            } else {
                hlsl_after_fail += 1;
            }
        } else if !ha {
            hlsl_after_fail += 1;
            eprintln!("  HLSL REGRESSION: {rel}");
        }
    }

    println!("\n===== SCAN SUMMARY =====");
    println!("total scanned: {total}");
    println!("conversion failures: {converted_fail}");
    println!("child crashes: {crashed}");
    println!("shaders with rewritten cbuffer views: {rewritten}");
    println!("GLSL regressions (ok -> fail): {glsl_regress}");
    println!("HLSL failures before: {hlsl_before_fail}");
    println!("HLSL failures after (still failing): {hlsl_after_fail}");
    println!("HLSL fixed by pass: {}", hlsl_fixed.len());
    for f in &hlsl_fixed {
        println!("  FIXED: {f}");
    }
    ExitCode::SUCCESS
}

fn scan_child(rel: &str) -> ExitCode {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .join("tests/shaders");
    let spirv = match convert_file(&root.join(rel).to_string_lossy()) {
        Ok(s) => s,
        Err(e) => {
            println!("__SCAN__|{rel}|conv=fail|{e}");
            return ExitCode::SUCCESS;
        }
    };
    let out = match dxil_spirv::non_upstream::hlsl_compat::vec4_align_cbuffers(&spirv) {
        Ok(o) => o,
        Err(_) => {
            println!("__SCAN__|{rel}|conv=ok|rew=-1||||");
            return ExitCode::SUCCESS;
        }
    };
    let rew = out.rewritten;
    let gb = glsl_compiles(&spirv) == "OK";
    let ga = glsl_compiles(&out.spirv) == "OK";
    // HLSL: only worth compiling if the module changed or it is known to fail.
    let hb = hlsl_compiles(&spirv) == "OK";
    let ha = if rew > 0 { hlsl_compiles(&out.spirv) == "OK" } else { hb };
    println!("__SCAN__|{rel}|conv=ok|rew={rew}|gb={gb}|ga={ga}|hb={hb}|ha={ha}");
    ExitCode::SUCCESS
}

fn convert_file(path: &str) -> Result<Vec<u32>, String> {
    let blob = std::fs::read(path).map_err(|e| e.to_string())?;
    let parsed = dxil_spirv::ParsedBlob::parse(&blob).map_err(|e| format!("parse: {e}"))?;
    let converter = dxil_spirv::Converter::new(&parsed).map_err(|e| format!("new: {e}"))?;
    converter.run().map_err(|e| format!("run: {e}"))?;
    converter.compiled_spirv().map_err(|e| format!("spirv: {e}"))
}

fn walk_dir(dir: &std::path::Path) -> Vec<String> {
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let dirname = p.file_name().unwrap().to_string_lossy().to_string();
                for sub in walk_dir(&p) {
                    out.push(format!("{dirname}/{sub}"));
                }
            } else if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".dxil") {
                    out.push(name.to_string());
                }
            }
        }
    }
    out
}

/// Temporary debugging: print every access chain rooted at each Uniform var
/// and every instruction referencing the chain results.
fn debug(spirv: &[u32]) -> ExitCode {
    use rspirv::dr::Operand;
    let module = match rspirv_parse(spirv) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    for var in &module.types_global_values {
        if var.class.opcode != Op::Variable {
            continue;
        }
        let var_id = var.result_id.unwrap();
        println!("--- var #{var_id} (ptr type {:?}) ---", var.result_type);
        for func in &module.functions {
            for block in &func.blocks {
                for inst in &block.instructions {
                    match inst.class.opcode {
                        Op::AccessChain | Op::InBoundsAccessChain => {
                            let base = match inst.operands.first() {
                                Some(Operand::IdRef(b)) => *b,
                                _ => 0,
                            };
                            if base != var_id {
                                continue;
                            }
                            println!(
                                "  AC #{} (res type {:?}) indices: {:?}",
                                inst.result_id.unwrap(),
                                inst.result_type,
                                inst.operands[1..]
                                    .iter()
                                    .map(|o| match o {
                                        Operand::IdRef(i) => format!("#{i}"),
                                        other => format!("{other:?}"),
                                    })
                                    .collect::<Vec<_>>()
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    // All references to every access chain result (first 60)
    println!("--- references to access chain results ---");
    let mut n = 0;
    for func in &module.functions {
        for block in &func.blocks {
            for inst in &block.instructions {
                for (pos, op) in inst.operands.iter().enumerate() {
                    if let Operand::IdRef(id) = op {
                        // is `id` an access chain result?
                        let is_ac = module.functions.iter().any(|f| {
                            f.blocks.iter().any(|b| {
                                b.instructions.iter().any(|i| {
                                    matches!(
                                        i.class.opcode,
                                        Op::AccessChain | Op::InBoundsAccessChain
                                    ) && i.result_id == Some(*id)
                                })
                            })
                        });
                        if is_ac {
                            println!(
                                "  ac#{id} referenced by {:?} pos {pos} (res {:?})",
                                inst.class.opcode, inst.result_id
                            );
                            n += 1;
                            if n > 60 {
                                return ExitCode::SUCCESS;
                            }
                        }
                    }
                }
            }
        }
    }
    ExitCode::SUCCESS
}

fn dump(spirv: &[u32]) -> ExitCode {
    let module = match rspirv_parse(spirv) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    let info = hc_ir::analyze(&module);

    println!("=== Uniform blocks ===");
    for var in &info.variables {
        if var.storage_class != StorageClass::Uniform {
            continue;
        }
        if !info.block_structs.contains(&var.struct_type) {
            continue;
        }
        let name = info.var_name(var.id);
        let binding = info.bindings.get(&var.id).copied();
        println!(
            "Variable #{}({}) struct#{} binding={binding:?}:",
            var.id, name, var.struct_type
        );
        print_struct(&module, &info, var.struct_type, 1);
    }
    println!();

    println!("=== stride-4 scalar views detected by the pass ===");
    let (targets, skipped) =
        dxil_spirv::non_upstream::hlsl_compat::detect::find_targets(&module, &info);
    for t in &targets {
        let v = &t.view;
        match &t.alias {
            Some(a) => println!(
                "  view var#{} struct#{}: float[{}] stride4  ->  alias var#{} struct#{}: float4[{}] stride16 (merge)",
                v.var_id, v.struct_id, v.array_len, a.var_id, a.struct_id, a.array_len
            ),
            None => println!(
                "  view var#{} struct#{}: float[{}] stride4  ->  in-place rewrite to float4[{}]",
                v.var_id,
                v.struct_id,
                v.array_len,
                v.array_len / 4
            ),
        }
        println!(
            "    access chains: {} (dynamic: {})",
            t.accesses.len(),
            t.accesses.iter().filter(|a| a.is_dynamic).count()
        );
    }
    for s in &skipped {
        println!("  SKIPPED var#{}: {}", s.var_id, s.reason);
    }
    ExitCode::SUCCESS
}

/// Prints a struct type tree with member offsets / array strides.
fn print_struct(
    module: &rspirv::dr::Module,
    info: &hc_ir::ModuleInfo,
    struct_id: u32,
    indent: usize,
) {
    let pad = "  ".repeat(indent);
    let Some(members) = info.struct_members.get(&struct_id) else {
        return;
    };
    for (mi, &mt) in members.iter().enumerate() {
        let off = info
            .member_offsets
            .get(&(struct_id, mi as u32))
            .copied()
            .unwrap_or(u32::MAX);
        let ty = describe_type(module, info, mt);
        println!("{pad}_m{mi} @ {off}: {ty}");
    }
}

/// Describes a type id as a string (float/int/vector/array with stride).
fn describe_type(module: &rspirv::dr::Module, info: &hc_ir::ModuleInfo, type_id: u32) -> String {
    if let Some(w) = info.float_width.get(&type_id) {
        return format!("float{w}");
    }
    if let Some((w, _)) = info.int_info.get(&type_id) {
        return format!("int{w}");
    }
    if let Some((comp, count)) = info.vector_info.get(&type_id) {
        let inner = describe_type(module, info, *comp);
        return format!("{inner}x{count}");
    }
    if let Some((elem, len_const)) = info.array_info.get(&type_id) {
        let inner = describe_type(module, info, *elem);
        let len = info.constants.get(len_const).copied().unwrap_or(u32::MAX);
        let stride = info
            .array_strides
            .get(&type_id)
            .copied()
            .map(|s| format!(" stride={s}"))
            .unwrap_or_default();
        return format!("{inner}[{len}]{stride}");
    }
    let op = module
        .types_global_values
        .iter()
        .find(|i| i.result_id == Some(type_id))
        .map(|i| i.class.opcode)
        .unwrap_or(Op::Nop);
    format!("<id {type_id} op {op:?}>")
}

fn repro(spirv: &[u32]) -> ExitCode {
    let before = hlsl_compiles(spirv);
    let normalized = match dxil_spirv::non_upstream::hlsl_compat::vec4_align_cbuffers(spirv) {
        Ok(out) => out,
        Err(e) => {
            eprintln!("vec4_align_cbuffers failed: {e}");
            return ExitCode::from(1);
        }
    };
    let after = hlsl_compiles(&normalized.spirv);

    println!("HLSL compile before: {before}");
    println!("HLSL compile after:  {after}");
    println!("GLSL compile before: {}", glsl_compiles(spirv));
    println!("GLSL compile after:  {}", glsl_compiles(&normalized.spirv));
    println!(
        "rewritten views: {} (skipped: {})",
        normalized.rewritten,
        normalized.skipped.len()
    );
    for s in &normalized.skipped {
        println!("  skipped var #{}: {}", s.var_id, s.reason);
    }
    if let Ok(m) = rspirv_parse(&normalized.spirv) {
        let info = hc_ir::analyze(&m);
        println!("--- Uniform blocks after pass ---");
        for var in &info.variables {
            if var.storage_class != StorageClass::Uniform {
                continue;
            }
            if !info.block_structs.contains(&var.struct_type) {
                continue;
            }
            let name = info.var_name(var.id);
            let binding = info.bindings.get(&var.id).copied();
            println!(
                "Variable #{}({}) struct#{} binding={binding:?}:",
                var.id, name, var.struct_type
            );
            print_struct(&m, &info, var.struct_type, 1);
        }
        // Print all ids referenced but never defined (dangling refs).
        let mut defined: std::collections::HashSet<u32> = Default::default();
        let mut referenced: Vec<(String, u32)> = vec![];
        for inst in &m.types_global_values {
            if let Some(id) = inst.result_id {
                defined.insert(id);
            }
        }
        for f in &m.functions {
            if let Some(d) = &f.def {
                if let Some(id) = d.result_id {
                    defined.insert(id);
                }
            }
            for p in &f.parameters {
                if let Some(id) = p.result_id {
                    defined.insert(id);
                }
            }
            for b in &f.blocks {
                if let Some(l) = &b.label {
                    if let Some(id) = l.result_id {
                        defined.insert(id);
                    }
                }
                for i in &b.instructions {
                    if let Some(id) = i.result_id {
                        defined.insert(id);
                    }
                    for op in &i.operands {
                        if let rspirv::dr::Operand::IdRef(id) = op {
                            referenced.push((format!("func:{:?}", i.class.opcode), *id));
                        }
                    }
                }
            }
        }
        for inst in &m.annotations {
            if let Some(id) = inst.result_id {
                defined.insert(id);
            }
            for op in &inst.operands {
                if let rspirv::dr::Operand::IdRef(id) = op {
                    referenced.push((format!("annot:{:?}", inst.class.opcode), *id));
                }
            }
        }
        for inst in &m.entry_points {
            for op in &inst.operands {
                if let rspirv::dr::Operand::IdRef(id) = op {
                    referenced.push(("entrypoint".into(), *id));
                }
            }
        }
        println!("--- dangling references ---");
        let mut n = 0;
        for (src, r) in &referenced {
            if *r != 0 && !defined.contains(r) {
                println!("  dangling: {r} ({src})");
                n += 1;
                if n > 20 {
                    break;
                }
            }
        }
        if n == 0 {
            println!("  none");
        }
        // Disassemble everything (module is small).
        use rspirv::binary::Disassemble;
        println!("--- full disassembly (after pass) ---");
        let text = m.disassemble();
        let lines: Vec<&str> = text.lines().collect();
        for line in lines.iter().take(80) {
            println!("{line}");
        }
    }
    ExitCode::SUCCESS
}

fn hlsl_compiles(spirv: &[u32]) -> String {
    let module = spirv_cross2::Module::from_words(spirv);
    let compiler = spirv_cross2::Compiler::<spirv_cross2::targets::Hlsl>::new(module);
    let Ok(compiler) = compiler else {
        return format!("Compiler::new failed: {:?}", compiler.err());
    };
    let mut options = spirv_cross2::targets::Hlsl::options();
    options.shader_model = spirv_cross2::compile::hlsl::HlslShaderModel::ShaderModel5_1;
    match compiler.compile(&options) {
        Ok(_) => "OK".to_string(),
        Err(e) => format!("{e:?}"),
    }
}

fn glsl_compiles(spirv: &[u32]) -> String {
    let module = spirv_cross2::Module::from_words(spirv);
    let compiler = spirv_cross2::Compiler::<spirv_cross2::targets::Glsl>::new(module);
    let Ok(compiler) = compiler else {
        return format!("Compiler::new failed: {:?}", compiler.err());
    };
    let options = spirv_cross2::targets::Glsl::options();
    match compiler.compile(&options) {
        Ok(_) => "OK".to_string(),
        Err(e) => format!("{e:?}"),
    }
}

fn rspirv_parse(spirv: &[u32]) -> Result<rspirv::dr::Module, String> {
    let mut loader = rspirv::dr::Loader::new();
    rspirv::binary::parse_words(spirv, &mut loader).map_err(|e| format!("parse: {e}"))?;
    Ok(loader.module())
}
