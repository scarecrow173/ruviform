---
id: "TSPEC-034kf807e2ePipelineBenchG4te"
type: "test-spec"
status: "approved"
title: "frozen E2E pipeline ベンチと normalized-wall / digest regression gate の契約"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/807"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "bench-tasks-and-ci-smoke.md"
    - "issue-807-frozen-dsp-bench.md"
    - "issue-628-scripted-backend.md"
    - "issue-628-deterministic-test-runtime.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# frozen E2E pipeline ベンチと normalized-wall / digest regression gate の契約

## Purpose

Issue #807 の E 層 frozen benchmark 契約を、本番オーディオパイプラインの決定論的 end-to-end 経路に拡張する（slice 2）。計測対象は mock や複製実装ではなく `ruviform-audio-test` の本番 `pipeline::output::PlaybackPipeline` / `pipeline::input::InputPipeline` 脚であり、scripted backend を通じて `Session` / `RtRuntime` の実経路を駆動する。証拠の格付けは OS-L1（決定論・ヘッドレス）のみで、WASAPI/WinMM・実スレッド・GUI・plugin ABI の証拠とはしない。廃止してよいのは、frozen E2E bench 運用方針を変更する ADR / spec 改訂が行われたとき。

## Feature

`crates/ruviform-audio-test/benches/pipeline_frozen.rs`（`harness = false` 独自 main の `[[bench]]` ターゲット）が、scripted backend 上で本番 pipeline を warmup + 7 rep（各 64 callback × 256 frame × 2ch、48 kHz）駆動し、rep あたり wall 中央値を in-process 参照 kernel（`mix64_fill_gain`）中央値で正規化した `norm` と pinned baseline の `max_norm_ratio` を比較して gate 判定する。`mise run bench-frozen-e2e` が入口で、JSONL（`rvf.e2e-frozen.run/v1` schema の run / ref_kernel / cell / gate レコード）を stdout に構造化出力する。baseline は `crates/ruviform-audio-test/benches/frozen/pipeline_baseline.json` に pin し、candidate → 人手 promote の 2 相更新のみを許す。

## Rules

- **計測脚は本番経路のみ**: `output/session` は `PlaybackPipeline::open`（`Session::open` + `Session::start`）→ `pump_output_next` で production `plan.drive` callback を発火 → backend captured PCM を独立 oracle と比較する。`input/typed-rt` は scripted input callback → `InputCapture` → `input_writer` → `RtRuntime::process`（typed runtime・GainNode）→ `output_reader` の本番 handoff 経路を `pump_block` で駆動し、観測 planar PCM を独立計算値と比較する。ハーネス内に production アルゴリズムの複製を持たない（oracle は `oracle.rs` の独立実装であり `PcmFixture::fill`/`sample_at` を呼ばない）。
- **gate metric は normalized wall**: `norm = median(rep wall ns) / median(ref kernel ns)` を baseline の per-cell `max_norm_ratio` と比較する。絶対 ns 値・env metadata（OS・arch・CPU・コア数・rustc・repo SHA）は informational 記録であり gate に使わない。参照 kernel は同一プロセス・同一 run で計測されるため、metric は機器速度に依存しない。
- **correctness は gate 評価より先**: 各 cell で `pcm_mismatch`（oracle 不一致）・`device_errors`（scripted Xrun / device error）・`capture_loss`（capture pool 枯渇）・`pipeline_errors`（pipeline エラー）・`rep_divergence`（rep 間 digest 不一致）を計数し、いずれかが 1 件以上なら `norm` 充足に関わらず cell verdict `fail`。さらに pin の `expected_digest` と観測出力 fingerprint（FNV-1a）が一致しない場合も `digest_mismatch` で `fail`。
- **対象 cell は脚識別子**: `output/session`・`input/typed-rt`。cell 識別子は `<leg>/<engine>`。`work_count` は callback 処理 frame × channel の総量（`reps × blocks_per_rep × frames_per_callback × channels`）。
- **baseline 不変・2 相更新**: gate は pinned baseline を読むだけで絶対に書き換えない。`--write-baseline <path>` は `"approval": null` の candidate JSON を出力するのみで、promote は「candidate をコピーし `approval` を人手で埋めて pin パスに置く」手順。`approval` が null の candidate を gate 入力に使うと `baseline_unreadable` として fail する。
- **gate は fail-closed**: run モードで baseline が読めない・未 promote・セルが `max_norm_ratio` 超過・correctness count > 0・digest 不一致のいずれかなら gate verdict `fail`（exit 1）。verdict 語彙は `pass` / `fail` / `not_run` / `unsupported`。
- **`--test` は smoke で perf gate を hard-fail させない**: `--test` = 1 rep・`gate_mode: report_only` とし、perf 系 cell 失敗（`norm_above_threshold`）は診断として残し gate verdict は `not_run`（exit 0）。correctness 系失敗（`digest_mismatch`・各 error count > 0）は機器差ではなく正しさの欠陥なので smoke でも gate `fail`・exit 1 とする。
- **`--selftest` は注入で gate fail を実証**: wall floor を `(pin max_norm_ratio + 1) × ref median` に注入して perf 閾値超過を強制し、scripted Xrun/device error も注入して `device_errors > 0` を発生させる。全 cell `fail`・gate `fail`・exit 1 を再現する（実機回帰を待たず証拠を作る注入経路。run レコードの `injected` フィールドに内容を記す）。
- **閾値の初期値は余裕側**: `max_norm_ratio` は candidate 実測 norm の約 1.5 倍で pin する（ceil 単位の丸め）。scripted backend は wall に含める限り速い実装ではなく、per-callback コスト回帰の検出を意図する。根拠は `evidence/<issue>/decision.tsv` に記録する。
- **limiter を cell ごとに記録**: 各 cell の `limiter` フィールドに wall が含む要素（driver pump・backend callback machinery・`plan.drive`・fixture render・capture record 等）を記し、normalized metric が検出する回帰の意味（per-callback / per-block コスト増）を明記する。
- **証拠格付けの正直性**: scripted backend は決定論的・ヘッドレス・手動 pump であり、callback 内の RT 不変条件（alloc-free 等）は `rt_pipeline_*` テスト群が別途証明する。本 lane が計測するのは本番経路の per-callback / per-block コストであり、OS オーディオドライバ・実スレッドスケジューリング・GUI・plugin ABI は対象外。
- **記録先は `evidence/<issue>/`**: 実走 JSONL・selftest 出力・smoke 出力・candidate・promote 後 pin・`decision.tsv`（hillclimb スキーマ）を `evidence/rf807/` にコミット対象の証拠として残す。
- **lane は manual/opt-in**: `mise.toml` に `bench-frozen-e2e` を additive で追加し、CI `bench` ジョブは従来どおり `bench-smoke` のみを呼ぶ（`ruviform-audio-test` の bench は `bench-smoke` の対象にも含めない）。`bench-frozen-e2e` を CI lane・workflow に追加しない。

## Examples

- Given: pinned baseline が promote 済み。When: `mise run bench-frozen-e2e`。Then: 各 cell の `norm` が `max_norm_ratio` 以下・correctness count 0・digest 一致で gate verdict `pass`、exit 0、run/ref_kernel/cell/gate の JSONL が stdout に出る。
- Given: 計測中に device error が 1 件以上。When: 該当 cell を gate 評価。Then: `norm` 充足に関わらず cell verdict `fail`（`device_errors`）、gate verdict `fail`。
- Given: 観測 digest が pin の `expected_digest` と不一致。When: 該当 cell を gate 評価。Then: cell verdict `fail`（`digest_mismatch`）— これは `--test` smoke でも gate `fail`・exit 1 とする correctness 失敗である。
- Given: `approval` が null の candidate を `--baseline` に指定。When: run モードで実行。Then: `baseline_unreadable` として全 cell `not_run`、gate verdict `fail`。
- Given: pin パスの baseline ファイルが存在しない。When: run モードで実行。Then: 同上（baseline 不存在は pass 材料にならない）。
- Given: 閾値が届かない pin（機器差・閾値引き上げ直後等）。When: `mise run bench-frozen-e2e --test --baseline <pin>`。Then: cells は `fail` 診断（`norm_above_threshold`）を保持するが gate verdict は `not_run` で exit 0 — smoke が perf gate でブロックしない。
- Given: 回帰検出を証明したい。When: `mise run bench-frozen-e2e --selftest`。Then: wall floor 注入で `norm` が `max_norm_ratio` 超過・scripted Xrun で `device_errors > 0`、全 cell `fail`、gate verdict `fail`、exit 1。
- Given: baseline を更新したい。When: `mise run bench-frozen-e2e --write-baseline evidence/rf807/e2e_baseline.candidate.json`。Then: candidate JSON（`"approval": null`）が出力され、gate は実行されない。promote は別途人手で行う。

## Guarantees

- `mise run bench-frozen-e2e` 1 コマンドで metric（norm + rep median/range）・error class 別 count・work count・limiter が構造化出力される。
- regression gate が閾値超過・correctness 失敗で fail することが `--selftest` で headless に証明できる。
- baseline が gate 実行の副作用で変化しない（読み取り専用・更新は candidate → 人手 promote のみ）。
- `--test` smoke が perf gate でブロックしない（report_only・gate verdict `not_run`・exit 0）。correctness 欠陥のみ smoke でも fail する。
- 計測経路が本番 `Session`/`RtRuntime` であり、mock や production 複製が計測対象に混ざらないことがコード構造で保たれる。

## Non-goals

- 実 OS オーディオ backend（WASAPI/WinMM/CoreAudio）・実スレッドドライバ・GUI・plugin ABI（CLAP/VST3）の計測・証拠（本 lane は OS-L1 のみ。これらは個別 lane の責務）。
- RT 不変条件（callback 内 alloc-free 等）の検証（`rt_pipeline_*` テスト群の責務）。
- pipeline 実装・graph 意味論・backend 実装の変更（計測ハーネスと gate のみを追加する）。
- baseline の全機器共通閾値保証（閾値は pin 環境の実測由来。環境差は informational metadata で観測可能とする）。

## Risk

- normalized wall は参照 kernel で機器速度を相殺するが、scripted backend / スケジューラ jitter の分布は機器差で揺れ得る。閾値が他環境で厳しすぎ／緩すぎの場合は candidate → promote で更新し、`measured_env` で環境差を追跡する。
- `--selftest` 注入経路（wall floor + Xrun）しか fail を実演しないため、実機回帰時の検出感度は閾値設定（実測 × ~1.5）に依存する。
- 手動 pump のため、実ドライバの並行 callback では観測できない競合系退行は検出できない——これは本 lane の格付け（OS-L1 deterministic）に内在する境界であり、real-thread 証拠を別 lane に委ねることで対処する。
