---
id: "PLAN-034VnMw6PrT9NBJxWK4N1L"
type: "plan"
status: "approved"
title: "Implement Issue #253 L1 wire report verb"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/rf-midi-l1-report-verb.md"
    - "docs/designs/rf-midi-l1-report-verb-design.md"
  implemented-by: []
  depends-on:
    - "docs/designs/rf-midi-l1-report-verb-design.md"
    - "docs/test-specs/rf-midi-l1-report-verb.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/rf-253-task1-l1-report-verb.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/rf-midi-l1-report-verb-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rf-midi-l1-report-verb.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/pull/563"
  defers: []
  deferred-by: []
metadata:
  issues: [253]
---

# Implement Issue #253 L1 wire report verb Implementation Plan

## Goal

L1 wire protocol に report drain verb を追加し、
`PluginHost::drain_output_reports` の L1 audio slot が `Drained`
+ canonical `Event` を返す全経路（wire codec → runner → host
proxy → composite → fixture E2E）を実装する。

## Scope

単一実装ストリーム — wire 語彙 1 verb/reply 対とその 4 層配線、
fixture、E2E、文書チェーンのみ。video report verb・bridge
`unsupported` 計数・consumer 消費・実機検証は含まない
（design の Out of Scope と一致）。

## Upstream Documents

- `docs/specs/rf-midi-l1-report-verb.md`
- `docs/designs/rf-midi-l1-report-verb-design.md`
- `docs/test-specs/rf-midi-l1-report-verb.md`
- 前提 slice: `docs/specs/issue-253-hostgraph-reports.md` /
  `docs/designs/issue-253-hostgraph-reports.md`（PR #563）

## Design Inputs

- `docs/designs/rf-midi-l1-report-verb-design.md` — wire tag 割当
  （command 23 / reply 8）、runner backlog chunking、host short-
  reply loop、fixture 形状、capability 判断の正本。

## File Map

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-plugin-core/src/wire.rs` | `WIRE_VERSION` 3→4、`Command::DrainOutputReports`（tag 23）・`Reply::Reports`（tag 8）の enum/codec arm、doc 更新、codec unit test |
| `crates/ruviform-plugin-export/src/lib.rs` | `InstanceEntry.report_backlog` + `CreateInstance` 初期化 + `DrainOutputReports` dispatch arm + `drain_output_reports` method |
| `crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-runner.rs` | `GainPlugin` に `pending_reports`/`blocks`、入力 event echo + `gain_marker`、`drain_output_reports` override |
| `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` | `e2e_drain_output_reports` — 実 runner child の全 semantics 検証 |
| `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` | `--reports-per-block` flag、`pending_reports`/`next_report_index`、`report_events`、成功 block のみ queue、`DrainOutputReports` dispatch arm |
| `crates/ruviform-plugin-host/src/l1/mod.rs` | `L1Instance::drain_output_reports` — short reply までの chunked loop |
| `crates/ruviform-plugin-host/src/composite.rs` | `Slot::OutOfProcess` arm を instance drain に配線、unit test 更新 |
| `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` | `worker_reports` mirror + drain E2E 3 件（pull/chunk/failed） |
| `crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs` | `l1_processing_with_args` + `l1_worker_reports` + `l1_reports_drain_through_host_while_endpoint_attached`（旧 `Unavailable` test を置換） |
| `docs/specs/rf-midi-l1-report-verb.md` ほか 5 点 | 文書チェーン（spec/design/plan/task/IR/test-spec） |

`crates/ruviform-audio-plugin-bridge`・`crates/ruviform-midi-device`・
`crates/ruviform-audio-core`・`mise.toml`・`.github/workflows/`・
`.devin/ci-metrics*` は **無変更**（編集範囲外）。

## Implementation Sequence

- [x] 1. `wire.rs`: version bump + 語彙 + codec arm + codec test
      （red → green、tag 8 の旧 unknown-tag test を 9 へ移動）。
- [x] 2. `plugin-export/src/lib.rs`: backlog + dispatch。
- [x] 3. `ruviform-plugin-l1-runner.rs`: GainPlugin fixture。
- [x] 4. `l1-test-worker.rs`: flag + queue + dispatch arm。
- [x] 5. `l1/mod.rs`: `L1Instance::drain_output_reports`。
- [x] 6. `composite.rs`: L1 arm 配線 + unit test 更新。
- [x] 7. E2E: `l1_runner_e2e.rs` → `l1_proxy_e2e.rs` →
      `plugin_graph_e2e.rs`。
- [x] 8. 文書チェーン 6 点作成。
- [ ] 9. `git fetch origin main && git merge origin/main` →
      canonical 検証 lane → review-protocol → commit/push/PR。

## Dependencies and Risks

- `WIRE_MAX_EVENTS`（=1024）を chunk サイズと decode bound の
  双方に使う — 値の齟齬があると host loop が premature 終了または
  過剰要求する。codec test の境界 assert で固定済み。
- host loop の終了条件は「chunk < WIRE_MAX_EVENTS」— child が
  常に full chunk を返し続ける経路は無い（serial child、backlog
  は単調減少）が、異常 reply は `Err(Ipc)` で fail-closed。
- `l1-test-worker` の report index は per-instance running —
  複数 instance 共存時も index が独立であることを fixture が保証
  （`WorkerEntry` フィールド）。
- E2E の report 期待値は worker の `report_events` を test 側に
  mirror — fixture 変更時に mirror も同期が必要（comment で
  明示済み）。

## Task Breakdown

- T1 wire codec + version bump（spec R1）
- T2 runner backlog + dispatch（spec R2）
- T3 host proxy + composite arm（spec R3）
- T4 fixtures（runner GainPlugin + l1-test-worker）（spec R2/R5）
- T5 E2E 3 面（codec unit / runner / proxy / graph）（spec R5）
- T6 文書チェーン + IR + canonical 検証（spec R5）

## Verification

- [x] `mise run fmt-check` exits 0
- [x] `mise run check` exits 0
- [x] `mise run lint` exits 0
- [x] `mise run test` exits 0
- [x] `mise run test-rt` exits 0
- [x] `mise run test-plugin-core` exits 0
- [x] `mise run lint-plugin-core` exits 0
- [x] `mise run verify-ci` exits 0

## Open Questions / Gaps

- なし — wire tag・chunk 意味論・error surface は design で確定済み。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一ストリーム・密結合な
  4 層配線のため分割しない）
- Independent work streams: none
- User approval required before dispatch: yes（worker contract に
  より review-protocol の blocker 無しを承認として扱う）
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute
  task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- reviewer focus: wire tag 衝突（reply 8 の旧 unknown-test 移動）、
  host loop 終了条件、fixture の deterministic report 式、
  lifecycle-ungated の一貫性、`WIRE_VERSION` bump の記録完全性。
