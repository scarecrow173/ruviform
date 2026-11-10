---
id: "PLAN-314rwCt3mN8qRsV6xZ1aB"
type: "plan"
status: "approved"
title: "Issue #314 slice 4 residual — real-window adapter 実装 plan"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "docs/specs/issue-314-real-window-adapter.md"
    - "docs/designs/issue-314-real-window-adapter.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-314-real-window-adapter.md"
---

# 実装 plan

## file map

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-ui-gpui/Cargo.toml` | `real-scenario` feature + `[[test]] real_window` gate |
| `crates/ruviform-ui-gpui/src/lib.rs` | `UiJob` 型、`Inner.ui_jobs`、`set_ui_jobs`、`drain_ui_jobs`、`view_of`/`minted_from_ipc` 共有化、re-export |
| `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs` | 新規: `RealOsGuiTest`/`RealWindowMount`/`RealWindowOptions`/`CaptureMeta` |
| `crates/ruviform-ui-gpui/src/gpui_driver/{observe,scenario}.rs` | 共有 helper 利用へ置換 |
| `crates/ruviform-ui-gpui/src/render.rs` | `host()` を `real-scenario` でも公開 |
| `crates/ruviform-ui-gpui/tests/real_window.rs` | 新規: story walk + pixel capture + golden loop |
| `crates/ruviform-ui-test/fixtures/driver-lab/stories.json` | `drivers.real-os` → pass + mechanism |
| `evidence/issue-314-real-window/` | driver-lab/golden JSONL + PNG + meta |
| `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/issue-314-real-window-adapter.md` | 文書チェーン + IR |

## 順序

1. feature + `UiJob`/`drain` seam + 共有 helper（`view_of`/`minted_from_ipc`）。
2. `real_window.rs` adapter 実装（run/settle/call + Semantic/Visual impl）。
3. `tests/real_window.rs` — canonical fixture walk + capture + golden loop。
4. `stories.json` 実績更新、evidence 生成、文書チェーン + IR。
5. 検証 lane → merge main → review → PR。

## risk / rollback

- 実 window が開けない環境では `run` が `UiError::Host` で失敗 —
  テストは desktop session 前提（golden_capture.rs と同じ扱い）。
- job が reply せず app thread が死んだ場合は 15 s timeout で
  `ScenarioError::Driver` — ハングしない。
- rollback は feature 追加のみで既存経路に影響しない
  （`real-scenario` 未指定時は全コードが cfg 外）。
