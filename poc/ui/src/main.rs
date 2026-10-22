//! ui-poc — Issue #28 UI 技術検証 PoC の単一バイナリハーネス。
//!
//! 終了コード規約（design/test-spec 正本）:
//! - 0: `status:"passed"` または `status:"skipped"`（前提不足・reason 非空）
//! - 1: `status:"failed"`（実験自体の失敗）
//! - 2: CLI エラー（引数不正・未知の実験名）または内部エラー（証跡書き出し失敗等）
//!
//! stdout は最終行の `poc-metric.v1` JSON のために予約し、診断は stderr のみ。

mod args;
mod assets;
mod check;
mod exp;
mod host;
mod metrics;

use std::path::PathBuf;

use serde_json::json;

fn main() {
    std::process::exit(real_main());
}

fn real_main() -> i32 {
    match args::parse() {
        Err(e) => {
            eprintln!("{}", e.0);
            2
        }
        Ok(args::Command::Help) => {
            // usage は stdout（通常操作でありエラーではない）。exit 0。
            println!("{}", args::usage());
            0
        }
        Ok(args::Command::Check) => {
            // capability 検出のみ。GUI は作らず常に exit 0。
            let env = check::detect();
            match serde_json::to_string(&check::check_json(&env)) {
                Ok(line) => {
                    println!("{line}");
                    0
                }
                Err(e) => {
                    eprintln!("error: failed to serialize check result: {e}");
                    2
                }
            }
        }
        Ok(args::Command::Run(run_args)) => run_experiment(&run_args),
    }
}

fn run_experiment(run_args: &args::RunArgs) -> i32 {
    let env = check::detect();
    let Some(mut experiment) = exp::lookup(&run_args.experiment) else {
        eprintln!(
            "error: unknown experiment '{}' (known: {})",
            run_args.experiment,
            exp::known_names().join(", ")
        );
        return 2;
    };

    // 前提ゲーティング → skipped は reason 非空で metrics 空のレコードを出す。
    let out_path = run_args
        .out
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| metrics::default_out_path(&run_args.experiment));
    let params = json!({
        "frames": run_args.frames,
        "duration_secs": run_args.duration_secs,
        "backend": run_args.backend,
        "mode": run_args.mode,
        "variant": run_args.variant,
        "peaks": run_args.peaks,
        "meters": run_args.meters,
        "preview": run_args.preview,
        "out": out_path.to_string_lossy(),
    });

    let outcome = match experiment.setup(&env) {
        Err(exp::Skip(reason)) => exp::Outcome {
            metrics: json!({}),
            status: metrics::Status::Skipped(reason),
        },
        Ok(()) => {
            let outcome = experiment.run(&env, run_args);
            experiment.teardown();
            outcome
        }
    };

    // レコードの experiment 名はレジストリ側の正規名を使う（CLI 入力ではなく）。
    let record = metrics::record(
        experiment.name(),
        &env,
        params,
        outcome.metrics,
        outcome.status,
    );

    if let Err(e) = metrics::emit(&record, &out_path) {
        eprintln!(
            "error: failed to write evidence to {}: {e}",
            out_path.display()
        );
        // 書き出し失敗でも最終 JSON 行は残す（契約の構造化 status を守る）。
        if let Ok(line) = serde_json::to_string(&record.to_json()) {
            println!("{line}");
        }
        return 2;
    }
    record.status.exit_code()
}
