---
id: "TASK-034L6yh0fBmRabh1TMLv2F"
type: "task"
status: "todo"
title: "Implement OFX video rendering"
created: "2026-10-24"
updated: "2026-10-24"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-98-ofx-host-adapter.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-98-ofx-host-adapter.md"
    - "docs/tasks/ofx-native-runtime-implementation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/video-plugin-plain-contract-tests.md"
    - "docs/test-specs/ofx-host-native-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Render implementation

## Work

approved plan の **actions 9/10/11/12 を 1 task の ordered substeps として実行**。

### A9 — render contract tests

- render 契約テスト: 2×2 orientation oracle・Source SDK corners・負 Source+Output rowBytes assert・`PlainParamValue::Double(0.5)` gain 画素反映・Begin/Render/End 各失敗で pixel+meta+padding sentinel 不変・Created staged setter/Destroyed reject・slave Bool/Choice→`GetClipPreferences` 変化で次 render reject・revert 回復・time 小値 oracle・lifetime assert（aux `libloading` handle は assert 前 drop）

### A10 — red

- `mise run test-ofx` — render 未実装由来の失敗記録（人工欠陥なし・source 不変で既 green なら証拠記録）

### A11 — implement render

- preflight（state+`Rgba8`/dims・`plane_mut` 一意・**算出 staging stride/size**）→ 単一 ClipPrefs validation helper を activation と**各 render で無条件**（local preflight 後・`BeginSequenceRender` 前）→ unsupported/failed prefs は Begin/Render/出力変化なしに reject
- adapter 所有 16B aligned staging・bottom-left 露出（最終物理行 pointer + 負 checked-i32 `rowBytes`・`bounds[0,0,w,h]`）・Begin→Render→End bracket（Render 失敗でも Begin close・primary error・全成功のみ commit）・`t = at.to_seconds_f64()*num/den`（finite・極端時刻の一致保証なし）・`set_pts`/`set_duration`/`set_color`
- typed plain 制御の native get/set は `ParamValue` を経由しない・native setter 失敗時 arbitrary rollback は保証しない（image output transaction のみ保証）

### A12 — green

- `mise run test-ofx`・`mise run lint-ofx`・`mise run test-ofx-msrv` 全 exit 0（実 log）

## Completion Gate

3 entry 全 green 実 log + lead review の後にのみ task 完了・buildable scope commit。

## Verification

- [ ] `mise run test-ofx` — exit 0
- [ ] `mise run lint-ofx` — exit 0
- [ ] `mise run test-ofx-msrv` — exit 0（実 log）
