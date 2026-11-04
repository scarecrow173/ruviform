---
id: "TSPEC-034RoRhmlBaEa2BqB4gYX1"
type: "test-spec"
status: "approved"
title: "Issue #295 ruviform-io-file Mock-driven publish 契約テスト仕様"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-file/src/staged.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/mock.rs"
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
    - "docs/designs/issue-295-io-file-staged-publish.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #295 ruviform-io-file Mock-driven publish 契約テスト仕様

## Purpose

`ruviform-io-file` の publish 契約 — 型状態・失敗分類・所有権/回復規則 — を
実 OS 非依存の Fault Inject Mock で pin する。Issue #295（AC1–AC6）と
IO-01 v0.4 の結果モデル（`PublishEffect`・`PublishPhase`・
`DurabilityEvidence`・`TempState`・RecoveryHandle）を検証する。
実 OS の原子性・耐久性の達成は IO-IMP-02 の Gate であり本 spec では検証しない。

## Feature

Staged publish パイプラインの決定的分類: `Validate → FileSync → Close →
Publish → DirectorySync → Cleanup` の各段階で注入した失敗が、設計どおりの
`effect`/`phase`/`evidence`/`temp_state`/`RecoveryHandle` に分類され、
`Ok(PublishReceipt)` は要求達成時にのみ返る。

## Rules

- R1（型状態）: `StagedFile` は `Write+Seek+Send`、`SealedFile` は `Write`/
  `Seek` を実装しない（compile-fail）。`seal` は消費遷移で `publish` は
  `SealedFile` にのみ存在する。
- R2（読み戻し）: `open_readback` は `Read+Seek` のみの借用 Reader を返し、
  初期位置 0・Temp と同一対象。borrow 生存中は `publish`/`abort`/再借用は
  コンパイル不可。アダプタ非対応時は `FileErrorKind::Unsupported`。
- R3（要求達成時のみ Ok）: FileSync 要求で `sync_file` 失敗 →
  `Err{phase:FileSync, effect:NotPublished}`。FileAndDirectorySync 要求で
  `sync_directory` 失敗 → `Err{phase:DirectorySync, effect:Published,
  evidence:FileSynced}`（Ok で誤魔化さない・未公開と誤報もしない）。
- R4（Publish 分類）: `NotApplied`→`effect:NotPublished`、
  `NotApplied`+`AlreadyExists`→`kind:DestinationExists`、`Unknown`→
  `effect:Indeterminate` かつ自動 cleanup しない。
- R5（evidence は実績）: Receipt/Failure の `durability` は成功した同期
  段階のみ（None/FileSynced/FileAndDirectorySynced）。要求値を偽装しない。
- R6（Cleanup 分離）: publish 成功後の temp 残留 (`Retained`) は cleanup へ
  進み、再失敗でも `Ok(Receipt{temp_state:Retained/Unknown,
  recovery:Some})` — 公開成立を覆さない。
- R7（所有権/回復）: `abort` は temp 削除し `CleanupReceipt`、失敗時は
  `CleanupFailure` が `RecoveryHandle` を返し `cleanup()` で再試行可能。
  `RecoveryHandle`/`PublishFailure`/`PublishReceipt` の Drop は temp を
  削除しない。`Indeterminate` 時の自動削除・再 publish をしない。
- R8（入口の正直さ）: `StagedFile::create` は OS backend 未実装のため
  `FileErrorKind::Unsupported`（IO-IMP-02 委譲を rustdoc に明記）。
  file name を持たない宛先は `InvalidInput`。

## Examples

- `Write+Seek+Send` の静的検証: `assert_impl::<StagedFile>()` がコンパイル。
  `SealedFile` に `write_all` を呼ぶ `compile_fail` doctest が失敗する。
- publish 行列（Mock）: `effect ∈ {Applied,NotApplied,Unknown}` ×
  `temp ∈ {Removed,Retained,Unknown}` × `durability ∈ {BestEffort,FileSync,
  FileAndDirectorySync}` × 各段階の注入失敗 → 期待 `effect`/`phase`/
  `evidence`/`temp_state` に分類される。
- `Applied` + `Retained` + cleanup 失敗 → `Ok` で `temp_state=Retained`、
  `receipt.take_recovery_handle()` → `handle.cleanup()` で `Removed`。
- `Unknown` publish → `Err{effect:Indeterminate}`、Drop しても Mock FS の
  temp が残る。`failure.take_recovery_handle().inspect()` が temp 状態を
  報告する。
- `abort` で temp 削除。abort の cleanup 失敗 → `CleanupFailure` から
  handle を取り出し再 `cleanup()` で `Removed`。

## Guarantees

- `publish()` が `Ok` を返すのは要求した mode/visibility/durability を
  adapter が達成した場合のみ。
- `NotPublished` は「公開されなかった」が確定した場合のみ。曖昧さは
  `Indeterminate` に正規化される。
- `PublishFailure`/`PublishReceipt`/`RecoveryHandle`/`CleanupFailure`
  の所有権移動で temp の二重削除・自動削除（結果不明時）が起きない。
- `ReadbackFile` から `File`・clone 可能 handle・書込面へ到達する API が
  存在しない。

## Non-goals

- 実 OS の CreateNew/Replace 原子性・File/Directory Sync の物理的達成
  （IO-IMP-02）。実 FS・実プロセス・権限・容量不足の試験。
- 旧 `StagedOutput` の互換/移行検証（IO-05/IO-IMP-11）。
- `PublishAttempt` の内部語彙そのものの外部公開（private のまま）。

## Risk

- 分類を誤ると「rename 成功後の dir sync 失敗を未公開と誤報」や
  「Indeterminate 残留物の自動削除」で Consumer 側のデータを失う。
- `create` の Unsupported 状態を隠すと Consumer が利用可能と誤認する。
- Mock が実 OS と乖離した契約を pin すると IO-IMP-02 の実装が公開 API を
  歪める原因になる。
