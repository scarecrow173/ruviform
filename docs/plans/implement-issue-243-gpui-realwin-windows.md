---
id: "PLAN-034UdQaz13FoOQV6oq6eKq"
type: "plan"
status: "completed"
title: "Issue #243 GPUI 実機 GUI 検証 Windows 実窓ライフサイクル証拠 — 実装計画"
created: "2026-11-09"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
  changes:
    added:
      - "crates/ruviform-ui-gpui/examples/window_lifecycle_probe.rs"
      - "docs/specs/issue-243-gpui-realwin-windows.md"
      - "docs/designs/issue-243-gpui-realwin-windows.md"
      - "docs/tasks/implement-issue-243-gpui-realwin-windows.md"
      - "docs/test-specs/issue-243-gpui-realwin-windows.md"
      - "docs/impl/ir/issue-243-gpui-realwin-windows.md"
    modified:
      - "crates/ruviform-ui-gpui/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../designs/issue-243-gpui-realwin-windows.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-243-gpui-realwin-windows.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-243-gpui-realwin-windows.md"
  derived-by:
    - "../tasks/implement-issue-243-gpui-realwin-windows.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  defers: []
  deferred-by: []
---

# Issue #243 GPUI 実機 GUI 検証 Windows 実窓ライフサイクル証拠 — 実装計画 Implementation Plan

## Goal

Windows 実機で GPUI Driver の Window lifecycle を実測する観測用 example を 1 本追加し、
probe・既存 lifecycle example・minimal window の実行証拠を impl record に記録する。

## Scope

単一の検証ストリーム。コード変更は example 1 本とその `[[example]]` マニフェスト
エントリのみ。製品コード・共有 fixture・mise.toml・docs index は変更しない。

## Upstream Documents

- `docs/specs/issue-243-gpui-realwin-windows.md`

## Design Inputs

- `docs/designs/issue-243-gpui-realwin-windows.md`（approved）

## File Map

- `crates/ruviform-ui-gpui/examples/window_lifecycle_probe.rs` — 観測用 example。
  既定モード: 2 窓生成 → focus/resize/monitors 操作 → OS `WM_CLOSE` による
  Veto/Defer/Approve/無応答既定 → reborn 再生成（id 非再利用）→
  `request_close`+`Veto` → `destroy_window` bypass → exit。
  `--degenerate-create` モード: 0x0 `WindowSpec` の生成失敗縮退を観測。
- `crates/ruviform-ui-gpui/Cargo.toml` — `[[example]]` エントリ追加
  （`required-features = ["gpui"]`。workspace `check --all-targets` 保護のため必須）。
- `docs/impl/ir/issue-243-gpui-realwin-windows.md` — 実測証拠の記録。

## Implementation Sequence

- [ ] T1: `window_lifecycle_probe.rs` を実装し `Cargo.toml` に example エントリを追加
- [ ] T2: `cargo build --example` でビルドし、probe 既定モードを
  `wm-close.ps1 -Steps "veto,veto,veto,main"` 経由で実機実行 → PASS/exit 0 を確認
- [ ] T3: probe `--degenerate-create` を実機実行し結果を分類記録
- [ ] T4: `real_window_lifecycle` を `wm-close.ps1 -Steps "inspector,inspector,main"` で
  再実行 → PASS/exit 0 を確認（現行 main での Dock detach/reattach 再確認）
- [ ] T5: `minimal_window` を `wm-close.ps1 -Steps "minimal"` で再実行 → exit 0 を確認
- [ ] T6: impl record に全証拠・NotRun/Unsupported 分類・再現手順を記録
- [ ] T7: `git fetch origin main && git merge origin/main` 後に
  `mise run fmt-check` / `mise run check` / `mise run lint-ui-gpui` /
  `mise run test-ui-gpui` を実行

## Dependencies and Risks

- `wm-close.ps1` は `evidence/issue-431-real-verify/` の既存資産を読み取り・実行のみ
  （変更しない）。`PostMessageW(WM_CLOSE)` は close ボタンと同じメッセージだが
  物理入力ではない — 証拠に明記する。
- `focus(w, false)` は gpui 0.2.2 で `Unsupported` が契約上の正解（観測値として記録）。
- `destroy_window` の `CloseRequested` bypass は契約上の期待。実測で逸脱した場合は
  契約違反の発見として FAIL 記録し、文書で報告する（黙って通さない）。
- degenerate 0x0 spec が gpui 内部で panic し得る → 別モード・別プロセスで実行。

## Task Breakdown

- `docs/tasks/implement-issue-243-gpui-realwin-windows.md` に T1〜T7 を記録。

## Verification

- [ ] `mise run fmt-check` exit 0
- [ ] `mise run check` exit 0
- [ ] `mise run lint-ui-gpui` exit 0
- [ ] `mise run test-ui-gpui` exit 0
- [ ] probe 既定モード `[probe] PASS` + exit 0（実機ログを impl record に引用）
- [ ] degenerate モードの分類結果を impl record に記録
- [ ] `real_window_lifecycle` / `minimal_window` の再実行結果を impl record に記録

## Open Questions / Gaps

- `ScaleFactorChanged` 実遷移・モニター切断・IME・物理入力・Plugin Dock・
  macOS/Linux・consumer 回帰は本スコープ外（NotRun/Unsupported、owner 明記）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatch worker が直接実装・実行）
- Independent work streams: none
- User approval required before dispatch: review-protocol で代替（blocker なし）

## Review Handoff

- `destroy_window` bypass の実測結果が契約と異なった場合は最重要の発見として報告。
- `monitors()` の scale factor は driver 既知の近似値 — 精度限界を証拠に明記。
