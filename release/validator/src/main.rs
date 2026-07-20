use std::path::PathBuf;
use std::process::Command;

fn main() {
    let mut args = std::env::args().skip(1);
    let mut root = None;
    let mut observe = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--root" => root = args.next().map(PathBuf::from),
            "--observe" => observe = args.next(),
            _ => {
                eprintln!("Usage: cargo run --manifest-path release/validator/Cargo.toml --offline -- --root <repository-root> [--observe <assertion-id>]");
                std::process::exit(2);
            }
        }
    }
    let Some(root) = root else {
        eprintln!("Usage: cargo run --manifest-path release/validator/Cargo.toml --offline -- --root <repository-root>");
        std::process::exit(2);
    };
    if let Some(assertion_id) = observe {
        let (provider, path) = match vexil_release_governance_validator::expected_observation_query(
            &root,
            &assertion_id,
        ) {
            Ok(query) => query,
            Err(error) => {
                eprintln!("observation request rejected: {error}");
                std::process::exit(2);
            }
        };
        if provider != "github" {
            eprintln!("observation request rejected: provider {provider} requires its dedicated read-only collector");
            std::process::exit(2);
        }
        let status = Command::new("gh")
            .args(["api", "--method", "GET", &path, "--silent"])
            .status()
            .unwrap_or_else(|error| {
                eprintln!("unable to start the GitHub GET-only collector: {error}");
                std::process::exit(1);
            });
        if !status.success() {
            eprintln!("GET-only observation failed for {assertion_id}");
            std::process::exit(1);
        }
        println!("GET-only observation completed for {assertion_id}; no provider state or repository evidence was changed.");
        return;
    }
    match vexil_release_governance_validator::validate_repository(&root) {
        Ok(()) => println!("stewardship records valid; contract validation does not prove live workflow or provider enforcement; the current unresolved continuity decision blocks Manifest approval and privileged publication, and unresolved external controls remain required"),
        Err(error) => {
            eprintln!("stewardship validation failed: {error}");
            std::process::exit(1);
        }
    }
}
