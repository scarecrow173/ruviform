---
id: "TASK-034JOulyBprt7B3tRAzkKZ"
type: "task"
status: "done"
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
   - 分割時オフセット: `frames_capacity = 4`・`sample_rate = 48000`・10 フレームで chunk 0/1/2 の `callback`/`playback` が base + 0 / 83_333ns / 166_666ns（`f_k * 1e9 / 48000` 整数除算で端数切捨て）
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

- [x] 4 テスト全てパス
- [x] `mise run test`/`test-rt` 全パス・既存テストの退行なし
- [x] 全量検証（fmt-check/check/lint/test/test-rt/bench-smoke/prek）パス
- [x] doc 監査で本変更起因 findings 0

## Verification

- [x] 上記 Done Criteria 全項目

## Completion Notes

- 4 テストを `convert.rs` の `mod tests` に追加・全パス（43 unit テスト全件グリーン、退行なし）:
  - `write_interleaved_synthesizes_per_chunk_timestamps` — 10 フレーム・capacity 4・48 kHz で chunk 0/1/2 が base + 0 / 83_333 / 166_666 ns（`callback`・`playback` 双方）
  - `write_interleaved_chunk_timestamp_none_playback_stays_none` — `playback: None` 維持・panic なし
  - `write_interleaved_chunk_timestamp_add_overflow_keeps_original` — base `i64::MAX - 10` で `checked_add` 失敗時に元 instant を保持
  - `write_interleaved_single_chunk_keeps_info_unchanged` — 単一 chunk で `info` 無変化
- `OutputDataCallback` が `'static` 要求のため、テストでは `Arc<Mutex<Vec<OutputCallbackInfo>>>` へ記録する `info_collecting_callback` ヘルパーを追加（テスト専用・RT パスには無関係）
- `stream.rs` の no-alloc テスト呼び出しに `48_000` を追加しシグネチャ追随
- 検証実績: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`・`mise exec -- prek run --all-files` 全パス。`audit_docs.js --type all` で本チェーン起因 findings 0（残存警告は wave-1 以前の既存非ブロッキング分）
- タスク本文のオフセット記述 `166_667ns` を整数除算の実値 `166_666ns` に訂正
