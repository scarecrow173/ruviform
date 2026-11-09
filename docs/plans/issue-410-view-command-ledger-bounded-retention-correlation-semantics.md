---
id: "PLAN-034UmJJcFMIKvjd1orEeeI"
type: "plan"
status: "approved"
title: "Issue #410 — view command ledger bounded retention & correlation semantics"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/410"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
    - "docs/designs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  derived-by:
    - "docs/tasks/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
    - "docs/test-specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  verifies:
    - "docs/test-specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #410 — view command ledger bounded retention & correlation semantics Implementation Plan

## Goal

`ViewHost`/`CommandBinding` のコマンド台帳に bounded retention と
明示的な correlation semantics を与え、長時間・大量 command・
event backlog 溢れ・`control_id` 衝突の各経路で保持量と receipt
帰属が破綻しないようにする。

## Scope

単一の実装ストリーム: `crates/ruviform-ui` の `view.rs` /
`behavior.rs` 内の ledger hardening と対応 unit test。
`ruviform-ui-gpui`/`ruviform-ui-test`/consumer は変更しない
（公開 API signature は維持）。

## Upstream Documents

- `docs/specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md`
- `docs/designs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md`（approved）
- `docs/specs/issue-387-component-view-specs.md`（親 contract）

## Design Inputs

- `docs/designs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md`

## File Map

- `crates/ruviform-ui/src/behavior.rs` — `CommandBinding` に
  record 生存問い合わせ（`pub(crate)`）と配送喪失記録 API を追加、
  `outbox` に `MAX_OUTBOX_DISPATCHES` bound、単調カウンタの
  checked 化、対応 unit test。
- `crates/ruviform-ui/src/view.rs` — `command_index` に
  `MAX_COMMAND_INDEX` bound + liveness-class eviction、
  `push_event` の drop 契約修正と `Intent` victim の退役処理、
  `validate_spec` の command owner id dedup、`key_for_control`
  の canonical 優先、単調カウンタの checked 化、対応 unit test。
- `docs/test-specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md`
  — 保証すべき性質の正本。

## Implementation Sequence

- [ ] `behavior.rs`: `CommandBinding::record_ledger_state`
  （`pub(crate)`）と `expire_command`（配送喪失 → `expired` 記録）
  を追加し、`outbox` push に `MAX_OUTBOX_DISPATCHES` bound を適用。
- [ ] `behavior.rs`: `counter`/`generation`/`observed_seq`/
  `sequence`/`atomic_counter`/`synthetic_seq` を checked_add 化。
- [ ] `view.rs`: `command_index` に `MAX_COMMAND_INDEX` bound と
  liveness-class eviction（Dead → Settled → LiveNonExpect →
  LiveExpect、全 LiveExpect なら yield、同 class は command_id
  辞書順）を実装。
- [ ] `view.rs`: `push_event` のコメントを実動作に合わせて修正し、
  victim が `Intent` の場合に `command_index` 除去 +
  `expire_command` で退役させる。
- [ ] `view.rs`: `validate_spec` に command owner id dedup を追加
  （tab alias は同一 `node.key()` で除外）。
- [ ] `view.rs`: `key_for_control` を canonical mount 優先にし、
  `generation`/`mount_epoch`/`action_sequence` を checked_add 化。
- [ ] unit test 追加: bounded retention（index/outbox）、
  in-flight 保護 yield、`push_event` drop → Timeout + unrouted
  receipt、dup `control_id` reject・alias 非 reject・detach 帰属。
- [ ] `mise run fmt-check` → `check` → `lint` → `test` →
  `test-ui-component-view` で検証。

## Dependencies and Risks

- 依存: なし（blocked by 無し）。consumer（synvelis）は distinct
  `control_id` を既に契約化しており dedup で reject されない
  （merge 前に再確認）。
- リスク: bound 圧での victim 選択が in-flight command を誤 evict
  → LiveExpect 最終 class + yield で防止（test 固定）。
- `Applied` 後着 `Rejected` の unrouted 化は settled 優先 evict
  の意図的な trade-off として文書化。

## Task Breakdown

- 単一 task: `docs/tasks/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md`
  （5 つの実装項目 + test は一つの論理変更として不可分 — 同一
  ファイル・同一 contract の hardening であり分割すると中間状態が
  不整合になる）。

## Verification

- [ ] `mise run fmt-check` exit 0
- [ ] `mise run check` exit 0
- [ ] `mise run lint` exit 0
- [ ] `mise run test` exit 0（`ruviform-ui` unit test を含む）
- [ ] `mise run test-ui-component-view` exit 0
- [ ] unit test で `command_index`/`outbox` の bound・
  in-flight yield・`Intent` drop → `Timeout`・dup reject・
  detach 帰属を固定

## Open Questions / Gaps

- `MAX_COMMAND_INDEX` = 512・`MAX_OUTBOX_DISPATCHES` = 64 の
  具体値は `MAX_VIEW_EVENTS`/`MAX_PENDING_RECORDS` の先例に揃えた
  判断。運用上の根拠があれば調整可能（public const ではなく
  private const のため外部契約ではない）。
- dead-generation `PendingRecord` zombie は本 slice 対象外
  （spec Deferred Design Concerns 参照）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 worktree の Worker が
  実施。独立 stream 無し）。
- Independent work streams: none
- User approval required before dispatch: no（Worker dispatch 内で
  実行）

## Review Handoff

- reviewer focus: `push_event` victim 退役の網羅性（droppable
  経路の `Intent` も退役対象に含めるか）、`command_index`
  eviction class の境界条件（binding 消失・Preview 未追跡）、
  `validate_spec` dedup が tab alias を誤 reject しないこと、
  checked 化の適用漏れ。
