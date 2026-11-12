---
id: "IMPL-034UnS9SUfwZF2v4zSQck6"
type: "impl"
status: "completed"
title: "Issue #389 Plugin Editor Session hardening 実装記録（timeout/degrade/GraphicsLost/DComp 記録）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "evidence/rf389-session-hardening/report.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/editor_session.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/editor_session.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-389-session-hardening.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-389-session-hardening.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-session-hardening.md"
    - "docs/plans/implement-issue-389-session-hardening.md"
    - "docs/test-specs/issue-389-session-hardening.md"
    - "docs/specs/issue-389-session-hardening.md"
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

# Issue #389 Plugin Editor Session hardening 実装記録

設計・計画・テスト仕様: `docs/designs|plans|test-specs/
issue-389-session-hardening.md`。証拠: `evidence/rf389-session-hardening/`。

## Summary

`editor_session` の契約層 hardening — in-flight request の bounded
wall-clock deadline（注入可能 clock + driver 主導 `tick`）、preferred
mode `Unsupported` 時の明示 degrade（`OpenOutcome::Degraded` +
`Degraded` event）、および `GraphicsLost` failure cause を additive
に実装。DComp pixel compositing の実測制約は design D5 の capability
matrix に `Unsupported`（実測）/`NotRun` として記録。

## Context

親 session contract（PR #427 `00324e10`）は state machine・request
世代管理・terminal-once を持つが、timeout は error variant のみで
deadline 強制がなく、Floating は `Unsupported` 拒否のみ、GPU cause
は `HostLost`/`Internal` に潰れうる状態だった。本 slice はその 3 点を
契約として閉じ、PR #501 の DComp 実測を capability matrix に正直に
転記する。

## Implementation

### Changes

- `editor_session.rs` — `SessionClock` trait・
  `MonotonicSessionClock`（`Instant` 経過）・`SessionDeadlinePolicy`
  （`timeout` + `Box<dyn SessionClock>`）・`DEFAULT_REQUEST_TIMEOUT`
  （10s）。`next_request` が全 in-flight request（open/detach/close/
  recover/detach 後 re-attach）に deadline を arm。`tick` は期限超過
  request を既存の error 経路（`complete_attach`/`complete_detach` の
  `Err(Timeout)`、`Closing` は `mark_failed`）で `Failed{Timeout}` +
  単一 `Failed` event に収束。`set_deadline_policy(None)` で disarm、
  `request_deadline` で driver が次回 tick を schedule 可能、
  `PluginEditorSessions::tick_all` で registry 一括評価。
- degrade — `open_or_degrade`（preferred `Unsupported` のみ 1 回
  fallback retry、成功は `OpenOutcome::Degraded` + `Degraded{mode}`
  event で `Attached`/`Recovered`/`Reattached` と排他）、
  `begin_degrade`（async 経路、`complete_attach` が `Degraded` を
  発火）。pending marker は `PendingAttach` enum
  （`None`/`Reattach`/`Degrade`）に統合 — 旧 `reattach_pending`/
  `degrade_pending` 2 bool は排他だったため enum が不変条件を正確に
  表す（clippy `struct_excessive_bools` 対応も兼ねる）。
- `SessionFailCause::GraphicsLost`・`EditorOpError::GraphicsLost` を
  additive 追加。`fail_cause` で `HostLost`/`Internal` と区別して写像、
  `notify_terminated(GraphicsLost)` も `Terminated{GraphicsLost}` を
  発火可能。
- `lib.rs` re-export・`README.md` 記述・test 17 件（SH-01..17、
  fake clock/`ModeOps` fixture）・design D5 capability matrix・
  evidence report。

### Decisions During Implementation

- **deadline arm は `next_request` で無条件**: arm 時点ではまだ
  request 前の state（`Closed`/`Attached`）であり `in_flight()` が
  false になるため、`in_flight` 判定を挟まず policy 存在時のみ arm
  する設計とした（全 caller が直後に in-flight state へ遷移する）。
  `set_deadline_policy` の再 arm のみ `in_flight` で gate。
- **`Closing` timeout は `mark_failed` 経路**: `complete_close` は
  error を運ばないため、timeout は `Failed{Timeout}` に落とす。
  `editor_live` は維持されるため adapter が後で close を完了しても
  `complete_close` は `StaleRequest` — session は `close` 再試行か
  `recover` で収束し二重終端しない。
- **sync convenience path は preempt しない**: `open`/`close`/
  `recover` 等は ops を inline 呼出しするため deadline は request
  mint 時に arm されるのみで、呼出し中の中断はしない。timeout が
  意味を持つのは `begin_*`/`complete_*` の async 経路 — doc に明記。
- **`open_or_degrade` の `preferred == fallback`**: retry 不能なため
  plain `open` に退化（`OpenOutcome::Attached` または通常失敗）。
- **`PendingAttach` enum への統合**: 2 bool（`reattach_pending`/
  `degrade_pending`）は API 経路上排他であり、clippy
  `struct_excessive_bools`（pedantic・`-D warnings` ゲート）が
  4 bool 目で警告したため、排他性を型で表す enum に統合した。
  `mark_failed`/成功着地で `None` に戻す挙動は旧実装と同一。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

canonical（mise）:

- `mise run fmt-check` → exit 0
- `mise run check`（`cargo check --workspace --all-targets`）→ exit 0
  — `ruviform-poc-native-dock` を含む全 member compile（API additive
  のみで非回帰）
- `mise run lint`（clippy `-D warnings`）→ exit 0
- `mise run test`（`cargo test --workspace`）→ exit 0

補助証拠（canonical = mise）:

- `cargo test -p ruviform-plugin-host --test editor_session` →
  45 passed / 0 failed（既存 28 + SH-01..17）

## Risks

- 実 driver が `tick`/`tick_all` を呼ばなければ timeout は発火しない
  — monitoring は driver 責務（design Deferred Design Concerns に記録、
  composition root 側の timer 接続は後続）。
- degrade retry は `ops.open` を 2 回呼ぶ — adapter が `Unsupported`
  を返す open で editor を生成していないという既存 refused-open
  前提に依存（doc 明記済み）。
- fake clock のみの検証 — 実 wall-clock・実 plugin/GPU/OS での
  timeout/degrade/graphics-lost 発生は NotRun（evidence report の
  分類表を参照）。

## Follow-ups

- GUI runtime の timer/cancel 基盤が `tick_all` を呼ぶ周期を所有
  （`ruviform-ui-gpui`/engine 側 — 本 slice 範囲外）。
- floating window 実 adapter・GPUI/DComp pixel compositing 修復は
  別 Issue。L1 wire の timeout/degrade 通知は issue-115 解除条件
  充足後。
- Issue #389 残 DoD: raw `GuiParentToken` の public API 除去、実
  VST3/CLAP/AU・L0/L1 実機受入、macOS/Linux・multi-plugin sweep。
