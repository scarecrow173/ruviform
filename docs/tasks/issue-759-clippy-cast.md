---
id: "TASK-034ZhkreemsrkXWFBQhQEX"
type: "task"
status: "done"
title: "Issue #759 plugin-clap host.rs f64 as u32 cast — lint lane 修復"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/759"
    - "docs/discovery/issue-759-clippy-cast.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-759-clippy-cast.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/discovery/issue-759-clippy-cast.md"
  implemented-by:
    - "docs/impl/ir/issue-759-clippy-cast.md"
  depends-on:
    - "docs/discovery/issue-759-clippy-cast.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-clap-l0-editor-attach.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-759-clippy-cast.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #759 plugin-clap host.rs f64 as u32 cast — lint lane 修復

## Work

`crates/ruviform-plugin-clap/src/host.rs` の `set_gui_size` にある
`GuiSize { f64 } → ClapGuiSize { u32 }` の `as` cast 2 箇所が
workspace clippy（pedantic + `-D warnings`）で 4 error を出し、
host feature 含有の lint lane（`mise run lint
ruviform-plugin-test-clap` 等）が main で常時 Fail する defect
（`ruviform#759`、`ruviform#754` merge `0450f92f` 由来）を修復する。

境界契約は discovery で確定済み: 各成分を最近接ピクセルへ round し、
`[0, u32::MAX]` に飽和させる（負値・NaN・非有限 → `0`、
`u32::MAX` 超過 → `u32::MAX`）。`gui_size_component(f64) -> u32`
ヘルパーに集約し、doc comment に契約を明記。cast lint は clamp で
実際に安全化した上で `#[expect]` + 理由を局所許可する
（`#[allow]` は使わない）。境界契約を固定する unit test を
host feature の test module に追加する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/discovery/issue-759-clippy-cast.md`（spec 相当 — Issue 本文
  の受入条件が確定済みの是正タスク）
- `docs/designs/issue-389-clap-l0-editor-attach.md`（editor verbs の
  上流設計）

## Dependencies

- なし（`ruviform#759` 単独。`ruviform#757` は発見元の記録のみ）

## Affected Files

- `crates/ruviform-plugin-clap/src/host.rs` — `gui_size_component`
  追加、`set_gui_size` の cast を helper 呼出へ置換、境界契約 test 追加
- `docs/discovery|tasks|impl/ir|impl/exp/issue-759-clippy-cast*` —
  doc chain（最小構成）
- `evidence/rf759/` — 検証 evidence

## Done When

- [x] `mise run lint ruviform-plugin-test-clap` が exit 0
- [x] `mise run lint ruviform-plugin-clap` が exit 0
- [x] cast の境界挙動（負値・NaN・u32::MAX 超過）がコードの doc
      comment と unit test に明記される
- [x] host feature 含有の他 lint lane で新規失敗を生じない

## Verification

- [x] `mise run fmt-check` exits 0
- [x] `mise run check ruviform-plugin-clap` exits 0
- [x] `mise run lint ruviform-plugin-clap` exits 0
- [x] `mise run test ruviform-plugin-clap` exits 0
- [x] `mise run lint ruviform-plugin-test-clap` exits 0（Issue の
      破損 lane — 受入条件）
- [x] `mise run check ruviform-plugin-test-clap` exits 0
- [x] `mise run test ruviform-plugin-test-clap` exits 0（非回帰）
- [x] `mise run test-l1-real-plugin` exits 0 — host feature の
      `--lib --bins` 段で新規 unit test を含む canonical 実行、
      `#[ignore]` E2E 段も全 pass（`evidence/rf759/`）
- [x] `mise exec -- prek run --all-files` exits 0
