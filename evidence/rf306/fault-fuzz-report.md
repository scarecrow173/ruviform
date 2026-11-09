# RF306 slice-2 実行証拠レポート — Fault/Fuzz fixture・OS/FS probe・Rooted boundary

対象: Issue #306（IO-IMP-12）2 本目の slice。`crates/ruviform-io-e2e`
の tests/fixture 層のみを拡張し、本番コード・公開 API・
`#[cfg(test)]` 内部 mock は一切変更していない。

作業 worktree: `C:\Users\akiyoshi\orca\workspaces\ruviform\rf306-io-e2e`
branch: `feature/306-io-e2e-fault`
base: `origin/main` = `9017934a863fd4c1bce0bb41c08560cf9fa13dc9`
（branch 起点。push 前に `git fetch origin main && git merge
origin/main` を実施 — 結果は PR 本文に記録）
環境: Windows 11 x64 (MINGW64_NT-10.0-26300) / rustc 1.99.0
(b940084d7 2026-09-28) / cargo 1.99.0 / NTFS (TEMP 配下の実 FS)
実行日時: 2026-11-08 (UTC)

## 追加した試験（29 件・公開 API のみ消費）

| ファイル | 件数 | 内容 |
| --- | --- | --- |
| `tests/structured_formats_staged.rs` | 8 | YAML/TOML `write_file→publish→read_file` 往復、XML event parse、publish 後 bytes 改竄・truncate の型付き decode 拒否（`DocumentFileReadError::Decode` + `TrailingData`/`Syntax` 系 kind） |
| `tests/fault_boundary.rs` | 6 | missing-parent create → `DocumentFileWriteError::Create`、destination dir 競合 → `Publish`、`PublishFailure` の effect/phase/kind/durability/recovery handle 所有権、CreateNew 競合で先勝ち bytes 保全 + temp cleanup、TAR+gzip 複合を publish 前に独立 verify |
| `tests/negative_fuzz.rs` | 10 | ZIP central-directory `external_attrs` patch による symlink/special entry 分類と extraction 拒否・staging 非残存、ZIP bomb（declared total・entry 数・streamed budget）、XML DOCTYPE/内部 subset/未定義 entity 拒否、corrupt checksum・truncated TAR・corrupt gzip・invalid UTF-8 JSON、決定的 mutation sweep（gzip trailer/magic/mid-stream/ISIZE・TAR terminator・TAR checksum）で publish gate が unverified composite を通さないこと |
| `tests/os_fs_probe.rs` | 1 | publish durability ladder（CreateNew/Replace × BestEffort/FileSync/FileAndDirectorySync = 6 通り）を実 FS で実行し、測定 `DurabilityEvidence` が要求 level と一致すること、または `PublishFailureKind::Unsupported` で正直に拒否されることを検証 |
| `tests/rooted_boundary.rs` | 4 | `RootedDir`/`RelativePath` を publish 済み tree・extraction 済み tree に適用。NoFollow read・`read_dir` 列挙・絶対パス/`..`/`\`/制御文字 locator の `InvalidInput` 拒否・root 外へ escape する link の `WithinRoot`=`EscapesRoot` / `NoFollow`=`NotRegularFile` 分類。link 生成不能環境では `eprintln!` で skip を明示 |

既存 31 件と合わせて `ruviform-io-e2e` は 60 件。

## 実施した検証（全て実走・実値）

| 検証 | コマンド | 結果 | 備考 |
| --- | --- | --- | --- |
| fmt | `mise run fmt-check` | Pass (exit 0) | `cargo fmt --all -- --check`（実装中に `cargo fmt -p ruviform-io-e2e` で整形済み） |
| crate check | `mise run check-io-e2e` | Pass (exit 0) | `cargo check -p ruviform-io-e2e --all-targets` |
| crate lint | `mise run lint-io-e2e` | Pass (exit 0) | `cargo clippy -p ruviform-io-e2e --all-targets -- -D warnings` |
| crate test | `mise run test-io-e2e` | Pass (exit 0) | 60 tests 全 pass（0 failed / 0 ignored） |
| workspace check | `mise run check` | Pass (exit 0) | `cargo check --workspace --all-targets` |
| workspace lint | `mise run lint` | Pass (exit 0) | `cargo clippy --workspace --all-targets -- -D warnings` |
| workspace test | `mise run test` | Pass (exit 0) | `cargo test --workspace` 全 pass |
| RT lane | `mise run test-rt` | Pass (exit 0) | `cargo test --workspace rt_` name filter。`ruviform-io-e2e` は非 RT のため対象外（crate README に明記済み） |

補助証拠（canonical = 上記 mise タスク）: `cargo test -p
ruviform-io-e2e --test negative_fuzz` 等の個別 cargo 実行は
デバッグ用途のみで、合否判定には使用していない。

## OS/FS durability 実測（`os_fs_probe` 出力・実値）

実行環境: Windows 11 x64 / NTFS / rustc 1.99.0。
`DurabilityEvidence` は要求 level に一致して観測された
（silent downgrade なし）。

```
OS/FS durability probe (6 success, 0 unsupported):
mode=CreateNew durability=BestEffort -> OK evidence=no sync performed
mode=CreateNew durability=FileSync -> OK evidence=file synced
mode=CreateNew durability=FileAndDirectorySync -> OK evidence=file and directory synced
mode=Replace durability=BestEffort -> OK evidence=no sync performed
mode=Replace durability=FileSync -> OK evidence=file synced
mode=Replace durability=FileAndDirectorySync -> OK evidence=file and directory synced
```

| OS/FS | 状態 | 根拠 |
| --- | --- | --- |
| Windows 11 x64 / NTFS | **Pass（実測）** | 上記 6/6 成功・evidence 一致 |
| macOS 実機 | **NotRun** | 実機環境なし |
| Linux 実機 | **NotRun** | 実機環境なし |
| Network/Cloud FS (SMB/NFS/OneDrive 等) | **Unsupported** | 実測環境なし。durability 保証は FS 依存のため主張しない |
| 性能特性 | **NotRun** | 本 slice の対象外 |

## Fault 分類の正直さ

| fault class | 本 slice での扱い | 根拠 |
| --- | --- | --- |
| `PublishEffect::NotPublished` + typed `PublishFailure`（Create/Publish 競合） | **実測 Pass** | `fault_boundary.rs` で公開 API 経路を実走 |
| `PublishEffect::Applied`（成功 publish の durability evidence） | **実測 Pass** | `os_fs_probe.rs` + 既存 staged 系試験 |
| `PublishEffect::Indeterminate` | **Unsupported（fixture 化せず）** | 実 OS で正直に誘発不能。`crates/ruviform-io-file/src/staged.rs` の `#[cfg(test)]` MockFileOps 試験が正本 |
| Directory-sync 後失敗 | **Unsupported（fixture 化せず）** | 同上。mock のみが phase を制御可能 |
| `TempState::Unknown` / temp 残留 recovery | **Unsupported（fixture 化せず）** | 同上。公開 API からは `PublishFailure::take_recovery_handle` の所有権移譲のみ検証（`fault_boundary.rs`） |
| link 依存試験 | 条件付き Pass | link 生成失敗時は `eprintln!` で skip を明示し pass を偽装しない（本環境 Windows では junction 作成に成功し実測） |

## 発見・修正した試験側の問題（本番コードは無変更）

1. `TarArchiveReader::finish` は単一 zero block を終端として受理
   するため、「terminator 1 block 欠落」は mutant にならない。
   両 zero block（1024 bytes）を除去する mutant に修正した。
2. `TarEntryReader` の payload read 失敗は typed `ArchiveError`
   を `io::Error` に包んで返す。`drain_tar` helper は
   `entry.finish()` で真の `ArchiveErrorKind`（`Truncated`）を
   再取得する形に修正した。
3. Windows では open handle を持つ directory が delete-pending
   状態で `try_exists` に残る。extraction tree の
   `RootedDir`/file handle を scope で解放してから
   `cleanup()` + 非存在 assert する形に修正した。

## #219/#294 への引き渡し（Consumer 実測は本 slice 対象外）

AudioLens（#139/#143）・Synvelis（#82/#84）の consumer 実測は
cross-repo gate（#219/#294）の責務であり、本 slice では
**NotRun**。以下を判断材料として引き渡す。consumer 側
repository は変更していない。

- **Test Command（推奨）**: `mise run test-io-e2e`
  （= `cargo test -p ruviform-io-e2e`）。consumer 側 CI で
  ruviform を path/git 依存として取り込む場合は同等の
  `cargo test -p ruviform-io-e2e` を実行する。
- **Fixtures**: 本 crate の `tests/` は実 FS（`tempfile` 系
  TestDir）上で完結し、外部サービス・ネットワーク・実デバイス
  を必要としない。AudioLens/Synvelis が依存する公開 API 面
  （`StagedFile` publish・Structured `write_file`/`read_file`・
  Archive secure extraction・WAV finalize/seal/readback）は
  すべて本試験で消費済み。
- **判断基準**: consumer 側で IO 経路を変更した場合、
  `test-io-e2e` 全 60 件の pass を合格条件とする。
  `os_fs_probe` の durability evidence は **Windows NTFS のみ
  実測**であり、consumer が macOS/Linux・Network/Cloud FS を
  対象にする場合は当該環境での再実測が必要（本証拠を根拠に
  それら環境の Supported を主張しない）。
- **片側 pass の扱い**: AudioLens と Synvelis のいずれか一方
  のみ pass した状態を両 Consumer 合格とはみなさない
  （Issue 契約どおり）。

## 対象外として記録する残件

- AudioLens #139・Synvelis #82 の consumer 実測 — **NotRun**
  （#219/#294 gate。本 report の引き渡し節を参照）
- #305 旧 Media File API 撤去との接続 — 別 Issue の責務
- macOS/Linux 実機・Network/Cloud FS — **NotRun/Unsupported**
- `Indeterminate`・DirSync 後失敗・`TempState::Unknown` —
  **Unsupported**（公開 API から誘発不能。`ruviform-io-file`
  `#[cfg(test)]` mock 試験が正本）
