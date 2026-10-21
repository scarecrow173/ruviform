---
id: "TASK-034JOulyBprt7B3tRAzkKZ"
type: "task"
status: "todo"
title: "chunk timestamp 単体テストと Issue #59 完了確認"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-audio-device/src/convert.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-59-chunk-timestamp.md"
  implemented-by: []
  depends-on:
    - "synthesize-per-chunk-timestamps.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-59-chunk-timestamp.md"
    - "synthesize-per-chunk-timestamps.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/chunk-timestamp-contracts.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/59"
    - "https://github.com/scarecrow173/ruviform/issues/25"
  defers: []
  deferred-by: []
---

# chunk timestamp 単体テストと Issue #59 完了確認

## Work

1. `convert.rs` の `mod tests` に per-chunk timestamp の単体テストを追加:
   - 分割時オフセット: `frames_capacity = 4`・`sample_rate = 48000`・10 フレームで chunk 0/1/2 の `callback`/`playback` が base + 0 / 83_333ns / 166_667ns（`f_k * 1e9 / 48000` 整数除算）
   - `playback: None` でも panic せず `callback` のみ合成
   - base instant が `i64::MAX` 近傍で `checked_add` 失敗時に元値を渡す
   - 単一 chunk（`n <= max_chunk`）で instant 無変化
2. `stream.rs` の既存テスト・`test_info()` ヘルパーを `sample_rate` 引数に合わせて更新
3. `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`・`bench-smoke`・`prek run --all-files` 全パス
4. `audit_docs.js --type all --json` で本変更起因 findings 0 を確認
5. 上流文書のチェックボックスを実測結果で更新・完了状態にする

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-59-chunk-timestamp.md](../plans/implement-issue-59-chunk-timestamp.md)
- [synthesize-per-chunk-timestamps.md](synthesize-per-chunk-timestamps.md)
- [../test-specs/chunk-timestamp-contracts.md](../test-specs/chunk-timestamp-contracts.md)

## Dependencies

- 先行タスク: [synthesize-per-chunk-timestamps.md](synthesize-per-chunk-timestamps.md)

## Affected Files

- `crates/ruviform-audio-device/src/convert.rs`（テスト）
- `crates/ruviform-audio-device/src/stream.rs`（テストヘルパー更新）
- `docs/specs/chunk-timestamp-synthesis.md`（AC 更新）
- `docs/designs/chunk-timestamp-design.md`（Verification Notes 更新）
- `docs/plans/implement-issue-59-chunk-timestamp.md`（Verification 更新）
- `docs/tasks/README.md`（ステータス同期）

## Implementation Notes

- テストは `OutputCallbackInfo` を closure で受けて `timestamp.callback`/`playback` の値を記録・検証する形（既存の chunk 分割テスト `write_interleaved_chunks_when_frames_exceed_capacity` のパターンを踏襲）
- `MediaTime` の値比較は `time_base()`/`value()` アクセサで `i64` 整数比較

## Done Criteria

- [ ] 4 テスト全てパス
- [ ] `mise run test`/`test-rt` 全パス・既存テストの退行なし
- [ ] 全量検証（fmt-check/check/lint/test/test-rt/bench-smoke/prek）パス
- [ ] doc 監査で本変更起因 findings 0

## Verification

- [ ] 上記 Done Criteria 全項目

## Completion Notes

<!-- Record outcome, deviations, and evidence after implementation. -->
