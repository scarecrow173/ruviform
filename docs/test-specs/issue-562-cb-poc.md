---
id: "TSPEC-034XeilXUvNJ8cB4hX1e45"
type: "test-spec"
status: "approved"
title: "Issue #562 — Alternate Evaluation Context headless PoC テスト仕様"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/562"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/impl/ir/issue-562-cb-poc.md"
  depends-on:
    - "docs/specs/issue-562-cb-poc.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-562-cb-poc-design.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-562-cb-poc.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #562 — Alternate Evaluation Context headless PoC テスト仕様

対象テスト: `crates/ruviform-audio/tests/alternate_evaluation.rs`
（`mod common` で既存 fixture を再利用。headless・決定的・
allocation 非依存の fixture のみ）。

## Feature: 2 候補 alternate evaluation（RF-CB-01）

### Rule CB-01: State Isolation + Deterministic Compare — Pass

- **Given** 同一 `ValidatedGraph`・同一固定入力・異なる DSP 状態
  （gain 1.0 vs 0.5）の 2 候補 runtime pair。
- **When** 両候補を interleave で process し、候補 A を単独で
  再実行する。
- **Then** 各候補の出力は `in·gain + per-instance state` の期待値に
  一致し、A の単独再実行は interleaved 実行と bit-identical
  （B の処理が A の mutable state に影響しない）。
- Case: `cb01_state_isolation_and_deterministic_compare`

### Rule CB-02: Preview A/B — Main/Recording 副作用なし + RT NoAlloc — Pass

- **Given** Main runtime・Preview 候補 P1/P2 の 3 系統。
- **When** Main 記録（drain 出力）を取りながら P1→P2 へ切替え、
  候補 scoped event（fresh/stale generation）を送る。
- **Then** Main の記録列は単独 control と bit-identical。P1/P2 の
  出力は fresh candidate の再現と一致。fresh event は自候補にのみ
  適用、stale event は `stale_events` counter で drop。
  warm-up 後の `rt.process` は `assert_no_alloc` で allocation なし。
- Case: `cb02_preview_switch_no_main_or_recording_side_effects`

### Rule CB-03: Bus/Sidechain + Event — Pass

- **Given** `PortRole::Sidechain` 入力を持つ 2 入力 Processor
  graph（main_src + side_src → proc → sink）。
- **When** sidechain 係数の異なる 2 候補を同一 main/key 入力で
  process し、key bus を無音化、generation-scoped event を送る。
- **Then** 出力は `main·(1 + side·key)` の期待値に一致。key 無音で
  `main` に一致。fresh event は自候補のみ適用、stale は counter
  drop。
- Case: `cb03_sidechain_bus_and_generation_scoped_events`

### Rule CB-04a: Offline Render 2 候補 — PDC/Tail/決定性 — Pass

- **Given** `OfflineRenderSupport::Verified` の 2 `PreparedRender`
  （gain 1.0/latency 0 vs gain 0.5/latency 32）、同一 feeder/
  transport/spec。
- **When** `render` を実行する。
- **Then** `frames_emitted`・`tail_evidence=Known`・
  `range_end_with_tail` が一致し `total_latency` が 0/32 と報告
  される。A emitted[p]=signal[p]、B emitted[p]=signal[p+32]·0.5
  （PDC で latency は報告されるが隠されない）。再 render は
  bit-identical。
- Case: `cb04a_offline_render_two_candidates_pdc_tail`

### Rule CB-04b: Offline Preflight Failure — 分離 — Pass

- **Given** `OfflineRenderSupport::Unknown` の候補 U と
  `Verified` の候補 G。
- **When** U を render する。
- **Then** `RenderError::Preflight(OfflineUnsupported)` で reject
  され出力は生成されない。G は影響を受けず正常 render。
- Case: `cb04b_preflight_failure_isolated`

### Rule CB-04c: Seek/Loop — 決定性 — Pass

- **Given** 同一 spec の seek 付き（Play@0 + Seek→512@frame128）
  / loop 付き（SetLoop[64,128)）transport を 2 回 render。
- **When** `render` を反復実行する。
- **Then** captured planes/runs が run 間で bit-identical。
  seek では post-seek signal window が出力に現れ skip 区間は
  現れない。loop では loop body が反復出現し linear render には
  現れない（position wrap の観測可能な証拠）。
- Case: `cb04c_seek_loop_determinism`

### Rule CB-05: Probe 観測整列（2 runtime 横断）— Pass（条件付き）

- **Given** 2 候補それぞれの独立 `probe::pair`（Monitor, proc_out）
  と同一固定入力 8 block。
- **When** drain → `ObservationCoverage::reconstruct` →
  `ObservationSpan` 構築 → `align_observation_points`。
- **Then** anchor 未配線では `Incomparable`（fail-closed）。
  両 `ProbeTimeContext` に共有 engine epoch の `ClockAnchor` を
  配線すると `Aligned{EngineFrame{epoch}}`・offset 0・全 overlap。
  両候補の generation は `single(1)` で同一（runtime ごとの
  counter）— 候補識別は `clock_mark.domain` /
  `runtime_instance` が担う。`effective_latency` は候補ごとに
  record に載る（0 vs 32）。
- Case: `cb05_probe_cross_candidate_alignment`
- 備考: 「固定入力同一性の宣言」は consumer が anchor で行う —
  Ruviform は同一性を推測しない（Partial の根拠、gap report 参照）。

## Feature: 記録のみ（本スライス対象外）

- AL f64/長尺/CLI oracle と SV f32/RT/Creative Branch の L1 独立
  Oracle・対象 SHA の #294 接続 — gap report の対応表に記録のみ。
- Windows L2/L3（実 Plugin DSP/試聴 GUI/モニター）、macOS/Linux
  GUI-L2/EXT-L3 — NotRun/Deferred として gap report に記録。
