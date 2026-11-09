---
id: "IMPL-jcCS5NAauic4OhiRJK2CAb"
type: "impl"
status: "completed"
title: "Issue #389 Plugin Editor Session headless fixture 実装記録（contract 認定 + DSP/RT 非干渉 fixture + NotRun 記録）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "docs/plans/implement-issue-389-editor-session-headless.md"
      - type: "file"
        path: "docs/tasks/issue-389-editor-session-headless.md"
      - type: "file"
        path: "docs/test-specs/issue-389-editor-session-headless.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/editor_session.rs"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-389-editor-session-headless.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-389-editor-session-headless.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-session-hardening.md"
    - "docs/plans/implement-issue-389-editor-session-headless.md"
    - "docs/test-specs/issue-389-editor-session-headless.md"
    - "docs/specs/issue-389-plugin-editor-session.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
  issues: [389]
---

# Issue #389 Plugin Editor Session headless fixture 実装記録

計画・タスク・テスト仕様:
`docs/plans|tasks|test-specs/issue-389-editor-session-headless.md`。

## Summary

driver-neutral Plugin Editor Session contract（型付き
`PluginEditorSession`/`PluginEditorSessions`/`EditorSessionOps`・
lifecycle state machine・generation/ownership・RT 非干渉）を
headless fixture で認定する slice。contract 本体は main 上で
実装済み（PR #399/#427/#503・VST3 L0 attach #402）であり、本
slice は **`src/` の契約変更ゼロ**で、残存していた fixture gap —
`set_visibility` no-op の event 抑止と、実 `InProcessHost` の
`process` 経路を跨いだ DSP/RT 非干渉 — を `tests/editor_session.rs`
に 4 case 追加して固定した。実 plugin/実 OS/GPU/実機共同検証は
NotRun/Unsupported のまま記録する。

## Context

Issue #389 の contract 層は approved design
（`docs/designs/issue-389-plugin-editor-session.md` 改訂 1・
`docs/designs/issue-389-session-hardening.md`）どおり main に
存在するが、(a) visibility no-op が event を発火しない規則
（event consumers count real state changes only）に直接の
fixture がなく、(b) session ops が DSP `process` に干渉しない
ことを実 `InProcessHost` 経路で示す fixture がなかった。
本 slice はその 2 点のみを埋める — 新規契約・API 変更は
行わない。

## Implementation

### Changes

- `crates/ruviform-plugin-host/tests/editor_session.rs` —
  EH-01..04 追加:
  - `visibility_noop_emits_no_event`（EH-01）: Attached/Hidden と
    同じ visibility 要求は ops 呼出のみで event/state を変えない
    — event 列は実 state 変更と 1:1。
  - `l0_session_open_close_leaves_dsp_processing_untouched`
    （EH-02）: `Processing` 中の instance で session
    open → 非 lifecycle op 拒否（`Unsupported`・非致命）→
    close。全段で `lifecycle_state == Processing`・
    `process().status == Continue`。
  - `l0_session_abnormal_termination_leaves_dsp_processing_untouched`
    （EH-03）: `notify_terminated(HostLost)` で session が
    `Terminated` → `Closed` に収束しても DSP 経路は不変。
  - `l0_refused_open_leaves_dsp_processing_untouched`
    （EH-04）: plugin 既定 `Unsupported` の refused open →
    `Failed` session → adapter 非呼出し close でも DSP 不変。
  - helper: `drive_to_processing`（Initialize → Activate →
    `StartProcessing` で `Processing` 到達）・`process_block`
    （mono planar 8-frame `process`）・`assert_dsp_untouched`。
- `docs/` チェーン新設:
  `docs/plans/implement-issue-389-editor-session-headless.md`・
  `docs/tasks/issue-389-editor-session-headless.md`・
  `docs/test-specs/issue-389-editor-session-headless.md`・本記録。
  EH-R5 として **focus 境界**を記録 — keyboard focus は session
  port の動詞ではなく #388 `NativeSurfaceSlot::request_focus` /
  `WindowDriver::focus`（presentation/slot 層）が所有し、dock
  focus の session 側意味論は `Attached ↔ Hidden`
  （`set_visibility`）が担う。approved design D4 の ops 一覧
  （focus を含まない）と ES/NSC 棲み分けが根拠であり、session
  port への focus verb 追加は行わない。
- 各 dir の README index に行追加（plans/tasks/test-specs は
  `doc_maintenance.js apply` で managed index 再生成、
  impl/ir は手動行追加）。

### Decisions During Implementation

- **focus は session contract の対象外と確定**: task spec の
  「attach/detach/focus/close」は focus を含むが、approved
  design D4 の `EditorSessionOps` は capabilities/open/close/
  set_size/set_visibility/reattach の 6 動詞で focus を持たず、
  focus の所有は #388 slot 層（`NativeSurfaceSlot::request_focus`・
  NSC-05）に確定済み。session port に focus verb を足すと
  常時 `Unsupported` の動詞と責務重複を生むため、境界を
  test-spec EH-R5 として明文化するに留めた。
- **fixture は既存規則のみを pin**: 追加 test は contract 変更を
  要求せず、no-op visibility 抑止と DSP 非干渉という既存の
  不変条件の直接証拠のみを固定する。
- **`src/` 差分ゼロを維持**: 公開契約・依存・lockfile・
  workspace manifest に一切触れていない（`git diff` で確認）。

## Validation

canonical（mise）— 全て実行済み:

- `mise run fmt-check` → exit 0
- `mise run check`（`cargo check --workspace --all-targets`）→
  exit 0
- `mise run lint`（clippy `-D warnings`）→ exit 0
  （初回 `doc_markdown` 指摘を修正後に green）
- `mise run test`（`cargo test --workspace`）→ exit 0
- `mise run test-rt`（`cargo test --workspace rt_` filter）→
  exit 0
- `test-msrv` 相当の汎用 task は `mise.toml` に存在しない
  （`test-ofx-msrv`/`test-ui-test-msrv` のみ・本変更の対象
  crate 外のため未実行 — 対象外として記録）

補助証拠（canonical = mise）:

- `cargo test -p ruviform-plugin-host --test editor_session` →
  49 passed / 0 failed（既存 45 + EH-01..04）

## Risks

- fixture green を実 Native Dock/実 plugin PASS と誤読される
  リスク → 本記録・test-spec・PR で NotRun/Unsupported を明記。
  headless fixture は状態機械と語彙の契約証拠であり、実機受入
  ではない。
- `process` 駆動の lifecycle 前提（`Processing` 到達）が将来の
  host 変更で退行するリスク → EH-02..04 が
  `lifecycle_state == Processing` を各段で明示 assert するため
  検出可能。

## NotRun / Unsupported（正直な記録）

- 実 plugin editor（実 VST3/CLAP/AU binary）での attach/detach/
  focus/close — **NotRun**（本 slice は headless fixture のみ）。
- 実 OS window・実 GPU・pixel compositing での非干渉 —
  **NotRun**。
- L1 out-of-process session ops — `L1EditorOps` は全 op
  `Unsupported` の defined refusal（変更なし・意図どおり）。
- #388 との実機共同検証（NativeSurfaceSlot 統合・window parent
  lease の実 HWND/NSView 経路）— **NotRun**（別 slice/Issue）。
- GUI hang/フリーズが DAW/RT を巻き込まない実機検証 —
  **NotRun**（Issue #389 残 DoD）。
- Synvelis Issue #176 の real-device acceptance — 対象外。

## Follow-ups

- Issue #389 残 DoD: raw `GuiParentToken` の public API 除去、
  実 VST3/CLAP/AU・L0/L1 実機受入、macOS/Linux・multi-plugin
  sweep、GUI hang 非波及の実機検証。
- driver（GPUI/composition root）からの `tick`/`tick_all` 定期
  呼出接続（hardening の deferred・実 driver 側作業）。
- L1 GUI wire 動詞（issue-115 解除条件充足後の別 Issue）。
