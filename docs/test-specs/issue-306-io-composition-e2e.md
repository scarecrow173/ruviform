---
id: "TSPEC-rviIWrpuHyJUT11Wykzopn"
type: "test-spec"
status: "approved"
title: "Issue #306 汎用 I/O 合成 E2E テスト仕様"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "docs/specs/issue-306-io-composition-e2e.md"
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/staged_lifecycle.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/structured_staged.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/archive_compression_staged.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/extraction.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/wav_staged.rs"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-306-io-composition-e2e.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-306-io-composition-e2e.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 汎用 I/O 合成 E2E テスト仕様

## Purpose

IO-01〜05 の公開 API を層横断で合成し、「完成証明 → seal →
公開前検証 → publish → 再 Open」の境界契約を実 OS で pin する。
個別クレートのユニット/形式試験の鏡写しを作らない。Issue #306
Acceptance（合成 Finish/Verified EOF/不完全形式の Publish 抑止・
Boundary E2E・Fault Injection 分類・Negative/Fuzz・リソース上限・
OS/FS 実測範囲の正直な主張）が正本。

## Feature

StagedFile を終端とする多層 writer 合成（Structured / TAR /
Compression / WAV）と、publish 済み archive の Secure Extraction が、
層ごとの完成証明を独立に要求し、未検証・失敗・競合状態で destination
を汚染しないこと。

## Rules

- R1（StagedFile 境界・正常系）: `StagedFile::create` → write →
  `seal` → `open_readback` で sealed bytes を読める →
  `publish(PublishOptions)` → `PublishReceipt`（destination・
  durability evidence）→ 再 Open で同一 bytes。publish 後の temp
  は消費される。
- R2（StagedFile 境界・競合/差替え）: staging 後に外部が
  destination を作成した場合、`CreateNew` → `NotPublished` +
  `PublishFailureKind::DestinationExists` で先入主 bytes は保全。
  `Replace` は staged bytes で置き換える（atomic 差替えの
  実 OS 確認）。seal 後の `abort` は destination を作らず temp
  を回収。未 seal の drop は temp を best-effort 回収する。
- R3（Structured→StagedFile）: `json::write_file`/`csv::write_file`
  で serialize→seal→publish した文書を `read_file` で再 Open
  できる。serialize 失敗 → `DocumentFileWriteError::Encode` で
  destination 不在・temp 残存なし。`CreateNew` 競合 →
  `DocumentFileWriteError::Publish` で `PublishFailure` の
  effect=`NotPublished`/kind=`DestinationExists` が保持される。
  公開後に外部が末尾 byte を追加した文書は `read_file` が
  `Decode`(`TrailingData`) で拒否する（strict EOF 境界）。
- R4（TAR→Compression→StagedFile）: `TarArchiveWriter` →
  `gzip::Encoder`/`zstd::Encoder` → `&mut StagedFile` の 3 層
  Write 合成で entry を書き、entry `finish` → archive `finish` →
  encoder `finish` → `seal` → readback で `Decoder` + `TarArchiveReader`
  が entry bytes を検証 → publish → 再 Open で同一解凍復元。
- R5（Verified EOF が publish を gate）: 切り詰めた gzip ストリーム
  を staged bytes とした場合、readback 側 `Decoder::finish` が
  VerifiedEnd 以外を返し、呼び出し側は `abort` で destination を
  作らない。archive entry writer を未 finish で drop した archive
  は `finish` が `InvalidState` で失敗し、publish されない。
- R6（ZIP/TAR→Secure Extraction）: publish 済み ZIP を再 Open →
  `ZipArchiveReader::verify_all` → `ExtractionSession::extract_zip`
  → `ExtractedTree` の file 内容照合。TAR over gzip でも同経路
  （`extract_tar`）。`../`・絶対・`\\`・予約名 entry は
  `InvalidInput` で拒否され `ExtractionFailure` 経路で staging
  root が残存しない。大小文字・canonical 衝突は書込前拒否。
  `max_total_uncompressed_bytes` 等の budget 超過は
  `LimitExceeded` で分類され超過 bytes は staging に出ない。
- R7（WAV→StagedFile）: `WavMuxer<&mut StagedFile>`（MediaSink
  blanket）で packet を書き `finalize()` → `WavReceipt` →
  `into_sink` で借用解除 → `seal` → `open_readback` で sealed
  bytes を読み、`ruviform-io-symphonia` 独立 decode で stream
  数・frame 数・PCM 値を照合 → `publish` → 公開 file を再度
  symphonia decode で照合。
- R8（WAV 未完成の publish 抑止）: 不完全な WAV コンテナは公開
  しない。実装時の検証で backend は `Drop` 時に best-effort
  finalize を行うことが判明した（未 `finalize` でも header が
  埋まり parse が通る）ため、「未完成」は末尾を途中で切った
  truncated payload で表現し、独立 decode が full payload に
  到達しないことを確認 → `abort` で destination は作らない。
  あわせて二重 `finalize` は `Err` で拒否されることも pin する。
- R9（主張境界）: 実測 OS/FS・toolchain・exit code を report に
  記録。`Indeterminate`/`DirSync` 後失敗/`Temp 残留` の adapter
  系分類・Network/Cloud FS・性能・macOS/Linux 実 OS は本クレート
  では `NotRun`（io-file mock ユニット試験・既存 CI lane に委譲）
  として区別し Pass にしない。

## Examples

- `json::write_file(path, &manifest, opts, PublishOptions::new(
  CreateNew, FileAndDirectorySync))` → `read_file::<Manifest>`
  で等値。先入主 destination への CreateNew →
  `Publish`(effect=NotPublished, kind=DestinationExists) + 先入主 bytes
  保全。
- `TarArchiveWriter::new(&mut enc)` → `start_entry("a/b.txt", len,
  opts)` → write → entry.finish → archive.finish → `enc.finish()`
  → `seal` → readback → `gzip::Decoder` `finish` →
  `TarArchiveReader` `finish` → publish → `extract_tar` で
  `ExtractedTree`。
- staged に破損 gzip（末尾 4 byte 欠落）を書く → seal → readback
  `Decoder::finish` → `Err` → `abort` → `destination.try_exists()
  == false`。
- `ZipArchiveWriter` に `"../evil.txt"`・`"dir/../up.txt"` 名の
  entry を含む archive → publish → `extract_zip` →
  `ExtractionFailure`（`InvalidInput`）・staging root 不在・
  destination 外に file なし。
- `WavMuxer::new(&mut staged, &[info])` + `WavEncoder` drain →
  `finalize` receipt(total_frames) → `into_sink` → `seal` →
  readback bytes → `open_demuxer` decode → `total_frames`・PCM
  一致 → publish → `File::open(dest)` で再 decode。

## Non-Tested Boundaries

- `#[cfg(test)]` MockFileOps の fault script（crate 内部）。
- `ExtractionRecoveryHandle`/`ExtractionCleanupFailure` の
  失敗注入系（io-archive `tests/extract_staging.rs` が既存正本
  — 本仕様は合成経路の失敗分類・staging 非残存のみを扱う）。
- `DocumentErrorKind` 全 variant・compression limits の網羅
  （各クレートの format/limit 試験が正本）。
