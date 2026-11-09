---
id: "SPEC-034UmGsNtxrfkDz2KxieUt"
type: "spec"
status: "approved"
title: "Issue #410 — view command ledger の bounded retention と correlation semantics"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/410"
    - "https://github.com/scarecrow173/ruviform/pull/406"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
      - type: "file"
        path: "docs/designs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
      - type: "file"
        path: "docs/plans/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
      - type: "file"
        path: "docs/tasks/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
      - type: "file"
        path: "docs/test-specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
      - type: "file"
        path: "docs/impl/ir/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "https://github.com/scarecrow173/ruviform/issues/387"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-387-component-view-specs.md"
    - "docs/specs/issue-414-projection-controlled-display.md"
    - "docs/specs/viewlink-authoritative-generation.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/410"
  derived-by:
    - "docs/designs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
    - "docs/plans/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
    - "docs/test-specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #410 — view command ledger の bounded retention と correlation semantics

## What Should Be Built

`ViewHost` / `CommandBinding` が保持するコマンド台帳
（`command_index`・`CommandBinding::outbox`・`ViewHost` の event
backlog・`validate_spec` の component 検証・単調カウンタ）に、
retention の上限と correlation semantics の一貫した契約を与える。
具体的には以下を `crates/ruviform-ui` 内で実現する。

- `ViewHost::command_index` に settle/epoch 終了に整合した上限・
  解放条件を導入し、長時間・大量 command で保持量が bounded に
  なること。
- `CommandBinding::outbox` が消費・drain または上限管理され、
  未 drain でも無制限に増加しないこと。
- `ViewHost` の event backlog（`MAX_VIEW_EVENTS` = 512）が溢れた
  際の non-droppable イベント（特に `Intent`）の扱いを、
  「失われない」という現行コメントの虚偽を正す形で明示的な契約に
  すること。配送されなかった `Intent` に対応する `PendingRecord`
  との整合（timeout/失敗として観測可能）を定義すること。
- `validate_spec` で実効的な command owner id（control の
  `control_id`、および action component の key 由来 id）の重複を
  reject し、key fallback による暗黙衝突・receipt 帰属の曖昧さを
  塞ぐこと。
- `generation` / `mount_epoch` / `counter` 等の unchecked な
  `u64` wrap を、`AttachEpochAllocator::mint` の checked+panic
  先例に合わせて正直にすること（overflow 時の挙動を定義する）。

## Why Now

PR #406（Issue #387 系）の独立レビュー（RF387 follow-up）で、
view command ledger の retention policy と correlation semantics
に定義済みの gap が複数指摘された。現状は全て「今日のところ正し
い」挙動だが、以下のリーク・曖昧さが残る。

- `command_index` は `Rejected`/`Unsupported` receipt・
  unregister/detach/unmount でのみ evict され、`Applied`/
  `Accepted`/`Queued` および receipt 不達の entry が host 生存中
  蓄積し続ける（epoch あたり無制限）。
- `CommandBinding::outbox` は dispatch ごとに push されるが
  `ViewHost` 側に consumer がなく、host 生存中は無制限に溜まる。
- `push_event` の 512-cap は droppable が無い場合に `pop_front`
  で最古を捨てるため、「non-droppable records are never lost」
  というコメントと実動作が矛盾する。配送されない `Intent` に
  対応する `PendingRecord` は pending のまま残る。
- `apply_projection` は `control_id` のみで相関し、remount 後の
  fresh binding が古い revision の projection を一度だけ受理し得る
  （次の新しい projection で自己修復）。epoch segment を envelope
  に加える wire-protocol 判断は別スライス。
- `validate_spec` は component key のみ dedup し、`control_id`
  （未指定時は key fallback）は dedup 対象外。2 component が同一
  `control_id` を持つと `bindings`/`command_index`/detach 帰属が
  暗黙に衝突する。

## Users and Value

- Ruviform を利用する consumer（Synvelis、AudioLens、および
  `ruviform-ui-test`/`ruviform-ui-gpui` の検証基盤）の view
  session が、長時間稼働・大量 command 発行時にも ledger の
  メモリと receipt 帰属が破綻しないことを保証する。
- 将来の driver/renderer 実装者に対し、「配送・保持・失敗」の
  契約を暗黙でなく明示した境界を提供する。

## Scope

### In Scope

- `crates/ruviform-ui/src/view.rs` と
  `crates/ruviform-ui/src/behavior.rs` 内の ledger retention・
  correlation semantics hardening。
- 上記各項目に対応する回帰テスト（unit test）。
- 本 slice の spec/design/plan/task/test-spec/impl 文書チェーン。

### Out of Scope

- projection envelope への epoch segment 追加（wire-protocol
  判断。別スライスまたは契約 Issue）。
- `GpuiSurfaceAdapter` の reentrant-borrow nit（rf388-s3 領域）。
- consumer 側（synvelis/audio-lens）の変更。dedup 導入で consumer
  spec が reject される場合は Coordinator へ確認する。
- `PendingRecord` の dead-generation zombie（detach 後も records
  に残る stale generation entry）の整理 — 別 Issue 候補として
  記録に留める。

## Requirements

- `command_index` の保持量は、settle・epoch 終了・上限のいずれかに
  整合した条件で bounded でなければならない。in-flight かつ
  receipt 期待の command を consumer が応答し得る限り失わない
  こと（`MAX_PENDING_RECORDS` の yield 先例と同じ誠実さ）。
- `outbox` は drain 接続または上限管理により無制限増加しては
  ならない。drain されない場合の契約を明示する。
- event backlog 溢れ時に non-droppable `Intent` が失われる経路は
  存在し得るため、その場合の `PendingRecord` の観測可能な終端
  （timeout/失敗）と receipt routing の整合を定義し、テストで
  固定する。
- 同一 spec 内で実効 command owner id が重複する component を
  `InvalidSpec` で reject する。tab alias（同一 node が tab key
  と自身の key の両方で mount される経路）は同一 component と
  して扱い、誤って reject してはならない。
- 単調 `u64` カウンタの overflow 挙動を `AttachEpochAllocator::
mint` と同じ checked+panic で定義する。

## Acceptance Criteria

- [ ] `command_index` に上限・解放条件があり、長時間・大量
  command で保持量が bounded である回帰テストがある。
- [ ] `outbox` が上限管理（または drain 接続）され、未 drain でも
  無制限でないことを示すテストがある。
- [ ] 512 件超過時に配送されない `Intent` と `PendingRecord` の
  整合（timeout/失敗として観測可能・receipt routing との一貫性）
  を検証するテストがある。
- [ ] 重複 `control_id`（および key fallback 衝突）が
  `InvalidSpec` で reject され、mount/unmount・detach attribution
  が一意に解決されるテストがある。
- [ ] `generation`/`mount_epoch`/`counter` 系カウンタの overflow
  挙動が checked として定義されている。
- [ ] 本 slice の文書チェーン（spec/design/plan/task/test-spec/
  impl record）が揃っている。

## Deferred Design Concerns

- projection envelope への attach/mount epoch segment 追加:
  wire-protocol 変更を伴うため本 slice では扱わない。consumer
  契約の合意が前提。再着手条件: envelope 契約 Issue の起票・
  合意。無視した場合のリスク: remount またぎの古い projection
  が一度だけ誤適用される（現行どおり次の新しい projection で
  自己修復）。
- `PendingRecord` の dead-generation zombie 整理: detach で
  generation が進んだ後も旧 record が `records` に残り、
  in-flight 扱いで eviction を免れ続け得る。影響は per-binding
  64 件上限の内側に限定される。再着手条件: 実害の観測または
  ledger 見直し Issue。

## Sources and Relations

- Issue #410 本文（PR #406 独立レビュー RF387 の non-blocking
  hardening cluster 指摘を集約）。
- `docs/specs/issue-387-component-view-specs.md`（Component/View
  contract の親仕様）。
- `docs/specs/issue-414-projection-controlled-display.md`
  （projection → controlled display の先行 slice。本 Issue の
  領域を明示的に対象外としていた）。
- `docs/specs/viewlink-authoritative-generation.md`。
- consumer 側の control_id 一意性確認:
  `synvelis/crates/synvelis-app/src/screens.rs` は「Every control
  carries a distinct `control_id`」を契約注記として明記しており、
  本 dedup で reject されないことを merge 前に確認済み。
