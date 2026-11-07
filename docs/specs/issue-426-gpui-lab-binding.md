---
id: "SPEC-bO4Bjvhj9IWEhVO0Vo3ghA"
type: "spec"
status: "proposed"
title: "Issue #426 GPUI lab binding・Story 2 scale × 2 theme fixture・Pixel Golden 境界・Consumer GPUI 型不使用検査"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/426"
    - "docs/specs/issue-390-driver-contract-lab.md"
    - "docs/discovery/issue-390-driver-contract-lab-inventory.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lab_binding.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/stories.json"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_lab.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/component_view.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
    deleted: []
    renamed: []
    moved: []
---

# Issue #426 GPUI lab binding 仕様

## 目的

#390 で確定した Driver Lab の manifest（`cases.json` / `stories.json`）のうち、
GPUI 層の `not_run` を実態化する。`pass` は対応する test の実在を機械検査で
裏付けたものに限る。

## 受入条件（Issue #426 本文に従う）

| ID | 条件 | 本 PR の状態 |
| --- | --- | --- |
| A1 | UI-DVR-01/02/03/04/07/08 の GPUI binding が manifest 上 `pass`、対応 test が実在 | 達成（UI-DVR-02 は #421 で既に pass、本 PR で mechanism に filter を付与） |
| A2 | Story 2 件（LAB-SV-FADER-DOCK / LAB-AL-WAVEFORM-SEEK）が 2 scale × 2 theme × semantic state を検証 | fake layer で達成。GPUI layer は未達（scale 1.0 を gpui test platform で再現不可） |
| A3 | Pixel Golden の取得・比較が #314 Utility 経由で記録 | 未達（#314 は OPEN、Golden Utility 未実装。NotRun として記録） |
| A4 | Consumer repo の GPUI 型不使用が機械的に検証 | 達成（dependency-tree 検査、consumer 側 2 件 pass。ruviform は consumer の pinned revision で評価） |

## 境界・非目標

- 実 OS lane・IME・a11y・Native Plugin Dock は #313 / #388 / #389 / #262 の owner。本 PR は参照・記録のみ。
- Consumer repo（audio-lens / synvelis）は read-only。変更は行わない。
- 共通層（`ruviform-ui` / `ruviform-ui-theme`）に GPUI 型を出さない。`check-ui-driver-boundary` は pass を維持する。

## 判定規則

- `pass` は mechanism 必須。`mise-task` 以外に `package` + `filter` を指定した場合、その package の
  `src/` と `tests/` に filter に一致する test 属性付き fn が実在しなければ manifest 検査が失敗する。
- `not_run` / `unsupported` / `blocked` は owner（Issue URL）と rerun 条件を必須とする。
- Fake 成功を GPUI Pass にしない。scale など layer で再現できない軸は `not_run` と理由で明示する。

## 関連

- 設計: `docs/designs/issue-426-gpui-lab-binding.md`
- 計画: `docs/plans/issue-426-gpui-lab-binding.md`
- 作業: `docs/tasks/issue-426-gpui-lab-binding.md`
- 実装記録: `docs/impl/ir/issue-426-gpui-lab-binding.md`
