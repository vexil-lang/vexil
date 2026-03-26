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
        _ => {
            eprintln!("Usage: vexilc <subcommand> [args]");
            eprintln!("  vexilc check <file.vexil>");
            eprintln!("  vexilc codegen <file.vexil> [--output <path>] [--target <rust>]");
            eprintln!(
                "  vexilc build <root.vexil> --include <dir> --output <dir> [--target <rust>]"
            );
            std::process::exit(1);
        }
    }
}
