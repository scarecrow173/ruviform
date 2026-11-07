---
id: "SPEC-P6IzbXqAJ2cgiMm8llJIIO"
type: "spec"
status: "approved"
title: "Issue #305 Media I/O 旧 StagedOutput 完全削除・WAV→StagedFile 移行 (IO-IMP-11)"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/287"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-305-media-io-staged-file-cutover.md"
      - type: "file"
        path: "docs/designs/issue-305-media-io-staged-file-cutover.md"
      - type: "file"
        path: "docs/plans/issue-305-media-io-staged-file-cutover.md"
      - type: "file"
        path: "docs/test-specs/issue-305-media-io-staged-file-cutover.md"
      - type: "file"
        path: "docs/tasks/issue-305-t1-staged-file-cutover.md"
    modified:
      - type: "file"
        path: "crates/ruviform-media-io/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/sink.rs"
      - type: "file"
        path: "crates/ruviform-audio/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio/src/render.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/render.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-wav/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/src/muxer.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/README.md"
      - type: "file"
        path: "crates/ruviform-io-wav/tests/contract.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted:
      - type: "file"
        path: "crates/ruviform-media-io/src/staged.rs"
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/plans/issue-305-media-io-staged-file-cutover.md"
  depends-on:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
    - "docs/designs/issue-198-sequential-offline-render.md"
    - "docs/adr/issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-305-media-io-staged-file-cutover.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #305 Media I/O 旧 StagedOutput 完全削除・WAV→StagedFile 移行 (IO-IMP-11)

## What Should Be Built

`crates/ruviform-media-io` の旧 `staged` モジュール
（`StagedOutput`・旧 `PublishOptions`・直接実装）を、互換層・Facade・
Deprecated を一切残さず削除し、Workspace 内の全 call site を
`ruviform-io-file` の `StagedFile`/`SealedFile`/`PublishOptions`/
`PublishReceipt`/`PublishFailure` 契約へ移行する。

公開面の変更:

- `ruviform_media_io::{StagedOutput, PublishOptions}` の re-export と
  `mod staged` の撤去。`MediaSource`/`MediaSink`/`Demuxer`/`Muxer`/
  `Decoder`/`Encoder`/`Packet`/`Frame`/`Error` の公開契約は維持する。
- `ruviform-media-io` の依存は Domain Core3
  （`ruviform-media-core`/`ruviform-audio-core`/`ruviform-video-core`）
  のまま維持し、`ruviform-io-file` への依存を media-io に追加しない
  （両 crate の合成は利用側の責務 — IO-05 #287 確定）。
- `ruviform-audio::render`（Issue #198 出力ライフサイクル）を
  `StagedFile` 消費に移行する:

  - `RenderOutput.staged: StagedFile`、
    `RenderOutput.publish_options: ruviform_io_file::PublishOptions`。
  - `make_muxer` は `for<'a> FnOnce(&'a mut StagedFile) ->
    Result<Box<dyn Muxer + 'a>, MediaError>` とし、
    `WavMuxer<&mut StagedFile>` の借用構築を可能にする
    （旧 `File::try_clone` 経路は廃止）。
  - `verify` hook は `FnOnce(&mut SealedFile, &OutputFacts) ->
    Result<(), MediaError>` とし、hook は
    `SealedFile::open_readback` の `Read + Seek` Reader 経由でのみ
    検証する（匿名 Path 再 Open を強制しない。Reader は Temp と
    同一対象を借用で指し、Readback borrow 生存中は publish 不可の
    型契約）。
  - `publish` は `FnOnce(SealedFile, PublishOptions) ->
    Result<PublishReceipt, PublishFailure>`。
    `RenderError::Publish { source: PublishFailure }` が
    Effect/Phase/DurabilityEvidence/RecoveryHandle を保持し、
    `PublishFailureKind::DestinationExists` は
    `RenderError::DestinationExists` へ写像する。
    旧 Media `ErrorKind::Io` への丸めはしない。
  - `RenderReceipt` は `publish: PublishReceipt` を保持し、
    Publish の実績証拠（DurabilityEvidence・TempState・残留時
    RecoveryHandle）を呼び出し側へ返す（`Clone` derive は
    `PublishReceipt` 非 Clone のため除去）。
  - 順序契約: `Muxer::finish()`（= `WavMuxer::finalize` の checked
    完了）成功 → muxer 借用終了 → `StagedFile::seal()` →
    verify hook（Readback 経由の独立検証、任意）→
    `SealedFile::publish()`。`into_sink()` や Drop の best-effort
    finalize を完了証拠にしない。

- `ruviform-io-wav` の contract test を `StagedFile` API へ移行し、
  `WavMuxer<&mut StagedFile>` → `finalize()` → `seal()` →
  `open_readback()` で独立 decode → `publish()` → 公開後再 Open の
  E2E を実機 FS で検証する。
- `ruviform-io-wav`・`ruviform-audio` の Rustdoc/README の
  `StagedOutput` 参照を `ruviform-io-file` 契約へ更新する。

## Why Now

IO-IMP-01（#295）・IO-IMP-02（#296）が `StagedFile` 型状態・
OS Atomic Publish・Durability・Readback を確定済み（両 Issue Closed）。
IO-05（#287）の合意どおり旧 `StagedOutput` を互換層なしで撤去し、
`ruviform-io-file` を File Publish の唯一の正本とする段階。
旧 API は `exists()`→`rename()` の非原子 CreateNew・過大な
「失敗時無変更」保証・clone 可能な `&mut File` 露出を含み、
新基盤の保証と両立しない。AL #143/SV #84 の Consumer 候補移行が
この候補 revision を前提とする。

## Users and Value

- Ruviform 内部: `ruviform-audio::render` のオフラインレンダ
  出力ライフサイクル、`ruviform-io-wav` の保存経路テストが
  Atomic CreateNew・Durability 実績・Readback 検証の正式契約を得る。
- AudioLens（#143）・Synvelis（#84）: 旧 `StagedOutput` 撤去後の
  唯一の File Publish 正本へ移行するための候補 revision。

## Scope

### In Scope

- `crates/ruviform-media-io/src/staged.rs` の削除、`lib.rs` の
  `mod staged`/`pub use` 撤去、`sink.rs` の `StagedOutput` 言及更新。
- `crates/ruviform-audio` の `render.rs`/`tests/render.rs` 移行と
  `Cargo.toml` の `ruviform-io-file` 依存追加。
- `crates/ruviform-io-wav` の `tests/contract.rs` 移行 +
  seal/readback/publish E2E 追加、`Cargo.toml` の
  `ruviform-io-file` dev 依存追加、Rustdoc/README の参照更新。
- `WavMuxer<&mut StagedFile>` の型と借用が成立することの
  コンパイル・動作証拠。
- publish 失敗時の Temp 処理: 非 Indeterminate の失敗では
  `RecoveryHandle::cleanup` を best-effort で実行し、
  `Indeterminate` では自動削除しない（io-file 契約）。
- 本 doc チェーン（spec/design/plan/test-spec/task/IR）。

### Out of Scope

- `ruviform-io-file` への新契約追加（所有外）。
- Consumer repo（audio-lens・synvelis）の編集 — 候補 revision のみ
  提供し、Consumer Gate（RF #306/#294・AL #142/SV #83）は
  別工程の Acceptance。
- `MediaSource`/`MediaSink`/4 trait 系・WAV の RIFF/PCM capability・
  scale・size・精度・sample 値の変更。
- RF64/BWF・非 PCM codec など既存 capability 外の format 拡張。
- `StagedOutput` の Deprecated/Facade、新 `StagedFile` の
  raw `&mut File` accessor。
- 歴史資料（`docs/adr/`・`docs/designs/`・`docs/discovery/`・
  `docs/impl/` の過去 Issue 記録）本文中の `StagedOutput` 言及 —
  Issue #305 AC により削除対象外。

## Requirements

- 互換 API を残さない完全撤去。公開シンボル差分は意図した
  Breaking Change として PR に明記する。
- `ruviform-media-io` は Domain Core3 依存のみを維持
  （`cargo tree -p ruviform-media-io` で 3 edge）。
- `ruviform-audio` は `ruviform-io-file` への下向き依存を追加する
  （engine → 汎用ファイル I/O 層。render モジュールは非 RT 専用）。
- `ruviform-io-wav` は `ruviform-io-file` への依存を
  dev-dependency に限定する（backend の公開 DAG に混入させない）。
- `finalize()` 成功を publish の前提にしないフロー
  （Drop/best-effort finalize だけで publish しない）は
  型でなく順序と検証で保証する。
- `unsafe` 不使用・`#[non_exhaustive]` 公開 enum 規約・
  `workspace.lints` 継承・`default` feature 最小・MSRV 1.89。

## Acceptance Criteria

- [ ] AC1: `crates/ruviform-media-io` の公開 API・実装から旧
  `StagedOutput`/旧 `PublishOptions` がゼロ。Workspace 内のコード・
  rustdoc・crate README の旧 API 参照がゼロ（歴史資料本文は対象外）。
  `mise run check`/`lint`/`test` が通り、新 API を使う
  コンパイルテストが存在する。
- [ ] AC2: `MediaSource`/`MediaSink`/`Demuxer`/`Muxer`/`Decoder`/
  `Encoder` の公開契約と feature・依存 DAG が維持される
  （`ruviform-media-io` は Domain Core3 のみに依存）。
- [ ] AC3: `ruviform-audio::render` が `WavMuxer<&mut StagedFile>`
  借用 → `finalize`/`finish` → `seal` → readback verify →
  `publish` の順序で動作し、Publish 失敗で `PublishFailure` の
  Effect/Phase/RecoveryHandle を `RenderError::Publish` に保持する。
  `DestinationExists` は既存の専用 variant へ写像される。
- [ ] AC4: WAV 正常系・ZeroFrames・PCM precision・RIFF/byte golden・
  Finalize 失敗・Cancel・DiskFull・DestinationExists・
  publish failure・abort/Drop の既存試験が新 API で維持され、
  finalize→seal→readback 独立 decode→publish→再 Open の
  E2E が追加される。
- [ ] AC5: 4GiB 境界・NaN/Inf/Precision の既存 contract 試験が
  維持される（`ruviform-io-wav` contract test 内）。
- [ ] AC6（候補報告・本 Issue 外の Gate）: AudioLens/Synvelis の
  候補 revision 更新と RF #306/#294・AL #142/SV #83 の Gate は
  Consumer 側工程であり、本 spec では候補 SHA と検証結果の提供に
  とどめる。未実測を PASS としない。

## Deferred Design Concerns

- `verify` hook に `&mut SealedFile` を渡す形のまま、Readback が
  `Send` な `MediaSource` を直接満たせない点 | `ReadbackFile` は
  `Read + Seek`（Send 非保証）。検証側は readback からの読取で
  独立 source（例: `Cursor<Vec<u8>>`）を構成して decode する |
  `ReadSeek: + Send` の io-file 側拡張は Consumer 実需が固まってから。
- `RenderReceipt` が `Clone` を失うこと | `PublishReceipt` 保持の
  対価。スカラ複製の Facade を作ると RecoveryHandle が失われるため
  非採用 | Consumer が `Clone` を要求する実需があれば再検討。

## Sources and Relations

- 正本: Issue #305 本文（作業範囲・Acceptance・横断レビュー追補）。
- 設計基準: Issue #287（IO-05）の対応表・順序・非互換方針。
  Issue #283（IO-01）v0.4 契約・Issue #282 統合設計。
- 前提実装: Issue #295（StagedFile 型状態・Mock Publish）、
  Issue #296（OS Atomic Publish・Durability・Recovery）— 両者 Closed。
- 既存実装の参考: `crates/ruviform-media-io/src/staged.rs`（撤去対象）、
  `crates/ruviform-audio/src/render.rs`（移行対象）、
  `crates/ruviform-io-file/src/staged.rs`（移行先 API）。
