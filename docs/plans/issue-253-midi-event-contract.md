---
id: "PLAN-034XCS9A8Ud1dQix6Eu3sX"
type: "plan"
status: "approved"
title: "Issue #253 MIDI event contract fixes — implementation plan"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-253-midi-event-contract.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-253-midi-event-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-253-unsupported-transparency-plan.md"
    - "docs/impl/ir/issue-188-midi-node-wiring-and-chase.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-253-midi-event-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-midi-event-contract.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 MIDI event contract fixes — implementation plan

## Goal

`merge_call_events` の defined truncation（V9）、`InputStage`
ledger の node 権威への一本化（V11）、anchor republish 後
pending event 契約の確定（V12）を headless で実装・検証する。

## Scope

単一 implementation stream。3 contract は同一ファイル群
（`event.rs`/`input.rs`/`node.rs`/`status.rs`/bridge tests）を
共有し、分割すると merge 競合と検証の分断を生むため分割しない。

## Upstream Documents

- `docs/specs/issue-253-midi-event-contract.md`（approved）

## Design Inputs

- `docs/designs/issue-253-midi-event-contract.md`（approved）

## File Map

- `crates/ruviform-plugin-core/src/event.rs` — `MergeOutcome`
  型追加、`merge_call_events` truncation 化、`MergeError::
  Overflow` 除去、unit test 更新・追加
- `crates/ruviform-audio-plugin-bridge/src/input.rs` —
  ledger 関連 field/method/counter 削除、`anchor_rejected`
  counter 追加、`collect_block` rejection 分類、`publish_anchor`
  契約 doc
- `crates/ruviform-audio-plugin-bridge/src/node.rs` —
  `merge_call_events` 新戻り値への追従、truncation counter
  反映
- `crates/ruviform-audio-plugin-bridge/src/status.rs` —
  `event_merge_truncated`・`input_anchor_rejected` mirror 追加、
  削除される input counter mirror 整理
- `crates/ruviform-audio-plugin-bridge/tests/midi_delivery.rs`
  — V9/V11/V12 の deterministic test、既存 test の契約追従
- `docs/test-specs/issue-253-midi-event-contract.md` — test
  仕様（approved 済み）
- `docs/impl/ir/`・`docs/impl/exp/`・`evidence/rf253/` —
  実装記録・実験記録・検証証跡

## Implementation Sequence

- [ ] T1: `event.rs` — `MergeOutcome` 追加、`merge_call_events`
  を defined truncation 化、`MergeError::Overflow` 除去、
  unit test を新契約へ更新 + truncation test 追加（red→green）
- [ ] T2: `status.rs` + `input.rs` — `Counter::
  EventMergeTruncated`・`event_merge_truncated`・
  `input_anchor_rejected` mirror 追加、`anchor_rejected`
  counter 追加、rejection 分類、stage ledger 削除
- [ ] T3: `node.rs` — `merge_call_events` 新戻り値への追従、
  truncation → overflow flag + counter 反映
- [ ] T4: bridge tests — V9 node 統合 test、V12 anchor
  republish/pending/rejection 分類 test、既存 stage-ledger
  依存 test の追従
- [ ] T5: doc チェーン完成（task/test-spec 既存分の status
  確定、impl/ir・impl/exp 生成・記入）、docs index 再生成
- [ ] T6: 検証 lane 実行（fmt-check/check/lint/test/test-rt/
  test-plugin-core/lint-plugin-core/test-l1-real-plugin）、
  evidence/rf253/ へ記録
- [ ] T7: `git fetch origin main && git merge origin/main`、
  index/lockfile 再生成、commit/push/PR 作成

## Dependencies and Risks

- `merge_call_events` は plugin-core の公開 API で戻り値型が
  変わる。呼出側は `node.rs` と plugin-core 内 test のみ
  （workspace 内で他 caller なし — grep 確認済み）。
- `MergeError::Overflow` 除去は非 additive。workspace 内の
  match は `node.rs` のみで追従する。
- stage ledger 削除で `InputStageCounters` の field が減る。
  `NodeStatusSnapshot` の対応 field 削除は additive でない
  観測面変更 — IR に明記。
- truncation で note-on 配送/off 喪失時の ledger 挙動は
  defined（次回 discontinuity で release）— test で固定。

## Task Breakdown

- T1–T4 は test-driven-development に従い red→green で進める。
- T5–T7 は doc-maintenance / doc-driven-dev-graph /
  review-protocol のフローに従う。

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check ruviform-plugin-core` /
  `mise run check ruviform-audio-plugin-bridge`
- [ ] `mise run lint ruviform-plugin-core` /
  `mise run lint ruviform-audio-plugin-bridge`
- [ ] `mise run test ruviform-plugin-core` /
  `mise run test ruviform-audio-plugin-bridge`
- [ ] `mise run test-plugin-core` / `mise run lint-plugin-core`
- [ ] `mise run test-rt`
- [ ] `mise run test-l1-real-plugin`
- [ ] `docs/test-specs/issue-253-midi-event-contract.md` の
  Rule V9/V11/V12 全てに対応する test が存在し Pass する

## Open Questions / Gaps

- `NodeReport::EventScratchOverflow` の payload 化は本 slice
  範囲外（control 面変更）。counter で観測性を確保する。
- `anchor_rejected` の分類境界は test で固定する
  （`UnmappedDomain`/`BaseMismatch`/`EpochMismatch`/epoch
  post-check → anchor_rejected、`BeforeAnchor`/`Overflow` →
  rejected のみ）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatched worker、
  subagent/helper worker 不使用）
- Independent work streams: none
- User approval required before dispatch: yes（review-protocol
  で代行 — spec/design/test-spec は approved 済み）
- If approved: execute task-by-task with review checkpoints.

## Review Handoff

- `MergeError::Overflow` 除去と `InputStage` ledger API 削除は
  非 additive 変更 — reviewer は公開 API 影響を確認すること。
- truncation の「先頭 N 件」定義が stable merge 順の先頭で
  あること、note-off priority が retained window 内で維持
  されることを確認すること。
- `anchor_rejected` の分類が `MapRejection` variant と
  post-check の対応表どおりか確認すること。
