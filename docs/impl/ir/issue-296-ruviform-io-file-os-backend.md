---
id: "IMPL-034RC7s5qGQoAFceg8oNDP"
type: "impl"
status: "completed"
title: "Issue 296 ruviform-io-file OS backend"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/296"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-file/src/os.rs"
      - type: "file"
        path: "crates/ruviform-io-file/tests/os_publish.rs"
      - type: "file"
        path: "docs/impl/ir/issue-296-ruviform-io-file-os-backend.md"
      - type: "file"
        path: "docs/specs/issue-296-io-file-os-backend.md"
      - type: "file"
        path: "docs/designs/issue-296-io-file-os-backend.md"
      - type: "file"
        path: "docs/plans/issue-296-io-file-os-backend.md"
      - type: "file"
        path: "docs/test-specs/issue-296-io-file-os-backend.md"
      - type: "file"
        path: "docs/tasks/issue-296-t1-os-adapter.md"
      - type: "file"
        path: "docs/tasks/issue-296-t2-real-fs-tests.md"
      - type: "file"
        path: "docs/tasks/issue-296-t3-docs-verify.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-file/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/platform.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/mock.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/staged.rs"
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
      - type: "file"
        path: "crates/README.md"
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
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
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
  implements:
    - "docs/tasks/issue-296-t1-os-adapter.md"
    - "docs/tasks/issue-296-t2-real-fs-tests.md"
    - "docs/tasks/issue-296-t3-docs-verify.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-296-io-file-os-backend.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
    - "docs/plans/issue-296-io-file-os-backend.md"
    - "docs/impl/ir/issue-295-ruviform-io-file-staged-publish-contract.md"
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/283"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 296 ruviform-io-file OS backend

## Summary

IO-IMP-02（Issue #296）として、#295 で確立した private `PlatformFileOps`
契約の実 OS 実装 `OsFileOps`（`src/os.rs`）を追加し、`staged_file`
factory を接続した。unix では `renameat2(RENAME_NOREPLACE)` /
`renamex_np(RENAME_EXCL)` / `link`+`unlink` fallback、Windows では
`MoveFileExW(temp, dest, 0)` を排他的 `CreateNew` に使用し
（`exists()`+`rename` 組合せは不使用）、`Replace` は `fs::rename`、
ファイル sync は `File::sync_all`、ディレクトリ sync は unix では親 dir
`fsync`・Windows では `FILE_FLAG_BACKUP_SEMANTICS`+read-write open の
`FlushFileBuffers` で実現した。エラーは適用可否の三分類
（definitive / ambiguous→`Unknown`→`Indeterminate`）で正規化し、
`RecoveryInfo.temp_path` を実 path で記録する。実 FS 結合試験
`tests/os_publish.rs`（複数プロセス `CreateNew` 競合を含む 16 件）を
追加した。公開 API・型状態・分類語彙は #295 契約のまま変更していない。

## Context

- 実施タスク: [T1](../../tasks/issue-296-t1-os-adapter.md) /
  [T2](../../tasks/issue-296-t2-real-fs-tests.md) /
  [T3](../../tasks/issue-296-t3-docs-verify.md)
- 上流: [plan](../../plans/issue-296-io-file-os-backend.md)、
  [design](../../designs/issue-296-io-file-os-backend.md)、
  [spec](../../specs/issue-296-io-file-os-backend.md)、
  [test-spec](../../test-specs/issue-296-io-file-os-backend.md)、
  前提契約 Issue #295（IO-IMP-01）・親設計 Issue #283（IO-01 v0.4）
- 境界: `PlatformFileOps` trait・Mock・pipeline 分類は #295 のまま再利用。
  Generic Directory Transaction / CAS / 複数 file transaction は範囲外。
  消費者移行は #305/#306、共有 OS/FS 受入は #288、cross-repo gate は #219。

## Implementation

### Changes

- `crates/ruviform-io-file/src/os.rs`（新設）— `OsFileOps`（dest 絶対パス・
  `temp_path`・`dir_path`・`Option<File>`・readback cursor `rpos`）。
  `create` は宛先 file_name を持つ sibling `<name>.rvf-tmp-<token>` を
  `create_new` で排他生成（collision retry 64 回）。`Write`/`Seek` は
  temp のファイルカーソル、readback は positioned read
  （unix `read_at` / Windows `seek_read`）+ `SeekFrom::End` を
  `metadata().len()` で解決する独立 cursor。`sync_file` は
  `File::sync_all`、`prepare_for_publish`/`publish`/`cleanup_temp` は
  rename/remove 前にハンドルを解放（Windows の共有違反回避）。
- atomic `CreateNew` — Linux/Android: 生 `libc::syscall(SYS_renameat2,
  AT_FDCWD, temp, AT_FDCWD, dest, RENAME_NOREPLACE)`（`libc::renameat2`
  wrapper は全 unix で宣言されないため std と同じく raw syscall）。
  `ENOSYS`→`link`+`unlink`、`EINVAL`→fallback 後に非 EEXIST 失敗なら元
  errno を報告（std の再検査規則と同一）。Apple:
  `renamex_np(RENAME_EXCL)`、`ENOTSUP`/`ENOSYS`→`link`+`unlink`。
  その他 Unix: `link`+`unlink`（`link` の `EPERM`/`EOPNOTSUPP`/`ENOSYS`/
  `ENOTSUP` → `Unsupported` 報告）。Windows: `MoveFileExW(temp, dest,
  0)` — `EEXIST`/`ALREADY_EXISTS` → `AlreadyExists`、
  `ERROR_ACCESS_DENIED` のみ宛先 `symlink_metadata` を 1 回診断 probe
  （分類専用・atomic 機構ではない）。
- atomic `Replace` — 両 OS で `fs::rename`（POSIX `rename` / Windows
  `MoveFileExW`+`FileRenameInfoEx` fallback）。
- ディレクトリ sync — unix は `File::open(dir)?.sync_all()`、Windows は
  `OpenOptions.read+write.custom_flags(FILE_FLAG_BACKUP_SEMANTICS)` で
  dir handle を開き `sync_all`（`FlushFileBuffers`）。FS が dir sync を
  提供できない場合は `DirectorySync` phase の失敗として正直に報告し、
  `Published` + `FileSynced` evidence を返す（false Ok にしない）。
- エラー分類 — `EEXIST`/`EACCES`/`ENOENT`/`EISDIR`/`EOPNOTSUPP` 等の
  確定的失敗は `NotApplied`+`Retained`、`EIO`/`ETIMEDOUT`/`ESTALE`/
  `ENOLINK`/ネットワーク・デバイス系コードは `Unknown`+`Unknown`（→
  `PublishEffect::Indeterminate`、`TempState::Unknown`）。
- `src/platform.rs` — trait に `temp_path() -> Option<&Path>`（診断
  identity）を追加し、`staged_file` factory を `OsFileOps::create` へ
  接続（`cfg(any(unix, windows))`）。非対応 target は `Unsupported`。
  `ApplyOutcome`/`TempDisposition` の `cfg_attr(not(test))` 注記を
  「非対応 target では test mock のみ構築」に更新。
- `src/mock.rs` — `temp_path` フィールド追加・trait 実装を追随
  （`dest.bin.rvf-tmp` を既定値として返す）。
- `src/staged.rs` — `create` が `ops.temp_path()` を `RecoveryInfo` に
  記録。`create_reports_unsupported_until_os_backend_lands` テストを
  実 backend 前提の `create_stages_real_temp_via_os_backend`（tempdir
  内の実 temp 生成・abort で除去）に更新。
- `src/lib.rs` — 「backend 未実装」の記述を実 OS backend（IO-IMP-02）の
  primitive 一覧と README 参照に更新。`mod os` を `cfg(any(unix,
  windows))` で宣言。
- `crates/ruviform-io-file/Cargo.toml` — `libc 0.2`（`cfg(unix)`）・
  `windows-sys 0.61`（`Win32_Foundation`+`Win32_Storage_FileSystem`、
  `cfg(windows)`）を target 限定で追加。`Cargo.lock` は
  `cargo check`/`test` の再解決で更新（手編集なし）。
- `crates/ruviform-io-file/README.md` — OS バックエンド節を新設:
  capability matrix（OS/FS × CreateNew/Replace/FileSync/DirSync）・
  Path root 信頼境界・Symlink・Network FS・Windows 共有モード・
  Temp identity・Close 段階の報告限界・検証状況を明記。
- `crates/README.md` — io-file 行の「実 OS backend は IO-IMP-02」
  注記を実装済みに更新。
- `tests/os_publish.rs`（新設）— 実 FS（`env::temp_dir` 配下の一意
  workdir）結合試験 16 件: 複数プロセス `CreateNew` 競合（同一 test
  binary を gate ファイル制御で 6 子プロセス再実行、ちょうど 1 勝者・
  他 `DestinationExists`・宛先 byte 不変・temp 全消去）・
  CreateNew/Replace・durability evidence 三段階・readback/seek・
  sparse write・abort/Drop・missing parent・宛先 dir・Windows 排他
  share・（unix）readonly dir・failure source が io::Error であること。
- `docs/` — spec・design・plan・test-spec・task×3・本 IR と生成索引を
  doc-driven-dev-graph チェーンで整備。

### Decisions During Implementation

- **`renameat2` は `libc::syscall` の raw 呼出**: `libc::renameat2`
  wrapper は glibc/musl 以外の unix target で宣言されず link 失敗
  し得るため、std の `rename_noreplace` と同じ raw syscall 経路を採用。
  `unsafe` は workspace lint 規約どおり `#[expect(unsafe_code,
  reason)]`+SAFETY コメントで局所化（FFI 呼出し 2 箇所のみ）。
- **`EINVAL` fallback は std と同一の再検査規則**: renameat2 が EINVAL
  を返した場合（flag 非対応 FS 等）は `link`+`unlink` を試し、失敗が
  `EEXIST` でなければ**元の EINVAL を報告**する — fallback 失敗の理由
  より primitive 非対応の事実を優先する std 実績 semantics に揃えた。
- **`prepare_for_publish` = ハンドル解放のみ**: Rust `File` drop は
  `close(2)` エラーを拾えないため、durability 観測点を `sync_file`
  （fsync）に置く設計。`publish`/`cleanup_temp` も rename/remove 前に
  ハンドルを defensive 解放し Windows 共有違反を防ぐ。
- **Windows `ERROR_ACCESS_DENIED` の単発診断 probe**: `CreateNew` で
  ACCESS_DENIED が返った場合のみ `symlink_metadata(dest)` で既存宛先か
  真の拒否かを分類。probe は判定専用で rename 自体が atomic 判定
  （`exists()`+`rename` の禁止パターンとは別物）。
- **宛先/temp/dir path は create 時に `std::path::absolute` で保持**:
  公開 `destination()` は呼出し時の path を返すが、adapter 内部の OS
  呼出しは絶対化済み path を使い、途中の cwd 変更で temp と宛先が
  別 dir に解決される分裂を防ぐ。
- **`link` の `EPERM`/`EOPNOTSUPP`/`ENOSYS`/`ENOTSUP` → `Unsupported`**:
  呼出し地点で「この FS は hardlink を提供できない」と確定できる
  errno のみ capability failure に正規化。`EPERM` 以外の権限失敗は
  `Io` に留め、根拠のない Unsupported を返さない。
- **`BestEffort` visibility でも機構は弱めない**: 要求を緩和するだけで
  atomic primitive は常に使用 — 設計の「弱い fallback は作らない」原則。
- **mock の `temp_path` は既定値**: `MockFileOps::new` のシグネチャを
  変えず `dest.bin.rvf-tmp` を返す（#295 の全テストがその値で整合）。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64（`x86_64-pc-windows-msvc`、mise pin rustc
1.99.0、MSRV 要求は 1.89）・Git Bash・F: NTFS。`git fetch origin` →
`git merge origin/main`（Already up to date）後に task の
`## Verification` 全項目を実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| 整形 | `mise run fmt` / `mise run fmt-check` | PASS・差分なし |
| 型チェック | `mise run check`（`cargo check --workspace --all-targets`） | PASS・exit 0 |
| lint | `mise run lint`（`cargo clippy --workspace --all-targets -- -D warnings`） | PASS・0 warnings（新規コード由来の clippy 指摘は全て実修正で解消） |
| テスト | `mise run test`（`cargo test --workspace`） | PASS・全クレート緑（io-file = lib 35 件・os_publish 15 件・doctest 10 件・compile_fail 2 件） |
| CI 相当 | `mise run verify-ci`（`fmt-check`→`check-ci`→`lint-ci`→`test-ci`、全て `--profile ci`・`CARGO_INCREMENTAL=0`） | PASS・4 ゲート全緑（126 `test result: ok`、0 failed） |

実 FS 検証（本ホスト Windows/NTFS）: `tests/os_publish.rs` 全 15 件 PASS
+ `child_create_new_once` は `#[ignore]` で子プロセス入口として使用。
複数プロセス競合は 6 プロセスでちょうど 1 勝者・5 `DestinationExists`・
宛先 byte 不変・temp 全消去を確認。Windows 排他 share での `Replace`
失敗が `NotPublished`+`Io` で確定的に分類されることを確認。

誘導不能な失敗（rename 後の dir sync 失敗で `Published`+`FileSynced`
evidence、`Unknown`→`Indeterminate`、ambiguous transport error）は
`src/staged.rs` の fault-injection mock で網羅済み（実 OS 誘導は非現実的）。

未実施・委譲: Linux（renameat2 実経路）・macOS（renamex_np 実経路）の
実 FS 検証は本ホストでは不可能 — CI lane の OS matrix（
`mise run test` で同一テストが走る）に委譲する。unix `link`+`unlink`
fallback・readonly dir 誘導失敗の実測も同様。Windows/NTFS 以外の FS
（FAT/exFAT・ネットワーク FS・dir sync 非対応 FS）での保証検証は
capability matrix に未検証として記録し、本 Issue では主張しない。

## Risks

- unix（renameat2/renamex_np/link fallback）の実経路は CI matrix でのみ
  実測される — 本ホストで直接は検証できない（コードは std
  `rename_noreplace` と同一 primitive 集合で構成）。
- `link`+`unlink` fallback は atomic だが FS 依存 — hardlink 不可 FS では
  `Unsupported` を返すため「使えない FS を supported と嘘報告」はしない。
- Windows `symlink_metadata` 診断 probe は分類の最良推定 — 稀に
  ACCESS_DENIED が宛既存を意味するケースは「既存として拒否」方向に
  誤分類し得るが、`NotPublished` 側への誤りであり `Published` 嘘報告
  より安全。
- ネットワーク FS / 共有 FS / 特殊 FS（exFAT・FUSE 等）では rename/link/
  fsync 保証が FS 依存 — 失敗・`Indeterminate` は正直に報告するが
  「その FS で保証が効くか」は本 Issue の検証範囲外。
- `PlatformFileOps` に `temp_path()` を追加した private trait 変更 —
  crate 内の impl は `OsFileOps`/`MockFileOps` の 2 つで同期済み、
  公開 API への影響なし。

## Follow-ups

- Issue #288（IO-06）: 共有 OS/FS 受入 — 本実装の capability matrix と
  実 FS 検証結果を受入エビデンスとして引き渡す。
- Issue #305/#306: 旧 `StagedOutput` 撤去・消費者移行・構成/OS/障害/消費者
  受入ゲート — 本 backend がその実動対象になる。
- Issue #219: cross-repo gate。
- AudioLens #143・Synvelis #84: 消費者側での移行利用（本 Issue は backend
  提供まで）。
- CI lane で linux/macos の実 FS 試験結果を確認し、capability matrix の
  未検証セルを実測に置き換える（本 Issue では Windows/NTFS 実測のみ）。
