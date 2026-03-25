fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: vexilc <file.vexil>");
        std::process::exit(1);
    }
    let source = match std::fs::read_to_string(&args[1]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}: {e}", args[1]);
            std::process::exit(1);
        }
    };
    let result = vexil_lang::parse(&source);
    // TODO: ariadne rendering
    for diag in &result.diagnostics {
        eprintln!("{:?}", diag);
    }
    if result
        .diagnostics
        .iter()
        .any(|d| d.severity == vexil_lang::diagnostic::Severity::Error)
    {
        std::process::exit(1);
    }
}
