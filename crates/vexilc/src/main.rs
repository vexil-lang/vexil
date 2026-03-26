use ariadne::{Color, Label, Report, ReportKind, Source};
use vexil_lang::diagnostic::{Diagnostic, Severity};

fn render_diagnostic(filename: &str, source: &str, diag: &Diagnostic) {
    let kind = match diag.severity {
        Severity::Error => ReportKind::Error,
        Severity::Warning => ReportKind::Warning,
    };
    let range = diag.span.range();
    Report::build(kind, (filename, range.clone()))
        .with_message(&diag.message)
        .with_label(
            Label::new((filename, range))
                .with_message(format!("{:?}", diag.class))
                .with_color(Color::Red),
        )
        .finish()
        .eprint((filename, Source::from(source)))
        .ok();
}

fn cmd_check(filename: &str) -> i32 {
    let source = match std::fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {filename}: {e}");
            return 1;
        }
    };
    let result = vexil_lang::compile(&source);
    for diag in &result.diagnostics {
        render_diagnostic(filename, &source, diag);
    }
    if result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error)
    {
        return 1;
    }
    if let Some(ref compiled) = result.compiled {
        let hash = vexil_lang::canonical::schema_hash(compiled);
        let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
        println!("schema hash: {hex}");
    }
    0
}

fn cmd_codegen(filename: &str, output: Option<&str>, target: &str) -> i32 {
    let source = match std::fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {filename}: {e}");
            return 1;
        }
    };
    let result = vexil_lang::compile(&source);
    for diag in &result.diagnostics {
        render_diagnostic(filename, &source, diag);
    }
    if result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error)
    {
        return 1;
    }
    let compiled = match result.compiled {
        Some(c) => c,
        None => {
            eprintln!("error: {filename}: compilation produced no output");
            return 1;
        }
    };
    let backend: Box<dyn vexil_lang::codegen::CodegenBackend> = match target {
        "rust" => Box::new(vexil_codegen_rust::RustBackend),
        other => {
            eprintln!("error: unknown target `{other}` (available: rust)");
            return 1;
        }
    };
    let code = match backend.generate(&compiled) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: codegen failed: {e}");
            return 1;
        }
    };
    match output {
        Some(path) => {
            if let Err(e) = std::fs::write(path, &code) {
                eprintln!("error: {path}: {e}");
                return 1;
            }
        }
        None => print!("{code}"),
    }
    0
}

fn cmd_build(root_file: &str, include_paths: &[String], output_dir: &str, target: &str) -> i32 {
    // Read root file
    let source = match std::fs::read_to_string(root_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {root_file}: {e}");
            return 1;
        }
    };
    let root_path = std::path::PathBuf::from(root_file);

    // Create loader
    let include_dirs: Vec<std::path::PathBuf> =
        include_paths.iter().map(std::path::PathBuf::from).collect();
    let loader = vexil_lang::resolve::FilesystemLoader::new(include_dirs);

    // Compile project
    let result = match vexil_lang::compile_project(&source, &root_path, &loader) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    // Render diagnostics
    let has_errors = result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    for diag in &result.diagnostics {
        let file = diag
            .source_file
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| root_file.to_string());
        let severity = match diag.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        eprintln!("{severity}: {file}: {}", diag.message);
    }
    if has_errors {
        return 1;
    }

    // Resolve backend
    let backend: Box<dyn vexil_lang::codegen::CodegenBackend> = match target {
        "rust" => Box::new(vexil_codegen_rust::RustBackend),
        other => {
            eprintln!("error: unknown target `{other}` (available: rust)");
            return 1;
        }
    };

    // Generate all files via backend
    let files = match backend.generate_project(&result) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: codegen failed: {e}");
            return 1;
        }
    };

    // Write files to output directory
    let output_path = std::path::Path::new(output_dir);
    for (rel_path, content) in &files {
        let full_path = output_path.join(rel_path);
        if let Some(parent) = full_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("error: creating directory {}: {e}", parent.display());
                return 1;
            }
        }
        if let Err(e) = std::fs::write(&full_path, content) {
            eprintln!("error: writing {}: {e}", full_path.display());
            return 1;
        }
        eprintln!("  wrote {}", full_path.display());
    }

    eprintln!("build complete: {} schemas compiled", result.schemas.len());
    0
}

fn cmd_info(filename: &str) -> i32 {
    let data = match std::fs::read(filename) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {filename}: {e}");
            return 1;
        }
    };

    let fmt = vexil_store::detect_format(&data);
    println!("file: {filename}");
    println!("format: {fmt:?}");

    match fmt {
        vexil_store::FileFormat::Vxb
        | vexil_store::FileFormat::Vxc
        | vexil_store::FileFormat::Vxbp
        | vexil_store::FileFormat::Vxcp => match vexil_store::read_header(&data) {
            Ok((header, _)) => {
                println!("version: {}", header.format_version);
                println!("compressed: {}", header.compressed);
                println!("namespace: {}", header.namespace);
                println!("schema_version: {}", header.schema_version);
                let hex: String = header
                    .schema_hash
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                println!("schema_hash: {hex}");
            }
            Err(e) => {
                eprintln!("error reading header: {e}");
                return 1;
            }
        },
        _ => {}
    }
    0
}

fn cmd_pack(vx_file: &str, schema_file: &str, type_name: &str, output: &str) -> i32 {
    // Read .vx source
    let source = match std::fs::read_to_string(vx_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {vx_file}: {e}");
            return 1;
        }
    };

    // Compile schema
    let schema_src = match std::fs::read_to_string(schema_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {schema_file}: {e}");
            return 1;
        }
    };
    let compile_result = vexil_lang::compile(&schema_src);
    if compile_result
        .diagnostics
        .iter()
        .any(|d| d.severity == vexil_lang::diagnostic::Severity::Error)
    {
        for diag in &compile_result.diagnostics {
            eprintln!("schema error: {}", diag.message);
        }
        return 1;
    }
    let schema = match compile_result.compiled {
        Some(s) => s,
        None => {
            eprintln!("error: schema compilation produced no output");
            return 1;
        }
    };

    // Parse .vx
    let values = match vexil_store::parse(&source, &schema) {
        Ok(v) => v,
        Err(errors) => {
            for e in &errors {
                eprintln!("parse error: {e}");
            }
            return 1;
        }
    };
    let value = match values.into_iter().next() {
        Some(v) => v,
        None => {
            eprintln!("error: no values in input file");
            return 1;
        }
    };

    // Encode
    let payload = match vexil_store::encode(&value, type_name, &schema) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: encoding failed: {e}");
            return 1;
        }
    };

    // Build header
    let ns = schema
        .namespace
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(".");
    let hash = vexil_lang::canonical::schema_hash(&schema);
    let header = vexil_store::VxbHeader {
        magic: vexil_store::Magic::Vxb,
        format_version: vexil_store::FORMAT_VERSION,
        compressed: false,
        schema_hash: hash,
        namespace: ns,
        schema_version: String::new(),
    };
    let mut out_bytes = Vec::new();
    vexil_store::write_header(&header, &mut out_bytes);
    out_bytes.extend_from_slice(&payload);

    if let Err(e) = std::fs::write(output, &out_bytes) {
        eprintln!("error: writing {output}: {e}");
        return 1;
    }
    println!("wrote {output} ({} bytes payload)", payload.len());
    0
}

fn cmd_unpack(vxb_file: &str, schema_file: &str, type_name: &str, output: Option<&str>) -> i32 {
    // Read binary file
    let data = match std::fs::read(vxb_file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {vxb_file}: {e}");
            return 1;
        }
    };

    // Read header
    let (_header, header_size) = match vexil_store::read_header(&data) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: reading header: {e}");
            return 1;
        }
    };
    let payload = &data[header_size..];

    // Compile schema
    let schema_src = match std::fs::read_to_string(schema_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {schema_file}: {e}");
            return 1;
        }
    };
    let compile_result = vexil_lang::compile(&schema_src);
    if compile_result
        .diagnostics
        .iter()
        .any(|d| d.severity == vexil_lang::diagnostic::Severity::Error)
    {
        for diag in &compile_result.diagnostics {
            eprintln!("schema error: {}", diag.message);
        }
        return 1;
    }
    let schema = match compile_result.compiled {
        Some(s) => s,
        None => {
            eprintln!("error: schema compilation produced no output");
            return 1;
        }
    };

    // Decode
    let value = match vexil_store::decode(payload, type_name, &schema) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: decoding failed: {e}");
            return 1;
        }
    };

    // Format
    let opts = vexil_store::FormatOptions::default();
    let text = match vexil_store::format(&[value], type_name, &schema, &opts) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: formatting failed: {e}");
            return 1;
        }
    };

    match output {
        Some(path) => {
            if let Err(e) = std::fs::write(path, &text) {
                eprintln!("error: writing {path}: {e}");
                return 1;
            }
            println!("wrote {path}");
        }
        None => print!("{text}"),
    }
    0
}

fn cmd_format_vx(vx_file: &str, schema_file: &str, type_name: &str) -> i32 {
    let source = match std::fs::read_to_string(vx_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {vx_file}: {e}");
            return 1;
        }
    };

    let schema_src = match std::fs::read_to_string(schema_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {schema_file}: {e}");
            return 1;
        }
    };
    let compile_result = vexil_lang::compile(&schema_src);
    if compile_result
        .diagnostics
        .iter()
        .any(|d| d.severity == vexil_lang::diagnostic::Severity::Error)
    {
        for diag in &compile_result.diagnostics {
            eprintln!("schema error: {}", diag.message);
        }
        return 1;
    }
    let schema = match compile_result.compiled {
        Some(s) => s,
        None => {
            eprintln!("error: schema compilation produced no output");
            return 1;
        }
    };

    let values = match vexil_store::parse(&source, &schema) {
        Ok(v) => v,
        Err(errors) => {
            for e in &errors {
                eprintln!("parse error: {e}");
            }
            return 1;
        }
    };

    let opts = vexil_store::FormatOptions::default();
    let text = match vexil_store::format(&values, type_name, &schema, &opts) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: formatting failed: {e}");
            return 1;
        }
    };
    print!("{text}");
    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("check") => {
            if args.len() != 3 {
                eprintln!("Usage: vexilc check <file.vexil>");
                std::process::exit(1);
            }
            std::process::exit(cmd_check(&args[2]));
        }
        Some("codegen") => {
            // vexilc codegen <file.vexil> [--output <path>] [--target <rust>]
            if args.len() < 3 {
                eprintln!("Usage: vexilc codegen <file.vexil> [--output <path>] [--target <rust>]");
                std::process::exit(1);
            }
            let filename = &args[2];
            let mut output = None;
            let mut target = "rust";
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--output" => {
                        i += 1;
                        if i < args.len() {
                            output = Some(args[i].as_str());
                        }
                    }
                    "--target" => {
                        i += 1;
                        if i < args.len() {
                            target = args[i].as_str();
                        }
                    }
                    other => {
                        eprintln!("unknown option: {other}");
                        std::process::exit(1);
                    }
                }
                i += 1;
            }
            std::process::exit(cmd_codegen(filename, output, target));
        }
        Some("build") => {
            // vexilc build <root.vexil> --include <dir> [--include ...] --output <dir> [--target <rust>]
            if args.len() < 6 {
                eprintln!("Usage: vexilc build <root.vexil> --include <dir> --output <dir> [--target <rust>]");
                std::process::exit(1);
            }
            let root_file = &args[2];
            let mut include_paths = Vec::new();
            let mut output_dir = None;
            let mut target = "rust".to_string();
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--include" => {
                        i += 1;
                        if i < args.len() {
                            include_paths.push(args[i].clone());
                        }
                    }
                    "--output" => {
                        i += 1;
                        if i < args.len() {
                            output_dir = Some(args[i].clone());
                        }
                    }
                    "--target" => {
                        i += 1;
                        if i < args.len() {
                            target = args[i].clone();
                        }
                    }
                    other => {
                        eprintln!("unknown option: {other}");
                        std::process::exit(1);
                    }
                }
                i += 1;
            }
            let output_dir = match output_dir {
                Some(d) => d,
                None => {
                    eprintln!("error: --output is required");
                    std::process::exit(1);
                }
            };
            std::process::exit(cmd_build(root_file, &include_paths, &output_dir, &target));
        }
        Some("info") => {
            if args.len() != 3 {
                eprintln!("Usage: vexilc info <file>");
                std::process::exit(1);
            }
            std::process::exit(cmd_info(&args[2]));
        }
        Some("pack") => {
            if args.len() < 8 {
                eprintln!("Usage: vexilc pack <file.vx> --schema <schema.vexil> --type <TypeName> -o <out.vxb>");
                std::process::exit(1);
            }
            let vx_file = &args[2];
            let mut schema_file = None;
            let mut type_name = None;
            let mut output = None;
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--schema" => {
                        i += 1;
                        if i < args.len() {
                            schema_file = Some(args[i].as_str());
                        }
                    }
                    "--type" => {
                        i += 1;
                        if i < args.len() {
                            type_name = Some(args[i].as_str());
                        }
                    }
                    "-o" | "--output" => {
                        i += 1;
                        if i < args.len() {
                            output = Some(args[i].as_str());
                        }
                    }
                    other => {
                        eprintln!("unknown option: {other}");
                        std::process::exit(1);
                    }
                }
                i += 1;
            }
            let schema_file = schema_file.unwrap_or_else(|| {
                eprintln!("--schema required");
                std::process::exit(1);
            });
            let type_name = type_name.unwrap_or_else(|| {
                eprintln!("--type required");
                std::process::exit(1);
            });
            let output = output.unwrap_or_else(|| {
                eprintln!("-o required");
                std::process::exit(1);
            });
            std::process::exit(cmd_pack(vx_file, schema_file, type_name, output));
        }
        Some("unpack") => {
            if args.len() < 6 {
                eprintln!("Usage: vexilc unpack <file.vxb> --schema <schema.vexil> --type <TypeName> [-o <out.vx>]");
                std::process::exit(1);
            }
            let vxb_file = &args[2];
            let mut schema_file = None;
            let mut type_name = None;
            let mut output = None;
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--schema" => {
                        i += 1;
                        if i < args.len() {
                            schema_file = Some(args[i].as_str());
                        }
                    }
                    "--type" => {
                        i += 1;
                        if i < args.len() {
                            type_name = Some(args[i].as_str());
                        }
                    }
                    "-o" | "--output" => {
                        i += 1;
                        if i < args.len() {
                            output = Some(args[i].as_str());
                        }
                    }
                    other => {
                        eprintln!("unknown option: {other}");
                        std::process::exit(1);
                    }
                }
                i += 1;
            }
            let schema_file = schema_file.unwrap_or_else(|| {
                eprintln!("--schema required");
                std::process::exit(1);
            });
            let type_name = type_name.unwrap_or_else(|| {
                eprintln!("--type required");
                std::process::exit(1);
            });
            std::process::exit(cmd_unpack(vxb_file, schema_file, type_name, output));
        }
        Some("format") => {
            if args.len() < 7 {
                eprintln!(
                    "Usage: vexilc format <file.vx> --schema <schema.vexil> --type <TypeName>"
                );
                std::process::exit(1);
            }
            let vx_file = &args[2];
            let mut schema_file = None;
            let mut type_name = None;
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--schema" => {
                        i += 1;
                        if i < args.len() {
                            schema_file = Some(args[i].as_str());
                        }
                    }
                    "--type" => {
                        i += 1;
                        if i < args.len() {
                            type_name = Some(args[i].as_str());
                        }
                    }
                    other => {
                        eprintln!("unknown option: {other}");
                        std::process::exit(1);
                    }
                }
                i += 1;
            }
            let schema_file = schema_file.unwrap_or_else(|| {
                eprintln!("--schema required");
                std::process::exit(1);
            });
            let type_name = type_name.unwrap_or_else(|| {
                eprintln!("--type required");
                std::process::exit(1);
            });
            std::process::exit(cmd_format_vx(vx_file, schema_file, type_name));
        }
        _ => {
            eprintln!("Usage: vexilc <subcommand> [args]");
            eprintln!("  vexilc check <file.vexil>");
            eprintln!("  vexilc codegen <file.vexil> [--output <path>] [--target <rust>]");
            eprintln!(
                "  vexilc build <root.vexil> --include <dir> --output <dir> [--target <rust>]"
            );
            eprintln!("  vexilc info <file>");
            eprintln!(
                "  vexilc pack <file.vx> --schema <schema.vexil> --type <TypeName> -o <out.vxb>"
            );
            eprintln!("  vexilc unpack <file.vxb> --schema <schema.vexil> --type <TypeName> [-o <out.vx>]");
            eprintln!("  vexilc format <file.vx> --schema <schema.vexil> --type <TypeName>");
            std::process::exit(1);
        }
    }
}
