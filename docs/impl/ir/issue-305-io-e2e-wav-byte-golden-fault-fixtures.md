---
id: "IMPL-034Uxng6BqRjROMnmJ4lor"
type: "impl"
status: "completed"
title: "Issue #305 io-e2e WAV byte golden + fault fixtures"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/296"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/common/wav.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/wav_golden.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/wav_fault.rs"
      - type: "file"
        path: "docs/specs/issue-305-io-e2e-wav-golden-fault.md"
      - type: "file"
        path: "docs/designs/issue-305-io-e2e-wav-golden-fault.md"
      - type: "file"
        path: "docs/test-specs/issue-305-io-e2e-wav-golden-fault.md"
      - type: "file"
        path: "docs/plans/issue-305-io-e2e-wav-golden-fault.md"
      - type: "file"
        path: "docs/tasks/issue-305-w1-fixture-extension.md"
      - type: "file"
        path: "docs/tasks/issue-305-w2-verification-handoff.md"
      - type: "file"
        path: "evidence/rf305/wav-golden-fault-report.md"
      - type: "file"
        path: "docs/impl/ir/issue-305-io-e2e-wav-byte-golden-fault-fixtures.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "crates/ruviform-io-e2e/src/lib.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-305-w1-fixture-extension.md"
    - "docs/tasks/issue-305-w2-verification-handoff.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-295-ruviform-io-file-staged-publish-contract.md"
    - "docs/impl/ir/issue-306-io-e2e-slice-2-fault-fuzz-fixture-os-fs-probe-rooted-boundary.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-305-io-e2e-wav-golden-fault.md"
    - "docs/designs/issue-305-io-e2e-wav-golden-fault.md"
    - "docs/specs/issue-305-io-e2e-wav-golden-fault.md"
    - "docs/test-specs/issue-305-io-e2e-wav-golden-fault.md"
    - "evidence/rf305/wav-golden-fault-report.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #305 io-e2e WAV byte golden + fault fixtures

## Summary

`ruviform-io-e2e` に WAV→`StagedFile` 統合の byte golden と
fault fixture を追加し、公開 API のみを消費する形で
finalize→borrow release→seal→independent decode→publish の
ライフサイクルと publish outcome 分類を実 FS 上で検証した。
本番コード・`ruviform-io-wav` src・依存は一切変更していない。

## Context

Issue #305 の残行 slice。spec→design→test-spec→plan→task の
文書チェーン（`docs/specs|designs|test-specs|plans|
tasks/issue-305-*`）に従い、`crates/ruviform-io-e2e/tests/`
のみを対象とした。既存 `wav_staged.rs`（finalize→borrow
release→seal→independent decode→publish→reopen・truncated/
second-finalize reject）は重複させず増分のみ実装した。

## Implementation

### Changes

- `tests/common/wav.rs`（新規）: RIFF/WAVE 仕様から独立に
  期待 byte 列を組み立てる `expected_wav_bytes`、`f32→wire`
  変換 policy の独立 oracle（`expected_wire_sample`/
  `expected_edge_payload`/`edge_samples`）、S16 payload/
  packet/spec helper、symphonia demux による独立 decode
  oracle（`independent_decode`）。すべて `ruviform-io-wav`
  の private helper に依存しない。
- `tests/wav_golden.rs`（新規・8 件）: ZeroFrames 44B
  WAVEFORMATEX、5.1 WAVEFORMATEXTENSIBLE 68B（tag 0xFFFE・
  cbSize 22・channel mask 0x3F・PCM subtype GUID）、S16
  stereo payload、U8/S16/S24/S32/F32 encoder edge sample
  （NaN→0・±Inf/clamp・round-half-away・F32 verbatim）、
  odd payload pad byte、classic RIFF 4 GiB 境界を discard
  sink で検証（`data_bytes=4_294_967_256` で finalize 成功・
  超過側は write/finalize とも `InvalidData`・receipt なし）。
  実 4 GiB ファイルは生成しない。
- `tests/wav_fault.rs`（新規・9 件）: `&mut StagedFile` への
  公開 API のみの `MediaSink` wrapper（`FlushFail`/`DiskFull`）
  で finalize flush・backpatch write・packet write の
  `Io` 失敗を `ErrorKind::Io`・receipt なし・publish なしで
  検証。seal 後 abort、muxer `Drop` best-effort finalize が
  completion 証拠でないこと、publish outcome classifier
  （`Published`/`NotPublished`/not-provably-unpublished・
  `#[non_exhaustive]` wildcard 込み）、CreateNew 競合
  `DestinationExists`/`PublishPhase::Publish`/`NotPublished`
  + 先勝ち bytes 保全 + recovery handle cleanup、Replace
  publish、`StagedFile::create` 失敗の型付き
  `FileErrorKind::Io` と residue なし。
- `tests/common/mod.rs`: `pub mod wav;` 追加のみ。
- `crates/ruviform-io-e2e/README.md`・`src/lib.rs`: fixture
  一覧に golden/fault を追記し、Windows NTFS のみ実測・
  macOS/Linux/network FS は NotRun/Unsupported と正直に
  記載。

### Decisions During Implementation

- byte golden の期待値は `ruviform-io-wav` の private
  helper を呼ばず、RIFF/WAVE 仕様と `f32→wire` 変換 policy
  から独立に構築した（実装に oracle を追従させない）。
- 4 GiB 境界は実ファイルを生成せず、累積 byte を追跡する
  discard sink で検証（Issue の sink 指定どおり）。
- `seal()` は `StagedFile` を消費するため lifecycle helper
  は by-value で受け、`into_sink()` で muxer 借用を明示
  release してから seal する形にした。
- `Indeterminate`/`Published` 失敗 effect・DirSync 後失敗・
  `TempState::Unknown` は公開 API から誘発不能のため
  NotRun とし、classifier が両者を not-provably-unpublished
  bucket に落とすことを型で pin した（`ruviform-io-file` の
  `#[cfg(test)]` mock 試験が正本）。
- trybuild 等の旧 API 非存在コンパイル時証拠は新依存追加
  禁止のため未導入。`StagedOutput` grep 0 件・
  `PublishOptions` 全参照が `ruviform_io_file` 新 API・
  `cargo tree -p ruviform-media-io` が Domain Core3 依存のみ
  の補助証拠で代替（evidence report に明記）。

## Related Experiments

### Adopted

なし（探索的 experiment は実施していない）。

### Rejected

なし。

## Validation

`mise run fmt-check` / `check-io-e2e` / `lint-io-e2e` /
`test-io-e2e`（77 tests 全 pass）/ `check` / `lint` /
`test` / `test-rt` を merge 後 HEAD で全実行・全 Pass。
実値・環境・NotRun/Unsupported 一覧は
`evidence/rf305/wav-golden-fault-report.md` を参照。
`ruviform-io-wav` src は無変更（真の defect 未発見）。

## Risks

- 実測は Windows 11 x64 / NTFS のみ。macOS/Linux・
  Network/Cloud FS での durability/atomicity は未検証であり、
  本証拠を根拠にそれら環境の Supported を主張しない。
- 4 GiB 境界は discard sink 上の検証であり、実ファイルの
  publish 経路は未実測。

## Follow-ups

- AudioLens #143・Synvelis #84 の consumer 実測・候補
  Revision/lockfile 依存更新連携は #219/#294 gate の責務
  （NotRun）。引き渡し条件は evidence report に記録。
- `Published`/`Indeterminate` 失敗 effect・DirSync 後失敗・
  `TempState::Unknown` の fixture 化は `ruviform-io-file`
  mock 試験側の責務として残る。
