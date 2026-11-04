---
id: "SPEC-034RBEkXlCcpgzIaiikzso"
type: "spec"
status: "approved"
title: "Issue #296 ruviform-io-file — 実 OS Atomic Publish・Durability・Recovery (IO-IMP-02)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/296"
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-file/src/os.rs"
      - type: "file"
        path: "crates/ruviform-io-file/tests/os_publish.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
      - type: "file"
        path: "crates/ruviform-io-file/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/platform.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/staged.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/media-io-backend-abstraction.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #296 ruviform-io-file — 実 OS Atomic Publish・Durability・Recovery (IO-IMP-02)

## What Should Be Built

Issue #295 で確定した `ruviform-io-file` の公開契約（`StagedFile`/`SealedFile`
型状態・`PublishOptions` 3 軸・`PublishReceipt`/`PublishFailure` 分類・
`RecoveryHandle`）と private `PlatformFileOps` 抽象の**実 OS バックエンド**を
実装する。対象 OS は Linux・macOS・Windows。

`staged_file()` factory が返す private Adapter が以下を実 OS API で提供する:

- **排他的 CreateNew** — 宛先が既存なら適用しない真のアトミック
  no-replace プリミティブ。Linux は `renameat2(RENAME_NOREPLACE)`、macOS は
  `renamex_np(RENAME_EXCL)`、Windows は `MoveFileExW(temp, dest, 0)`（非
  `MOVEFILE_REPLACE_EXISTING`）。`exists()`→`rename()` の組合せは
  一切使用しない。プリミティブ不在/非対応 FS では検証済みの
  `link()+unlink()` フォールバック（POSIX）または `Unsupported`/不確定の
  正直な報告。
- **Atomic Replace** — `rename(2)`（POSIX）/ `MoveFileExW` 系（Windows）。
- **File sync / Directory sync** — `fsync`/`FlushFileBuffers` と親
  ディレクトリの同期（POSIX は dir fd の fsync、Windows は
  `FILE_FLAG_BACKUP_SEMANTICS` + `FlushFileBuffers`）。
- **Close / Cleanup / Temp 分類** — publish 前のハンドル解放、temp の
  `Removed`/`Retained`/`Unknown` 分類、`ApplyOutcome` への
  `Applied`/`NotApplied`/`Unknown` 分類。ネットワーク FS 系の
  あいまいエラーは `Unknown`（→ `Indeterminate`）へ、確定的な拒否は
  `NotApplied` へ正規化する。

## Why Now

#295 は公開契約と Mock 駆動パイプラインのみを納め、`StagedFile::create`
は `FileErrorKind::Unsupported` を返す状態でマージされた。Consumer
（AL #143・SYN #84・RF #305/#306）が実 OS 動作を必要とするため、
実 OS バックエンドが IO チェーン（IO-06 #288 の共有 OS/FS 受入、
#219 Cross-Repo gate）の次の必須ピース。型は #295 で確定済みのため、
本 Issue は adapter 実装と実 OS 検証に集中できる。

## Users and Value

- AudioLens / Synvelis / Ruviform 本体の format layer —
  `StagedFile`/`SealedFile` が実 FS 上で原子公開・耐久性・回復を
  本当に提供するようになる。
- 後続 Issue（#297・#300・#302・IO-06 #288 の受入）が依存する
  実動作前提の土台。

## Scope

### In Scope

- `PlatformFileOps` の実 OS 実装（private。`platform.rs` の `staged_file()`
  factory が返す具象 adapter。`src/os.rs` + `cfg(unix)`/`cfg(windows)` 実装）。
- 排他的 `CreateNew`・`Replace`・file/dir sync・close/cleanup・inspect の
  各段階の実 OS 実装と分類語彙への正規化。
- temp sibling の一意生成・同一 FS 保証・`Write`/`Seek`/readback の
  実ファイル実装。
- 実 FS 結合試験（`tests/os_publish.rs`）: 複数プロセス CreateNew 競合、
  既存宛先の非上書き、各 durability の evidence、回復ハンドル経由の
  inspect/cleanup、誘導可能な実故障（既存宛先・宛先がディレクトリ・
  親不在・権限）。
- OS/FS × mode/durability の保証範囲・Path root 信頼境界・
  Symlink/Network FS・Windows 共有 Mode・Temp identity の保証範囲を
  rustdoc + README + 設計書に文書化。
- `libc`（unix）と `windows-sys`（Windows）の target 限定依存追加
  （std の安定 API では atomic no-replace rename が表現不能なため。
  workspace 既出バージョン `libc 0.2`・`windows-sys 0.61` に合わせる）。
- `unsafe` は既存規約どおり `#[expect(unsafe_code, reason=…)]` + SAFETY
  コメントの局所例外のみ。

### Out of Scope

- Generic Directory Transaction・複数ファイル atomic・CAS/conditional
  replace・リビジョン管理・永続 Journal。
- 公開 API の再設計・新規公開型（#295 の契約のまま）。
- Consumer 側の移行（#305/#306/#287）。
- `#[cfg]` でカバーしない他 OS（BSD 系・wasm 等）への保証 — 未対応 OS は
  `Unsupported` を返し、本 Issue でサポートを主張しない。
- Mock のクレート外公開。

## Requirements

- 型・公開シグネチャ・分類語彙は #295 契約を変更しない（private adapter
  のみの追加。必要最小限の `RecoveryInfo` temp_path 記録などの内部拡張は
  許容するが公開型の非互換変更は不可）。
- `exists()`→`rename()`・`metadata`→`create` 等の TOCTOU 代替を atomic
  CreateNew として提示しない。
- `Ok(PublishReceipt)` は要求した mode・durability を全て観測した場合のみ。
  DirSync 失敗は `Err{effect:Published, evidence:FileSynced}`。
- `Indeterminate`/不明の temp を `Drop` や cleanup で無条件削除しない。
- 保証を提供できない FS/ターゲットは `Unsupported` か
  `Indeterminate`/`Io` を返し、未検証 `Supported` としない。
- unsafe は局所 `#[expect]` + SAFETY コメント。依存追加は最小 feature。
- MSRV 1.89・edition 2024・`workspace.lints` 継承・依存 DAG（層規約:
  Primitives 相当の非 RT・下位層からのみ依存）維持。

## Acceptance Criteria

- [ ] AC1（排他的 CreateNew・複数プロセス競合）: N 個の別プロセスが同一
      宛先へ `CreateNew` を同時試行した実 FS 試験で、ちょうど 1 プロセスのみ
      `PublishReceipt`、残りは `kind:DestinationExists, effect:NotPublished`、
      既存宛先内容が byte-for-byte 不変。CI（OS matrix）で再現可能。
- [ ] AC2（各段階 Fault Injection + 実 OS）: FileSync/Publish/DirSync/
      Close/Cleanup 各段階の失敗分類を Mock fault injection で網羅し、
      実 OS では誘導可能な失敗（既存宛先・宛先が dir・親不存在・権限・
      読み取り専用宛先など）を実試験する。誘導不能な段階は「Mock 駆動」
      として正直に記録する。Rename 後 DirSync 失敗は Ok/完全達成と
      報告しない（`Err` + `Published` + `FileSynced` evidence）。
- [ ] AC3（Unknown temp の非削除・RecoveryHandle）: publish 効果が
      `Unknown`/`Indeterminate` の temp を無条件削除しないことを Mock で
      検証済みであること（#295 継続）。実 FS 上で RecoveryHandle による
      inspect/cleanup が動くこと（実 temp の `Retained`→inspect→
      `cleanup()`→`Absent`）。
- [ ] AC4（正直な capability）: 保証不能な FS/ターゲットを Supported と
      扱わない — hard-link fallback が使えない FS では CreateNew が
      `Unsupported` 系を返す設計・実装であること。OS/FS × mode/durability
      の試験済み/未試験の範囲が capability matrix 文書と CI ログから
      追跡できること。
- [ ] AC5（公開契約）: Consumer が `PublishFailure`（kind/phase/effect/
      durability/source）と `RecoveryHandle`（inspect/cleanup）を受け取れる
      公開 API のままであること（丸めず・握り潰さず）。MSRV 1.89・
      依存 DAG・non-RT を維持し `mise run lint`/`test`/`check` が通ること。
- [ ] AC6（文書化の保証範囲）: Path root 信頼境界・Symlink/Network FS・
      Windows 共有 Mode・Temp identity・close 段階の報告限界を rustdoc や
      crate README/設計書に明記し、実保証と一致していること。

## Deferred Design Concerns

- `renameat2`/`renamex_np` 非対応カーネル（Linux < 3.15）での link
  fallback 実測 | 実 OS が古い環境が出たら | 未検証保証の誤記リスク。
- `O_TMPFILE`+`linkat(AT_EMPTY_PATH)` 等の別 temp 方式、io_uring 経路 |
  必要性が確認されたら別 Issue | 現時点では YAGNI。
- クラッシュをまたいだ temp の再発見（`RecoveryHandle` の永続化）|
  Consumer 保存モデル固まってから | 診断値を同一性証拠と誤認するリスク。
- POSIX `close(2)` の遅延書込エラー検出（std の `File` drop は close
  エラーを報告しない）| 実害の報告が出たら検討 | `Durability::BestEffort`
  では durability を未主張なため一貫（FileSync 要求時は fsync が
  遅延エラーの観測点になる）| 設計書に制限として明記。

## Sources and Relations

- 正本: Issue #296 本文（作業範囲・Acceptance・実装指針）。
- 設計基準: Issue #283（IO-01）v0.4・Issue #282（統合設計）。
- 実装基盤: `docs/specs/issue-295-io-file-staged-publish-contract.md`・
  `docs/designs/issue-295-io-file-staged-publish.md`（approved）。
- std 参考実装: `std::fs::rename_noreplace`（unstable・issue #161427）
  の内部実装 — Linux `renameat2(RENAME_NOREPLACE)`→`link+unlink`
  fallback・Apple `renamex_np(RENAME_EXCL)`→同 fallback・Windows
  `MoveFileExW(0)`+`FileRenameInfoEx` fallback。
