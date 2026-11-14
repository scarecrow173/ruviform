---
id: "SPEC-034XCLrvoEmreSmTn3sL4E"
type: "spec"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-188-timed-midi.md"
    - "docs/specs/issue-253-unsupported-transparency.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-253-vst3-midi-canonical-path.md"
    - "docs/specs/issue-253-os-midi-backend.md"
    - "docs/impl/ir/issue-188-midi-node-wiring-and-chase.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-188-timed-midi.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/188"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 MIDI event contract fixes — merge truncation / ledger 一本化 / anchor republish

## What Should Be Built

Issue #188 実装記録（`docs/impl/ir/issue-188-midi-node-wiring-and-chase.md`
の review follow-up V9/V11/V12）が non-blocking として残した 3 件の
event contract 是正を、headless で完走する形で実装する。

1. **V9 — `merge_call_events` の defined truncation**:
   `ruviform-plugin-core` の `merge_call_events` は、merge 結果が
   caller 提供 scratch に収まらない場合に `Err(MergeError::
   Overflow)` を返し、呼出側（`PluginNode::run_call`）は call 全体を
   空 `EventList` で配送する — 収まる先頭イベントすら失われる。
   これを「容量内に収まる先頭 N 件を配送する defined truncation」
   へ縮退させ、truncate された件数を counter で可観測にする。
   RT no-alloc 契約は維持する。
2. **V11 — `InputStage` ledger の node 権威への一本化**:
   `InputStage` が独自に持つ `NoteLedger` + `release_all` +
   `active_notes` + 関連 counter（`released`/`ledger_overflows`）
   は、本番経路で未使用のまま node 側 ledger（merge 後の
   delivered event list を feed する権威）と二重管理になっており、
   merge truncation 導入後はなおさら乖離し得る。Stage 側の ledger
   API を正直に削減し、note state 追跡の権威を node ledger 一箇所
   に集約する（公開 API 削除は IR に明記）。
3. **V12 — anchor republish 後 pending event の契約化**:
   `SetClockAnchor → InputStage::publish_anchor` で anchor が
   差し替わった後、`pending` に残る `TimedMidiInput` の取扱いを
   契約として確定する。確定契約は **保全**: pending は anchor
   差し替えで消費・drain されず、各 event は `collect_block`
   時点の現行 anchor のみを権威として `map_stamp_to_frame` で
   写像され、写像不能な stamp は defined rejection として計数
   される。epoch/domain 不一致による rejection を識別できる
   counter を additive に追加し、契約を test で固定する。

## Why Now

- Issue #253 は tracker として open 継続であり、V9/V11/V12 は
  「関連する non-blocking follow-up」として明示的に包含可否の
  判断対象に挙がっている（Issue 本文「関連する non-blocking
  follow-up」節・判定節「V9/V11/V12/V13 deferredの採否は個別
  判断」）。本 slice はそのうち V9/V11/V12 の 3 件を対象とし、
  V13（CC/param chase）は別 Issue 起票検討中のため対象外。
- V9 は現状「over-budget → call 全体が空配送」という最悪の
  縮退であり、部分配送+計数の方が情報損失として厳密に優る。
- V11 は二重 ledger が merge truncation（V9）導入でさらに
  乖離し得る構造的問題であり、先に権威を一本化しておく方が
  契約が明確になる。
- V12 は「republish 後に pending がどうなるか」が未定義のまま
  実装依存（lazy rejection）になっており、spec/design で契約を
  確定しないと consumer が republish の意味を誤解し得る。

## Users and Value

- engine/consumer 実装者: over-budget block で「全部消える」
  ではなく「収まる分は届き、落ちた分は計数される」挙動を
  仕様として信頼できる。anchor 再校正時に pending event の
  行方を契約として予測できる。
- plugin/adapter 実装者: `merge_call_events` の truncation
  契約と `InputStage` の公開 API 削減が明文化され、今後の
  backend 追加時に同じ契約を再利用できる。
- 検証担当: headless の deterministic test で 3 契約全てを
  固定でき、実 device/実 plugin 無しに受入判定できる。

## Scope

### In Scope

- `crates/ruviform-plugin-core/src/event.rs`:
  `merge_call_events` の defined truncation 化（戻り値を
  merge 結果 + truncate 件数を運ぶ形へ変更）、`MergeError`
  の縮小、unit test 更新・追加、RT no-alloc test 非退行。
- `crates/ruviform-audio-plugin-bridge/src/input.rs`:
  Stage 側 `NoteLedger`/`release_all`/`active_notes`/関連
  counter の削除、`publish_anchor` 契約 doc、`anchor_rejected`
  counter 追加、`collect_block` の rejection 分類。
- `crates/ruviform-audio-plugin-bridge/src/node.rs`:
  `merge_call_events` 新契約への追従、truncation 計数の
  `NodeStatus` 反映。
- `crates/ruviform-audio-plugin-bridge/src/status.rs`:
  truncation/anchor-rejection 計数の mirror 追加、削除される
  input counter mirror の整理。
- `crates/ruviform-audio-plugin-bridge/tests/`:
  V9/V11/V12 の deterministic test 追加・既存 test の契約
  追従（stuck-note 系は node 権威側で非退行）。
- `docs/` 文書チェーン（spec/design/plan/task/test-spec/impl）、
  `evidence/rf253/` 検証記録。

### Out of Scope

- V13（CC/param chase の deferred contract）— 別 Issue 起票
  検討中のため本 slice では着手しない。
- `crates/ruviform-audio-test/`（別 Worker 統合済み・参照のみ）。
- `Cargo.toml`/`Cargo.lock` の意味変更（依存追加なし予定）。
- macOS/Linux MIDI backend、実 device/実 plugin 検証、
  `mise.toml` lane 追加、workflow/CI 変更。
- Issue #253 の Close — tracker として open 維持（PR 本文は
  `Refs #253`、auto-close 語彙を使わない）。

## Requirements

- R1: `merge_call_events` は、merge 結果が scratch 収容を超える
  場合、収容可能な先頭 `min(scratch.len(), src_scratch.len())`
  件を stable merge 順（最小 offset 優先）で返し、収まらなかっ
  た件数を呼出側へ報告しなければならない。部分 merge は defined
  であり、silent drop・panic・allocation をしてはならない。
  `TooManySources` は従来どおり defined rejection とする。
- R2: truncation 量は `NodeStatus` counter として可観測でなけ
  ればならない。既存 `EventScratchOverflow` report/counter の
  incident 意味は維持する（report variant 変更は control 面の
  別 slice とする）。
- R3: `InputStage` は `NoteLedger` を保持してはならない。
  `release_all`/`active_notes`/`released`/`ledger_overflows`
  は削除し、note state の追跡・release 権威は merge 後
  delivered event を feed する node 側 `NoteLedger` 一箇所に
  集約する。seek/stop 系の stuck-note 防止は node 権威で
  非退行でなければならない。
- R4: `publish_anchor` は `pending` を消費・並べ替え・drain
  してはならない。pending event は `collect_block` 時点の
  現行 anchor のみで写像され、写像不能 stamp は per-event の
  defined rejection として計数される。anchor の domain/base/
  epoch coverage に起因する rejection は専用 counter
  （`anchor_rejected`）で識別可能でなければならない。
- R5: RT パス（`merge_call_events`・`collect_block`・node の
  call 合成）は no-alloc/no-block を退行させない
  （`mise run test-rt` 非退行）。
- R6: 公開 API の削除・戻り値変更は additive でない変更と
  して IR に明記する。`#[non_exhaustive]` enum/struct の
  既存 variant/field 意味は不変とする（`MergeError::Overflow`
  の除去は到達不能化に伴う契約縮小として明記）。
- R7: 既存 lane（`fmt-check`/`check`/`lint`/`test`/`test-rt`/
  `test-plugin-core`/`lint-plugin-core`/`test-l1-real-plugin`）
  は回帰してはならない。

## Acceptance Criteria

- [ ] V9: `merge_call_events` が over-budget 時に defined
  truncation（先頭 N 件配送 + truncate 件数報告）を返し、
  `Err(Overflow)` による call 全体空配送が解消されている。
  truncate 量が `NodeStatus` counter で観測可能。RT no-alloc
  非退行。
- [ ] V11: `InputStage` 側 ledger API が削除され、note state
  権威が node ledger に一本化されている。既存 stuck-note 系
  test（seek/stop release・live note release・ledger eviction）
  が非退行。
- [ ] V12: anchor republish 後の pending event 取扱いが
  「保全 + 現行 anchor 権威の写像 + defined rejection 計数」
  として spec/design に確定し、`anchor_rejected` counter と
  test で固定されている。
- [ ] 全て headless（fake/deterministic backend）で完走し、
  実 device/実 plugin の Pass は主張しない。
- [ ] Issue #253 は open 維持（`Refs #253`、auto-close 語彙
  なし）。

## Deferred Design Concerns

- `NodeReport::EventScratchOverflow` への truncate 件数 payload
  追加 | `control.rs`/`dispatch.rs` は本 slice の編集範囲外で
  あり、report variant 変更は `DropReason` 写像を伴う契約変更
  になる | report 面の additive 見直しを行う別 slice | incident
  のみ report・件数は counter の分担が続く（観測性は counter
  で確保済み）。
- `InputStage` を `PluginNode` 無しで単独利用する場合の
  note-release API | 現行 workspace では `InputStage` は
  `attach`→`PluginNode` 経路でのみ使われ、stage 単独の
  release 需要は存在しない | 単独利用 consumer が現れた時点 |
  stage 側に release API を再導入する判断が必要になる
  （node ledger は delivered event のみを知るため、stage
  単独では plan/chase 由来 note を追跡できない制約を再確認
  すること）。
- `MapRejection` 別の細分化 counter（`BeforeAnchor`/`Overflow`
  の独立計数） | V12 の関心は anchor coverage 起因の rejection
  であり、`anchor_rejected` で識別可能。残り 2 系統は頻度が
  低く `rejected` 合計で観測可能 | rejection 内訳の運用
  必要性が出た時点 | anchor 以外の rejection 要因が `rejected`
  に埋もれたままになる。

## Sources and Relations

- 上流 Issue: `scarecrow173/ruviform#253`（non-blocking
  follow-up V9/V11/V12 の明示的包含対象）。
- 契約の出典: `docs/specs/issue-188-timed-midi.md`（R4 stamp→
  frame 写像・R6 merge 規約・R8/R9 note safety）、
  `docs/designs/issue-188-timed-midi-detailed.md` §5。
- follow-up の記録: `docs/impl/ir/issue-188-midi-node-wiring-
  and-chase.md`（V9「excess のみ truncate」、V11「独立
  ledger は node 権威と乖離し得る」、V12「live-source 不連続
  扱いまたは pending 保全を検討」）。
- 関連 slice: `docs/specs/issue-253-unsupported-transparency.md`
  （`InputStageCounters`/`NodeStatus` mirror の直前拡張）、
  `docs/specs/issue-253-vst3-midi-canonical-path.md`。
