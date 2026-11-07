---
id: "TASK-9KS9Fd9Lzf1TjWIavKU8Uz"
type: "task"
status: "done"
title: "T1 Media I/O 旧 StagedOutput 撤去・StagedFile 移行 (Issue #305)"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source: []
  changes:
    added: []
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
  implements:
    - "docs/plans/issue-305-media-io-staged-file-cutover.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-305-media-io-staged-file-cutover.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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

# T1 Media I/O 旧 StagedOutput 撤去・StagedFile 移行 (Issue #305)

## Work

- `crates/ruviform-media-io/src/staged.rs` を削除し、`lib.rs` の
  `mod staged`・`pub use staged::{PublishOptions, StagedOutput}` を
  撤去。`sink.rs` の module doc を `ruviform-io-file` 契約へ更新。
- `crates/ruviform-audio/Cargo.toml` に `ruviform-io-file` 依存を追加
  （render のみ非 RT で消費）。`render.rs` を `StagedFile`/
  `SealedFile`/`PublishOptions`/`PublishReceipt`/`PublishFailure`
  契約へ移行:

  - `RenderOutput.staged: StagedFile`、
    `publish_options: ruviform_io_file::PublishOptions`。
  - `make_muxer: for<'a> FnOnce(&'a mut StagedFile) ->
    Result<Box<dyn Muxer + 'a>, MediaError>`。
  - `verify: Option<FnOnce(&mut SealedFile, &OutputFacts) ->
    Result<(), MediaError>>`。
  - `publish: FnOnce(SealedFile, PublishOptions) ->
    Result<PublishReceipt, PublishFailure>`。
  - `RenderError::Publish { source: PublishFailure }` に変更し
    `PublishFailureKind::DestinationExists` → `DestinationExists`
    写像。非 Indeterminate の publish 失敗は
    `RecoveryHandle::cleanup` を best-effort 実行。
  - `RenderReceipt` に `publish: PublishReceipt` を追加し
    `Clone` derive を除去。
  - `tests/render.rs` を移行（`MuxerCtor`/`VerifyFn`/`PublishFn`
    型・`FailSink` を `FailSink<'a> { inner: &'a mut StagedFile }`
    借用に変更・`verify_wav` を `&mut SealedFile` +
    `open_readback` + `Cursor` 独立 decode へ・publish 失敗注入は
    実 FS 失敗へ）。

- `crates/ruviform-io-wav/Cargo.toml` に `ruviform-io-file` dev 依存
  追加。`tests/contract.rs` を `StagedFile` API へ移行し、
  `WavMuxer<&mut StagedFile>` + finalize → seal → `open_readback` →
  独立 decode → `publish` → 再 Open E2E を追加。
  `src/lib.rs`・`src/muxer.rs`・`README.md` の `StagedOutput` 言及を
  `ruviform-io-file` 契約へ更新。
- `origin/main` 取込 → `mise run fmt`/`check`/`lint`/`test` →
  コミット → push → PR 作成 → IR 記録。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-305-media-io-staged-file-cutover.md`
- `docs/designs/issue-305-media-io-staged-file-cutover.md`
- `docs/specs/issue-305-media-io-staged-file-cutover.md`
- `docs/test-specs/issue-305-media-io-staged-file-cutover.md`
- Issue #305 / #287 / #295 / #296

## Done Criteria

- AC1–AC5（spec）を満たす: 旧 API ゼロ・依存 DAG 維持・
  render の StagedFile 移行・WAV/失敗行列の新 API 緑・
  E2E 追加・未実測を PASS にしない。
- `mise run fmt`/`check`/`lint`/`test` 全緑（Windows 11 x64、
  `CARGO_BUILD_JOBS=2`）。
- `cargo tree -p ruviform-media-io` が Domain Core3 のみ（補助）。
- 独立レビュー（review-protocol）で blocker ゼロ。
