//! Contract tests for the `logging` feature — Issue #276 (OBS-01).
//!
//! Built only with `--features logging` (`required-features` in Cargo.toml).
//! Most tests route through rolling-file sinks with `RotationPeriod::Never`
//! so the file name is exactly `{prefix}` and content checks are
//! deterministic; shutdown is always explicit (`drop(result)`) so the
//! worker drains before assertions run.

use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use ruviform_observability::logging::{
    LogFormat, LoggingBuildError, LoggingConfig, RotationPeriod, SinkConfig, SinkDestination,
    SinkFailureMode, SinkKind, SinkState, SpanEvents, build_logging, build_logging_layers,
};

fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}

fn file_sink(dir: &Path, prefix: &str, filter: &str, format: LogFormat) -> SinkConfig {
    file_sink_with_mode(dir, prefix, filter, format, SinkFailureMode::Required)
}

fn file_sink_with_mode(
    dir: &Path,
    prefix: &str,
    filter: &str,
    format: LogFormat,
    failure_mode: SinkFailureMode,
) -> SinkConfig {
    SinkConfig {
        destination: SinkDestination::RollingFile {
            directory: dir.to_path_buf(),
            filename_prefix: prefix.to_owned(),
            filename_suffix: None,
            rotation: RotationPeriod::Never,
            max_files: nz(4),
            queue_capacity: nz(64),
        },
        format,
        span_events: SpanEvents::NONE,
        filter: filter.to_owned(),
        failure_mode,
    }
}

fn file_sink_with_span_events(
    dir: &Path,
    prefix: &str,
    filter: &str,
    format: LogFormat,
    span_events: SpanEvents,
) -> SinkConfig {
    SinkConfig {
        span_events,
        ..file_sink(dir, prefix, filter, format)
    }
}

/// Reads every file under `dir` whose name starts with `prefix`,
/// concatenated in name order. With `RotationPeriod::Never` there is
/// exactly one such file (`{prefix}`); with real rotation the current
/// dated file also matches.
fn read_sink_files(dir: &Path, prefix: &str) -> String {
    let mut names: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (path.is_file() && name.starts_with(prefix)).then_some(path)
        })
        .collect();
    names.sort();
    let mut out = String::new();
    for name in names {
        out.push_str(&fs::read_to_string(name).unwrap());
    }
    out
}

fn matching_file_count(dir: &Path, prefix: &str) -> usize {
    fs::read_dir(dir)
        .unwrap()
        .filter(|entry| {
            let path = entry.as_ref().unwrap().path();
            path.is_file()
                && path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(prefix)
        })
        .count()
}

/// Sorted names of the regular files directly under `dir`.
fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            path.is_file()
                .then(|| path.file_name().unwrap().to_string_lossy().into_owned())
        })
        .collect();
    names.sort();
    names
}

/// Asserts `part` has the exact `YYYY-MM-DD` shape daily rotation emits
/// (ten chars, dashes at 4/7, digits elsewhere).
fn assert_daily_date_shape(part: &str, full_name: &str) {
    let bytes = part.as_bytes();
    assert_eq!(bytes.len(), 10, "not a daily date in {full_name}");
    assert_eq!(bytes[4], b'-', "not a daily date in {full_name}");
    assert_eq!(bytes[7], b'-', "not a daily date in {full_name}");
    for (index, byte) in bytes.iter().enumerate() {
        if index == 4 || index == 7 {
            continue;
        }
        assert!(
            byte.is_ascii_digit(),
            "non-digit date character in {full_name}"
        );
    }
}

// E1: sink 別 filter — 同一イベントが独立にルーティングされる。
#[test]
fn dual_sinks_have_independent_filters() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![
            file_sink(dir.path(), "all", "info", LogFormat::Text),
            file_sink(dir.path(), "warns", "warn", LogFormat::Text),
        ],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info!(target: "contract", "an info event");
        tracing::warn!(target: "contract", "a warn event");
    });
    drop(result);

    let all = read_sink_files(dir.path(), "all");
    let warns = read_sink_files(dir.path(), "warns");
    assert!(all.contains("an info event"), "info sink got: {all}");
    assert!(all.contains("a warn event"), "info sink got: {all}");
    assert!(
        !warns.contains("an info event"),
        "warn sink wrongly got: {warns}"
    );
    assert!(warns.contains("a warn event"), "warn sink got: {warns}");
}

// E1b: target 別 filter — 指定 target のみ通る。
#[test]
fn sink_filter_can_select_by_target() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![file_sink(
            dir.path(),
            "targeted",
            "wanted_target=trace",
            LogFormat::Text,
        )],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info!(target: "wanted_target", "picked up");
        tracing::info!(target: "other_target", "filtered out");
    });
    drop(result);

    let content = read_sink_files(dir.path(), "targeted");
    assert!(content.contains("picked up"), "got: {content}");
    assert!(!content.contains("filtered out"), "got: {content}");
}

// E2: JsonLines — 1 event = 1 JSON object、level/target/fields を含む。
#[test]
fn jsonl_sink_emits_one_object_per_event_with_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![file_sink(dir.path(), "json", "info", LogFormat::JsonLines)],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info!(target: "contract::json", answer = 42, "json event");
    });
    drop(result);

    let content = read_sink_files(dir.path(), "json");
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "expected 1 JSONL line, got: {content}");
    let value: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(value["level"], "INFO");
    assert_eq!(value["target"], "contract::json");
    assert_eq!(value["fields"]["answer"], 42);
    assert_eq!(value["fields"]["message"], "json event");
    assert!(value["timestamp"].is_string(), "missing timestamp: {value}");
}

// R2: Text format は level・target・message を含む人間可読行。
#[test]
fn text_sink_writes_human_readable_line() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![file_sink(dir.path(), "text", "info", LogFormat::Text)],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::warn!(target: "contract::text", "text event");
    });
    drop(result);

    let content = read_sink_files(dir.path(), "text");
    assert!(content.contains("WARN"), "got: {content}");
    assert!(content.contains("contract::text"), "got: {content}");
    assert!(content.contains("text event"), "got: {content}");
}

// E3 / R5: retention — 構築時に max_files を超えた古いファイルを削除する。
#[test]
fn rolling_file_retention_deletes_oldest_at_build() {
    let dir = tempfile::tempdir().unwrap();
    for day in 1..=5 {
        fs::write(
            dir.path().join(format!("app.2026-01-0{day}")),
            format!("day {day}\n"),
        )
        .unwrap();
    }
    // max_files=3・daily rotation・prefix "app" — 構築時に古い順で削除。
    let result = build_logging(&LoggingConfig {
        sinks: vec![SinkConfig {
            destination: SinkDestination::RollingFile {
                directory: dir.path().to_path_buf(),
                filename_prefix: "app".to_owned(),
                filename_suffix: None,
                rotation: RotationPeriod::Daily,
                max_files: nz(3),
                queue_capacity: nz(64),
            },
            format: LogFormat::Text,
            span_events: SpanEvents::NONE,
            filter: "info".to_owned(),
            failure_mode: SinkFailureMode::Required,
        }],
    })
    .unwrap();

    // 当日ファイルを 1 件作る — 候補は seed 5 + 当日 1 = 6。
    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info!("retention probe");
    });
    drop(result);

    let remaining = matching_file_count(dir.path(), "app.");
    assert!(
        remaining <= 3,
        "expected <= 3 retained files, found {remaining}"
    );
    assert!(
        !dir.path().join("app.2026-01-01").exists() && !dir.path().join("app.2026-01-02").exists(),
        "oldest files must be deleted first"
    );
}

// Issue #334: `filename_suffix` — `{prefix}.{rotation-date}.{suffix}` 契約。
// AudioLens AL-OBS-01 互換確認: daily + prefix "al" + suffix "log" で
// `al.YYYY-MM-DD.log` が生成される。
#[test]
fn rolling_file_with_suffix_produces_dated_suffixed_name() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![SinkConfig {
            destination: SinkDestination::RollingFile {
                directory: dir.path().to_path_buf(),
                filename_prefix: "al".to_owned(),
                filename_suffix: Some("log".to_owned()),
                rotation: RotationPeriod::Daily,
                max_files: nz(7),
                queue_capacity: nz(64),
            },
            format: LogFormat::Text,
            span_events: SpanEvents::NONE,
            filter: "info".to_owned(),
            failure_mode: SinkFailureMode::Required,
        }],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info!("suffix event");
    });
    drop(result);

    // 唯一のファイルが `al.YYYY-MM-DD.log` の形であること。
    let names = file_names(dir.path());
    assert_eq!(names.len(), 1, "expected exactly one log file: {names:?}");
    let name = &names[0];
    let date_part = name
        .strip_prefix("al.")
        .and_then(|n| n.strip_suffix(".log"))
        .unwrap_or_else(|| panic!("expected `al.YYYY-MM-DD.log`, got: {name}"));
    assert_daily_date_shape(date_part, name);
    assert!(read_sink_files(dir.path(), "al.").contains("suffix event"));
}

// Issue #334 補強: retention は prefix+suffix 双方に整合 — suffix 付き
// ファイル群だけが max_files の対象になり、suffix 不一致は残る。
#[test]
fn rolling_file_retention_matches_prefixed_and_suffixed_files() {
    let dir = tempfile::tempdir().unwrap();
    for day in 1..=5 {
        fs::write(
            dir.path().join(format!("app.2026-01-0{day}.log")),
            format!("day {day}\n"),
        )
        .unwrap();
    }
    // prefix 一致・suffix 不一致は retention 対象外に残るはず。
    fs::write(dir.path().join("app.2020-01-01.txt"), "not a log\n").unwrap();

    let result = build_logging(&LoggingConfig {
        sinks: vec![SinkConfig {
            destination: SinkDestination::RollingFile {
                directory: dir.path().to_path_buf(),
                filename_prefix: "app".to_owned(),
                filename_suffix: Some("log".to_owned()),
                rotation: RotationPeriod::Daily,
                max_files: nz(3),
                queue_capacity: nz(64),
            },
            format: LogFormat::Text,
            span_events: SpanEvents::NONE,
            filter: "info".to_owned(),
            failure_mode: SinkFailureMode::Required,
        }],
    })
    .unwrap();
    drop(result);

    // seed 5 件から 2 件残り + 当日ファイル = 3 件が上限。
    let log_count = file_names(dir.path())
        .iter()
        .filter(|name| Path::new(name).extension().is_some_and(|ext| ext == "log"))
        .count();
    assert!(
        log_count <= 3,
        "expected <= 3 retained .log files, found {log_count}"
    );
    assert!(
        !dir.path().join("app.2026-01-01.log").exists()
            && !dir.path().join("app.2026-01-02.log").exists(),
        "oldest suffixed files must be deleted first"
    );
    assert!(
        dir.path().join("app.2020-01-01.txt").exists(),
        "suffix-mismatched file must be left alone"
    );
}

// Issue #334 後方互換: suffix 未指定(`None`)は `{prefix}.{rotation-date}`
// を維持する。
#[test]
fn rolling_file_without_suffix_keeps_plain_dated_name() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![SinkConfig {
            destination: SinkDestination::RollingFile {
                directory: dir.path().to_path_buf(),
                filename_prefix: "app".to_owned(),
                filename_suffix: None,
                rotation: RotationPeriod::Daily,
                max_files: nz(7),
                queue_capacity: nz(64),
            },
            format: LogFormat::Text,
            span_events: SpanEvents::NONE,
            filter: "info".to_owned(),
            failure_mode: SinkFailureMode::Required,
        }],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info!("no suffix event");
    });
    drop(result);

    let names = file_names(dir.path());
    assert_eq!(names.len(), 1, "expected exactly one log file: {names:?}");
    let date_part = names[0]
        .strip_prefix("app.")
        .unwrap_or_else(|| panic!("expected `app.YYYY-MM-DD`, got: {}", names[0]));
    assert_daily_date_shape(date_part, &names[0]);
}

// RotationPeriod::Never + suffix — date 成分なしで `{prefix}.{suffix}`。
#[test]
fn rolling_file_suffix_with_never_rotation_is_prefix_dot_suffix() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![SinkConfig {
            destination: SinkDestination::RollingFile {
                directory: dir.path().to_path_buf(),
                filename_prefix: "app".to_owned(),
                filename_suffix: Some("log".to_owned()),
                rotation: RotationPeriod::Never,
                max_files: nz(4),
                queue_capacity: nz(64),
            },
            format: LogFormat::Text,
            span_events: SpanEvents::NONE,
            filter: "info".to_owned(),
            failure_mode: SinkFailureMode::Required,
        }],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info!("never-rotated suffix event");
    });
    drop(result);

    assert_eq!(file_names(dir.path()), ["app.log"]);
    assert!(read_sink_files(dir.path(), "app.").contains("never-rotated suffix event"));
}

// E4: Required sink の初期化失敗は型付き error。
#[test]
fn required_file_sink_bad_dir_fails_build() {
    let dir = tempfile::tempdir().unwrap();
    let not_a_dir = dir.path().join("occupied");
    fs::write(&not_a_dir, b"i am a file").unwrap();

    let error = build_logging(&LoggingConfig {
        sinks: vec![file_sink(&not_a_dir, "app", "info", LogFormat::Text)],
    })
    .unwrap_err();

    match error {
        LoggingBuildError::RequiredSinkFailed { failure } => {
            assert_eq!(failure.sink_index, 0);
            assert_eq!(failure.kind, SinkKind::RollingFile);
        }
        other => panic!("expected RequiredSinkFailed, got: {other:?}"),
    }
}

// E4b: 同じ失敗が BestEffort では degraded として記録され、残 sink が動く。
#[test]
fn best_effort_sink_failure_is_degraded_and_rest_works() {
    let dir = tempfile::tempdir().unwrap();
    let not_a_dir = dir.path().join("occupied");
    fs::write(&not_a_dir, b"i am a file").unwrap();

    let result = build_logging(&LoggingConfig {
        sinks: vec![
            file_sink_with_mode(
                &not_a_dir,
                "app",
                "info",
                LogFormat::Text,
                SinkFailureMode::BestEffort,
            ),
            file_sink(dir.path(), "live", "info", LogFormat::Text),
        ],
    })
    .unwrap();

    assert_eq!(result.degraded.len(), 1);
    assert_eq!(result.degraded[0].sink_index, 0);
    assert_eq!(result.degraded[0].kind, SinkKind::RollingFile);
    // 健全な sink は 1 件だけ live で、degraded sink は health に出ない。
    assert_eq!(result.health.snapshot().sinks.len(), 1);
    assert_eq!(result.health.snapshot().sinks[0].sink_index, 1);

    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info!("still routed");
    });
    drop(result);
    assert!(read_sink_files(dir.path(), "live").contains("still routed"));
}

// E4c (Issue #336): degraded record は Clone 可能 — Host がスナップショット
// として複製保持できる(借用経路の workaround 不要)。
#[test]
fn sink_degraded_record_is_cloneable() {
    let dir = tempfile::tempdir().unwrap();
    let not_a_dir = dir.path().join("occupied");
    fs::write(&not_a_dir, b"i am a file").unwrap();

    let result = build_logging(&LoggingConfig {
        sinks: vec![
            file_sink_with_mode(
                &not_a_dir,
                "app",
                "info",
                LogFormat::Text,
                SinkFailureMode::BestEffort,
            ),
            file_sink(dir.path(), "live", "info", LogFormat::Text),
        ],
    })
    .unwrap();

    let snapshot = result.degraded.clone();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].sink_index, 0);
    assert_eq!(snapshot[0].kind, SinkKind::RollingFile);
    assert_eq!(snapshot[0].reason, result.degraded[0].reason);
}

// E5: 全 sink BestEffort 全滅 → AllSinksFailed(黙って成功扱いしない)。
#[test]
fn all_best_effort_failures_error() {
    let dir = tempfile::tempdir().unwrap();
    let not_a_dir = dir.path().join("occupied");
    fs::write(&not_a_dir, b"i am a file").unwrap();

    let error = build_logging(&LoggingConfig {
        sinks: vec![
            file_sink_with_mode(
                &not_a_dir,
                "a",
                "info",
                LogFormat::Text,
                SinkFailureMode::BestEffort,
            ),
            file_sink_with_mode(
                &not_a_dir,
                "b",
                "info",
                LogFormat::Text,
                SinkFailureMode::BestEffort,
            ),
        ],
    })
    .unwrap_err();

    match error {
        LoggingBuildError::AllSinksFailed { failures } => {
            assert_eq!(failures.len(), 2);
        }
        other => panic!("expected AllSinksFailed, got: {other:?}"),
    }
}

// E5b: 空の sink 構成は黙った no-op 成功を防ぐため error。
#[test]
fn empty_sink_list_is_typed_error() {
    let error = build_logging(&LoggingConfig { sinks: vec![] }).unwrap_err();
    assert!(matches!(error, LoggingBuildError::NoSinks));
}

// E6: 不正 filter は型付き error(failure_mode に関係なく)。
#[test]
fn invalid_filter_is_typed_error() {
    let dir = tempfile::tempdir().unwrap();
    let error = build_logging(&LoggingConfig {
        sinks: vec![file_sink(dir.path(), "app", "!!bogus!!", LogFormat::Text)],
    })
    .unwrap_err();
    match error {
        LoggingBuildError::InvalidFilter {
            sink_index, filter, ..
        } => {
            assert_eq!(sink_index, 0);
            assert_eq!(filter, "!!bogus!!");
        }
        other => panic!("expected InvalidFilter, got: {other:?}"),
    }
}

// R4 補足: Required の後の sink があっても Required 失敗で即終了する。
#[test]
fn required_failure_aborts_before_later_sinks() {
    let dir = tempfile::tempdir().unwrap();
    let not_a_dir = dir.path().join("occupied");
    fs::write(&not_a_dir, b"i am a file").unwrap();

    let error = build_logging(&LoggingConfig {
        sinks: vec![
            file_sink(&not_a_dir, "app", "info", LogFormat::Text),
            file_sink(dir.path(), "late", "info", LogFormat::Text),
        ],
    })
    .unwrap_err();
    assert!(matches!(
        error,
        LoggingBuildError::RequiredSinkFailed { .. }
    ));
    // 後続 sink は作られない — ファイルも作られない。
    assert!(!dir.path().join("late").exists());
}

// E11: guard drop で drain → flush → join され、全 event が書かれる。
#[test]
fn guard_drop_drains_flushes_and_joins() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![file_sink(dir.path(), "drain", "info", LogFormat::Text)],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        for i in 0..20 {
            tracing::info!("event {i}");
        }
    });
    let health = result.health.clone();
    drop(result);

    let content = read_sink_files(dir.path(), "drain");
    for i in 0..20 {
        assert!(
            content.contains(&format!("event {i}")),
            "missing {i}: {content}"
        );
    }
    let snapshot = health.snapshot();
    assert_eq!(snapshot.sinks[0].events_written, 20);
    // 正常 shutdown 後の sink は書込みを受け付けない → Failed。
    assert_eq!(snapshot.sinks[0].state, SinkState::Failed);
}

// scoped dispatch 未使用の emit はどこにも届かない — ライブラリは
// global subscriber を一切設定しない。
#[test]
fn library_never_installs_a_global_subscriber() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![file_sink(dir.path(), "unglobal", "info", LogFormat::Text)],
    })
    .unwrap();

    // with_default なしで発行 — global default が無い限りどこへも行かない。
    tracing::info!("not routed anywhere");
    drop(result);

    let content = read_sink_files(dir.path(), "unglobal");
    assert!(
        !content.contains("not routed anywhere"),
        "event escaped without a dispatch: {content}"
    );
}

// stderr sink も同じ有界 writer 経路で、health に計測される。
#[test]
fn stderr_sink_works_through_bounded_writer() {
    let result = build_logging(&LoggingConfig {
        sinks: vec![SinkConfig {
            destination: SinkDestination::Stderr,
            format: LogFormat::Text,
            span_events: SpanEvents::NONE,
            filter: "info".to_owned(),
            failure_mode: SinkFailureMode::Required,
        }],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info!("to stderr");
    });
    let health = result.health.clone();
    drop(result);

    let snapshot = health.snapshot();
    assert_eq!(snapshot.sinks[0].kind, SinkKind::Stderr);
    assert_eq!(snapshot.sinks[0].events_written, 1);
}

// guard drop 後の event は channel_disconnected に計測される。
#[test]
fn events_after_sink_shutdown_are_counted_disconnected() {
    let dir = tempfile::tempdir().unwrap();
    let mut result = build_logging(&LoggingConfig {
        sinks: vec![file_sink(dir.path(), "gone", "info", LogFormat::Text)],
    })
    .unwrap();
    let health = result.health.clone();

    // sink だけを先に shutdown — dispatch は生きたまま。
    result.guards.clear();
    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info!("into the void");
    });

    let snapshot = health.snapshot();
    assert_eq!(snapshot.sinks[0].channel_disconnected, 1);
    assert_eq!(snapshot.sinks[0].state, SinkState::Failed);
}

// E12 / R7: snapshot は event を発行しない(非再帰計測)。
#[test]
fn health_snapshot_is_non_recursive() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![file_sink(dir.path(), "quiet", "info", LogFormat::Text)],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info!("one event");
    });
    let health = result.health.clone();
    let before = health.snapshot();
    // 失敗の記録が sink の出力に混入しないことを兼ねて 2 回 snapshot。
    let after = health.snapshot();
    drop(result);

    assert_eq!(
        before.sinks[0].events_written,
        after.sinks[0].events_written
    );
    let content = read_sink_files(dir.path(), "quiet");
    assert!(
        !content.to_lowercase().contains("health") && !content.contains("snapshot"),
        "health observation leaked into the log: {content}"
    );
}

// build_logging_layers — Host の registry に合成できる。
#[test]
fn layers_compose_into_host_registry() {
    use tracing_subscriber::layer::SubscriberExt;

    let dir = tempfile::tempdir().unwrap();
    let ruviform_observability::logging::LoggingLayersResult { layers, guards, .. } =
        build_logging_layers(&LoggingConfig {
            sinks: vec![file_sink(dir.path(), "composed", "info", LogFormat::Text)],
        })
        .unwrap();

    let dispatch = tracing::Dispatch::new(tracing_subscriber::registry().with(layers));
    tracing::dispatcher::with_default(&dispatch, || {
        tracing::info!("through host composition");
    });
    drop(dispatch);
    drop(guards);

    assert!(read_sink_files(dir.path(), "composed").contains("through host composition"));
}

// 複数スレッドからの emit が同じ sink に届く。
#[test]
fn events_from_multiple_threads_reach_the_sink() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![file_sink(dir.path(), "multi", "info", LogFormat::Text)],
    })
    .unwrap();

    let mut handles = Vec::new();
    for t in 0..4 {
        let dispatch = result.dispatch.clone();
        handles.push(std::thread::spawn(move || {
            tracing::dispatcher::with_default(&dispatch, || {
                for i in 0..5 {
                    tracing::info!("thread {t} event {i}");
                }
            });
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    let health = result.health.clone();
    drop(result);

    assert_eq!(health.snapshot().sinks[0].events_written, 20);
}

// sink の失敗が他の健全な sink の動作を止めない(E4b の補強 —
// shutdown 後も片方だけ Failed)。
#[test]
fn one_sink_shutdown_does_not_stop_the_other() {
    let dir = tempfile::tempdir().unwrap();
    let mut result = build_logging(&LoggingConfig {
        sinks: vec![
            file_sink(dir.path(), "dying", "info", LogFormat::Text),
            file_sink(dir.path(), "surviving", "info", LogFormat::Text),
        ],
    })
    .unwrap();

    // 先頭 sink だけ shutdown。
    result.guards.drain(..1);
    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info!("post-shutdown event");
    });
    drop(result);

    assert!(read_sink_files(dir.path(), "surviving").contains("post-shutdown event"));
}

// Issue #335: `SpanEvents::CLOSE` — 内部 event を持たない leaf span が
// close record として sink に残る(text)。AudioLens AL-OBS-01 の leaf
// span 観測が要求する契約。
#[test]
fn close_span_events_record_leaf_spans_in_text_sink() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![file_sink_with_span_events(
            dir.path(),
            "spans",
            "info",
            LogFormat::Text,
            SpanEvents::CLOSE,
        )],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        // Leaf span: entered, then dropped without any inner event —
        // only the synthesized close record can carry it to the sink.
        tracing::info_span!("leaf_span", component = "ncc").in_scope(|| {});
    });
    drop(result);

    let content = read_sink_files(dir.path(), "spans");
    assert!(
        content.contains("leaf_span"),
        "close record lost the span name: {content}"
    );
    assert!(content.contains("close"), "no close record: {content}");
    assert!(
        content.contains("time.busy"),
        "close record lost its timing fields: {content}"
    );
}

// Issue #335 同契約の JSONL 形 — close record は span 名・span field・
// time.busy/time.idle を保持する 1 JSON object。
#[test]
fn close_span_events_record_leaf_spans_in_jsonl_sink() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![file_sink_with_span_events(
            dir.path(),
            "spans",
            "info",
            LogFormat::JsonLines,
            SpanEvents::CLOSE,
        )],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info_span!("leaf_span", component = "ncc").in_scope(|| {});
    });
    drop(result);

    let content = read_sink_files(dir.path(), "spans");
    let close = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .find_map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).unwrap();
            (value["fields"]["message"] == "close").then_some(value)
        })
        .unwrap_or_else(|| panic!("no close record in: {content}"));
    assert_eq!(close["span"]["name"], "leaf_span", "got: {close}");
    assert_eq!(close["span"]["component"], "ncc", "got: {close}");
    assert!(
        close["fields"]["time.busy"].is_string() && close["fields"]["time.idle"].is_string(),
        "close record lost its timing fields: {close}"
    );
}

// Issue #335: span_events は per-sink — CLOSE sink だけが leaf span の
// record を持ち、既定 sink は出さない。
#[test]
fn span_events_are_configured_per_sink() {
    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![
            file_sink_with_span_events(
                dir.path(),
                "lifecycle",
                "info",
                LogFormat::Text,
                SpanEvents::CLOSE,
            ),
            file_sink(dir.path(), "events-only", "info", LogFormat::Text),
        ],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info_span!("leaf_span").in_scope(|| {
            tracing::info!("inside the span");
        });
    });
    drop(result);

    let lifecycle = read_sink_files(dir.path(), "lifecycle");
    let events_only = read_sink_files(dir.path(), "events-only");
    assert!(lifecycle.contains("close"), "got: {lifecycle}");
    assert!(
        events_only.contains("inside the span"),
        "got: {events_only}"
    );
    assert!(
        !events_only.contains("close"),
        "default sink leaked a lifecycle record: {events_only}"
    );
}

// Issue #335 後方互換: 既定 `SpanEvents::NONE` では leaf span は
// record を残さない(現行挙動維持)。
#[test]
fn default_span_events_emit_no_lifecycle_records() {
    // The declared default is the pre-#335 behaviour.
    assert_eq!(SpanEvents::default(), SpanEvents::NONE);

    let dir = tempfile::tempdir().unwrap();
    let result = build_logging(&LoggingConfig {
        sinks: vec![file_sink(dir.path(), "quiet", "info", LogFormat::Text)],
    })
    .unwrap();

    tracing::dispatcher::with_default(&result.dispatch, || {
        tracing::info_span!("leaf_span", component = "ncc").in_scope(|| {});
        tracing::info!("plain event");
    });
    drop(result);

    let content = read_sink_files(dir.path(), "quiet");
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "leaf span must leave no record by default: {content}"
    );
    assert!(lines[0].contains("plain event"), "got: {content}");
}
