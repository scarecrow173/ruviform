---
id: "TSPEC-034XCNB0kMuKY2YL0umBJp"
type: "test-spec"
status: "approved"
title: "Issue #253 MIDI event contract fixes テスト仕様"
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
    - "docs/designs/issue-253-midi-event-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-188-timed-midi.md"
    - "docs/test-specs/issue-253-unsupported-transparency.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-253-midi-event-contract.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-253-midi-event-contract.md"
    - "docs/designs/issue-253-midi-event-contract.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 MIDI event contract fixes テスト仕様

## Purpose

`merge_call_events` defined truncation（V9）・`InputStage`
ledger 一本化（V11）・anchor republish pending event 契約
（V12）の 3 contract を deterministic test で固定する。
headless/fake backend のみで完走し、実 device/実 plugin の
Pass は主張しない。

## Feature

Feature: MIDI event contract — merge truncation / ledger
一本化 / anchor republish pending

## Rules

- **Rule V9-1 (defined truncation)**: `merge_call_events` は
  merge 結果が `min(scratch.len(), src_scratch.len())` を超える
  場合、stable merge 順の先頭 `capacity` 件を返し、
  `truncated = needed - capacity` を報告する。`Err(Overflow)`
  による全損失は存在しない。
- **Rule V9-2 (truncation 観測)**: `truncated > 0` の call は
  `NodeReport::EventScratchOverflow` を発火し、
  `NodeStatusSnapshot::event_merge_truncated` が truncate 量を
  報告する。配送される event は retained window のみ。
- **Rule V9-3 (no-alloc 非退行)**: truncation 経路は
  allocation・blocking・I/O を追加しない（`test-rt` lane）。
- **Rule V11-1 (ledger 一本化)**: `InputStage` は
  `NoteLedger`/`release_all`/`active_notes`/`released`/
  `ledger_overflows` を持たない。note state の追跡・release
  権威は node ledger のみ。
- **Rule V11-2 (stuck-note 非退行)**: seek/stop/loop の
  discontinuity で node ledger が delivered note を release
  し、chase が機能する既存 test が全て Pass する。
- **Rule V12-1 (pending 保全)**: `publish_anchor` は pending
  を消費・drain・並べ替えしない。anchor が `None` の間は
  pending は待機し、republish 後に現行 anchor で写像される。
- **Rule V12-2 (現行 anchor 権威)**: pending event の写像は
  常に `collect_block` 時点の anchor のみで行われる。
  republish で anchor が差し替われば pending は新 anchor で
  再写像される（旧 anchor での事前写像は存在しない）。
- **Rule V12-3 (defined rejection + anchor 識別)**: 写像不能
  stamp は per-event で `rejected` に計数され除去される。
  anchor の domain/base/epoch coverage 起因の rejection は
  `anchor_rejected` にも計数される（`anchor_rejected ≤
  rejected`）。`BeforeAnchor`/`Overflow` 等の非 anchor 起因
  rejection は `anchor_rejected` に含まれない。

## Examples

### V9 — merge truncation

- Example: over-budget merge は先頭 N 件を配送する
  - Given: 2 source の merge 結果が 8 件、`scratch`/`src_scratch`
    の capacity が 5
  - When: `merge_call_events` を呼ぶ
  - Then: `MergeOutcome.events.len() == 5`、stable merge 順の
    先頭 5 件が保持され、`truncated == 3`
- Example: 非対称容量は min が effective capacity
  - Given: `scratch.len() == 8`、`src_scratch.len() == 3`、
    needed == 6
  - When: merge する
  - Then: `events.len() == 3`、`truncated == 3`
- Example: capacity 0 は空配送 + 全件 truncated
  - Given: `scratch`/`src_scratch` が空、needed > 0
  - When: merge する
  - Then: `events.is_empty()`、`truncated == needed`、
    panic しない
- Example: node 統合で truncation が観測される
  - Given: node の event window が scratch を超える構成
  - When: `run_call` が merge する
  - Then: retained event のみ plugin に配送され、
    `EventScratchOverflow` report + `event_merge_truncated`
    counter が truncate 量を報告する

### V11 — ledger 一本化

- Example: stage ledger API が存在しない
  - Given: `InputStage` を構築
  - Then: `release_all`/`active_notes`/`released`/
    `ledger_overflows` が存在せず、`collect_block` は
    ledger feed を行わない（コンパイル時に保証）
- Example: stop で live note が release される（node 権威）
  - Given: live MIDI note-on が配送済み
  - When: stop/discontinuity を実行
  - Then: node ledger の `release_all` が合成 note-off を
    emit し plugin に配送される（既存 test 非退行）
- Example: seek で chase が機能する
  - Given: plan 再生中に seek
  - When: discontinuity 処理
  - Then: node ledger が active note を release し、
    `chase_span_events` が plan event を chase する

### V12 — anchor republish pending

- Example: republish で pending が保全される
  - Given: anchor A で stamp S の pending event が queue にある
  - When: `publish_anchor(B)`（B は S を cover する anchor）
    して `collect_block`
  - Then: pending は消費されず、B で写像された frame で
    delivered される
- Example: domain 不一致 stamp は anchor_rejected
  - Given: anchor が domain D のみ cover、pending に domain
    E の stamp がある
  - When: `collect_block`
  - Then: その event は `rejected`+`anchor_rejected` に
    計数され除去、他の pending は保持される
- Example: BeforeAnchor は anchor_rejected に入らない
  - Given: anchor より前の stamp の pending event
  - When: `collect_block`
  - Then: `rejected` に計数されるが `anchor_rejected` は
    増えない
- Example: anchor None 中は pending が待機する
  - Given: `publish_anchor(None)` 後に stamp pending を push
  - When: `collect_block`
  - Then: mapped は空、pending は保持、rejection 計数なし
  - And: その後 anchor を publish して `collect_block` すると
    pending が写像・配送される
- Example: 同一 domain 再校正で pending が新 anchor で再写像
  - Given: anchor A（stamp_ticks 基準）で pending がある
  - When: 同一 domain で base の異なる anchor B を republish
  - Then: pending は B の写像で delivered される
    （A での事前写像結果は使われない）

## Guarantees

- merge truncation は決定的で、retained window は常に
  stable merge 順の先頭 `min(needed, capacity)` 件。
- truncate 量は `NodeStatusSnapshot` で観測可能。
- `InputStage` は note state を持たず、ledger 権威は
  node ledger 一箇所。
- pending event は anchor republish を生き延び、写像は
  常に現行 anchor のみで行われる。
- anchor 起因 rejection は `anchor_rejected` で識別可能。

## Non-goals

- 実 device/実 plugin での検証（headless のみ）。
- `NodeReport` payload 変更、`MapRejection` 全 variant
  細分化、stage 単独 release API。
- V13（CC/param chase）。
- macOS/Linux backend、`crates/ruviform-audio-test/`。

## Risk

- truncation で note-on 配送・off 喪失した場合、node ledger
  が active 認識し次回 discontinuity で release される。
  stuck-note にはならないが、release が遅れる — 計数で
  観測可能な defined 挙動として受容。
- stage ledger 削除で stage 単独利用時の note 観測が失われる
  が、現行 workspace にその利用はない。
- `anchor_rejected` の分類境界（`UnmappedDomain` vs
  `BeforeAnchor`）を誤ると観測性が劣化する — test で
  区別を固定する。
