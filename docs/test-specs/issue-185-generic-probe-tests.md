---
id: "TSPEC-034QibxcrYUagyZX3rtGKj"
type: "test-spec"
status: "approved"
title: "Issue #185 generic probe: capability, modes, metrics, bounded capture, metadata, RT safety, faults"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/185"
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
    - "docs/test-specs/issue-195-m2-typed-graph-compiler-runtime-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md"
    - "docs/discovery/issue-185-generic-probe-observation-inventory.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-185-generic-probe-rt-capture.md"
    - "docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md"
    - "docs/adr/issue-185-generic-probe-contract-placement-and-rt-handoff-model.md"
  verified-by: []
  references:
    - "docs/adr/realtime-contract-and-verification-policy.md"
  defers: []
  deferred-by: []
---

# Issue #185 generic probe: capability, modes, metrics, bounded capture, metadata, RT safety, faults

## Purpose

Issue #185 spec の AC-1〜AC-10（+AC-5b）を `ruviform-audio` の integration test `tests/typed_probe.rs`・`rt_probe_*` 契約テスト（fixture は `tests/common/mod.rs` 拡張）で保証する。実行した検証コマンド・結果・未検証項目は impl record `docs/impl/ir/issue-185-generic-probe.md` に記録する。

## Feature

Generic Probe: Node/Port 観測点の capability 列挙・Off/Monitor/Capture 実行・Peak/RMS・bounded PCM Capture・時間/識別メタデータ・RT→非 RT 配送。

## Rules

- R-CAP: capability 報告は mock graph の全 audio port（node input/output・graph boundary）を網羅し、物理解決先行で `Configured`/`Inferred`/`Unavailable`/`Unsupported`/`Unknown` を分類。同一 spec+plan の再列挙は byte 等価（決定的）。
- R-MODE: `Off`/`Monitor`/`Capture` を `ProbeId` 単位で遷移。`mode == Off` は tap 点で slot borrow・record 発行を行わない。ON/OFF は boundary 出力 PCM を bit 一致で変えない。
- R-METRIC: 同一 block の node input(pre)/output(post) の Peak/RMS が `ChannelLevel` 契約と一致し、metric record も `RecordMeta` 全項目を持つ。
- R-CAPTURE: `CaptureConfig` 予算内で PCM chunk が `chunks_out` 経由で非 RT へ届き、`probe_id`/`node`/`port`/`generation`/`sample_position`/`frames`/`frames_valid`/`channels`/declared `sample_rate`/`effective_latency`/loss 状態を持つ。budget 到達で `BudgetExhausted` + Monitor 復帰。
- R-META: `sample_position` は `frame_cursor` 由来の絶対位置（可変 `frames` で連続・重複なし）。`generation` は plan generation（`GraphRevision` ではない）。`effective_latency` は tap 点別規則（input direct→`accum[prod]`、scratch→`accum[prod]+delay_frames`、output→`accum[node]`、boundary output→`total_latency`）。
- R-FRESH: tap は `stamp() == seq` の鮮度監査を適用し、stale/uninitialized slot では `Unobserved` status を記録してデータ record を発行しない。
- R-BIND: `bound_generation != plan.generation` の binding は `Unbound` status + 観測スキップ（誤 slot を観測しない）。`Rebind` 適用後は新 plan の正しい slot を観測する。
- R-FAULT: free 枯渇・telemetry 満杯・明示 `Stop`・generation 更新・collector 遅延 drain/購読解除の各注入で `process` はブロックせず、fault/loss が非 RT 観測できる。長時間 stall 回復後も pool 健全・capture 再開（leak なし）。
- R-ERR: 非 RT 制御 API の失敗（未知 `ProbeId`・`Unavailable`/`Unknown` 点への install・予算超過・command channel 満杯）は定義済み `ProbeError` で返り、panic/無言 no-op でない。
- R-RT: `rt_probe_*` で `assert_no_alloc` 計測窓の alloc が全 mode（Off/Monitor/Capture）で 0。RT 側 destructor/lock/OS I/O なし。
- R-COMPAT: probe 有無で同一入力の boundary 出力が bit 一致。既存 suite（typed_m2/ownership_poc 等）は不変で緑。

## Examples

- `ex01_capability_report`: `ProbePoint` 宣言 port・未宣言 port・未接続 input・非 Audio port・存在しない port を含む spec で `capabilities()` が 5 分類を返し、2 回列挙が同一。
- `ex02_mode_transition`: 2 観測点を install → Off→Monitor→Capture→Off を往復し、各遷移が telemetry status で観測できる。
- `ex03_off_bit_identity`: probe 全 ON の process 出力と全 OFF の出力が byte 一致（routing 不変）。
- `ex04_pre_post_metrics`: chain fixture の node input/output 両点 Monitor で、同一 `generation`+`sample_position` の MetricRecord が Peak/RMS の差（例: gain 2x で ratio≈2）を持つ。
- `ex05_bounded_capture`: `CaptureConfig{max_frames: 3 ブロック分}` で chunk が連続 `sample_position` で届き、`BudgetExhausted` status で止まり Monitor に復帰。
- `ex06_metadata_envelope`: capture chunk の `probe_id`/`node`/`port`/`generation`/`sample_position`/`frames`/`channels`/`sample_rate`/`effective_latency` が期待値と一致。
- `ex07_free_exhaustion`: `free` を空にした状態で capture → 当該 block が `lost_blocks++` で落ち、RT 非ブロック。
- `ex08_telemetry_overflow`: collector を止めたまま monitor を大量に回し telemetry 満杯 → `lost_records` 増加・RT 非ブロック・capture chunk は届き続ける（分離）。
- `ex09_explicit_stop`: `Capture` 中に `stop_capture` → `CaptureStopped{Requested}` + Monitor 復帰・以降 chunk なし。
- `ex10_republish_rebind`: probe 有効中に graph を変更して publish → `Rebind` 適用後に新 plan の正しい slot を観測・間の `Unbound` window が status で見える。
- `ex11_stale_binding_guard`: broker が `on_publish` を呼ばず generation 更新のみ行われた状態で process → `Unbound` status・誤 slot 観測なし・データ record なし。
- `ex12_delayed_drain_unsubscribe`: collector を長時間 drain せず `process` 継続 → RT 非ブロック・loss 計数のみ。drain 停止（購読解除）で追加 record なし。
- `ex13_unobserved_stamp`: processor 欠落（missing processor）等で stamp 不一致の点 → `Unobserved` status・Metric/Captured なし。
- `ex14_defined_errors`: `set_mode`/`remove`/`capture`/`install` への未知・不正・予算超過入力が `ProbeError` variant で返る。
- `ex15_pdc_latency`: merge fixture で遅延辺を持つ input tap と output tap の `effective_latency` が規則どおりで、同一 `sample_position` の chunk が整列可能。
- `ex16_rt_alloc`: `assert_no_alloc` 窓で install 済み runtime の `process` を全 mode で 0 alloc。

## Guarantees

- spec AC-1〜AC-10（+AC-5b）をこの例集合が網羅する。
- 全 record は `RecordMeta` 封筒を持ち、`generation`/`sample_position`/loss・未観測・不連続の区別が読み取れる。
- `rt_probe_*` は `rt_` prefix で `mise run test-rt` に載る。

## Non-goals

- TimedEvent/Control payload 観測・plugin 内部 stage・cross-session ID・永続ストレージ・Recording Writer 契約・旧 pull 型 graph は検証しない。
- `chunks_out` の構造的不満杯保証の形式証明 — `#[debug_assert]` + 単体テストでの挙動検証に留める。

## Risk

- rebind/stale-binding/未観測の誤検出をテストしない場合、observability として最悪の失敗（誤 slot の PCM を正しい観測として返す）を見逃す — R-BIND/R-FRESH を必須とする。
