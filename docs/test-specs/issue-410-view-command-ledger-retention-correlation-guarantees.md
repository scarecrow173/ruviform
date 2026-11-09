---
id: "TSPEC-034UmKFzwIT1TdxeUWwDrp"
type: "test-spec"
status: "approved"
title: "Issue #410 — view command ledger retention & correlation guarantees"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/410"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-410-view-command-ledger-retention-correlation-guarantees.md"
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
  related:
    - "docs/specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  verified-by:
    - "docs/tasks/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #410 — view command ledger retention & correlation guarantees

## Purpose

`ViewHost`/`CommandBinding` のコマンド台帳が「保持量 bounded・
receipt 帰属一意・配送喪失が観測可能」という契約を実装し続ける
ことを保証する test spec。ledger の retention/correlation 設計を
将来変更する際に、失われてはならない性質を残す。

## Feature

View command ledger の bounded retention と correlation semantics
（Issue #410 view.rs/behavior.rs slice）。

## Rules

- Rule: `command_index` は `MAX_COMMAND_INDEX` を超えて保持
  しない。ただし owner binding に未 settle の receipt 期待
  record を持つ entry（LiveExpect）は consumer が応答し得るため
  保護し、全 entry が LiveExpect の場合は bound を超過するまま
  yield する（`MAX_PENDING_RECORDS` の先例と同じ誠実さ）。
- Rule: `command_index` の eviction は Dead（live record 無し）
  → Settled（receipt 記録済み）→ LiveNonExpect（未 settle・
  receipt 非期待）→ LiveExpect の優先度で行い、同 class 内は
  command_id 辞書順で決定的に選ぶ。
- Rule: `CommandBinding::outbox` は `MAX_OUTBOX_DISPATCHES`
  を超えて保持しない。超過分は最古から捨てる。権威ある配送経路は
  `ViewHost` の `ViewEvent::Intent` ストリームであり、outbox は
  bound 付きの dispatch 記録である。
- Rule: `push_event` が 512-cap で victim を捨てる場合、
  droppable（coalescible gesture / Preview intent）を優先し、
  無ければ最古を non-droppable でも捨てる。捨てられた `Intent`
  の `command_id` は `command_index` から除去され、対応する
  `PendingRecord` は配送喪失として expired 記録される
  （head が未 settle の receipt 期待 command なら phase は
  `Timeout`）。settled 済み record には影響しない。
- Rule: 同一 spec 内で、異なる component が同一の実効 command
  owner id（control の `control_id` fallback 込み、または
  action component の key 由来 id）を持つ場合 `InvalidSpec`
  で reject する。同一 node が tab alias で `flat` に 2 回現れる
  のは重複とみなさない。
- Rule: detach/unmount の gesture event 帰属は `control_id` に
  一致する mounted entry のうち canonical mount（key ==
  `spec.key()`）を優先して一意に決まる。
- Rule: 単調 `u64` カウンタ（`generation`/`mount_epoch`/
  `action_sequence`/`counter`/`observed_seq`/`sequence`/
  `atomic_counter`/`synthetic_seq`）の overflow は
  `checked_add` + panic で定義する（`AttachEpochAllocator::
  mint` 先例）。

## Examples

- Example: 600 件の Preview intent を発行 → `command_index` は
  512 を超えず、Dead class（未追跡 id）が優先的に evict される。
- Example: receipt 期待 command を bound 超過分 mint → 全て
  LiveExpect なら index は bound を超えて保持される（yield）。
- Example: `MAX_OUTBOX_DISPATCHES` 超の dispatch → `drain_outbox`
  は最新 64 件のみ返す。
- Example: event backlog を non-droppable で満たして `Intent` を
  押し出す → その command の receipt は `apply_receipt` が
  `false` を返し、binding phase は `Timeout` になる。
- Example: 2 つの Fader が同一 `control_id` を持つ spec →
  `mount` が `InvalidSpec` を返す。tab の direct child control
  （alias mount）は reject されない。

## Guarantees

- 長時間・大量 command 発行で `command_index`/`outbox` の保持量が
  bounded（in-flight receipt 期待を除く）であること。
- 配送されなかった `Intent` が pending のまま宙に浮かず、
  `Timeout` として観測可能で receipt routing と整合すること。
- `control_id` 衝突が spec 受理時に拒否され、detach/unmount の
  帰属が一意であること。
- カウンタ overflow の挙動が checked+panic として定義済みで
  あること（panic path 自体は到達不能のため実行 test は不要）。

## Non-goals

- projection envelope への epoch segment（wire-protocol、別
  slice）。
- `PendingRecord` dead-generation zombie の整理。
- `GpuiSurfaceAdapter` reentrant-borrow nit。
- consumer（synvelis/audio-lens）側の spec 変更。
- カウンタ overflow の実行時再現 test（u64 枯渇は到達不能。
  checked 化はコードレビューと mint 先例との整合で担保）。

## Risk

- eviction 優先度や drop 退役の契約が将来の変更で崩れると、
  receipt が静かに unrouted 化するか ledger が再び無制限化する。
  本 test spec の Rule がその回帰を検出する。
