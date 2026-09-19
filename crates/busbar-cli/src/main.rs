//! `busbar` — native CLI for the ESLD toolchain.
//!
//! Command surface per `Design/esld-implementation.md` §5.7. Phase 1
//! implements `check` and `fmt`; render/solve/export arrive with M5/M6/M2.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") => cmd_check(&args[1..]),
        Some("fmt") => cmd_fmt(&args[1..]),
        Some("--version" | "-V") => {
            println!(
                "busbar {} / spec esld/1.0 (draft)",
                env!("CARGO_PKG_VERSION")
            );
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "busbar {} — Electrical Single Line Diagram toolchain (ESLD)

USAGE:
  busbar <COMMAND> <FILE...>

COMMANDS:
  check    Parse + validate; report diagnostics
  fmt      Canonical formatting (round-trip safe)
  -V       Version",
                env!("CARGO_PKG_VERSION")
            );
            ExitCode::from(2)
        }
    }
}

fn cmd_check(args: &[String]) -> ExitCode {
    let mut any_errors = false;
    let mut usage_error = false;
    for arg in args {
        let path = PathBuf::from(arg);
        let Ok(source) = std::fs::read_to_string(&path) else {
            eprintln!("error: cannot read {}", path.display());
            usage_error = true;
            continue;
        };
        let dir = path.parent().map(|p| p.to_path_buf());
        match busbar_check::check(&source, dir.as_deref()) {
            Ok(diags) => {
                for d in &diags {
                    let sev = match d.severity {
                        busbar_check::Severity::Error => "error",
                        busbar_check::Severity::Warning => "warning",
                    };
                    println!(
                        "{}:{}: {}[{}]: {}",
                        path.display(),
                        d.line,
                        sev,
                        d.code,
                        d.message
                    );
                    if d.severity == busbar_check::Severity::Error {
                        any_errors = true;
                    }
                }
            }
            Err(e) => {
                eprintln!("error: {}: {e}", path.display());
                any_errors = true;
            }
        }
    }
    if args.is_empty() {
        usage_error = true;
    }
    if usage_error {
        return ExitCode::from(2);
    }
    if any_errors {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn cmd_fmt(args: &[String]) -> ExitCode {
    let check_only = args.iter().any(|a| a == "--check");
    let files: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if files.is_empty() {
        return ExitCode::from(2);
    }
    let mut failed = false;
    for arg in files {
        let path = PathBuf::from(arg);
        let Ok(source) = std::fs::read_to_string(&path) else {
            eprintln!("error: cannot read {}", path.display());
            failed = true;
            continue;
        };
        match busbar_syntax::fmt::format(&source) {
            Ok(formatted) => {
                if check_only {
                    if formatted != source {
                        println!("{}: not formatted", path.display());
                        failed = true;
                    }
                } else {
                    if let Err(e) = std::fs::write(&path, &formatted) {
                        eprintln!("error: cannot write {}: {e}", path.display());
                        failed = true;
                    }
                }
            }
            Err(e) => {
                eprintln!("error: {}: line {}: {}", path.display(), e.line, e.message);
                failed = true;
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
