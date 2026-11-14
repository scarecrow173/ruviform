---
id: "IMPL-034XeimTwfyibURJX60q4f"
type: "impl"
status: "completed"
title: "Issue #562 Alternate Evaluation Context PoC IR"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/562"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio/tests/alternate_evaluation.rs"
      - type: "file"
        path: "evidence/rf562/gap-report.md"
      - type: "file"
        path: "docs/impl/exp/issue-562-cb-poc.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-562-cb-poc.md"
    - "docs/designs/issue-562-cb-poc-design.md"
    - "docs/test-specs/issue-562-cb-poc.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-562-cb-poc.md"
    - "docs/designs/issue-562-cb-poc-design.md"
    - "docs/test-specs/issue-562-cb-poc.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "evidence/rf562/gap-report.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-562-cb-poc-design.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [562]
  experiments:
    adopted: []
    rejected: []
---

# Issue #562 Alternate Evaluation Context PoC IR

## Summary

headless 2 候補 alternate evaluation PoC を
`crates/ruviform-audio/tests/alternate_evaluation.rs` に実装した。
`crates/ruviform-audio/src/` の変更は **なし** — 既存 API で全 Case
を検証でき、機能 Gap は実証されなかった。判定台帳は
`evidence/rf562/gap-report.md`。

## 実装内容

- `build_runtime`: 同一 `GraphSpec` を候補ごとに compile/publish し
  独立 `GraphRuntime`+`RtRuntime` pair を構築（candidate model D1）。
- `block`: 固定入力 feed → process → drain の 1 block 駆動。
- `signal`（period-97 決定的 pattern）/ `signal_nr`（LCG 非周期、
  window-membership assert 用）。
- `GainDsp`（gain+declared latency+offline_support）、`KeyDsp`
  （2 入力 sidechain mix + event 可変係数）— `GraphProcessor` 実装。
- render 系 fixture: `SignalFeeder`/`CaptureEncoder`/`NullMuxer`/
  `Scratch`/`transport`/`play_at`/`stage`/`spec`/`capture_output`/
  `run_render`（render.rs test の pattern を self-contained に再現）。
- probe 系 fixture: `probe_chain`（`port_probe` 付き）/`ProbeRig`/
  `collect_series`（drain→coverage/span facts 再構成）。

## 検証結果（全 7 test Pass、2026-11-14）

| Case | Test | 結果 |
| --- | --- | --- |
| CB-01 | `cb01_state_isolation_and_deterministic_compare` | Pass |
| CB-02 | `cb02_preview_switch_no_main_or_recording_side_effects` | Pass |
| CB-03 | `cb03_sidechain_bus_and_generation_scoped_events` | Pass |
| CB-04a | `cb04a_offline_render_two_candidates_pdc_tail` | Pass |
| CB-04b | `cb04b_preflight_failure_isolated` | Pass |
| CB-04c | `cb04c_seek_loop_determinism` | Pass |
| CB-05 | `cb05_probe_cross_candidate_alignment` | Pass（条件付き） |

検証コマンド（canonical = mise、Windows 11 x86_64、
mise 管理 toolchain）:

- `mise run fmt-check` — Pass
- `mise run check ruviform-audio` — Pass
- `mise run lint ruviform-audio` — Pass（`-D warnings`、clippy 12
  件を修正済み）
- `mise run test ruviform-audio` — Pass（全 test target + doctest）

## Gap 判定サマリ

機能 Gap **なし** — 既存 Typed Graph/Offline Render/Probe API で
2 候補評価の全契約を検証できた。consumer 側に必要な bookkeeping
（評価入力同一性の anchor 宣言・domain→candidate 対応表）は
gap report の Partial 項目として記録。後続 Issue 起票は不要と
判断（Coordinator 確認事項として PR 本文に記載）。

## 残課題（スライス外）

- 実 Plugin/GUI/Device での Preview A/B（Windows L2/L3）— NotRun。
- `crates/ruviform-plugin-*` fixture での 2 候補 instance/state
  分離の専用テスト — 既存 plugin-host/plugin-core テストで
  lifecycle/state 分離は担保済みだが、本スライス編集範囲外の
  ため Partial 記録。
- #294 への AL/SV oracle 対応表接続 — 記録のみ。
