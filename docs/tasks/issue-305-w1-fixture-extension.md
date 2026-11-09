---
id: "TASK-034UwchDKTHXsAzupuKHkC"
type: "task"
status: "in-progress"
title: "W1 Issue #305 io-e2e WAV golden/fault fixture 実装"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/305"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/wav_golden.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/wav_fault.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "crates/ruviform-io-e2e/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-305-io-e2e-wav-golden-fault.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-305-io-e2e-wav-golden-fault.md"
  blocks:
    - "docs/tasks/issue-305-w2-verification-handoff.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-305-io-e2e-wav-golden-fault.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-305-io-e2e-wav-golden-fault.md"
  references: []
  defers: []
  deferred-by: []
---

# W1 Issue #305 io-e2e WAV golden/fault fixture 実装

## Work

`crates/ruviform-io-e2e/tests/` に 2 つの fixture を追加する。
公開 API のみ消費し、`vfs`/内部 mock/新規 dev-dependency は使わ
ない。

- `wav_golden.rs`: `WavMuxer`/`WavEncoder` → `&mut StagedFile` →
  `finalize` → `seal` → `publish` → destination bytes を、
  RIFF/WAVE 仕様から独立に組み立てた期待 byte 列と比較する
  golden。ZeroFrames/`WAVEFORMATEX`/`WAVEFORMATEXTENSIBLE`/
  S16 payload byte-exact/encoder edge sample（U8・S16・S24・
  S32・F32 の NaN/±Inf/clip/`.5` 丸め/F32 verbatim）/
  4GiB 境界（`DevNull` sink）をカバーし、独立 decode
  （`ruviform-io-symphonia`）と照合する。
- `wav_fault.rs`: `&mut StagedFile` へ委譲する sink wrapper
  （`FlushFail`・`DiskFull`）で finalize flush 失敗・mid-stream
  書込失敗を注入し、`ErrorKind::Io`・receipt 無し・abort 後の
  destination/temp 非残存を assert。seal 後 abort（Cancel）と
  muxer 未 finalize Drop（best-effort finalize は完了証拠に
  ならない）を pin。publish outcome の `Published`/
  `NotPublished`/その他 effect の三分類を明示する
  `classify` helper を置き、`CreateNew` 成功・競合の実測のみ
  fixture 化する（`Published`/`Indeterminate` 失敗 effect は
  誘発不能のため NotRun — test 内コメントと evidence に記録）。
- `README.md`/`src/lib.rs` の fixture 一覧を更新する。

## Classification

- `normal-plan-task`

## Upstream Documents

- [docs/plans/issue-305-io-e2e-wav-golden-fault.md](../plans/issue-305-io-e2e-wav-golden-fault.md)
- [docs/specs/issue-305-io-e2e-wav-golden-fault.md](../specs/issue-305-io-e2e-wav-golden-fault.md)
- [docs/designs/issue-305-io-e2e-wav-golden-fault.md](../designs/issue-305-io-e2e-wav-golden-fault.md)
- [docs/test-specs/issue-305-io-e2e-wav-golden-fault.md](../test-specs/issue-305-io-e2e-wav-golden-fault.md)

## Dependencies

- Issue #305 T1（`StagedFile` 移行・`wav_staged.rs`）merge 済み。

## Affected Files

- `crates/ruviform-io-e2e/tests/wav_golden.rs`（新規）
- `crates/ruviform-io-e2e/tests/wav_fault.rs`（新規）
- `crates/ruviform-io-e2e/README.md`
- `crates/ruviform-io-e2e/src/lib.rs`

## Done When

- [ ] `wav_golden.rs` が ZeroFrames/PCM/RIFF 両形式/4GiB/
      NaN/Inf/Precision/Byte Golden を網羅し、独立 decode と
      照合する
- [ ] `wav_fault.rs` が Finalize 失敗/Cancel/DiskFull 相当/
      publish outcome 三分類を網羅し、typed 分類 + 非残存を
      assert する
- [ ] 旧 `StagedOutput`/旧 `PublishOptions` を参照しない・
      新規依存を追加しない
- [ ] `cargo test -p ruviform-io-e2e` が新規 fixture を含め
      pass（Windows NTFS 実測）

## Verification

- [ ] `mise run fmt-check` / `check-io-e2e` / `lint-io-e2e` /
      `test-io-e2e` pass
