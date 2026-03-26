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

fn cmd_codegen(filename: &str, output: Option<&str>) -> i32 {
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
    let code = match vexil_codegen_rust::generate(&compiled) {
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

fn cmd_build(
    root_file: &str,
    include_paths: &[String],
    output_dir: &str,
    _rust_prefix: &str,
) -> i32 {
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

    // Generate code for each schema
    let output_path = std::path::Path::new(output_dir);
    let mut mod_tree: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();

    for (ns, compiled) in &result.schemas {
        // Map namespace to path: "foo.bar.types" -> foo/bar/types.rs
        let segments: Vec<&str> = ns.split('.').collect();
        if segments.is_empty() {
            continue;
        }
        // SAFETY: segments is non-empty (checked above)
        let file_name = segments[segments.len() - 1];
        let dir_segments = &segments[..segments.len() - 1];
        let mut file_path = output_path.to_path_buf();
        for seg in dir_segments {
            file_path.push(seg);
        }

        // Track mod.rs entries: for each prefix, record the next child module name
        for i in 0..segments.len() - 1 {
            let parent_key = segments[..i].join("/");
            let child = segments[i].to_string();
            let entry = mod_tree.entry(parent_key).or_default();
            if !entry.contains(&child) {
                entry.push(child);
            }
        }
        // Register the file itself under its parent
        if segments.len() >= 2 {
            let parent_key = dir_segments.join("/");
            let child = file_name.to_string();
            let entry = mod_tree.entry(parent_key).or_default();
            if !entry.contains(&child) {
                entry.push(child);
            }
        } else {
            // Top-level namespace -> goes in root mod.rs
            let entry = mod_tree.entry(String::new()).or_default();
            let child = file_name.to_string();
            if !entry.contains(&child) {
                entry.push(child);
            }
        }

        // Create directory and write file
        if let Err(e) = std::fs::create_dir_all(&file_path) {
            eprintln!("error: creating directory {}: {e}", file_path.display());
            return 1;
        }
        file_path.push(format!("{file_name}.rs"));

        let code = match vexil_codegen_rust::generate_with_imports(compiled, None) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: codegen for `{ns}` failed: {e}");
                return 1;
            }
        };

        if let Err(e) = std::fs::write(&file_path, &code) {
            eprintln!("error: writing {}: {e}", file_path.display());
            return 1;
        }
        eprintln!("  wrote {}", file_path.display());
    }

    // Generate mod.rs files
    for (dir_key, children) in &mod_tree {
        let mut mod_path = output_path.to_path_buf();
        if !dir_key.is_empty() {
            for seg in dir_key.split('/') {
                mod_path.push(seg);
            }
        }
        if let Err(e) = std::fs::create_dir_all(&mod_path) {
            eprintln!("error: creating directory {}: {e}", mod_path.display());
            return 1;
        }
        mod_path.push("mod.rs");

        let child_refs: Vec<&str> = children.iter().map(|s| s.as_str()).collect();
        let mod_content = vexil_codegen_rust::generate_mod_file(&child_refs);

        if let Err(e) = std::fs::write(&mod_path, &mod_content) {
            eprintln!("error: writing {}: {e}", mod_path.display());
            return 1;
        }
        eprintln!("  wrote {}", mod_path.display());
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
            // vexilc codegen <file.vexil> [--output <path>]
            if args.len() < 3 {
                eprintln!("Usage: vexilc codegen <file.vexil> [--output <path>]");
                std::process::exit(1);
            }
            let filename = &args[2];
            let output = if args.len() >= 5 && args[3] == "--output" {
                Some(args[4].as_str())
            } else {
                None
            };
            std::process::exit(cmd_codegen(filename, output));
        }
        Some("build") => {
            // vexilc build <root.vexil> --include <dir> [--include ...] --output <dir> [--rust-path-prefix <prefix>]
            if args.len() < 6 {
                eprintln!("Usage: vexilc build <root.vexil> --include <dir> --output <dir> [--rust-path-prefix <prefix>]");
                std::process::exit(1);
            }
            let root_file = &args[2];
            let mut include_paths = Vec::new();
            let mut output_dir = None;
            let mut rust_prefix = "crate".to_string();
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
                    "--rust-path-prefix" => {
                        i += 1;
                        if i < args.len() {
                            rust_prefix = args[i].clone();
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
            std::process::exit(cmd_build(
                root_file,
                &include_paths,
                &output_dir,
                &rust_prefix,
            ));
        }
        _ => {
            eprintln!("Usage: vexilc <subcommand> [args]");
            eprintln!("  vexilc check <file.vexil>");
            eprintln!("  vexilc codegen <file.vexil> [--output <path>]");
            eprintln!("  vexilc build <root.vexil> --include <dir> --output <dir> [--rust-path-prefix <prefix>]");
            std::process::exit(1);
        }
    }
}
