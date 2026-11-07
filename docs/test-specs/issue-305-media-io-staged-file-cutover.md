---
id: "TSPEC-0kY8BSgqOCJHezrud22f3h"
type: "test-spec"
status: "approved"
title: "Issue #305 Media I/O StagedFile 移行 テスト仕様"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-audio/tests/render.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/tests/contract.rs"
    deleted:
      - type: "file"
        path: "crates/ruviform-media-io/src/staged.rs"
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-305-media-io-staged-file-cutover.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #305 Media I/O StagedFile 移行 テスト仕様

## Purpose

旧 `StagedOutput` 撤去と `ruviform-io-file` 移行が、Issue #305 AC と
IO-05 #287 の順序契約（`finalize` → 借用解除 → `seal` → 公開前
readback 検証 → `publish`）を満たすことを pin する。旧 API が
公開面に残らないこと、Publish 失敗の Effect/Recovery が失われない
ことを実機 FS（Windows）で検証する。

## Feature

- `ruviform-io-wav` contract test の `StagedFile` API 移行と
  finalize→seal→readback 独立 decode→publish→再 Open E2E。
- `ruviform-audio::render` の出力ライフサイクル移行
  （`WavMuxer<&mut StagedFile>`・`SealedFile` verify・
  `PublishReceipt`/`PublishFailure` 保持）。

## Rules

- R1（撤去）: `ruviform_media_io::{StagedOutput, PublishOptions}` が
  存在しない — workspace 検索ゼロ・`mise run check` 緑が証拠。
  `ruviform-media-io` の依存は Domain Core3 のみ。
- R2（借用）: `WavMuxer<&mut StagedFile>` が構築でき、muxer drop で
  借用が終わり `staged.seal()` へ進める。raw `File`・clone 可能
  handle へ到達する API を使わない。
- R3（完了証拠）: `finalize()`/`finish()` の成功と（設定時のみ）
  verify 通過が publish の前提。`into_sink()`/Drop の best-effort
  finalize を完了証拠にしない。
- R4（独立検証）: `SealedFile::open_readback` は書込不能な
  `Read+Seek` で、読取 bytes を独立 source（`Cursor`）へ移して
  demux/decode する。検証失敗ファイルは publish しない。
- R5（失敗分類）: `RenderError::Publish` は `PublishFailure`
  （effect/phase/durability/recovery）を保持し Media `Io` に
  丸めない。`PublishFailureKind::DestinationExists` は
  `RenderError::DestinationExists` へ写像される。
- R6（Temp 所有権）: verify 失敗・cancel・abort では Temp が残らず
  既存 destination 無変更。publish の非 Indeterminate 失敗は
  `RecoveryHandle::cleanup` で Temp 除去を試み、`Indeterminate`
  は自動削除しない。
- R7（既存契約維持）: WAV capability・RIFF/byte golden・PCM
  precision・NaN/Inf・4GiB 境界・ZeroFrames・finalize 二重呼出・
  packet stream 不一致の既存試験は変更なしに緑。

## Examples

- `ruviform-io-wav/tests/contract.rs`:
  - `staged_publish_produces_reopenable_wav`:
    `StagedFile::create` → `open_muxer(&mut staged)` → packets →
    `finalize()` → `seal()` → `publish(PublishMode::CreateNew,
    FileSync)` → destination が symphonia で reopen 可能・RIFF。
  - `staged_seal_readback_publish_e2e`（新規）:
    finalize → seal → `open_readback()` で bytes 読取 →
    `Cursor` 経由の独立 demuxer で stream/PCM を検証 →
    `publish()` → 公開後再 Open で同一内容。
  - `staged_abort_preserves_existing_destination` /
    `staged_drop_after_write_preserves_destination`:
    `staged.abort()`・Drop で既存 destination 無変更・Temp 除去。
  - `staged_no_overwrite_conflict_preserves_destination`:
    `PublishMode::CreateNew` + 既存 dest →
    `PublishFailureKind::DestinationExists`・既存 dest 無変更。
  - `staged_replace_overwrites_destination`:
    `PublishMode::Replace` で上書き。
- `ruviform-audio/tests/render.rs`:
  - `of05_wav_reopen_verify`: `verify` hook が `&mut SealedFile` の
    `open_readback` → bytes → `Cursor` → symphonia 独立 decode で
    frame/channel/rate/PCM を検証 → publish。
  - `of06_failure_injection`/`of06_staging_policy`:
    cancel・sink write/flush 失敗（`FailSink<&mut StagedFile>`）・
    publish 失敗（宛先=既存 dir で Replace publish → 実 FS
    `NotPublished`/Io）→ `RenderError::Publish`・DestinationExists
    早期検出・verify 拒否で publish 未到達。

## Guarantees

- 失敗経路は全て型付きで、失敗後に destination が意図せず変更
  されない（`CreateNew` 系失敗）か、`Published`/`Indeterminate` が
  正直に報告される。
- `RenderReceipt.publish` は `PublishReceipt` の実績
  （DurabilityEvidence・TempState・RecoveryHandle）を保持する。
- テスト用の publish 失敗注入は `PublishFailure` の偽造ではなく
  実 FS の失敗で行う（crate 外で `PublishFailure` は構築不能）。

## Non-goals

- `ruviform-io-file` 自体の publish 分類行列（#295/#296 で
  カバー済み）。ここでは Consumer としての結合のみ検証する。
- macOS/Linux 実機検証（本環境 Windows のみ。io-file OS backend
  実測は #296 Gate）。
- Consumer repo（audio-lens/synvelis）側の Gate — 候補提供のみ。
- `ruviform-media-io` crate 内での readback/publish 検証
  （staged 機能は撤去され存在しない）。

## Risk

- `PublishFailure` を crate 外で構築できないため失敗注入が
  実 FS 依存になり、OS 間で失敗種別が揺れる可能性 — 実 FS 失敗の
  方が契約に近い証拠であり許容するが、`NotPublished`+dest 無変更の
  本質を assert する形に限定する。
- `verify` hook が `&mut SealedFile` を受ける設計は hook が
  `open_readback` を呼べない環境（Unsupported）で `Verify` 失敗へ
  落ちる — 検証未完のまま publish しない既定で安全側。
