---
id: "TASK-034VnNxw0tKz4kSLCz9cRZ"
type: "task"
status: "done"
title: "Issue #253 L1 wire report verb task1（wire verb + runner + host 配線 + fixture E2E）"
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
    - "docs/plans/rf-midi-l1-report-verb-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/rf-midi-l1-report-verb-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/rf-midi-l1-report-verb-plan.md"
  derived-by: []
  verifies:
    - "docs/test-specs/rf-midi-l1-report-verb.md"
  verified-by:
    - "docs/test-specs/rf-midi-l1-report-verb.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 L1 wire report verb task1（wire verb + runner + host 配線 + fixture E2E）

## Work

Issue #253 残件の「L1 wire verb」1 行を実装する。wire protocol
を v4 に bump して `Command::DrainOutputReports`（tag 23）と
`Reply::Reports`（tag 8）を追加し、runner（`plugin-export`）に
per-instance report backlog + dispatch、host（`l1/mod.rs` +
`composite.rs`）に short-reply までの chunked drain loop と L1
audio arm 配線を行う。`l1-test-worker` に `--reports-per-block`
fixture、reference runner の `GainPlugin` に report echo を追加し、
codec unit / runner E2E / proxy E2E / graph E2E で全経路を検証する。

- [x] T1 `wire.rs`: `WIRE_VERSION` 3→4、`DrainOutputReports`/
      `Reports` 語彙・codec arm・境界/bounds/scope/malformed test
      （WR-01..06）
- [x] T2 `plugin-export/src/lib.rs`: `InstanceEntry.report_backlog`
      + dispatch arm + oldest-first `WIRE_MAX_EVENTS` chunk
      （RN-01..04）
- [x] T3 `l1/mod.rs` + `composite.rs`: `L1Instance::
      drain_output_reports` chunked loop、L1 audio arm 配線、
      video `Unavailable` 維持、unit test 更新（HP-01..04）
- [x] T4 fixtures: `l1-test-worker --reports-per-block` +
      `GainPlugin` echo/marker（FX-01..02）
- [x] T5 E2E: `l1_runner_e2e`・`l1_proxy_e2e`（pull/chunk/failed）・
      `plugin_graph_e2e`（endpoint attach 中 drain、旧
      `Unavailable` 期待を置換）（E2E-01..04）
- [x] T6 文書チェーン 6 点（spec/design/plan/task/test-spec/IR）
- [x] T7 `git fetch origin main && git merge origin/main` →
      canonical 検証 lane → review-protocol → commit/push/PR

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/rf-midi-l1-report-verb-plan.md`
- `docs/specs/rf-midi-l1-report-verb.md`
- `docs/designs/rf-midi-l1-report-verb-design.md`
- `docs/test-specs/rf-midi-l1-report-verb.md`

## Dependencies

- PR #563（Issue #253 host-graph consumer 配線）— `drain_output_
  reports` host API と `ReportDrain`、merge 済み。
- PR #555（`641f49dd`、VST3 MIDI 正規経路）— canonical
  `Event`/`EventKind` codec と plugin-core 契約、merge 済み。

## Affected Files

- `crates/ruviform-plugin-core/src/wire.rs`
- `crates/ruviform-plugin-export/src/lib.rs`
- `crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-runner.rs`
- `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs`
- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`
- `crates/ruviform-plugin-host/src/l1/mod.rs`
- `crates/ruviform-plugin-host/src/composite.rs`
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`
- `crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs`
- `docs/impl/ir/rf-253-l1-report-verb.md`（新規）

## Done When

- [x] L1 wire に report drain verb があり、`PluginHost::
      drain_output_reports` の L1 audio slot が `Drained` +
      canonical `Event` を返す。
- [x] `WIRE_VERSION` 3→4、runner dispatch、composite L1 arm、
      `l1-test-worker` arm が配線されている。
- [x] codec 往復 + `l1_proxy_e2e` + `plugin_graph_e2e` の L1 経路
      E2E が pass し、L0/video の既存 coverage を維持する。
- [x] 実 OS MIDI・実デバイス・実 DAW・consumer 消費・capability
      昇格は `NotRun`/`Unsupported` として記録される。

## Verification

- [x] `mise run fmt-check` exits 0
- [x] `mise run check` exits 0
- [x] `mise run lint` exits 0
- [x] `mise run test` exits 0
- [x] `mise run test-rt` exits 0
- [x] `mise run test-plugin-core` exits 0
- [x] `mise run lint-plugin-core` exits 0
- [x] `mise run verify-ci` exits 0
