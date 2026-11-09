---
id: "DESIGN-034UmHwuYp52pp4RtddpwM"
type: "design"
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
        path: "docs/designs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-387-component-view-specs.md"
    - "docs/specs/issue-414-projection-controlled-display.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  derived-by:
    - "docs/plans/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
    - "docs/test-specs/issue-410-view-command-ledger-retention-correlation-guarantees.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #410 — view command ledger bounded retention & correlation semantics

## Context

`ViewHost`（`crates/ruviform-ui/src/view.rs`）は spec 木・
`InteractionDispatcher`・control ごとの `CommandBinding`
（`crates/ruviform-ui/src/behavior.rs`）を所有し、gesture を
`CommandIntent` に翻訳して `ViewEvent::Intent` として配送する。
receipt は IPC envelope の `command_id` のみで相関し、
`command_index: HashMap<CommandId, ControlId>` が owner control
へ routing する。

現行の保持契約には以下の gap がある（Issue #410 / PR #406 独立
レビュー指摘）。

- `command_index` の evict は `Rejected`/`Unsupported` receipt・
  `unregister_control`/`release`・`detach`/`unmount` の clear のみ。
  `Applied`/`Accepted`/`Queued`・receipt 不達の entry は host
  生存中に蓄積する。
- `CommandBinding::outbox`（`Dispatched` キュー）は `dispatch`
  ごとに push されるが `drain_outbox` の consumer が存在せず、
  binding 生存中は無制限に溜まる。実際の intent 配送経路は
  `ViewHost::drain_events` → `ViewEvent::Intent` であり、
  outbox は並行する未配線の ledger である。
- `push_event`（`MAX_VIEW_EVENTS` = 512）は droppable
  （coalescible gesture / `is_droppable` intent = Preview）が
  無い場合に `pop_front` で最古を捨てる。「non-droppable records
  are never lost」のコメントはこの経路で偽になる。落ちた
  `Intent` の command は minted・index 登録・`PendingRecord`
  生成済みだが consumer へ配送されず、receipt が来ないため
  pending のまま残る（`ViewHost` は `ack_deadline` を常に `None`
  で dispatch し、`on_tick` も駆動しないため timeout も発火しない）。
- `validate_spec` は component key のみ dedup する。control の
  実効 `control_id`（明示値または key fallback）と action
  component（Button・選択可能 Table/Tabs）の key 由来 binding
  id は dedup 対象外で、`bindings`/`command_index`/detach 帰属が
  暗黙に衝突し得る。
- `generation`/`mount_epoch`/`action_sequence`/
  `CommandBinding::{counter,generation,observed_seq}`/
  `InteractionControl::{sequence,atomic_counter}`/
  `InteractionDispatcher::synthetic_seq` は unchecked `+= 1`
  で、`AttachEpochAllocator::mint` の checked+panic と誠実さが
  不整合（u64 枯渇は実質到達不能）。

## Scope

### In Scope

- `ViewHost::command_index` の bounded retention（上限 +
  settle/epoch に整合した eviction 優先度）。
- `CommandBinding::outbox` の上限管理と drain 契約の明示。
- `push_event` の non-droppable drop を正直な契約に修正し、
  落ちた `Intent` に対応する command を timeout として観測
  可能にする（`PendingRecord` との整合）。
- `validate_spec` での実効 command owner id 重複 reject と、
  detach 帰属（`key_for_control`）の一意化。
- 単調 `u64` カウンタの checked 化。
- `crates/ruviform-ui` 内の unit test と文書チェーン。

### Out of Scope

- projection envelope への epoch segment（wire-protocol 変更）。
- `GpuiSurfaceAdapter` reentrant-borrow nit。
- consumer 側（synvelis/audio-lens）の変更。
- `PendingRecord` dead-generation zombie の整理（記録のみ）。
- tab alias による `register_control` 二重呼び出しの統合
  （観測される影響は idle control への `noop` Gesture event のみで
  command は mint されない。別途観察事項として impl record に記録）。

## Components and Boundaries

- `ViewHost::command_index`: `CommandId → owner id` の receipt
  routing index。本設計で保持量を bounded にし、eviction 優先度を
  binding の `PendingRecord` 生存状態に整合させる。
- `CommandBinding`: per-control の command ledger
  （`records`/`outbox`/`head`）。`records` は既に
  `MAX_PENDING_RECORDS` = 64 で bounded（settled 優先 evict、
  全 in-flight なら yield）。本設計はこの先例に倣う。
- `ViewHost::push_event` / `events`: consumer が
  `drain_events` で受け取る配送・観測ストリーム。`Intent` は
  command の唯一の配送経路であり、loss は command の未配送を
  意味する。
- `validate_spec`: spec 受理境界。command owner id の一意性を
  ここで強制し、内部構造（`bindings`/`command_index`）が一意な
  owner を仮定できるようにする。

## Data and Control Flow

### command_index の bounded retention

`emit_intent` が `command_index.insert` するたびに、
`MAX_COMMAND_INDEX`（新設、512 — `MAX_VIEW_EVENTS` と同程度の
host-wide 上限）を超えた分を eviction する。victim は owner
binding の `PendingRecord` 生存状態で分類する。

| class | 条件 | 優先度 |
| --- | --- | --- |
| Dead | owner binding に live record が無い（Preview 等の未追跡 id、records から eviction 済み、binding 消失） | evict 最優先 |
| Settled | record があり receipt status が記録済み | evict |
| LiveNonExpect | record があり未 settle・`expect_receipt` = false | evict |
| LiveExpect | record があり未 settle・`expect_receipt` = true | 保護。全 entry がこれなら bound を超過するまま yield（`MAX_PENDING_RECORDS` と同じ誠実さ） |

同 class 内の victim は command_id の辞書順最小で決定的に選ぶ
（HashMap 走査順に依存しない）。eviction は insert 経路のみで
行い、receipt 適用時の eager evict は現行どおり
`Rejected`/`Unsupported` のみとする（`Applied` は rank 3 の
`Rejected`/`Unsupported` が後着し得るため terminal ではない）。

`CommandBinding` に `pub(crate)` の record 生存問い合わせを
追加する（`record_ledger_state(command_id) ->
Option<(expect_receipt, settled)>`）。

### outbox の上限管理

`CommandBinding::outbox` に `MAX_OUTBOX_DISPATCHES`（64、
`MAX_PENDING_RECORDS` と同値）を設け、push 時に超過分を
`pop_front`（最古）で捨てる。`Dispatched` には droppable 区分が
無いため drop-oldest のみが選択肢であり、「outbox は bound 付き
の dispatch 記録であり、権威ある配送経路は `ViewHost` の
`ViewEvent::Intent` ストリーム」という契約を doc コメントで
明示する。`drain_outbox` を定期的に呼ぶ consumer は従来どおり
全件を受け取れる。

### push_event の non-droppable drop

`push_event` の eviction コメントを実動作に合わせて修正する:
droppable（coalescible gesture / droppable intent）を優先的に
追い出し、それが無い場合は最古イベントを — non-droppable
であっても — 捨てる。

victim が `ViewEvent::Intent` の場合、その `command_id` の
command は consumer へ配送されない。整合のため `ViewHost` は
該当 command を「配送喪失」として退役させる:

- `command_index` から entry を除去（以後の receipt は
  unrouted として `false`）。
- owner binding の `PendingRecord` に expired を記録する新 API
  （`CommandBinding::expire_undelivered` 相当）で、head が
  未 settle の receipt 期待 command なら derived phase が
  `Timeout` になる。settled 済み（`Applied` 等 rank >= 2）の
  record には影響しない（`expired && rank <= 1` の条件で
  Timeout は出ない）。

victim が `Gesture`/`Op`/`Unsupported` の場合は観測ログの損失
として doc で明示する（gesture の状態機械は control 側が権威を
持ち、Op の喪失は renderer の focus handle 解放遅延など限定的
影響に留まる — 契約として記録）。

### validate_spec の command owner id dedup

各 node の実効 command owner id を計算する:

- `node.control()` が Some → `control_id.unwrap_or(key)`
  （`register_control` と同一規則）。
- それ以外で `action_binding_id(node)` が Some（Button、
  選択可能 Table/Tabs）→ `key.0`。
- どちらでもない node（Panel/Label/Waveform 等）は owner id を
  持たず検査対象外。

同一 spec 内で異なる component（`node.key()` が異なるもの）が
同一 owner id を持つ場合 `InvalidSpec` で reject する。同一
node が tab alias 経由で `flat` に 2 回現れる場合（tab key と
child key の両方で mount される経路）は `node.key()` が同一な
ので重複とみなさない。

併せて `key_for_control` を決定的にする: `control_id` に一致する
mounted entry が複数ある場合（tab alias の同一 spec 重複 mount）、
canonical mount（mounted key == `spec.key()`）を優先する。これに
より detach/unmount 時の Gesture event 帰属 key が一意に決まる。

### 単調カウンタの checked 化

`AttachEpochAllocator::mint` と同じ `checked_add(1).expect(...)`
パターンを適用する。対象: `ViewHost::{generation, mount_epoch,
action_sequence}`、`CommandBinding::{counter, generation,
observed_seq}`、`InteractionControl::{sequence,
atomic_counter}`、`InteractionDispatcher::synthetic_seq`。
panic message は対象カウンタと 2^64 枯渇を明記する。

## Data and Control Flow

変更後の流れ:

1. `emit_intent` → `binding.dispatch`（records/outbox は各 bound
   内）→ `command_index.insert` + `MAX_COMMAND_INDEX` eviction
   （owner binding の record 生存状態で class 分類）→
   `push_event(Intent)`。
2. `push_event` が 512-cap で victim を選択。`Intent` victim は
   `retire_undelivered` 相当で index 除去 + record expired
   （→ phase `Timeout`）。
3. `apply_receipt` は現行どおり index → owner binding へ routing。
   `Rejected`/`Unsupported` で index evict。その他 status は
   index に残り、次の bound 圧で settled class として回収される。
4. `validate_spec` は mount/update の受理時に owner id 一意性を
   検査し、重複は `InvalidSpec`。
5. detach/unmount の `detach_all` event 帰属は
   `key_for_control`（canonical 優先）で一意に決まる。

## Non-Functional Constraints

- RT 制約: `ViewHost`/`CommandBinding` は non-RT の UI 側契約。
  eviction は insert 経路の O(n log n)（n ≦ bound+1）で、
  user-driven dispatch の頻度では問題にならない。
- 公開 API 互換: `drain_outbox`/`apply_receipt`/`apply_projection`/
  `deliver_event` の signature は維持する。`command_index` の
  内部表現変更は外部に露出しない。`CommandBinding` への追加は
  `pub(crate)` または doc 付き `pub` の最小限に留める。
- consumer 影響: `validate_spec` の dedup 追加は spec 受理を
  厳しくする方向の変更。synvelis は「distinct `control_id`」を
  既に契約化しており reject されない（merge 前確認済み）。
  fixtures（driver-lab parity 等）は `control_id: null` で
  key fallback のため影響なし。

## Risks and Trade-offs

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | bound 超過時の victim 選択が in-flight receipt 期待 command を誤って evict し、receipt が unrouted になる | low | med | LiveExpect を最終 class とし、全 entry が LiveExpect なら yield（records と同じ誠実さ）。in-flight 保護を test で固定 |
| 2 | `Applied` 後着の `Rejected` が index evict で unrouted になる | low | low | evict は bound 圧時のみで settled 優先。`Applied`→`Rejected` は rank 上可能だが実害は phase 表示のみ。契約として文書化 |
| 3 | outbox drop-oldest で `drain_outbox` consumer が古い dispatch を取りこぼす | low | low | consumer 不在が現状。契約を doc 明示し、bound 内に収まる利用（定期 drain）を前提とする |
| 4 | `push_event` victim の `Intent` retire で、実は配送済み（drain 済み）だった command を timeout にする | low | low | victim は queue 内の未 drain 分のみ。drain 済み Intent は queue に存在しない。退役は「配送されなかった」場合に限る |
| 5 | `validate_spec` dedup が既存 spec/fixture を reject する | low | med | fixtures は `control_id: null`、synvelis は distinct 保証済み。tab alias は同一 node.key() で除外。test で alias 非 reject を固定 |
| 6 | checked counter の panic が到達不能パスで発火 | very low | low | u64 枯渇は実質到達不能。mint 先例と同じ判断 |

## Deferred Design Concerns

- projection envelope への attach/mount epoch segment:
  wire-protocol 変更のため本設計では扱わない（spec の
  Deferred Design Concerns と同じ）。
- `PendingRecord` dead-generation zombie: detach で世代が進んだ
  record が `records` に残る件は per-binding bound の内側であり、
  本 slice では `command_index` が detach で clear されるため
  host-level の保持量には影響しない。観察事項として記録。
- tab alias の `register_control` 二重呼び出し: idle control では
  `noop` Gesture event が余分に出るのみで command は mint されない。
  mount 経路の整理は別 slice 候補。

## Verification Notes

- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` /
  `mise run test` / `mise run test-ui-component-view` が exit 0。
- [ ] `command_index` の bounded retention・outbox bound・
  `push_event` drop 整合・`control_id` dedup・detached 帰属の
  各 unit test が `cargo test -p ruviform-ui` で pass
  （`mise run test` 経由）。
- [ ] synvelis 側 spec が distinct `control_id` を維持している
  ことを merge 前に再確認（本 PR では consumer 無変更）。
