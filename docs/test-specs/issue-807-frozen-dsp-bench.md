---
id: "TSPEC-034kf807dspFrozenBenchG4te"
type: "test-spec"
status: "approved"
title: "frozen DSP SIMD ベンチと ratio regression gate の契約"
created: "2026-11-20"
updated: "2026-11-20"
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
    - "issue-650-bench-smoke-coverage.md"
    - "simd-equivalence-matrix-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/dsp-simd-poc.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# frozen DSP SIMD ベンチと ratio regression gate の契約

## Purpose

Issue #807 の pstack 検証環境 E 層（benchmark / hillclimb）として、`ruviform-math::dsp` の本番 SIMD カーネルに対し、機器非依存の in-process A/B 比による frozen 計測と regression gate を `bench-frozen` lane で運用する契約を固定する。計測対象は PoC 実装ではなく `dsp.rs` の `SimdToken` ディスパッチ本番 API であり、ハーネスは「baseline を書き換えて pass を取らない」fail-closed 運用を保証する。廃止してよいのは、frozen bench 運用方針を変更する ADR / spec 改訂が行われたとき。

## Feature

`crates/ruviform-math/benches/dsp_frozen.rs`（`harness = false` 独自 main の `[[bench]]` ターゲット）が、同一プロセス・同一 run で `SimdToken::force_fallback()`（A 側）と `SimdToken::detect()`（B 側）を交互に計測し、median A/B 時間比と pinned baseline の `min_ratio` 閾値を比較して gate 判定する。`mise run bench-frozen` が入口で、JSONL（run / cell / gate レコード）を stdout に構造化出力する。baseline は `crates/ruviform-math/benches/frozen/dsp_baseline.json` に pin し、candidate → 人手 promote の 2 相更新のみを許す。

## Rules

- **gate metric は in-process A/B 比**: pass/fail 判定は `median(A)/median(B)` の時間比と baseline の `min_ratio` の比較のみ。絶対 ns 値・env metadata（OS・arch・CPU・コア数・rustc・repo SHA・detected level）は informational 記録であり gate に使わない。baseline ファイルは絶対時間を gate 値として保持しない。
- **A/B 交互反復・median + range**: 実走は `reps = 7`（>= 5）で、各 rep で A→B の順に交互計測する。cell ごとに per-call ns（rep 総時間 / inner 回数）の median・min・max を報告する。`inner` は側ごとに warm-up calibrate で決め、出力に記録する（rep 時間が TARGET_REP_NS 付近になるよう調整し、短すぎる計測の量子化誤差を避ける）。
- **対象 cell は kernel/len の組**: `dot/{256,1024,4096}` と `fir/{256,1024,4096}`（deterministic・RT/audio 経路の本番カーネル代表）。セル識別子は `<kernel>/<len>`。
- **error count は scalar 参照との出力不一致数**: 各計測後に `tests/simd_poc/` の scalar 参照実装出力と `equiv` 契約で要素比較し、不一致要素数を `errors_a` / `errors_b` として記録する。1 件以上あればその cell は gate 評価の前に `fail`。
- **work count は処理要素数の総量**: `len * (inner_a + inner_b) * reps` を `work_count` として記録し、計測した仕事量を示す。
- **baseline 不変・2 相更新**: gate は pinned baseline を読むだけで絶対に書き換えない。`--write-baseline <path>` は `"approval": null` の candidate JSON を出力するのみで、promote は「candidate をコピーし `approval` を人手で埋めて pin パスに置く」手順。`approval` が null の candidate を gate 入力（`--baseline` / pin パス）に使うと unreadable として fail する。
- **gate は fail-closed**: run モードで baseline が読めない・未 promote・セルが `min_ratio` 未達・エラー数 > 0 のいずれかなら gate verdict `fail`（exit 1）。verdict 語彙は `pass` / `fail` / `not_run` / `unsupported` で、verify-ruviform skill の規約と整合させる（全 cell が unsupported のときのみ gate verdict `unsupported`）。
- **`--test` は smoke で hard-fail させない**: `cargo bench ... -- --test` は全 `[[bench]]` ターゲットに渡るため、`--test` = 1 反復・`gate_mode: report_only` とし、閾値 gate で exit 1 にしない。CI `bench` ジョブ（`bench-smoke` のみ）の契約を変えない。
- **`--selftest` は注入で gate fail を実証**: B 側を `detect()` ではなく `force_fallback()` に注入し、ratio ~1.0 < `min_ratio` で全 cell `fail`・gate `fail`・非零 exit を再現する（実機回帰を待たず証拠を作る注入経路。run レコードの `injected` フィールドに内容を記す）。
- **閾値の初期値は余裕側**: `min_ratio = floor(1 + (measured_ratio - 1) * 0.5, 0.1)`——pin 時実測の SIMD 優位が半減したら fail（dispatch が scalar に退化 → ratio ~1.0 で確実に検出）しつつ、機器ノイズには十分な余裕を持たせる。根拠は `evidence/<issue>/decision.tsv` に記録する。
- **limiter を cell ごとに記録**: 律速（compute-bound / memory-bandwidth 等）を各 cell の `limiter` フィールドに記し、特定手順（内側ループ構造・入力サイズ・キャッシュ常駐性・scalar-routing floor の有無から推定）と記録先（JSONL + baseline candidate の `limiter` 節）をこの契約とする。
- **記録先は `evidence/<issue>/`**: 実走 JSONL・selftest 出力・candidate・promote 後 pin・`decision.tsv`（id/hypothesis/change/before/after/delta/tests/verdict/note の hillclimb スキーマ）を `evidence/rf807/` にコミット対象の証拠として残す。
- **lane は manual/opt-in**: `mise.toml` に `bench-frozen` を additive で追加し、CI `bench` ジョブは従来どおり `bench-smoke` のみを呼ぶ。`bench-frozen` を CI lane・workflow に追加しない。
- **転用元を明記**: 入力生成器と等価性契約は `tests/simd_poc/{inputs,equiv}.rs` を `#[path]` include で転用し、`benches/simd_poc.rs` の include 技法・env metadata 出力・baseline candidate→promote は `crates/ruviform-ui-test/src/golden/` BaselineStore 先例に倣う。

## Examples

- Given: pinned baseline が promote 済み。When: `mise run bench-frozen`。Then: 各 cell の median ratio が `min_ratio` 以上で gate verdict `pass`、exit 0、run/cell/gate の JSONL が stdout に出る。
- Given: 計測中にエラー数が 1 件以上。When: 該当 cell を gate 評価。Then: ratio 充足に関わらず cell verdict `fail`、gate verdict `fail`。
- Given: `approval` が null の candidate を `--baseline` に指定。When: run モードで実行。Then: baseline unreadable として全 cell `not_run`、gate verdict `fail`。
- Given: pin パスの baseline ファイルが存在しない。When: run モードで実行。Then: 同上（baseline 不存在は pass 材料にならない）。
- Given: CI bench ジョブ相当。When: `mise run bench-smoke`。Then: `dsp_frozen` に `--test` が渡り、1 反復・report_only で完走し exit 0。
- Given: dispatch 退行を証明したい。When: `mise run bench-frozen --selftest`。Then: B 側が fallback 注入で ratio ~1.0、全 cell `fail`、gate verdict `fail`、非零 exit。
- Given: baseline を更新したい。When: `mise run bench-frozen --write-baseline evidence/rf807/dsp_baseline.candidate.json`。Then: candidate JSON（`"approval": null`）が出力され、gate は実行されない。promote は別途人手で行う。

## Guarantees

- `mise run bench-frozen` 1 コマンドで metric（median/range）・error count・work count・limiter が構造化出力される。
- regression gate が閾値超過で fail することが `--selftest` で headless に証明できる。
- baseline が gate 実行の副作用で変化しない（読み取り専用・更新は candidate → 人手 promote のみ）。
- `--test` smoke が CI bench 契約を破壊しない（report_only・hard-fail なし）。
- 計測・判定・記録の手順が本契約として文書化され、`decision.tsv` に試行の記録先が固定される。

## Non-goals

- E2E 測定（io-e2e `io_composition` 等）・他計測対象（rt_observability_poc・ipc wire codec 等）への拡張（後続 slice 候補）。
- product 意味論・カーネル最適化本体の変更（dsp.rs の dispatch 閾値・実装は不変）。
- criterion 統計処理・`estimates.json` スキーマ（上流ツールの責務）。
- baseline の全機器共通閾値保証（閾値は pin 環境の実測由来。環境差は informational metadata で観測可能とする）。

## Risk

- A/B 比が機器差で有意に揺れた場合、pin 閾値が他環境で厳しすぎ／緩すぎになり得る——その場合は candidate → promote で閾値を更新し、環境差は `measured_env` で追跡する。
- `--selftest` 注入経路しか fail を実演しないため、実機退行時の検出感度は閾値設定（優位半減ルール）に依存する。
- calibrate された `inner` が側間で非対称になり得るが、per-call 時間として正規化済みであり ratio の意味は保たれる。
