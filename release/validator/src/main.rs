use std::path::PathBuf;
use std::process::Command;

fn main() {
    let mut args = std::env::args().skip(1);
    let mut root = None;
    let mut observe = None;
    let mut collect_history_tags = None;
    let mut render_catalog = false;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--root" => match args.next() {
                Some(value) if !value.starts_with("--") => root = Some(PathBuf::from(value)),
                _ => usage_and_exit(),
            },
            "--observe" => match args.next() {
                Some(value) if !value.starts_with("--") => observe = Some(value),
                _ => usage_and_exit(),
            },
            "--collect-history-tags" => match args.next() {
                Some(value) if !value.starts_with("--") => collect_history_tags = Some(value),
                _ => usage_and_exit(),
            },
            "--render-catalog" => render_catalog = true,
            _ => {
                usage_and_exit();
            }
        }
    }
    if let Some(remote) = collect_history_tags {
        if root.is_some() || observe.is_some() || render_catalog {
            eprintln!("--collect-history-tags is a standalone read-only collector");
            std::process::exit(2);
        }
        let output = Command::new("git")
            .args(["ls-remote", "--tags", &remote])
            .output()
            .unwrap_or_else(|error| {
                eprintln!("unable to start the read-only historical-tag collector: {error}");
                std::process::exit(1);
            });
        if !output.status.success() {
            eprintln!("read-only historical-tag collection failed; no repository or provider state was changed");
            std::process::exit(1);
        }
        let stdout = String::from_utf8(output.stdout).unwrap_or_else(|error| {
            eprintln!("read-only historical-tag collector returned non-UTF-8 output: {error}");
            std::process::exit(1);
        });
        let observed_at = current_utc_timestamp();
        match vexil_release_governance_validator::parse_history_tag_collection(
            &remote,
            &stdout,
            &observed_at,
        ) {
            Ok(record) => println!("{}", serde_json::to_string_pretty(&record).unwrap()),
            Err(error) => {
                eprintln!("read-only historical-tag collection rejected: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    let Some(root) = root else {
        usage_and_exit();
    };
    if render_catalog {
        if observe.is_some() {
            eprintln!("--render-catalog cannot be combined with --observe");
            std::process::exit(2);
        }
        let catalog_path = root.join("release/catalog.json");
        let catalog = std::fs::read_to_string(&catalog_path).unwrap_or_else(|error| {
            eprintln!("unable to read {}: {error}", catalog_path.display());
            std::process::exit(1);
        });
        let catalog = serde_json::from_str(&catalog).unwrap_or_else(|error| {
            eprintln!("unable to parse {}: {error}", catalog_path.display());
            std::process::exit(1);
        });
        let validation = vexil_release_governance_validator::validate_catalog_schema(
            &root, &catalog,
        )
        .and_then(|()| vexil_release_governance_validator::validate_catalog(&root, &catalog));
        if let Err(error) = validation {
            eprintln!("catalog render rejected: {error}");
            std::process::exit(1);
        }
        match vexil_release_governance_validator::render_catalog_markdown(&root, &catalog) {
            Ok(markdown) => print!("{markdown}"),
            Err(error) => {
                eprintln!("catalog render rejected: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
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

fn usage_and_exit() -> ! {
    eprintln!("Usage: cargo run --manifest-path release/validator/Cargo.toml --offline -- --root <repository-root> [--observe <assertion-id>] | --collect-history-tags <remote> | --render-catalog --root <repository-root>");
    std::process::exit(2);
}

fn current_utc_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (seconds / 86_400) as i64;
    let remainder = seconds % 86_400;
    let (year, month, day) = civil_from_unix_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        remainder / 3_600,
        (remainder % 3_600) / 60,
        remainder % 60
    )
}

fn civil_from_unix_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (year + if month <= 2 { 1 } else { 0 }, month, day)
}
