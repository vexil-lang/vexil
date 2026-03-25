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
    let result = vexil_lang::parse(&source);
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
    let code = match vexil_codegen::generate(&compiled) {
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
        _ => {
            eprintln!("Usage: vexilc <subcommand> [args]");
            eprintln!("  vexilc check <file.vexil>");
            eprintln!("  vexilc codegen <file.vexil> [--output <path>]");
            std::process::exit(1);
        }
    }
}
