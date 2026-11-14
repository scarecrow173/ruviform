---
id: "DESIGN-034XCQnVGvNjCrIO3C5trY"
type: "design"
status: "approved"
title: "Issue #253 MIDI event contract fixes — merge truncation / ledger 一本化 / anchor republish"
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
    - "docs/specs/issue-253-midi-event-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-188-timed-midi-detailed.md"
    - "docs/designs/issue-253-unsupported-transparency-design.md"
    - "docs/impl/ir/issue-188-midi-node-wiring-and-chase.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-253-midi-event-contract.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 MIDI event contract fixes — merge truncation / ledger 一本化 / anchor republish

## Context

Issue #188 の node wiring 実装で確立した MIDI event 経路は、
`InputStage`（非 RT input 準備・stamp→frame 写像・pending
queue）→ `PluginNode::run_call`（merge・note-off priority・
`EventList` 構築・ledger feed・plugin 配送）という分離を持つ。
`docs/impl/ir/issue-188-midi-node-wiring-and-chase.md` の
review follow-up が 3 件の contract gap を残した:

- **V9**: `merge_call_events` は merge 結果が scratch 収容を
  超えると `Err(MergeError::Overflow)` を返し、`run_call` は
  call 全体を空 `EventList` で配送する（全損失）。
- **V11**: `InputStage` が独自 `NoteLedger`+`release_all`+
  `active_notes` を保持し、node ledger（merge 後 delivered
  event を feed する権威）と二重管理。本番経路では stage 側
  release API は未使用で、乖離し得る。
- **V12**: `SetClockAnchor → publish_anchor` 後に `pending`
  に残る event の取扱いが未定義。現状は次回 `collect_block`
  で現行 anchor により写像され、写像不能 stamp は
  `MapRejection` として計数・消費される（lazy rejection）。
  epoch/domain 不一致による rejection が `rejected` 合計に
  埋もれ、anchor 起因かどうか識別できない。

## Scope

### In Scope

- `merge_call_events` の戻り値を defined truncation を運ぶ
  型へ変更し、`PluginNode::run_call` の call 合成を追従。
- truncation 量の `Counter`/`NodeStatusSnapshot` 追加。
- `InputStage` から ledger 関連 API/counter を削除し、
  node ledger 一本化を完了。
- `publish_anchor` 契約の doc 化と anchor 起因 rejection の
  `anchor_rejected` counter 追加。
- 上記 3 契約の deterministic test と doc チェーン。

### Out of Scope

- `NodeReport::EventScratchOverflow` の payload 変更
  （control 面・`DropReason` 写像を伴う契約変更。counter で
  観測性を確保する現行分担を維持）。
- `MapRejection` 全 variant の細分化 counter。
- `InputStage` 単独利用向けの release API 再導入。
- V13（CC/param chase）・実 device/plugin・macOS/Linux
  backend・`crates/ruviform-audio-test/`。

## Components and Boundaries

- **`ruviform-plugin-core::event`**: merge の唯一の実装。
  `merge_call_events(sources, scratch, src_scratch)` は
  K-way stable merge（最小 `offset_in_block` 優先、同 offset は
  source index の低い順）を caller 提供 scratch 上で行う。
  本設計で「収容可能な先頭 N 件」を返す defined truncation
  を追加する。domain policy（何を truncate するか）はここが
  所有する。
- **`ruviform-audio-plugin-bridge::input::InputStage`**:
  非 RT input 準備層。`push`/`drain_dropped` で pending に
  `TimedMidiInput` を積み、`collect_block` で現行
  `ClockAnchor` により stamp→frame を写像して `MappedInput`
  を `scratch` へ書き出す。pending queue 自体は bounded
  （`MAX_PENDING_INPUTS`）で溢れは計数 drop。本 slice で
  ledger を完全に除去し、note state を一切持たない
  「stamp→frame 写像 + bounded queue」の層に純化する。
- **`ruviform-plugin-core` / bridge `node::PluginNode`**:
  call 合成の権威。`run_call` が `[evict, release, chase,
  plan, live]` を `merge_call_events` で merge し、note-off
  priority 適用済み `EventList` を構築して plugin へ配送し、
  同じ event 列を node `NoteLedger` に feed する。V9 の
  truncation 計数と V11 の ledger 権威はここに集約される。
- **`bridge::status::NodeStatus`**: `InputStageCounters` の
  atomic mirror + node 側 counter。`set_input` 経由で
  per-block counter を写し、snapshot で観測面を提供する。
  本 slice で `event_merge_truncated`・`input_anchor_rejected`
  を additive に追加し、削除される input counter mirror を
  整理する。

## Data and Control Flow

### V9 — merge defined truncation

現行:

```text
merge_call_events(sources, scratch, src_scratch)
  needed = Σ sources[i].len()
  if scratch.len() < needed || src_scratch.len() < needed
      → Err(Overflow)            # run_call は空 EventList を配送
  merge 全 needed 件 → scratch[..needed] / src_scratch[..needed]
  apply_note_off_priority
  → Ok(&mut scratch[..needed])
```

変更後:

```text
capacity = min(scratch.len(), src_scratch.len())
emitted  = min(needed, capacity)   # K-way merge の先頭 emitted 件のみ実行
merge → scratch[..emitted] / src_scratch[..emitted]
apply_note_off_priority(out[..emitted], src[..emitted])
→ Ok(MergeOutcome { events: &mut scratch[..emitted],
                    truncated: needed - emitted })
```

- merge loop は「各 source の head が最小 offset のものを
  1 件取り出す」を `emitted` 回だけ回す。打ち切り後の
  source 残りは読まない（bounded: emitted ≤ capacity ≤
  caller scratch）。
- `truncated = needed - emitted` は呼出側がそのまま counter
  に載せられる値として返す。silent drop は存在しない。
- note-off priority は truncate 後の retained window 内でのみ
  適用される。cross-source の same-offset off 優先規則は
  retained event 間で維持される。truncated 側に押し出された
  off は計数対象（truncated）に含まれる。
- `MergeError::Overflow` は到達不能になるため variant を
  除去する（`#[non_exhaustive]` enum の variant 削除は公開
  API の非 additive 変更として IR に明記）。`TooManySources`
  は従来どおり。
- 戻り値型は新設 `MergeOutcome<'a> { events: &'a mut [Event],
  truncated: usize }`。`events` は `scratch` の先頭
  `emitted` 要素への mutable slice、`src_scratch[..emitted]`
  が対応する provenance。

`run_call` 側:

```text
match merge_call_events(...) {
  Ok(outcome) => {
      events = EventList 化(outcome.events)
      if outcome.truncated > 0 {
          overflow = true;                       # 既存 incident 意味維持
          status.event_merge_truncated += truncated;
      }
  }
  Err(MergeError::TooManySources{..}) => { /* 従来どおり overflow 経路 */ }
}
```

- 既存 `EventScratchOverflow` report/counter は「event window
  が scratch に収まらなかった」incident として継続発火する
  （truncation 発生時も含む）。report は payload を持たない
  ため、件数は `event_merge_truncated` counter が担う。
- ledger feed は truncated 後の delivered event 列に対して
  行われるため、truncated で届かなかった note-on は ledger
  にも載らず、stuck-note を生じない。逆に note-on が届き
  対応 off が truncated された場合は ledger が active と
  認識し、次の discontinuity（seek/stop）で `release_all`
  により defined release される — これは「delivered event
  のみを権威とする」ledger 契約と整合する。

### V11 — ledger 一本化

`InputStage` から以下を削除する:

- field: `ledger: NoteLedger`、`release_scratch`（`release_all`
  用 scratch）。
- method: `release_all(&mut self, out: &mut [Event]) -> usize`、
  `active_notes(&self) -> usize`。
- `InputStageCounters` field: `released`、`ledger_overflows`。
- `collect_block` 内の `ledger.note_on/note_off/drain_evicted`
  feed と `ledger_overflows`/`released` counter 更新。
- `Debug` impl の ledger 表示、`NoteLedger` import。

権威は `PluginNode` の `ledger`（`run_call` で merge 後
`EventList` を feed 済み）に一本化される。`prepare_
discontinuity` の `ledger.release_all` + `chase_span_events`
は既に node ledger を使っており変更不要。seek/stop での
stuck-note 防止は node 権威で非退行（既存 test
`stop_releases_live_notes_too`・seek chase test がそのまま
カバーする）。

`status.rs` から `input_released`/`input_ledger_overflows`
相当の atomic mirror・`set_input` arm・snapshot field を削除
する（`InputStageCounters` field 削除に追従）。

公開 API 削除（`InputStage::release_all`/`active_notes`、
`InputStageCounters::{released,ledger_overflows}`、
`NodeStatusSnapshot` 対応 field）は非 additive 変更として
IR に明記する。workspace 内で stage ledger API を呼ぶのは
test のみで、本番経路は存在しない。

### V12 — anchor republish pending event 契約 = 保全

契約（spec R4）: `publish_anchor` は `pending` を一切
消費・並べ替え・drain しない。`pending` は absolute
source-clock の claim 列であり、写像権威は常に
`collect_block` 時点の現行 anchor のみ。

- anchor が `None` → `collect_block` は mapped を空で返し
  pending は待機（計数なし — rejection ではない）。
- anchor republish（domain/base/epoch 変更を含む）→ pending
  は保持され、次回 `collect_block` で**新しい** anchor に
  より写像される。同一 domain の再校正（base stamp 変更）
  では pending が新 anchor で再写像される — これが
  「現行 anchor が唯一の権威」の意味。
- 写像不能 stamp は `map_stamp_to_frame` の
  `MapRejection` または post-check（`!mapped.frame.
  same_epoch(block_start)`）で per-event defined rejection
  となり、`pending` から除去・計数される。

rejection 分類（`collect_block` 内、1 event ごと）:

| 条件 | `rejected` | `anchor_rejected` |
| --- | --- | --- |
| `MapRejection::{UnmappedDomain, BaseMismatch, EpochMismatch}` | +1 | +1 |
| `MapRejection::{BeforeAnchor, Overflow}` | +1 | ±0 |
| map 成功後 `!mapped.frame.same_epoch(block_start)` | +1 | +1 |

- `rejected` は従来どおり全 rejection の合計（意味不変）。
- `anchor_rejected` は「anchor の domain/base/epoch
  coverage に起因する rejection」の subset で、新規
  `InputStageCounters::anchor_rejected` field として追加。
  `anchor_rejected ≤ rejected` が常に成立する。
- anchor が一時的に不適合でも pending が失われないため、
  republish で新 anchor が来れば pending は救済される。
  これは「保全」選択の核心: 不適合期間中に pending を
  drain しない。

`NodeStatus` mirror: `input_anchor_rejected` atomic +
`snapshot().input_anchor_rejected` field を additive 追加。

## Non-Functional Constraints

- RT no-alloc/no-block: merge truncation は既存 K-way loop
  の打ち切りのみで allocation・I/O を追加しない。
  `collect_block` の分類は既存 match arm への counter 分岐
  追加のみ。`mise run test-rt` 非退行。
- bounded: merge の emitted ≤ capacity、pending queue は
  既存 `MAX_PENDING_INPUTS` で bounded。anchor republish
  で pending が増えることはない（republish は push しない）。
- deterministic: truncation は merge 順序の先頭 N 件で
  決定的。rejection 分類は event ごとに決定的。

## Risks and Trade-offs

- **truncated note-on/note-off の不整合**: note-on 配送・off
  truncate で ledger が active 認識 → 次回 discontinuity で
  defined release。delivered event のみを権威とする契約と
  整合し、計数（`event_merge_truncated`）で観測可能。
  受容。
- **`MergeError::Overflow` 除去の破壊性**: `#[non_exhaustive]`
  enum だが variant 削除は match する downstream を壊し得る。
  workspace 内の match は node.rs のみで追従済み。外部
  consumer は semver 的に非保証（crate は workspace 内
  利用）。IR に明記。
- **stage ledger 削除の影響**: stage 単独利用での
  active note 観測が失われるが、現行 workspace にその利用は
  なく、node ledger が delivered event の superset を権威
  として持つ（plan/chase/evict/release 由来を含む）。
  受容・IR 明記。
- **`anchor_rejected` の意味解釈**: `UnmappedDomain` は
  「stamp の domain に対応する anchor が無い」= anchor が
  その source を cover していない、を含む。domain を持た
  ない stamp（legacy）で anchor 未 publish 時は pending
  待機であり rejection ではない — この区別は既存動作を
  変更しない。

## Deferred Design Concerns

- `NodeReport::EventScratchOverflow` payload 化 | control 面
  変更・`DropReason` 写像 | 別 slice | counter で件数観測は
  確保済み。
- `MapRejection` 全 variant 細分化 | V12 の関心は anchor
  coverage 起因のみ | rejection 内訳の運用必要性 | 非 anchor
  rejection が `rejected` に埋もれる。
- stage 単独 release API | 単独利用 consumer 不在 | consumer
  出現時 | 再導入判断が必要（node ledger は plan/chase 由来
  を含まない制約あり）。

## Verification Notes

- [ ] `merge_call_events` truncation: plugin-core unit test
  （over-budget → 先頭 N 件 + truncated 件数、capacity=0、
  src_scratch < scratch の非対称容量）。
- [ ] node 統合: `midi_delivery.rs` または新規 test で
  truncation → `EventScratchOverflow` report +
  `event_merge_truncated` counter + 部分配送を確認。
- [ ] V11: stage ledger API 削除後、seek/stop stuck-note
  test（`stop_releases_live_notes_too`・seek chase）が非退行。
- [ ] V12: anchor republish で pending 保全・新 anchor での
  再写像・domain 不一致 stamp の `anchor_rejected` 計数・
  `BeforeAnchor` が `anchor_rejected` に入らない区別を test。
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/
  `test-plugin-core`/`lint-plugin-core`/`test-l1-real-plugin`。
