---
id: "TSPEC-034Uwdiu2k3CqklLUCSLe0"
type: "test-spec"
status: "approved"
title: "Issue #410 P2 — view ledger retention & correlation floor guarantees"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/410"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-410-p2-view-ledger-correlation-hardening.md"
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
    - "docs/specs/issue-410-p2-view-ledger-correlation-hardening.md"
    - "docs/test-specs/issue-410-view-command-ledger-retention-correlation-guarantees.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-410-p2-view-ledger-correlation-hardening.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-410-p2-view-ledger-correlation-hardening.md"
  verified-by:
    - "docs/tasks/issue-410-p2-view-ledger-correlation-hardening.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #410 P2 — view ledger retention & correlation floor guarantees

## Purpose

`ViewHost`/`CommandBinding` のコマンド台帳が「保持量 bounded・
receipt 帰属一意・配送喪失が観測可能・projection が epoch 境界を
越えて誤適用されない」という契約を実装し続けることを保証する
test spec。slice 1（PR #502）の保証を継承しつつ、本 slice で
追加する projection revision floor の性質を固定する。

## Feature

View command ledger の bounded retention と projection correlation
floor（Issue #410 P2 slice）。

## Rules

### slice 1 から継承する保証（既存 test で固定済み）

- Rule: `command_index` は `MAX_COMMAND_INDEX` = 512 を超えて
  保持しない。Dead → Settled → LiveNonExpect → LiveExpect の
  liveness-class 優先度で eviction し、全 entry が LiveExpect
  なら bound を超過するまま yield する。
  - 固定 test: `command_index_bounded_dead_entries_evict_first`、
    `command_index_protects_in_flight_and_evicts_deterministically`。
- Rule: `CommandBinding::outbox` は `MAX_OUTBOX_DISPATCHES` = 64
  を超えず、未 drain 時は最古から drop する。
  - 固定 test: `outbox_stays_bounded_when_undrained`（behavior.rs）。
- Rule: `push_event` が `Intent` を drop した場合、その
  `command_id` は `command_index` から除去され、対応 record は
  delivery-lost（`Timeout`）になる。settled record には影響せず、
  late receipt は unrouted。
  - 固定 test: `event_backlog_drop_retires_undelivered_intent`、
    `event_backlog_drop_leaves_settled_command_alone`。
- Rule: 同一 spec 内で異なる component が同一の実効 command
  owner id を持つ場合 `InvalidSpec`。tab alias の同一 node は
  重複とみなさず、detach 帰属は canonical mount を優先する。
  - 固定 test: `duplicate_command_owner_id_rejected`、
    `tab_alias_shares_owner_and_attributes_canonically`、
    `alias_invocation_uses_canonical_command_owner`。
- Rule: 単調 `u64` カウンタの overflow は checked+panic。
  （到達不能のため実行 test 不要 — コードレビューで担保。）

### 本 slice で追加する保証

- Rule: `ViewHost::apply_projection`（`deliver_event` の `Notify`
  経路を含む canonical entry point）は、`control_id` ごとの
  受理済み revision floor を attach epoch の存続期間中保持し、
  floor を advance しない projection を `false` で drop する。
- Rule: floor は `unmount`/`detach`/`update` のいずれでも clear
  されず、`unmount` → `mount`（binding 再生成・`last_observed`
  リセット）を跨いで維持される。これにより remount 前の古い
  revision が fresh binding に誤適用されない。
- Rule: `detach` → `attach` では binding が存続するため
  `last_observed` guard が効き、detached 中の projection は
  `attached=false` で drop、reattach 後は revision guard で
  古い projection を drop する。
- Rule: floor は受理された projection の revision にのみ更新
  され、棄却された projection で巻き戻らない。
- Rule: floor は `control_id` ごとに独立し、ある control の
  projection 受理が別 control の floor に影響しない。
- Rule: revision advance 規則は `CommandBinding::on_projection`
  と `ViewHost` の floor で同一（`Num` は厳密増加、`Str` は
  相違、型跨ぎは常に advance）。

## Examples

- Example: rev 10 を受理 → `unmount` → `mount` → rev 5 の
  projection は `apply_projection` が `false` を返し display/
  observed に触れない。rev 11 は受理される。
- Example: rev 5 受理 → `detach` → detached 中に rev 6 は drop
  → `attach` → rev 4 は revision guard で drop、rev 7 は受理。
- Example: `deliver_event` に旧 revision の `Notify` を
  remount 後に配送 → `false`（unrouted と同じ誠実な棄却）。
- Example: 棄却された rev 5 の後に rev 8 を送ると受理され、
  floor は 8 に進む（棄却で floor が 5 に戻らない）。

## Guarantees

- mount/attach epoch を跨ぐ古い projection が新 binding に
  誤適用されないこと（floor が remount 前の高水位を保持）。
- 順序逆転した projection（同一 binding 内・detach/attach またぎ）
  が revision guard で drop されること。
- slice 1 の bounded retention・outbox 上限・delivery-loss 退役・
  owner dedup の保証が回帰しないこと。

## Non-goals

- projection envelope への epoch segment（wire-protocol、別
  slice/契約 Issue）。
- host 間誤配送・revision 単調性自体の違反の検証（delivery
  path の attach_epoch pin 義務 — 契約として文書化）。
- `binding_mut()` 経由の `on_projection` 直接呼出し（caller が
  correlation を所有する上級 API）。
- `PendingRecord` dead-generation zombie、reentrant-borrow nit、
  consumer 側の変更。

## Risk

- floor が将来の変更で clear される・更新規則が崩れると、
  remount またぎの stale projection 誤適用が再発する。本
  test-spec の Rule がその回帰を検出する。
- consumer が revision を attach epoch 内でリセットした場合、
  floor が全 projection を reject し display が凍結する
  （binding 内 guard と同じ失敗型。契約として明記済み）。
