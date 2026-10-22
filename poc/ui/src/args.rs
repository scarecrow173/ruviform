//! CLI 引数パース（std のみ。clap 等の引数クレートは入れない — design 確定事項）。
//!
//! 契約:
//! - `--check`                : capability 検出 JSON を出して exit 0（GUI 非生成）
//! - `--experiment <name>`    : 実験名（host / render / surface / ipc / frontend / dist）
//! - `--frames N`             : フレーム数ベースの自己終了条件（既定 600）
//! - `--duration SEC`         : 時間ベースの自己終了条件（実験別既定。host は 20）
//! - `--backend <name>`       : 描画バックエンド選択（canvas2d|webgl2|webgpu）
//! - `--mode <name>`          : 実験モード（adjacent-parent|adjacent-child 等）
//! - `--variant <name>`       : フロントエンド variant（vanilla|solid 等）
//! - `--out <path>`           : 生サンプル JSONL の追記先（既定 out/<exp>-<ts>.jsonl）

/// パース済みコマンド。
pub enum Command {
    /// `--check`: capability 検出のみ（GUI を作らない）。
    Check,
    /// `-h`/`--help`: usage を stdout に出して exit 0。
    Help,
    /// `--experiment ...`: 実験実行。
    Run(RunArgs),
}

/// `--experiment` 実行時の引数束。
pub struct RunArgs {
    /// 実験名。
    pub experiment: String,
    /// フレーム数（自己終了条件）。実験がフレーム駆動でない場合は記録のみ。
    pub frames: u64,
    /// タイムアウト秒（自己終了条件）。None は実験既定。
    pub duration_secs: Option<u64>,
    /// `--backend` 値（render 実験用）。
    pub backend: Option<String>,
    /// `--mode` 値（surface 実験用）。
    pub mode: Option<String>,
    /// `--variant` 値（frontend 実験用）。
    pub variant: Option<String>,
    /// `--out` で指定された JSONL 追記先。None は既定パスを後で解決。
    pub out: Option<String>,
    /// `--peaks` 値（render 実験のワークロード列数。Issue #57 高負荷再計測）。
    pub peaks: Option<u64>,
    /// `--meters` 値（render 実験のメータチャンネル数。同上）。
    pub meters: Option<u64>,
    /// `--preview`（render 実験に 720p 合成プレビューレイヤーを追加。同上）。
    pub preview: bool,
}

/// 引数エラー（exit 2 / CLI エラーに対応）。
pub struct ArgsError(pub String);

const USAGE: &str = "usage: ui-poc --check\n\
       ui-poc --experiment <name> [--frames N] [--duration SEC] \
       [--backend <b>] [--mode <m>] [--variant <v>] [--out <path>]\n\
       [--peaks N] [--meters N] [--preview]\n\
       \n\
       experiments: host render ipc surface frontend dist\n\
       \n\
       exit codes: 0 = passed/skipped, 1 = failed, 2 = CLI/internal error";

/// usage 文字列（`-h`/`--help` の stdout 出力とエラーメッセージで共用）。
pub fn usage() -> &'static str {
    USAGE
}

/// `std::env::args()` をパースする。
pub fn parse() -> Result<Command, ArgsError> {
    parse_from(std::env::args().skip(1))
}

fn parse_from<I: Iterator<Item = String>>(mut it: I) -> Result<Command, ArgsError> {
    let mut experiment: Option<String> = None;
    let mut frames: u64 = 600;
    let mut duration_secs: Option<u64> = None;
    let mut backend: Option<String> = None;
    let mut mode: Option<String> = None;
    let mut variant: Option<String> = None;
    let mut out: Option<String> = None;
    let mut peaks: Option<u64> = None;
    let mut meters: Option<u64> = None;
    let mut preview = false;
    let mut check = false;

    let take = |flag: &str, it: &mut I| -> Result<String, ArgsError> {
        it.next()
            .ok_or_else(|| ArgsError(format!("{flag}: missing value\n{USAGE}")))
    };

    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--experiment" => experiment = Some(take("--experiment", &mut it)?),
            "--frames" => {
                let v = take("--frames", &mut it)?;
                frames = v
                    .parse::<u64>()
                    .map_err(|_| ArgsError(format!("--frames: invalid integer '{v}'\n{USAGE}")))?;
            }
            "--duration" => {
                let v = take("--duration", &mut it)?;
                duration_secs = Some(v.parse::<u64>().map_err(|_| {
                    ArgsError(format!("--duration: invalid seconds '{v}'\n{USAGE}"))
                })?);
            }
            "--backend" => backend = Some(take("--backend", &mut it)?),
            "--mode" => mode = Some(take("--mode", &mut it)?),
            "--variant" => variant = Some(take("--variant", &mut it)?),
            "--out" => out = Some(take("--out", &mut it)?),
            "--peaks" => {
                let v = take("--peaks", &mut it)?;
                peaks =
                    Some(v.parse::<u64>().map_err(|_| {
                        ArgsError(format!("--peaks: invalid integer '{v}'\n{USAGE}"))
                    })?);
            }
            "--meters" => {
                let v = take("--meters", &mut it)?;
                meters =
                    Some(v.parse::<u64>().map_err(|_| {
                        ArgsError(format!("--meters: invalid integer '{v}'\n{USAGE}"))
                    })?);
            }
            "--preview" => preview = true,
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(ArgsError(format!("unknown argument '{other}'\n{USAGE}"))),
        }
    }

    if check {
        return Ok(Command::Check);
    }
    match experiment {
        Some(name) => Ok(Command::Run(RunArgs {
            experiment: name,
            frames,
            duration_secs,
            backend,
            mode,
            variant,
            out,
            peaks,
            meters,
            preview,
        })),
        None => Err(ArgsError(format!(
            "missing --experiment or --check\n{USAGE}"
        ))),
    }
}
