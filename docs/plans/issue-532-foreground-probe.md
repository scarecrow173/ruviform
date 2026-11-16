---
id: "PLAN-034YxG8sLN3ehSqJxikrYs"
type: "plan"
status: "approved"
title: "Issue #532 foreground-probe 実装計画"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-532-foreground-probe.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
  implements:
    - "docs/specs/issue-532-foreground-probe.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-532-foreground-probe.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-532-foreground-probe.md"
    - "docs/specs/issue-532-foreground-probe.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-532-foreground-probe.md"
  derived-by:
    - "docs/tasks/issue-532-foreground-probe.md"
  verifies:
    - "docs/test-specs/issue-532-foreground-probe.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 foreground-probe 実装計画 Implementation Plan

## Goal

`ruviform-plugin-dock-test` に real-lane case `real-dcomp-foreground`
を追加し、GPUI root の z-order/foreground を段階的に変異させて
外部 HWND stacking と compositor 内部遮蔽を二分する probe を
実装・実走・記録する。

## Scope

In: `src/osmut.rs` 新規、`real_dock.rs`/`real_catalog.rs`/
`lib.rs`/`Cargo.toml`(comment)/`tests/console.rs`/`README.md`
更新、doc chain、evidence。
Out: 方式採否・本 fix、他 crate、`mise.toml`、docs index、
Issue close。

## Upstream Documents

- spec: `docs/specs/issue-532-foreground-probe.md` (SPEC-034YxCZzURRuUuQPn2Wdgk)
- design: `docs/designs/issue-532-foreground-probe.md` (DESIGN-034YxD6W7IXEfl432tYKlt)

## Design Inputs

変異段階・verdict 意味論・foreground 非対称規則・復元契約・
`AttachThreadInput` 等不採用の評価は design doc に確定済み。
本 plan はそれを file/task に落とす。

## File Map

| file | 変更 |
| --- | --- |
| `crates/ruviform-plugin-dock-test/src/osmut.rs` | 新規 — RootSnapshot/MutationOutcome/RestoreOutcome + observe/raise_to_top/set_foreground/set_topmost/restore |
| `crates/ruviform-plugin-dock-test/src/lib.rs` | `mod osmut` 追加（real-dock feature）+ module doc 更新 |
| `crates/ruviform-plugin-dock-test/src/real_dock.rs` | `real_entries` 14 要素化 + `real_dcomp_foreground_case`/`observe_editor_foreground_probe`/`foreground_probe_children`/`ForegroundVerification` + 件数 assertion/doc 更新 |
| `crates/ruviform-plugin-dock-test/src/real_catalog.rs` | 14 行目追加 + doc 更新 |
| `crates/ruviform-plugin-dock-test/Cargo.toml` | windows-sys dep コメント更新 |
| `crates/ruviform-plugin-dock-test/tests/console.rs` | 21→22 行・case 一覧・コメント |
| `crates/ruviform-plugin-dock-test/README.md` | case 表 + `--include-real` 件数 |
| `docs/{discovery,specs,designs,plans,tasks,test-specs,impl}/issue-532-foreground-probe*` | doc chain |
| `evidence/rf631/plugin-dock-real-scenario.jsonl` | baseline lane 再生成 |
| `evidence/rf631/plugin-dock-real-scenario-dcomp-disabled.jsonl` | env-on lane 再生成 |
| `evidence/rf631/verification.md` | 実走記録 append |
| `evidence/rf532/foreground-probe-bisect.md` | 二分結論文書（新規） |

## Implementation Sequence

1. `osmut.rs` 実装（Windows + stub、plain 値境界、SAFETY）。
2. `lib.rs`/`Cargo.toml` 配線。
3. `real_dock.rs` case 実装（catalog/entry/件数/doc）。
4. `real_catalog.rs`/`tests/console.rs`/`README.md` 整合。
5. `cargo fmt` + `check`/`lint`/`test`（default + real-dock）。
6. console/contract/boundary/prek lane。
7. interactive real lane baseline + env-on 実走。
8. evidence + impl docs + 二分結論。

## Dependencies and Risks

- 依存: `windows-sys 0.61`（既存 feature のみ）、`osoccl`/`ospix`/
  `oschild` の既存 helper。新規 crate 依存なし。
- リスク: `SetForegroundWindow` 拒否（記録のみ）・復元不完全
  （topmost flag のみ検証）・z-order race（各段階 snapshot 記録）。

## Task Breakdown

単一 task `docs/tasks/issue-532-foreground-probe.md` に集約
（1 slice = 1 task、内部は実装 sequence で順序付け）。

## Verification

`mise run fmt-check`、`mise run check/lint/test
ruviform-plugin-dock-test`（default + `--features real-dock`）、
`mise run test-ui-plugin-dock`、`test-ui-plugin-dock-console`、
`test-ui-scenario-contract`、`check-ui-driver-boundary`、
`mise exec -- prek run --all-files`、interactive real lane
baseline/env-on。

## Open Questions / Gaps

なし — 変異段階・verdict・復元・lock 回避不採用は design で確定。

## Self-Review

- spec の全 In Scope 項目が file map/sequence に対応。
- verdict が screen green 非依存であることを design/spec と整合。
- 復元契約が topmost flag 検証 + 観測記録の非対称規則どおり。

## Implementation Handoff

`docs/tasks/issue-532-foreground-probe.md` へ。

## Review Handoff

review-protocol で 2 reviewer pass → blocker 解消 → commit/push/PR。
