use ariadne::{Color, Label, Report, ReportKind, Source};
use serde::Serialize;
use vexil_lang::compat::{BumpKind, ChangeKind, CompatResult};
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

fn cmd_check(filename: &str, include_paths: &[String]) -> i32 {
    let source = match std::fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {filename}: {e}");
            return 1;
        }
    };

    // Detect import statements — if present, --include is needed for full
    // type resolution.  Without it, imported types become unresolved stubs.
    let has_imports = source.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("import ")
    });

    if include_paths.is_empty() && has_imports {
        eprintln!(
            "warning: {filename} contains import statements but no --include paths were given"
        );
        eprintln!("hint: use `vexilc check {filename} --include <dir>` to resolve imports");
    }

    if include_paths.is_empty() {
        // Single-file mode
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
            if !has_imports {
                let hash = vexil_lang::canonical::schema_hash(compiled);
                let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
                println!("schema hash: {hex}");
            }
        }
    } else {
        // Multi-file mode with --include
        let root_path = std::path::PathBuf::from(filename);
        let include_dirs: Vec<std::path::PathBuf> =
            include_paths.iter().map(std::path::PathBuf::from).collect();
        let loader = vexil_lang::resolve::FilesystemLoader::new(include_dirs);

        let result = match vexil_lang::compile_project(&source, &root_path, &loader) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error: {e}");
                return 1;
            }
        };

        let has_errors = result
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error);
        for diag in &result.diagnostics {
            let file = diag
                .source_file
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| filename.to_string());
            let severity = match diag.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            eprintln!("{severity}: {file}: {}", diag.message);
        }
        if has_errors {
            return 1;
        }

        eprintln!("check passed: {} schemas compiled", result.schemas.len());
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
        "typescript" => Box::new(vexil_codegen_ts::TypeScriptBackend),
        "go" => Box::new(vexil_codegen_go::GoBackend),
        other => {
            eprintln!("error: unknown target `{other}` (available: rust, typescript, go)");
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
        "typescript" => Box::new(vexil_codegen_ts::TypeScriptBackend),
        "go" => Box::new(vexil_codegen_go::GoBackend),
        other => {
            eprintln!("error: unknown target `{other}` (available: rust, typescript, go)");
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
        // lgtm[rs/cleartext-logging] — decoded user data printed by request; not a credential.
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
    // Format each value individually. A .vx file may contain multiple records
    // of the same type; if any value fails to format under `type_name` the user
    // needs to check whether the file contains mixed types (use --type correctly).
    let mut output = String::new();
    for (i, value) in values.iter().enumerate() {
        match vexil_store::format(std::slice::from_ref(value), type_name, &schema, &opts) {
            Ok(t) => output.push_str(&t),
            Err(e) => {
                eprintln!("error: value {i} could not be formatted as `{type_name}`: {e}");
                eprintln!("hint: if the file contains mixed types, re-run with the correct --type");
                return 1;
            }
        }
    }
    // lgtm[rs/cleartext-logging] — formatted user data printed by request; not a credential.
    print!("{output}");
    0
}

fn cmd_compile(schema_file: &str, output: &str) -> i32 {
    let source = match std::fs::read_to_string(schema_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {schema_file}: {e}");
            return 1;
        }
    };
    let result = vexil_lang::compile(&source);
    for diag in &result.diagnostics {
        render_diagnostic(schema_file, &source, diag);
    }
    if result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error)
    {
        return 1;
    }
    let compiled = match result.compiled {
        Some(ref c) => c,
        None => {
            eprintln!("error: compilation produced no result");
            return 1;
        }
    };

    let meta = vexil_store::meta_schema();
    let schema_hash = vexil_lang::canonical::schema_hash(compiled);
    let ns = compiled
        .namespace
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(".");

    // Determine output format from extension: .vxcp -> SchemaStore, else .vxc single
    let is_store = output.ends_with(".vxcp");

    let (value, type_name) = if is_store {
        (
            vexil_store::schema_store_to_value(&[compiled]),
            "SchemaStore",
        )
    } else {
        (
            vexil_store::compiled_schema_to_value(compiled),
            "CompiledSchema",
        )
    };

    let payload = match vexil_store::encode(&value, type_name, meta) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("error: failed to encode {type_name}: {e}");
            return 1;
        }
    };

    let magic = if is_store {
        vexil_store::Magic::Vxcp
    } else {
        vexil_store::Magic::Vxc
    };

    let meta_hash = vexil_lang::canonical::schema_hash(meta);
    let header = vexil_store::VxbHeader {
        magic,
        format_version: vexil_store::FORMAT_VERSION,
        compressed: false,
        schema_hash: meta_hash,
        namespace: "vexil.schema".to_string(),
        schema_version: String::new(),
    };

    let mut buf = Vec::new();
    vexil_store::write_header(&header, &mut buf);
    buf.extend_from_slice(&payload);

    if let Err(e) = std::fs::write(output, &buf) {
        eprintln!("error: {output}: {e}");
        return 1;
    }

    let hex: String = schema_hash.iter().map(|b| format!("{b:02x}")).collect();
    eprintln!("compiled {ns} -> {output} (hash: {hex})");
    0
}

// ---------------------------------------------------------------------------
// Compat subcommand — JSON output types (serde stays out of vexil-lang)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct JsonChange {
    kind: String,
    declaration: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    detail: String,
    classification: String,
}

#[derive(Serialize)]
struct JsonReport {
    changes: Vec<JsonChange>,
    result: String,
    suggested_bump: String,
}

fn bump_str(b: BumpKind) -> &'static str {
    match b {
        BumpKind::Patch => "patch",
        BumpKind::Minor => "minor",
        BumpKind::Major => "major",
    }
}

fn kind_str(k: ChangeKind) -> &'static str {
    match k {
        ChangeKind::FieldAdded => "field_added",
        ChangeKind::FieldRemoved => "field_removed",
        ChangeKind::FieldTypeChanged => "field_type_changed",
        ChangeKind::FieldOrdinalChanged => "field_ordinal_changed",
        ChangeKind::FieldRenamed => "field_renamed",
        ChangeKind::FieldDeprecated => "field_deprecated",
        ChangeKind::FieldEncodingChanged => "field_encoding_changed",
        ChangeKind::VariantAdded => "variant_added",
        ChangeKind::VariantRemoved => "variant_removed",
        ChangeKind::VariantOrdinalChanged => "variant_ordinal_changed",
        ChangeKind::DeclarationAdded => "declaration_added",
        ChangeKind::DeclarationRemoved => "declaration_removed",
        ChangeKind::DeclarationKindChanged => "declaration_kind_changed",
        ChangeKind::NamespaceChanged => "namespace_changed",
        ChangeKind::NonExhaustiveChanged => "non_exhaustive_changed",
        ChangeKind::FlagsBitAdded => "flags_bit_added",
        ChangeKind::FlagsBitRemoved => "flags_bit_removed",
        ChangeKind::FlagsBitOrdinalChanged => "flags_bit_ordinal_changed",
    }
}

fn cmd_compat(old_file: &str, new_file: &str, format: &str) -> i32 {
    // Read and compile old schema
    let old_source = match std::fs::read_to_string(old_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {old_file}: {e}");
            return 2;
        }
    };
    let old_result = vexil_lang::compile(&old_source);
    for diag in &old_result.diagnostics {
        render_diagnostic(old_file, &old_source, diag);
    }
    if old_result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error)
    {
        return 2;
    }
    let old_compiled = match old_result.compiled {
        Some(c) => c,
        None => {
            eprintln!("error: {old_file}: compilation produced no output");
            return 2;
        }
    };

    // Read and compile new schema
    let new_source = match std::fs::read_to_string(new_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {new_file}: {e}");
            return 2;
        }
    };
    let new_result = vexil_lang::compile(&new_source);
    for diag in &new_result.diagnostics {
        render_diagnostic(new_file, &new_source, diag);
    }
    if new_result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error)
    {
        return 2;
    }
    let new_compiled = match new_result.compiled {
        Some(c) => c,
        None => {
            eprintln!("error: {new_file}: compilation produced no output");
            return 2;
        }
    };

    // Run compat check
    let report = vexil_lang::compat::check(&old_compiled, &new_compiled);

    match format {
        "json" => {
            let json_changes: Vec<JsonChange> = report
                .changes
                .iter()
                .map(|c| JsonChange {
                    kind: kind_str(c.kind).to_string(),
                    declaration: c.declaration.clone(),
                    field: c.field.clone(),
                    detail: c.detail.clone(),
                    classification: bump_str(c.classification).to_string(),
                })
                .collect();
            let json_report = JsonReport {
                changes: json_changes,
                result: match report.result {
                    CompatResult::Compatible => "compatible".to_string(),
                    CompatResult::Breaking => "breaking".to_string(),
                },
                suggested_bump: bump_str(report.suggested_bump).to_string(),
            };
            match serde_json::to_string_pretty(&json_report) {
                Ok(json) => println!("{json}"),
                Err(e) => {
                    eprintln!("error: JSON serialization failed: {e}");
                    return 2;
                }
            }
        }
        _ => {
            // Human format
            if report.changes.is_empty() {
                println!("No changes detected.");
            } else {
                for change in &report.changes {
                    let icon = if change.classification >= BumpKind::Major {
                        "\u{2717}" // ✗
                    } else {
                        "\u{2713}" // ✓
                    };
                    let level = match change.classification {
                        BumpKind::Patch => "patch",
                        BumpKind::Minor => "minor",
                        BumpKind::Major => "BREAKING",
                    };
                    let compat = if change.classification >= BumpKind::Major {
                        "BREAKING"
                    } else {
                        "compatible"
                    };
                    println!("  {icon} {} \u{2014} {compat} ({level})", change.detail);
                }
                println!();
                match report.result {
                    CompatResult::Compatible => {
                        println!(
                            "Result: COMPATIBLE \u{2014} suggests {} version bump",
                            bump_str(report.suggested_bump)
                        );
                    }
                    CompatResult::Breaking => {
                        println!("Result: BREAKING \u{2014} requires major version bump");
                    }
                }
            }
        }
    }

    match report.result {
        CompatResult::Compatible => 0,
        CompatResult::Breaking => 1,
    }
}

fn print_usage() {
    println!(
        "\
vexilc — the Vexil schema compiler

Usage: vexilc <subcommand> [args]

Subcommands:
  check    <file.vexil>                        Validate a schema
  codegen  <file.vexil> [options]              Generate code for one file
  build    <root.vexil> --include <dir> [opts] Generate code for a project
  watch    <file.vexil> [options]              Watch and rebuild on changes
  compat   <old.vexil> <new.vexil> [--format]  Compare schemas for breaking changes
  hash     <file.vexil>                        Print BLAKE3 schema hash
  init     [name]                              Create a new schema file
  format   <file.vx> --schema <s> --type <T>   Format a .vx text file
  pack     <file.vx> --schema <s> --type <T>   Encode .vx to .vxb binary
  unpack   <file.vxb> --schema <s> --type <T>  Decode .vxb to .vx text
  info     <file>                              Inspect .vxb/.vxc file headers
  compile  <file.vexil> -o <output>            Compile to .vxc binary schema

Options:
  --target <rust|typescript|go>   Code generation target (default: rust)
  --output <path>                 Output file or directory
  --include <dir>                 Additional schema search directory
  --format <human|json>           Output format for compat (default: human)
  -V, --version                   Print version
  -h, --help                      Print this help"
    );
}

fn cmd_init(name: &str) -> i32 {
    let filename = format!("{name}.vexil");
    if std::path::Path::new(&filename).exists() {
        eprintln!("error: {filename} already exists");
        return 1;
    }
    let content = format!(
        r#"namespace {name}

message Hello {{
    name     @0 : string
    greeting @1 : string
    count    @2 : u32
}}
"#
    );
    if let Err(e) = std::fs::write(&filename, &content) {
        eprintln!("error: {e}");
        return 1;
    }
    println!("Created {filename}");
    0
}

fn cmd_hash(path: &str) -> i32 {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {path}: {e}");
            return 1;
        }
    };
    let result = vexil_lang::compile(&source);
    for diag in &result.diagnostics {
        if diag.severity == vexil_lang::diagnostic::Severity::Error {
            render_diagnostic(path, &source, diag);
        }
    }
    match result.compiled {
        Some(compiled) => {
            let hash = vexil_lang::canonical::schema_hash(&compiled);
            let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
            println!("{hex}  {path}");
            0
        }
        None => 1,
    }
}

fn cmd_watch(root_file: &str, target: &str, output: Option<&str>, includes: &[String]) -> i32 {
    use notify::{RecursiveMode, Watcher};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    // Initial build
    println!("[watch] Initial build...");
    let code = match output {
        Some(out) => cmd_build(root_file, includes, out, target),
        None => cmd_check(root_file, includes),
    };
    if code == 0 {
        println!("[watch] Ready. Watching for changes...");
    } else {
        println!("[watch] Initial build had errors. Watching for changes...");
    }

    let (tx, rx) = mpsc::channel();

    let mut watcher = match notify::recommended_watcher(tx) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: failed to create file watcher: {e}");
            return 1;
        }
    };

    // Watch the root file's directory
    let root_path = std::path::Path::new(root_file);
    if let Some(parent) = root_path.parent() {
        let watch_dir = if parent.as_os_str().is_empty() {
            std::path::Path::new(".")
        } else {
            parent
        };
        if let Err(e) = watcher.watch(watch_dir, RecursiveMode::Recursive) {
            eprintln!("error: failed to watch {}: {e}", watch_dir.display());
            return 1;
        }
    }

    // Watch include directories
    for inc in includes {
        let inc_path = std::path::Path::new(inc);
        if let Err(e) = watcher.watch(inc_path, RecursiveMode::Recursive) {
            eprintln!("error: failed to watch {inc}: {e}");
            return 1;
        }
    }

    // Debounce loop
    let debounce = Duration::from_millis(200);
    let mut last_build = Instant::now();

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                // Only react to modifications and creations
                if !event.kind.is_modify() && !event.kind.is_create() {
                    continue;
                }

                // Only react to .vexil file changes
                let dominated_by_vexil = event
                    .paths
                    .iter()
                    .any(|p| p.extension().map(|ext| ext == "vexil").unwrap_or(false));
                if !dominated_by_vexil {
                    continue;
                }

                // Drain any queued events
                while rx.try_recv().is_ok() {}

                // Debounce
                let now = Instant::now();
                if now.duration_since(last_build) < debounce {
                    continue;
                }
                last_build = now;

                println!("\n[watch] Change detected, rebuilding...");
                let code = match output {
                    Some(out) => cmd_build(root_file, includes, out, target),
                    None => cmd_check(root_file, includes),
                };
                if code == 0 {
                    println!("[watch] OK");
                } else {
                    println!("[watch] Build failed (see errors above)");
                }
            }
            Ok(Err(e)) => {
                eprintln!("[watch] Watcher error: {e}");
            }
            Err(_) => break,
        }
    }

    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-V" => {
                println!("vexilc {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--help" | "-h" | "help" => {
                print_usage();
                return;
            }
            _ => {}
        }
    }

    match args.get(1).map(|s| s.as_str()) {
        Some("check") => {
            if args.len() < 3 {
                eprintln!("Usage: vexilc check <file.vexil> [--include <dir> ...]");
                std::process::exit(1);
            }
            let filename = &args[2];
            let mut include_paths = Vec::new();
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--include" => {
                        i += 1;
                        if i < args.len() {
                            include_paths.push(args[i].clone());
                        }
                    }
                    other => {
                        eprintln!("unknown option: {other}");
                        std::process::exit(1);
                    }
                }
                i += 1;
            }
            std::process::exit(cmd_check(filename, &include_paths));
        }
        Some("compile") => {
            // vexilc compile <file.vexil> -o <out.vxc|out.vxcp>
            if args.len() < 5 {
                eprintln!("Usage: vexilc compile <file.vexil> -o <output.vxc|output.vxcp>");
                std::process::exit(1);
            }
            let filename = &args[2];
            let mut output = None;
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
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
            let output = output.unwrap_or_else(|| {
                eprintln!("-o required");
                std::process::exit(1);
            });
            std::process::exit(cmd_compile(filename, output));
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
        Some("watch") => {
            if args.len() < 3 {
                eprintln!("Usage: vexilc watch <file.vexil> [--target <rust|typescript|go>] [--output <dir>] [--include <dir>]");
                std::process::exit(1);
            }
            let root_file = &args[2];
            let mut target = "rust".to_string();
            let mut output = None::<String>;
            let mut includes = Vec::new();
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--target" => {
                        i += 1;
                        if i < args.len() {
                            target = args[i].clone();
                        }
                    }
                    "--output" | "-o" => {
                        i += 1;
                        if i < args.len() {
                            output = Some(args[i].clone());
                        }
                    }
                    "--include" => {
                        i += 1;
                        if i < args.len() {
                            includes.push(args[i].clone());
                        }
                    }
                    other => {
                        eprintln!("unknown option: {other}");
                        std::process::exit(1);
                    }
                }
                i += 1;
            }
            std::process::exit(cmd_watch(root_file, &target, output.as_deref(), &includes));
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
        Some("compat") => {
            // vexilc compat <old.vexil> <new.vexil> [--format human|json]
            if args.len() < 4 {
                eprintln!("Usage: vexilc compat <old.vexil> <new.vexil> [--format human|json]");
                std::process::exit(1);
            }
            let old_file = &args[2];
            let new_file = &args[3];
            let mut format = "human";
            let mut i = 4;
            while i < args.len() {
                match args[i].as_str() {
                    "--format" => {
                        i += 1;
                        if i < args.len() {
                            format = args[i].as_str();
                        }
                    }
                    other => {
                        eprintln!("unknown option: {other}");
                        std::process::exit(1);
                    }
                }
                i += 1;
            }
            if format != "human" && format != "json" {
                eprintln!("error: --format must be 'human' or 'json'");
                std::process::exit(1);
            }
            std::process::exit(cmd_compat(old_file, new_file, format));
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
        Some("hash") => {
            if args.len() < 3 {
                eprintln!("Usage: vexilc hash <file.vexil>");
                std::process::exit(1);
            }
            std::process::exit(cmd_hash(&args[2]));
        }
        Some("init") => {
            let name = if args.len() > 2 { &args[2] } else { "example" };
            std::process::exit(cmd_init(name));
        }
        _ => {
            print_usage();
            std::process::exit(1);
        }
    }
}
