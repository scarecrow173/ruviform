---
id: "IMPL-034XDbU0qW4r971UaTlEUp"
type: "impl"
status: "completed"
title: "issue-253-midi-event-contract"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-253-midi-event-contract.md"
      - type: "file"
        path: "docs/designs/issue-253-midi-event-contract.md"
      - type: "file"
        path: "docs/plans/issue-253-midi-event-contract.md"
      - type: "file"
        path: "docs/tasks/issue-253-midi-event-contract.md"
      - type: "file"
        path: "docs/test-specs/issue-253-midi-event-contract.md"
      - type: "file"
        path: "docs/impl/exp/issue-253-midi-event-contract.jsonl"
      - type: "file"
        path: "evidence/rf253/verification-2026-11-15.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/event.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/input.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/node.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/status.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/tests/midi_delivery.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-253-midi-event-contract.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-188-midi-node-wiring-and-chase.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-188-midi-node-wiring-and-chase.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-253-midi-event-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-midi-event-contract.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# issue-253-midi-event-contract

## Summary

Issue #253 追跡の non-blocking follow-up slice。Issue #188 の MIDI node
wiring/chase 実装に対するレビュー指摘 V9/V11/V12 の 3 件を是正した:

- **V9**: `merge_call_events` の over-budget 挙動を `Err(MergeError::Overflow)`
  による call 全体空配送から、容量 `min(scratch, src_scratch)` に収まる
  安定マージ先頭 N 件への defined truncation に変更。省略量は新公開型
  `MergeOutcome::truncated` と `NodeStatusSnapshot::event_merge_truncated`
  で可観測。`TooManySources` は引き続き defined rejection。
- **V11**: `InputStage` が独自に所有していた `NoteLedger` と `release_all`・
  `active_notes`・`released`/`ledger_overflows` カウンタを削除し、note 状態の
  権威を node 側 `NoteLedger` に一本化。stage は mapping/rejection の計数と
  pending 保持のみを担う。
- **V12**: `publish_anchor` は pending input event を保全し、mapping は
  `collect_block` 実行時点の anchor で行う契約に確定。anchor の
  domain/base/epoch 不一致による rejection を新 counter
  `anchor_rejected`（`NodeStatusSnapshot::input_anchor_rejected`）で
  `BeforeAnchor`/算術 `Overflow` と区別して計数。anchor `None` の間は
  pending を保持し rejection を計上しない。

## Context

`docs/impl/ir/issue-188-midi-node-wiring-and-chase.md` のレビューで挙がった
non-blocking gap 3 件への対応。spec → design → plan → task → test-spec の
文書チェーンは `docs/{specs,designs,plans,tasks,test-specs}/
issue-253-midi-event-contract.md` を参照。Issue #253 は tracker として open
継続し、本 slice では close しない。

## Implementation

### Changes

- `crates/ruviform-plugin-core/src/event.rs`
  - `MergeError::Overflow` を削除し、`merge_call_events` は
    `Result<MergeOutcome<'a>, MergeError>` を返す。`MergeOutcome` は
    `events: &'a mut [Event]`（配送される retained prefix）と
    `truncated: usize`（省略イベント数）を持つ。
  - 容量は `min(scratch.len(), src_scratch.len())`。k-way merge は retained
    分のみ emit し、`truncated = needed - emitted`。note-off priority は
    truncation 後の retained window に適用。allocation/panic/blocking なし。
  - ユニットテストを truncation 契約（ゼロ容量・優先度境界・no-alloc）に
    更新。
- `crates/ruviform-plugin-core/src/lib.rs`: `MergeOutcome` を export。
- `crates/ruviform-audio-plugin-bridge/src/input.rs`
  - `NoteLedger` フィールドと `release_all`/`active_notes`、および
    `released`/`ledger_overflows` カウンタを削除。`collect_block` の ledger
    feed/eviction 処理を除去し、mapping 済みイベントのみ返す。
  - rejection 分類を追加: `UnmappedDomain`/`BaseMismatch`/`EpochMismatch`
    は `anchor_rejected` を加算し、`BeforeAnchor`/`Overflow` は `rejected`
    のみ。anchor `None` では pending を保持し計数しない。
- `crates/ruviform-audio-plugin-bridge/src/node.rs`
  - merge 呼出を `MergeOutcome` に適合させ、truncation 発生時に
    `Counter::EventMergeTruncated` を bump（`EventScratchOverflow` report
    経路は既存の overflow flag で維持）。
  - `run_call` の行数超過を避けるため merge 窓の処理を `merge_window`
    ヘルパーに抽出（挙動不変の局所リファクタ）。
- `crates/ruviform-audio-plugin-bridge/src/status.rs`
  - `Counter::EventMergeTruncated`・`Counter::AnchorRejected` を追加し、
    `NodeStatusSnapshot::event_merge_truncated`/`input_anchor_rejected`
    へミラー。削除した stage ledger カウンタのミラーを除去。
- `crates/ruviform-audio-plugin-bridge/tests/midi_delivery.rs`
  - V12 契約テスト 4 件追加（anchor None pending 保持→後続 anchor で map、
    domain 切替、同一 domain 再校正での再 map、rejection 分類）。
  - V11 に伴い stage ledger API 参照を node ledger/配送イベントベースの
    検証へ更新。

### Decisions During Implementation

- **V9 の失敗形状**: `TooManySources` は引き続き `Err`（呼出側バグ級の
  defined rejection）とし、容量不足のみを `Ok` + `truncated` に縮退。
  `merge_call_events` の戻り値を `MergeOutcome` struct にしたのは、
  slice 長と truncation 量を一つの値で運ぶため（タプルより意図が明確）。
- **V9 の観測**: 既存 `NodeReport::EventScratchOverflow` の incident 意味
  （部分配送でも overflow 報告）は維持し、正確な欠落量のみ専用 counter
  `event_merge_truncated` に載せる二層構成とした。
- **V11 の一本化方向**: node 側 ledger は seek/stop/loop の release_all と
  chase の権威として既に存在し、stage 側 ledger は配送イベントに対する
  二重管理だった。stage 側を削減（node 権威へ一本化）を選択 — 公開 API
  削減を伴う非加法変更であり本 record に明記する。
- **V12 の契約**: 「保全」側を採用。republish は pending を消費・drain・
  並べ替えず、mapping は `collect_block` 時点の anchor で評価する。
  これにより anchor 差し替え直後のイベントが新 anchor で正しく map され、
  不一致はイベント単位の defined rejection として観測可能になる。
- **Lint 対応**: `run_call` が clippy `too_many_lines` を超えたため、
  merge→truncation 計数→空配送 fallback の一連を `merge_window` に抽出。
  `#[allow]` は使用していない。

## Related Experiments

### Adopted

なし（本 slice は実験的分岐を伴わない契約確定 + 実装）。

### Rejected

なし。

## Validation

`evidence/rf253/verification-2026-11-15.md` に記録。全て `mise run` 経由、
Windows headless、rustc 1.99.1。fmt-check / check / lint / test（両 crate）/
test-plugin-core / lint-plugin-core / test-rt / test-l1-real-plugin が全て
exit 0。`midi_delivery.rs` の V9/V11/V12 関連テストを含め全 pass。
`real_midi_device_drives_input_tx_to_stage` は opt-in ignored のまま
（実機 Pass は主張しない）。

## Risks

- V9 truncation は配送イベント数を減らすため、scratch が恒常的に不足する
  構成では note-on 優先度により後続イベントが落ち得る。`event_merge_truncated`
  と `EventScratchOverflow` report で観測可能だが、ホスト側の scratch サイジ
  ングは運用依存。
- V11 の公開 API 削減（`InputStage::release_all`/`active_notes`、
  `released`/`ledger_overflows` カウンタ）は破壊的変更。workspace 内の
  参照は全て更新済みだが、クレート外 consumer がいれば追従が必要。
- V12 の「anchor None 中 pending 保持」は pending バッファ容量に依存する。
  長時間 anchor 未設定の入力は従来どおり bounded。

## Follow-ups

- Issue #253 tracker の残受入条件（virtual-first EXT-L3、macOS/Linux L1、
  実機/実 plugin 検証）は本 slice 対象外。Issue は open 継続。
- V13（CC/parameter chase）は別 Issue 起票検討中で本 slice 対象外。
