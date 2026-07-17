use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = match (args.next().as_deref(), args.next()) {
        (Some("--root"), Some(path)) => PathBuf::from(path),
        _ => {
            eprintln!("Usage: cargo run --manifest-path release/validator/Cargo.toml --offline -- --root <repository-root>");
            std::process::exit(2);
        }
    };
    match vexil_release_governance_validator::validate_repository(&root) {
        Ok(()) => println!("stewardship records valid; contract validation does not prove live workflow or provider enforcement; the current unresolved continuity decision blocks Manifest approval and privileged publication, and Epic 2 external controls remain required"),
        Err(error) => {
            eprintln!("stewardship validation failed: {error}");
            std::process::exit(1);
        }
    }
}
