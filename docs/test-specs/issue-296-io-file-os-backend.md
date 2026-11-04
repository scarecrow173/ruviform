---
id: "TSPEC-034RBIVDSp1VYbb5Tt3x9B"
type: "test-spec"
status: "approved"
title: "Issue #296 ruviform-io-file 実 OS backend テスト仕様"
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
    - "docs/designs/issue-296-io-file-os-backend.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-296-io-file-os-backend.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---
# Issue #296 ruviform-io-file 実 OS backend テスト仕様

## Purpose

`ruviform-io-file` の実 OS backend（IO-IMP-02 / Issue #296）が実 FS 上で
公開契約どおりの原子性・耐久性・回復を提供することを pin する。
#295 の Mock 駆動分類は変えず、本 spec は「実 OS で成立する」側を検証する。
未検証の OS/FS・誘導不能な失敗はここで正直に範囲外と記録する。

## Feature

実 OS PlatformFileOps による staged publish: temp sibling 生成 →
write/seek/readback → 要求耐久性の sync → OS 別 atomic publish →
dir sync → cleanup を、実 FS 上で検証する。

## Rules

- R1（temp 生成）: temp は宛先の sibling・`*.rvf-tmp-*` 命名・
  `create_new` で排他作成され、create 時点で宛先を一切触らない。
  宛先 parent 不在では `create` が `FileErrorKind::Io` で失敗する。
- R2（write/seek/readback）: 実ファイルへの `Write`/`Seek` は
  cursor/ゼロフィルが正しく、`open_readback` は位置 0 開始の
  `Read+Seek` で staged 内容を返し、write cursor を汚染しない。
- R3（CreateNew・排他）: 宛先が既存なら `kind:DestinationExists,
  effect:NotPublished`・宛先 byte 不変・temp は `RecoveryHandle` に
  `Retained` として残る。複数プロセスの同時 `CreateNew` は
  ちょうど 1 つが成功し他は `DestinationExists`。
- R4（Replace）: 既存宛先を staged 内容で置換し `PublishReceipt`。
  宛先がディレクトリの場合は `NotPublished`+`Io` で拒否。
- R5（durability evidence）: `BestEffort`→`None`、`FileSync`→
  `FileSynced`、`FileAndDirectorySync`→`FileAndDirectorySynced` を
  実 FS で観測。Rename 後 DirSync 失敗は `Err`+`Published`+
  `FileSynced` で `Ok` にしない（Mock で pin。実 FS での強制不能分）。
- R6（回復）: `RecoveryHandle::inspect` が実 temp を `Retained`/`Absent`
  で報告し、`cleanup()` で実削除 → `Absent`。`abort`/drop で temp が
  実 FS から消える。`Indeterminate` temp を無条件削除しない（Mock で pin）。
- R7（エラー誠実性）: 保証不能な FS/target は `Unsupported` 系を返す
  設計であり、確定的失敗は `NotApplied`、ネットワーク系のあいまい
  失敗は `Unknown`/`Indeterminate` に分類される（分類表の単体検証）。
- R8（契約不変）: 公開 API・失敗分類語彙は #295 のまま。`PublishFailure`
  と `RecoveryHandle` が Consumer へそのまま渡る。

## Examples

- 競合例: 8 子プロセスが同一 `dest` に gate 解除後一斉 `CreateNew`
  → exit(0) は 1 プロセスのみ、他は DestinationExists、dest は勝者の
  payload と byte 一致。
- 回復例: 既存 dest へ `CreateNew` → `PublishFailure` →
  `take_recovery_handle().inspect()==Retained` → `cleanup()` →
  `temp_state()==Absent`・実 FS に temp が存在しない。
- 非上書き例: dest 既存バイト列を事前配置 → `CreateNew` 失敗後も
  dest が byte-for-byte 不変。

## Guarantees

- 試験は全て実 FS（`std::env::temp_dir` 配下の一意 workdir）で行い、
  Mock を使わない。複数プロセス試験は `current_exe` 再実行の子
  プロセスで実プロセス競合を作る。
- 試験不能・誘導不能な失敗（DirSync 強制失敗・真の `Indeterminate`・
  あいまいネットワークエラー実注入）は Mock 駆動に留め、本 spec の
  検証範囲外として記録する。
- Linux/macOS の実機検証は CI OS matrix へ委譲（本環境 Windows NTFS
  で実測済みの範囲と分けて報告する）。

## Non-goals

- クラッシュ実機試験（電源断・SIGKILL 中の中途 temp 観測）、
  ネットワーク FS/ReFS/FAT32 実機、ベンチ・性能測定。
- `exists()`→`rename()` の不在を静的に検査すること — 実装レビューと
  設計文書で担保（機械的検査対象外）。

## Risk

- 実 FS 依存テストは CI の FS 差で flake し得る — 権限試験は
  `cfg(unix)` に限定し、Windows では読み取り専用宛先の Replace 失敗を
  代替として使う。
- 子プロセス gate 同期はタイムアウト付きで行い、CI 遅延時も hang しない。
