---
id: "PLAN-034UnzhplY5WerbCeab8un"
type: "plan"
status: "approved"
title: "Implement Issue #389 Plugin Editor Session hardening"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-389-session-hardening.md"
      - type: "file"
        path: "docs/test-specs/issue-389-session-hardening.md"
      - type: "file"
        path: "docs/impl/ir/issue-389-session-hardening-implementation.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-session-hardening.md"
  implemented-by:
    - "docs/tasks/issue-389-session-hardening.md"
  depends-on:
    - "docs/designs/issue-389-session-hardening.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-session-hardening.md"
    - "docs/designs/issue-389-session-hardening.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-session-hardening.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 389
---

# Implement Issue #389 Plugin Editor Session hardening Implementation Plan

## Goal

`ruviform-plugin-host::editor_session` に bounded timeout
（注入可能 clock port）・明示 degrade 経路・`GraphicsLost` cause を
additive に実装し、DComp 制約を capability matrix / evidence に正直に
記録する単一 stream の hardening slice。

## Scope

単一実装 stream。4 項目は同一ファイル（`editor_session.rs`）への
additive 変更であり、分割する利益がない。編集範囲は
`crates/ruviform-plugin-host/`（`src/editor_session.rs`・`src/lib.rs`
export・`tests/editor_session.rs`・`README.md`）、`docs/` チェーン、
`evidence/rf389-session-hardening/` のみ。`ruviform-plugin-core`・
`ruviform-ui-gpui`・`ruviform-poc-native-dock`・workspace
`Cargo.toml`/`Cargo.lock` は不変。

## Upstream Documents

- `docs/specs/issue-389-session-hardening.md`（approved）
- `docs/specs/issue-389-plugin-editor-session.md`（親 spec・approved）
- `docs/designs/issue-389-plugin-editor-session.md`（親 design・改訂 1
  R6 の未達項目が対象）

## Design Inputs

- `docs/designs/issue-389-session-hardening.md`（approved・D1–D5）

## File Map

- `crates/ruviform-plugin-host/src/editor_session.rs` — `SessionClock`/
  `MonotonicSessionClock`/`SessionDeadlinePolicy`/`DEFAULT_REQUEST_TIMEOUT`/
  `deadline_policy`/`deadline_at`/`degrade_pending` field、`next_request`
  での deadline arm、`tick`/`request_deadline`/`set_deadline_policy`/
  `open_or_degrade`/`begin_degrade`、`OpenOutcome` enum、
  `EditorSessionEvent::Degraded`、`SessionFailCause::GraphicsLost`/
  `EditorOpError::GraphicsLost` + `fail_cause`/`Display` 写像、
  `PluginEditorSessions::tick_all`。
- `crates/ruviform-plugin-host/src/lib.rs` — `pub use editor_session`
  の export 追加（`MonotonicSessionClock`/`OpenOutcome`/`SessionClock`/
  `SessionDeadlinePolicy`）。
- `crates/ruviform-plugin-host/tests/editor_session.rs` — fake clock・
  mode 別 open result を持つ FakeOps 拡張、SH-xx contract test 追加。
- `crates/ruviform-plugin-host/README.md` — session hardening の
  1 段落追記（timeout/degrade/GraphicsLost の存在）。
- `docs/test-specs/issue-389-session-hardening.md` — SH-xx test 仕様。
- `docs/tasks/issue-389-session-hardening.md` — 実施 task。
- `docs/impl/ir/issue-389-session-hardening-implementation.md` —
  実装記録（実装後）。
- `evidence/rf389-session-hardening/report.md` — 検証結果と
  NotRun/Unsupported の正直記録。

## Implementation Sequence

- [ ] T-1 `editor_session.rs` — D1/D2: `SessionClock` port・
  `MonotonicSessionClock`・`SessionDeadlinePolicy`・
  `DEFAULT_REQUEST_TIMEOUT`・session fields・`next_request` arm・
  `tick`/`request_deadline`/`set_deadline_policy`/`tick_all`。
- [ ] T-2 `editor_session.rs` — D3: `OpenOutcome`・`Degraded` event・
  `degrade_pending` flag・`open_or_degrade`・`begin_degrade`・
  `complete_attach` の Degraded 発火分岐。
- [ ] T-3 `editor_session.rs` — D4: `GraphicsLost` 両 enum + 写像。
- [ ] T-4 `lib.rs` export + `README.md` 追記。
- [ ] T-5 `tests/editor_session.rs` — FakeOps mode 別 result・
  FakeClock・SH-xx 全 case 追加。
- [ ] T-6 capability matrix（design D5）・`evidence/rf389-session-hardening/
  report.md`・impl record 作成。
- [ ] T-7 `mise run fmt-check`/`check`/`lint`/`test` green +
  review-protocol + PR。

## Dependencies and Risks

- 依存: 親契約層（PR #427 merge 済み `00324e10`）と PR #501 の
  DComp 実測記録（merge 済み）。新規外部依存なし。
- リスク: default deadline arm が driver の `tick` 呼出開始後に
  timeout を発火させうる → 10s default・disarm 可能・sync path
  非 preempt で緩和（design「Risks」）。
- リスク: `ops.open` 2 回呼出の degrade retry で adapter が部分
  editor を残す可能性 → 「`Unsupported` を返す open は editor を
  生成していない」前提を doc 明記（既存 refused-open 規律と同一）。

## Task Breakdown

- `docs/tasks/issue-389-session-hardening.md` に T-1〜T-7 を記録。
  単一 worker が逐次実施（本 task は Orca dispatch 1 件）。

## Verification

- [ ] `mise run fmt-check` exit 0
- [ ] `mise run check` exit 0（`cargo check --workspace --all-targets`）
- [ ] `mise run lint` exit 0（clippy `-D warnings`）
- [ ] `mise run test` exit 0（workspace 全 suite — 既存 28
  contract test を含む非回帰）
- [ ] `cargo test -p ruviform-plugin-host --test editor_session`
  で SH-xx 全 case green（補助証拠・canonical は mise test）
- [ ] `git diff` で `ruviform-ui-gpui`/`ruviform-poc-native-dock`/
  `ruviform-plugin-core`/workspace `Cargo.toml`/`Cargo.lock` に
  差分がないこと

## Open Questions / Gaps

- `DEFAULT_REQUEST_TIMEOUT` の値（10s）は契約上の妥当な上限としての
  暫定値 — 実 adapter の計測後に見直す余地あり（const 1 箇所）。
- async driver が `tick` を呼ぶ周期の所有は composition root 側
  （本 slice 対象外・Deferred）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 worktree・単一 worker）。
- Independent work streams: none。
- User approval required before dispatch: Orca dispatch spec により
  coordinator 委譲済み — worker は review-protocol で承認相当を確認。
- If approved: 本 plan の T-1〜T-7 を逐次実施。

## Review Handoff

- reviewer 重点: degrade retry が `Unsupported` のみに限定されて
  いるか、`Degraded` event が `Attached`/`Recovered` を排他して
  いるか、timeout が terminal event を重複させないか、
  capability matrix が実測/NotRun を偽装していないか。
