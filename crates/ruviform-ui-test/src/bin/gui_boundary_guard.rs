//! Issue #390 driver boundary guard CLI。
//!
//! `cargo metadata` の依存グラフ検査 + syn による public signature
//! 漏出検査を ruleset（`fixtures/driver-lab/boundary.json`）に
//! 従って実行し、verdict を返す。
//!
//! 使い方:
//!   gui-boundary-guard [--ruleset <path>] [--manifest-path <path>]
//!                      [--metadata <json>] [--format human|json]
//!                      [--offline]
//!
//! exit code: 0 = pass / 1 = violation / 2 = tool error。

use std::path::PathBuf;
use std::process::ExitCode;

use cargo_metadata::MetadataCommand;
use ruviform_ui_test::driver_boundary;

fn usage() -> ! {
    eprintln!(
        "usage: gui-boundary-guard [--ruleset <path>] [--manifest-path <path>] \
         [--metadata <json>] [--format human|json] [--offline]"
    );
    std::process::exit(2);
}

fn main() -> ExitCode {
    let mut ruleset_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/driver-lab/boundary.json");
    let mut manifest_path: Option<PathBuf> = None;
    let mut metadata_file: Option<PathBuf> = None;
    let mut format = "human".to_string();
    let mut offline = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--ruleset" => ruleset_path = PathBuf::from(args.next().unwrap_or_else(|| usage())),
            "--manifest-path" => {
                manifest_path = Some(PathBuf::from(args.next().unwrap_or_else(|| usage())));
            }
            "--metadata" => {
                metadata_file = Some(PathBuf::from(args.next().unwrap_or_else(|| usage())));
            }
            "--format" => {
                format = args.next().unwrap_or_else(|| usage());
            }
            "--offline" => offline = true,
            _ => usage(),
        }
    }
    if format != "human" && format != "json" {
        usage();
    }

    let ruleset = match driver_boundary::load_ruleset(&ruleset_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[gui-boundary-guard] {e}");
            return ExitCode::from(2);
        }
    };

    let metadata = if let Some(path) = &metadata_file {
        match std::fs::File::open(path) {
            Ok(f) => match serde_json::from_reader(f) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("[gui-boundary-guard] metadata JSON parse 失敗: {e}");
                    return ExitCode::from(2);
                }
            },
            Err(e) => {
                eprintln!(
                    "[gui-boundary-guard] metadata {} を読めません: {e}",
                    path.display()
                );
                return ExitCode::from(2);
            }
        }
    } else {
        let mut cmd = MetadataCommand::new();
        if let Some(mp) = &manifest_path {
            cmd.manifest_path(mp);
        }
        if offline {
            cmd.other_options(vec!["--offline".to_string()]);
        }
        match cmd.exec() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[gui-boundary-guard] cargo metadata 失敗: {e}");
                return ExitCode::from(2);
            }
        }
    };

    let report = driver_boundary::run(&ruleset, &metadata);

    if format.as_str() == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
    } else {
        println!("[gui-boundary-guard] verdict: {}", report.verdict);
        for check in &report.checks {
            println!("  {}: {}", check.id, check.status);
            for f in &check.findings {
                let loc = match (&f.file, f.line) {
                    (Some(file), Some(line)) => format!(" {file}:{line}"),
                    (Some(file), None) => format!(" {file}"),
                    _ => String::new(),
                };
                println!("    [{}] {}{} — {}", f.code, f.package, loc, f.detail);
            }
        }
    }

    if report.verdict == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
