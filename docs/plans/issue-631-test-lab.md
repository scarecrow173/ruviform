---
id: "PLAN-034YaYv5gxaA5bas6AXXIq"
type: "plan"
status: "approved"
title: "Issue #631 GPUI Test Lab / Scenario Console GUI v1 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-631-test-lab.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 GPUI Test Lab / Scenario Console GUI v1 実装計画 Implementation Plan

## Goal

`ruviform-plugin-dock-test` に feature-gated `plugin-dock-lab` bin
（GPUI windowed Test Lab / Scenario Console GUI v1）を追加し、
`mise.toml` の #631 lane 簿記（console canonical 化・lab opt-in
lane・real description 修正）を整える。

## Scope

単一 implementation stream — GUI bin + mise 簿記 + docs/evidence。
分割しない（bin は 1 ファイル、mise/docs は同一 slice の簿記）。

## Upstream Documents

- `docs/specs/issue-631-test-lab.md`（approved）
- `docs/designs/issue-631-test-lab.md`（approved）
- `docs/discovery/issue-631-test-lab.md`

## Design Inputs

- `docs/designs/issue-631-test-lab.md`
- 先例: `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-console.rs`、
  `src/bin/plugin-dock-real-scenario.rs`、
  `crates/ruviform-ui-gpui/src/bin/windowed_test_app.rs`、
  `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs`（app-thread
  + ready-channel 構成）、`src/real_dock.rs`（blocked_reports の
  正直形）。

## File Map

- `crates/ruviform-plugin-dock-test/Cargo.toml` — `lab-gui` feature・
  `plugin-dock-lab` bin（required-features）追加、依存追加理由の
  記録更新。
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-lab.rs` —
  GUI v1 全実装 + pure 部分の `#[cfg(test)]` unit test
  （catalog row 写像・spec 組立・blocked report・arg parse）。
- `mise.toml` — `test-ui-plugin-dock-console`・
  `test-ui-plugin-dock-lab` 追加、`test-ui-plugin-dock-real`
  description/コメント修正。
- `docs/development-environment.md` — #631 lane 4 task の表行追加。
- `evidence/rf631/verification.md` — 本 slice 節追記。
- `docs/impl/ir|exp/issue-631-gpui-test-lab-*.md|jsonl` — impl 記録。

## Implementation Sequence

- [ ] 1. `Cargo.toml`: `lab-gui` feature + `[[bin]] plugin-dock-lab`
      追加（依存コメントに lab-gui の位置づけを記録）。
- [ ] 2. `plugin-dock-lab.rs` 実装:
      arg parse（`--out`）→ catalog row 写像 → ViewSpec 組立 →
      ViewHost mount → app thread + Ready handshake → on_event
      dispatch（select/run/quit/receipt）→ worker + Tick drain →
      JSONL append → blocked path。
- [ ] 3. bin 内 unit test（pure 部分のみ — window/driver は開かない）。
- [ ] 4. `mise.toml` 3 変更。
- [ ] 5. `docs/development-environment.md` lane 行追加。
- [ ] 6. 検証 lane 実行 → `evidence/rf631/verification.md` 追記 →
      impl/ir + impl/exp 記録。

## Dependencies and Risks

- `ruviform-ui-gpui/gpui` feature は既存 — 新規 dependency なし。
- `GpuiDriver`/`ViewMount`/`COMMAND_INTENT_OP` は `gpui` feature
  export — `lab-gui` で有効化されることを Cargo.toml で保証。
- `WindowLifecycle::Ready` の timeout 判定は real lane と同じ
  観測水準 — session-0 誤検出リスクは design に記録済み。
- `cargo test --features lab-gui` は bin を build するため GPUI
  link が走る — 重いが 1 回限り。unit test は window を開かない
  pure 関数のみ。

## Task Breakdown

- T1: Cargo.toml feature/bin 登録。
- T2: plugin-dock-lab.rs 実装 + unit test。
- T3: mise.toml 簿記 3 点。
- T4: development-environment.md 更新。
- T5: 検証 + evidence + impl 記録。

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check|lint|test ruviform-plugin-dock-test`（default）
- [ ] `mise run check|lint|test ruviform-plugin-dock-test --features real-dock`
- [ ] `mise run check|lint|test ruviform-plugin-dock-test --features lab-gui`
- [ ] `mise run test-ui-plugin-dock`（verdict 非回帰）
- [ ] `mise run test-ui-plugin-dock-console`（新設・canonical）
- [ ] `mise run test-ui-plugin-dock-lab`（新設・opt-in・fail-closed）
- [ ] `mise run test-ui-scenario-contract`（contract 非回帰）
- [ ] `mise run check-ui-driver-boundary`（boundary guard）
- [ ] `mise exec -- prek run --all-files`

## Open Questions / Gaps

- `lab-gui` feature 名は `real-dock`/`gpui-test`/`uitest` の命名と
  衝突しない — 確定。
- interactive 実走は本 worker の環境（Windows interactive なら可）
  に依存 — headless なら `environment_blocked` を実測記録。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatched worker が直接実装）。
- Independent work streams: none。
- User approval required before dispatch: no（worker dispatch 済み）。

## Review Handoff

- reviewer focus: verdict honesty（projection≠evidence・real row
  非実行・blocked fail-closed）、driver boundary（GPUI 型の非流出）、
  mise description の実態一致、receipt 契約の正しさ。
