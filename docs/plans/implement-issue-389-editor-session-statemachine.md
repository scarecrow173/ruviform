---
id: "PLAN-HDjvE9BDnWUjWa0unFqU0g"
type: "plan"
status: "approved"
title: "Issue #389 Plugin Editor Session fake host driver 状態機械 fixture slice 実装計画"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-389-editor-session-statemachine.md"
      - type: "file"
        path: "docs/test-specs/issue-389-editor-session-statemachine.md"
      - type: "file"
        path: "docs/impl/ir/issue-389-editor-session-statemachine-implementation.md"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/editor_session_driver.rs"
    modified:
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
    - "docs/specs/issue-389-plugin-editor-session.md"
  implemented-by:
    - "docs/tasks/issue-389-editor-session-statemachine.md"
  depends-on:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-session-hardening.md"
    - "docs/test-specs/issue-389-editor-session-headless.md"
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
    - "docs/test-specs/issue-389-editor-session-statemachine.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 389
---

# Issue #389 Plugin Editor Session fake host driver 状態機械 fixture slice 実装計画

## Goal

Issue #389 受入条件の「fake host driver で Open/Attach/Reparent/
Detach/Close・stale generation・Double close・Timeout・OS
unsupported・Plugin/GPU Host crash の terminal-event 検証」と
「失敗時 Floating/Generic Editor への明示的 degrade の契約」を、
**driver 側の async protocol（`begin_*`/`complete_*`/`tick`/
`tick_all`）を実際に駆動する再利用可能な fake host driver
fixture** として固定する。既存 49 test は session/ops を test 毎に
手動で駆動しており、driver が担うべき「dispatch → 後で
complete → deadline 監視」のループを再現する fixture がない —
本 slice はその fixture を追加し、状態機械の terminal-event
不変条件（duplicate/missing なし）を driver 視点で検証する。

contract 本体（`src/editor_session.rs`）は main 上で実装済み
（PR #399/#427/#503・headless fixture #530）であり、本 slice は
**src の契約変更を行わない** — `tests/` への fixture 追加と
`docs/` チェーンのみ。

## Scope

- `crates/ruviform-plugin-host/tests/editor_session_driver.rs` 新設
  — `FakeHostDriver`（`PluginEditorSessions` + `ScriptedOps` +
  共有 `FakeClock` + pending completion queue + driver 側
  `editor_live` 帳簿）と SM-xx scenario test。
- `docs/` チェーン（本 plan・task・test-spec・impl record）と
  `docs/{plans,tasks,test-specs,impl/ir}/README.md` index。
- 触らない: `src/` 全て・`ruviform-plugin-core`・`ruviform-ui*`・
  `ruviform-poc-native-dock`・`.github/`・`Cargo.toml`/
  `Cargo.lock`/`mise.toml`（依存追加なし）・既存
  `tests/editor_session.rs`（新規 test file のみ追加）。

## Upstream Documents

- `docs/specs/issue-389-plugin-editor-session.md`（approved・親 spec）
- `docs/designs/issue-389-plugin-editor-session.md`（approved・改訂 1
  R3 状態遷移表/R4 所有責務/R6 DoD）
- `docs/designs/issue-389-session-hardening.md`（approved・D1–D5・
  async driver contract の Data flow 節）
- `docs/specs/issue-389-session-hardening.md`（approved）
- `docs/test-specs/issue-389-editor-session-headless.md`（EH-01..05）
- `docs/test-specs/issue-389-session-hardening.md`（SH-01..18）

## Design Inputs

contract の正本は既存 approved design 群。本 slice が fixture で
pin するのは design D3（terminal event 一度切り・request 世代）・
hardening D1–D3（deadline/tick・明示 degrade）・D7（RT・DSP 独立）
の driver 側規則のみ。新規契約・API 変更は追加しない。

fixture の driver モデル（hardening design Data flow の
`ops.* を非同期 dispatch → complete_*(request, result)`・
`定期的に tick/tick_all` を忠実に再現）:

- verb（`open`/`recover`/`degrade`/`reparent`/`close`）は
  `begin_*` で request を mint し、op を pending queue に積む。
- `pump()` が ready な pending を FIFO で実行: `ops.*` 呼出 →
  対応する `complete_*` 配送。`hung` に登録した op kind は
  配送されず残り、deadline 超過を `tick`/`tick_all` が
  `Failed{Timeout}` へ収束させる。後の配送は `StaleRequest`。
- `reparent` は 2 相: platform detach（scripted result）→
  `complete_detach` → Ok なら新 request で `ops.reattach` →
  `complete_attach`（`Detached`/`Reattached` event 語彙）。
- `close` は driver 側 `live_editors` 帳簿で adapter 呼出を制御
  する。帳簿は adapter 側の真実を追う（`ops.open` `Ok` で真・
  `ops.close` `Ok` 配送/termination で偽）— `complete_attach`
  が stale を返した成功 open でも adapter editor は存在し、
  orphan として `close` の dispatch 対象に残る。
  async `complete_close` は error を運ばないため、ops.close 拒否は
  配送せず deadline timeout → `Failed{Timeout}` → 再 `close` の
  収束経路を fixture で固定する。
- session id が registry から消えた（`end_session` 後の generation
  recycle 含む）pending 配送は `stale_drops`、request 不一致は
  `stale_rejections` として記録し event/state を変えない。

## File Map

- `crates/ruviform-plugin-host/tests/editor_session_driver.rs` —
  `FakeClock`/`ScriptedOps`/`FakeHostDriver` fixture + SM-xx test。
- `docs/test-specs/issue-389-editor-session-statemachine.md` —
  SM-Rx rule + SM-xx case 表。
- `docs/tasks/issue-389-editor-session-statemachine.md` — 実施 task。
- `docs/impl/ir/issue-389-editor-session-statemachine-implementation.md`
  — 実装記録（実装後）。

## Implementation Sequence

- [x] T-1 `tests/editor_session_driver.rs` — fixture + SM-xx 追加。
- [x] T-2 `docs/test-specs/`・`docs/tasks/` 作成。
- [ ] T-3 impl record・NotRun 記録・`mise run` 検証 + review-protocol
  + PR。

## Dependencies and Risks

- 依存: 既存 contract 層（全て main merge 済み）。新規外部依存・
  Cargo.toml/lock 変更なし。
- リスク: fixture が実 driver の責務を越えて session 内部状態を
  推測しないこと — driver は public API（`begin_*`/`complete_*`/
  `tick*`/`session*`/`notify_terminated`）のみを使い、
  `editor_live` 相当は driver 側帳簿として明示的に持つ。
- リスク: fixture green を実 Native Dock/実 plugin PASS と誤読
  → test-spec/IR/PR で NotRun・非 PASS を明記。
- リスク: `tests/` は crate 毎に独立 compile のため
  `editor_session.rs` の helper（`FakeOps`/`FakeClock`/`lease`/
  `instance`/`SIZE`）は再利用できない → 新 file 内に最小限の
  同等 helper を置く（重複は test crate 独立性の範囲内）。

## Task Breakdown

- `docs/tasks/issue-389-editor-session-statemachine.md` に T-1〜T-3
  を記録。単一 worker が逐次実施（本 task は Orca dispatch 1 件）。

## Verification

- [x] `mise run fmt-check` exit 0
- [x] `mise run check` exit 0
- [x] `mise run lint` exit 0
- [x] `mise run test` exit 0（workspace 全 suite 非回帰）
- [x] `mise run test-rt` exit 0（`rt_*` filter — zero match でも
  exit 0）
- [x] `cargo test -p ruviform-plugin-host --test editor_session_driver`
  で SM-xx 全 case green（補助証拠・canonical は mise test）
- [x] `git diff` で `src/`・`ruviform-plugin-core`・`ruviform-ui*`・
  workspace `Cargo.toml`/`Cargo.lock`/`mise.toml`・
  `tests/editor_session.rs` に差分がないこと

## Open Questions / Gaps

- focus は session port の動詞ではない（EH-R5 で確定済み）—
  fake driver fixture も focus verb を追加しない。dock focus の
  session 側意味論は `Attached ↔ Hidden`（`set_visibility`）。
- 「Generic Editor」は host 側 fallback UI の語彙であり session
  contract の `AttachMode` には存在しない — 本 slice が固定するのは
  Floating への明示 degrade（`Degraded` event・`OpenOutcome` 相当の
  driver 記録）まで。Generic Editor UI 自体は consumer/#388 側の
  別 slice。
- 実 plugin/実 OS/実 GUI・consumer gate・実機共同検証（#388 gate・
  Synvelis #176）は本 slice 対象外 — NotRun/Unsupported として記録。
- async `complete_close` に error channel がないため、adapter の
  close 拒否は deadline timeout 経由でしか `Failed` に落ちない
  — これは現行 contract の定義済み挙動であり、本 slice はその
  収束経路を fixture で固定する（contract 変更はしない）。

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
  おり新規契約を捏造していないか、driver が public API のみを
  使い session 内部を覗いていないか、terminal-event 検証が
  duplicate/missing を実際に assert しているか、NotRun/Unsupported
  が PASS に偽装されていないか、src 差分がゼロであること。
