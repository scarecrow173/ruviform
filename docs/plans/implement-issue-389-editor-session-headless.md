---
id: "PLAN-KtWrNWuBuOq3OuF5yJ0x9o"
type: "plan"
status: "approved"
title: "Issue #389 Plugin Editor Session headless fixture slice 実装計画（contract 認定 + DSP/RT 非干渉 fixture + NotRun 記録）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-389-editor-session-headless.md"
      - type: "file"
        path: "docs/test-specs/issue-389-editor-session-headless.md"
      - type: "file"
        path: "docs/impl/ir/issue-389-editor-session-headless-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/editor_session.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-plugin-editor-session.md"
  implemented-by:
    - "docs/tasks/issue-389-editor-session-headless.md"
  depends-on:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-session-hardening.md"
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
    - "docs/test-specs/issue-389-session-hardening.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-389-session-hardening.md"
    - "docs/designs/issue-388-native-surface-slot.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-session-hardening.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-editor-session-headless.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 389
---

# Issue #389 Plugin Editor Session headless fixture slice 実装計画

## Goal

driver-neutral Plugin Editor Session の型付き contract
（attach/detach/close lifecycle・generation/ownership・RT 非干渉）を
headless fixture で表現し、本 slice の受入を「契約層の fixture 証拠 +
実機/実 OS/共同検証行の NotRun 記録」として確定する。contract 本体
（`editor_session` module・`EditorSessionOps` port・L0/L1 adapter）は
main 上で既に実装済み（PR #399/#427/#503・VST3 L0 attach は #402）
であり、本 slice は **src の契約変更を行わず**、headless fixture の
残存 gap を埋めて slice 証拠を固定する。

## Scope

- `crates/ruviform-plugin-host/tests/editor_session.rs` への
  contract fixture 追加のみ（fake ops + 実 `InProcessHost` L0 経路）。
- `docs/` チェーン（本 plan・task・test-spec・impl record）。
- 触らない: `src/` の公開契約・`ruviform-plugin-core`・
  `ruviform-ui*`・`ruviform-poc-native-dock`・`.github/`・
  `Cargo.toml`/`Cargo.lock`/`mise.toml`（依存追加なし）。

## Upstream Documents

- `docs/specs/issue-389-plugin-editor-session.md`（approved・親 spec）
- `docs/designs/issue-389-plugin-editor-session.md`（approved・改訂 1
  R3 状態遷移表/R4 所有責務/R5 capability matrix/R6 DoD 達成状況）
- `docs/designs/issue-389-session-hardening.md`（approved・D1–D5）
- `docs/specs/issue-389-session-hardening.md`（approved）
- `docs/test-specs/issue-388-389-native-dock-contract.md`（ES-01..24）
- `docs/test-specs/issue-389-session-hardening.md`（SH-01..18）

## Design Inputs

contract の正本は既存 approved design 群。本 slice は新規設計判断を
追加しない — fixture が pin する規則は design D7（RT・DSP 独立性
規約）と `set_visibility` no-op 規則（「event consumers count real
state changes only」）のみ。

## File Map

- `crates/ruviform-plugin-host/tests/editor_session.rs` — EH-xx
  contract test 追加（`visibility_noop_emits_no_event`・L0
  DSP/RT 非干渉 3 件）+ `drive_to_processing`/`process_block`/
  `assert_dsp_untouched` helper。
- `docs/test-specs/issue-389-editor-session-headless.md` — EH-xx
  test 仕様 + focus 境界の記録。
- `docs/tasks/issue-389-editor-session-headless.md` — 実施 task。
- `docs/impl/ir/issue-389-editor-session-headless-implementation.md`
  — 実装記録（実装後）。

## Implementation Sequence

- [x] T-1 `tests/editor_session.rs` — EH-01..04 追加
  （visibility no-op・L0 graceful/abnormal/refused の DSP 非干渉）。
- [x] T-2 `docs/test-specs/`・`docs/tasks/` 作成。
- [x] T-3 impl record・NotRun 記録・`mise run` 検証 + review-protocol
  + PR。

## Dependencies and Risks

- 依存: 既存 contract 層（全て main merge 済み）。新規外部依存・
  Cargo.toml/lock 変更なし。
- リスク: `process` 駆動に必要な lifecycle 遷移
  （Initialize→Activate→StartProcessing）が stub plugin の
  `prepare` で失敗する可能性 → `GuiPlugin::prepare` は `Ok(())`
  固定のため非該当（test で実測 green）。
- リスク: fixture green を実 Native Dock PASS と誤読されること →
  test-spec/IR/PR で NotRun・非 PASS を明記。

## Task Breakdown

- `docs/tasks/issue-389-editor-session-headless.md` に T-1〜T-3 を
  記録。単一 worker が逐次実施（本 task は Orca dispatch 1 件）。

## Verification

- [ ] `mise run fmt-check` exit 0
- [ ] `mise run check` exit 0
- [ ] `mise run lint` exit 0
- [ ] `mise run test` exit 0（workspace 全 suite 非回帰）
- [ ] `mise run test-rt` exit 0（`rt_*` filter — zero match でも
  exit 0）
- [ ] `cargo test -p ruviform-plugin-host --test editor_session`
  で EH-xx 全 case green（補助証拠・canonical は mise test）
- [ ] `git diff` で `src/`・`ruviform-plugin-core`・`ruviform-ui*`・
  workspace `Cargo.toml`/`Cargo.lock`/`mise.toml` に差分がないこと

## Open Questions / Gaps

- focus は session port の動詞ではない: keyboard focus の要求・
  ルーティングは #388 `NativeSurfaceSlot::request_focus` /
  `WindowDriver::focus`（presentation/slot 層）が所有し、dock focus
  の session 側意味論は `Attached ↔ Hidden`（`set_visibility`）が
  担う。approved design D4 の ops 一覧（focus を含まない）と
  test-spec の ES/NSC 棲み分けが根拠 — 本 slice で focus verb を
  session port に追加しない（重複責務・常時 Unsupported の verb を
  増やすだけになる）。
- 実 plugin/実 OS/共同検証（#388 gate・Synvelis #176・実
  VST3/CLAP/AU・GPUI driver tick 接続）は本 slice 対象外 —
  NotRun/Unsupported として記録。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 worktree・単一 worker）。
- Independent work streams: none。
- User approval required before dispatch: Orca dispatch spec により
  coordinator 委譲済み — worker は review-protocol で承認相当を確認。
- If approved: 本 plan の T-1〜T-3 を逐次実施。

## Review Handoff

- reviewer 重点: fixture が contract の**既存**規則のみを pin して
  おり新規契約を捏造していないか、DSP 非干渉 test が実 `process`
  経路を通っているか、NotRun/Unsupported が PASS に偽装されて
  いないか、src 差分がゼロであること。
