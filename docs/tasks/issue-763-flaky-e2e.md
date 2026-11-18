---
id: "TASK-034ZkA8nPvQrS5tUxWzHbC"
type: "task"
status: "done"
title: "Issue #763 l1_real_test_plugin_e2e instrument tests flaky — test oracle 修復"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/763"
    - "docs/discovery/issue-763-flaky-e2e.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-763-flaky-e2e.md"
      - type: "file"
        path: "evidence/rf763/reproduction.md"
      - type: "file"
        path: "evidence/rf763/verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_real_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs"
      - type: "file"
        path: "docs/test-specs/issue-630-test-instrument.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/discovery/issue-763-flaky-e2e.md"
  implemented-by:
    - "docs/impl/ir/issue-763-flaky-e2e.md"
  depends-on:
    - "docs/discovery/issue-763-flaky-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-test-instrument.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-763-flaky-e2e.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #763 l1_real_test_plugin_e2e instrument tests flaky — test oracle 修復

## Work

`l1_real_test_plugin_e2e` の instrument 系 test が負荷下で
frame-exact PCM assertion により断続 fail する defect
（`ruviform#763`）を修復する。diagnosis は discovery で確定済み:
test oracle が substituted output（`last_output` の stale replay）
と dropped submit（event 喪失）を区別できていなかったのが原因で、
production の hold-last semantics は設計意図どおり。

両 adapter crate の test file で、delivered block の content assert
を `PluginHost::audio_counters` の `passthrough_submits` /
`dropped_submits` delta による matched/substituted 分類へ改修し、
単発 NoteOff は `dropped_submits` delta = 0 までの上限付き
resend-until-landed を実装する。release tail は arrived-block 数で
rendered space の release 完了を判定する。assertion・最低到達数
gate は一切緩めない。spec doc（`issue-630-test-instrument.md`）の
R7 を substituted-output semantics に修正し Risk 節を更新する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/discovery/issue-763-flaky-e2e.md`（spec 相当 — Issue 本文
  の受入条件が確定済みの是正タスク）
- `docs/test-specs/issue-630-test-instrument.md`（検証対象 oracle
  の正本 — R7/Risk を本 slice で更新）

## Dependencies

- なし（`ruviform#763` 単独。`ruviform#630` の test spec 系譜）

## Affected Files

- `crates/ruviform-plugin-test-clap/tests/l1_real_test_plugin_e2e.rs`
- `crates/ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs`
- `docs/test-specs/issue-630-test-instrument.md` — R7 + Risk 更新
- `docs/discovery|tasks|impl/ir|impl/exp/issue-763-flaky-e2e*` —
  doc chain（lease 拡張承認済みの縮小構成）
- `evidence/rf763/` — reproduction / verification evidence

## Done When

- [x] instrument 系 test の content assert が matched block のみに
      適用され、substituted block は counter delta で分類・skip
      される（assertion・最低到達数 gate 不変）
- [x] 単発 NoteOff が bounded resend-until-landed で着地確認され、
      未着地時は正直に fail する（無限 loop なし）
- [x] `docs/test-specs/issue-630-test-instrument.md` の R7 が
      substituted-output semantics に更新される
- [x] canonical lane + looped e2e 検証が green（N・環境を
      `evidence/rf763/verification.md` に実測記録）

## Verification

- [x] `mise run fmt-check` exits 0
- [x] `mise run check ruviform-plugin-test-clap` exits 0
- [x] `mise run check ruviform-plugin-test-vst3` exits 0
- [x] `mise run lint ruviform-plugin-test-clap` exits 0
- [x] `mise run lint ruviform-plugin-test-vst3` exits 0
- [x] `mise run test ruviform-plugin-test-clap` exits 0
- [x] `mise run test ruviform-plugin-test-vst3` exits 0
- [x] `mise run check-l1-real-plugin` exits 0
- [x] `mise run lint-l1-real-plugin` exits 0
- [x] `mise run test-l1-real-plugin` exits 0（N≥3・内 ≥1 は loop +
      load lane — `evidence/rf763/verification.md`）
- [x] `mise exec -- prek run --all-files` exits 0
