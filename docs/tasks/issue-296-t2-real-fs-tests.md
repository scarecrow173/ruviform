---
id: "TASK-034RBIX9gn32W8GMyhNMqw"
type: "task"
status: "done"
title: "T2 実 FS 結合試験・複数プロセス競合 (Issue #296)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-296-io-file-os-backend.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-296-io-file-os-backend.md"
    - "docs/tasks/issue-296-t1-os-adapter.md"
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
    - "docs/test-specs/issue-296-io-file-os-backend.md"
  references: []
  defers: []
  deferred-by: []
---

# T2 実 FS 結合試験・複数プロセス競合 (Issue #296)

## Work

`crates/ruviform-io-file/tests/os_publish.rs` を新設し、実 FS
（`std::env::temp_dir` 配下の一意 workdir）結合試験を書く:
(1) 複数プロセス CreateNew 競合 — `current_exe` を gate ファイル制御で
    N 子プロセス再実行し、ちょうど 1 つが `PublishReceipt`、他が
    `DestinationExists`+`NotPublished`、宛先 byte 不変を検証。
(2) CreateNew/Replace・durability evidence・readback・seek の実 FS 試験。
(3) `RecoveryHandle` の実 temp inspect/cleanup。
(4) 誘導可能な実故障: 宛先ディレクトリへの Replace、親不在 create、
    Windows 読み取り専用宛先 Replace、unix 書込不可 dir。
(5) adapter 単体試験: temp 名生成の一意性・エラー分類テーブル。
誘導不能な失敗は Mock 駆動のままとし、未検証範囲を記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-296-io-file-os-backend.md`
- `docs/designs/issue-296-io-file-os-backend.md`
- `docs/specs/issue-296-io-file-os-backend.md`

## Dependencies

- 先行: `docs/tasks/issue-296-t1-os-adapter.md`
- 依存: なし（std テスト機構のみ）

## Affected Files

- `crates/ruviform-io-file/tests/os_publish.rs`（新設）
- `crates/ruviform-io-file/src/os.rs` 内 `#[cfg(test)]` 単体試験

## Done When

 - [ ] 複数プロセス CreateNew 競合が実 FS で 1 勝者のみ・他 DestinationExists
- [ ] 既存宛先の非上書きが byte-for-byte で検証される
- [ ] RecoveryHandle inspect/cleanup が実 temp で動作する
- [ ] `mise run test` で全試験が pass

## Verification

 - [ ] `mise run test`（Windows NTFS・dev profile）で os_publish 全緑
- [ ] 誘導不能な失敗範囲が test-spec と一致して記録される
