---
id: "DESIGN-034RBFdth7UW7A6IS10Ha3"
type: "design"
status: "approved"
title: "Issue #296 ruviform-io-file — 実 OS PlatformFileOps アダプタ設計 (IO-IMP-02)"
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
  depends-on:
    - "docs/specs/issue-296-io-file-os-backend.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/media-io-backend-abstraction.md"
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-296-io-file-os-backend.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #296 ruviform-io-file — 実 OS PlatformFileOps アダプタ設計 (IO-IMP-02)

## Context

#295（IO-IMP-01）は公開契約と private `PlatformFileOps` 抽象を確定し、
`staged_file()` factory は `Unsupported` を返すままとした。本設計は
同 trait の実 OS 実装 — `OsFileOps` — を定義する。公開 API・パイプライン
分類ロジック（`staged.rs`）・結果語彙は不変で、変更は adapter 層に閉じる。

設計の核心は 3 点:

1. **真のアトミック no-replace プリミティブの OS 別選択** —
   `exists()`→`rename()` を使わない `CreateNew` の実現。
2. **エラーの確定/あいまい分類** — OS エラーを `NotApplied`/`Unknown` と
   `Removed`/`Retained`/`Unknown` に正規化するポリシー。
3. **依存最小化** — std 安定 API で表現不能な syscall のみ
   `libc`/`windows-sys` に落とし、それ以外は `std::fs` に留める。

## Scope

### In Scope

- `src/os.rs`（新設）: `OsFileOps` — `PlatformFileOps` の実 OS 実装。
  unix（POSIX 共通部 + Linux/macOS 別の CreateNew プリミティブ）と
  windows の `cfg` 実装。未対応 OS（`not(any(unix, windows))`）は factory が
  `Unsupported` を返す。
- `src/platform.rs`: `staged_file()` factory を `OsFileOps` 生成に差替え
  （`create` 失敗時の `Unsupported` 文面も実装に合わせ更新）。
- `src/staged.rs`: `StagedFile::create` が `RecoveryInfo.temp_path` に
  adapter の temp path を載せる最小拡張（公開型不変）。
- `src/lib.rs`/`README.md`: 「create は Unsupported」記述を実装済みに
  更新し、保証範囲・capability matrix・信頼境界を文書化。
- `Cargo.toml`: `libc`（unix のみ）・`windows-sys`（windows のみ、
  `Win32_Foundation`+`Win32_Storage_FileSystem`）の target 限定依存。
- `tests/os_publish.rs`: 実 FS 結合試験（後述）。

### Out of Scope

- 公開 API の変更・`PublishOptions` への新軸追加・他 OS 対応・
  ディレクトリトランザクション・Consumer 移行。

## Components and Boundaries

### モジュール構成

| ファイル | 責務（差分） |
| --- | --- |
| `src/os.rs` | `OsFileOps` 本体: temp 生成・`Write`/`Seek`・readback・validate・sync_file・prepare(close)・publish(CreateNew/Replace)・sync_directory・cleanup_temp・inspect_temp・エラー分類ヘルパ |
| `src/os.rs` 内 `cfg` 部 | unix: `renameat2`/`renamex_np`/`link`/`unlink`/errno 集合。windows: `MoveFileExW`・`FILE_FLAG_BACKUP_SEMANTICS` dir open・Win32 エラー集合 |
| `src/platform.rs` | `staged_file()` が `OsFileOps::create(destination)` を返すよう接続。trait・分類語彙は不変 |
| `src/staged.rs` | `create` で adapter の `temp_path()` を `RecoveryInfo` に記録（trait に `temp_path()` 診断メソッドを 1 本追加） |

### `OsFileOps` 内部構造

```text
OsFileOps {
    dest: PathBuf,            // 宛先
    temp_path: PathBuf,       // sibling temp
    dir_path: PathBuf,        // 宛先の親（dir sync 用）
    file: Option<File>,       // temp の RW ハンドル（prepare で close）
    rpos: u64,                // readback カーソル（write カーソルと独立）
}
```

- **temp 名**: `<dest-file-name>.rvf-tmp-<16hex>`。hex は
  `SystemTime` nanos・`process::id`・プロセス内 atomic counter・OS 由来の
  混在値から生成する**診断用一意名**（暗号的保証ではなく同一 dir 内の
  衝突回避が目的）。`OpenOptions::new().read(true).write(true)
  .create_new(true)` で `AlreadyExists` 時は別名で最大 N 回リトライ。
- **書込面**: `PlatformFileOps::write`/`seek` は `File` のカーソルを動かす。
- **readback**: `OsFileOps` 自身が `Read+Seek` を実装し `rpos` を使う
  positioned I/O — unix は `FileExt::read_at`、windows は
  `FileExt::seek_read`。`try_clone` は使わない（clone はファイル記述/
  オフセットを共有し write カーソルを汚染するため、また別ハンドルは
  「同一対象」保証の説明を複雑化する）。positioned read には open 時の
  `read(true)` が必須（O_WRONLY fd の pread は失敗する）。
  readback の `SeekFrom::End` は `file.metadata().len()` を基準に
  `rpos` を解決する。
- **prepare_for_publish（Close 段階）**: `file.take()` でハンドルを drop
  （close）。Rust `File` の drop は close(2) のエラーを報告しないため、
  この段階は「ハンドル解放」として正直に振る舞う — 遅延書込エラーは
  `Durability::FileSync` 以上の `sync_file` 段階（fsync）が観測点であり、
  `BestEffort` では durability を未主張で一貫。Windows では publish 前に
  自ハンドルを閉じることで rename の share 制約を回避する。
- **cleanup_temp**: まず `file.take()` で残ハンドルを閉じてから
  `remove_file(temp_path)`。`Ok`/`NotFound` → `Removed`、確定的失敗 →
  `Retained`、あいまい失敗 → `Unknown`。
- **inspect_temp**: `symlink_metadata(temp_path)` で `Ok`→`Retained`、
  `NotFound`→`Removed`、他エラー→`Ok(Unknown)`（診断の一時失敗を
  ハードエラーにしない）。

## OS プリミティブ選択

### `PublishMode::CreateNew`（排他的 no-replace・単一アトミック操作）

| OS | 第一プリミティブ | fallback（OS 別検証済み条件のみ） | 既存宛先時 |
| --- | --- | --- | --- |
| Linux/Android | `libc::syscall(SYS_renameat2, AT_FDCWD, temp, AT_FDCWD, dest, RENAME_NOREPLACE)`（`libc::renameat2` ではなく raw syscall — glibc/musl で確実に存在する std 方式） | `ENOSYS`/`EINVAL` → `link(temp,dest)`+`unlink(temp)` | `EEXIST` → `AlreadyExists` |
| macOS | `libc::renamex_np(temp, dest, RENAME_EXCL)` | `ENOTSUP`/`ENOSYS` → `link()+unlink()` | `EEXIST` → `AlreadyExists` |
| その他 unix | `link(temp,dest)`+`unlink(temp)` | — | `EEXIST` → `AlreadyExists` |
| Windows | `MoveFileExW(temp, dest, 0)`（REPLACE フラグなし） | — | `ERROR_ALREADY_EXISTS` → `AlreadyExists` |

- `renameat2(RENAME_NOREPLACE)` は VFS 層で宛先存在を原子検査するため
  hard-link 非対応 FS（vfat 等）でも CreateNew が動く — `link()+unlink()`
  では実現不能な保証であり、libc 依存を取る実益。
- `EINVAL` は `RENAME_NOREPLACE` 非対応 FS と誤用の両方に返るため、std
  `rename_noreplace` と同じフォールバック規則に従う: `EINVAL`/`ENOSYS`
  で `link()+unlink()` を試行し、fallback が `EEXIST` なら宛先存在として
  `AlreadyExists`、fallback 成功なら Applied、別失敗なら元 `EINVAL`
  エラーを報告する（誤用 EINVAL を覆い隠さない）。
- Windows `ACCESS_DENIED` は宛先が read-only で存在する場合と真正の
  拒否の両方を含むため、CreateNew 経路のみ `symlink_metadata(dest)` で
  1 度だけ存在を調べ、`存在`→`AlreadyExists`、`不在`→`NotApplied`+Io
  に分類する（probe は atomic 性の機構ではなく分類用診断であり、race は
  誤分類を生じても publish が実際に失敗した事実を偽らない）。
- `link()+unlink()` 採用条件は「該当 OS/FS で検証済みの場合のみ」の
  規約どおり、プリミティブが `ENOSYS`/`EINVAL`/`ENOTSUP` を返した場合
  に限定する。無条件には使わない。

### `PublishMode::Replace`（atomic create-or-replace）

| OS | プリミティブ |
| --- | --- |
| unix 全般 | `fs::rename`（`rename(2)`） |
| Windows | `fs::rename`（std 内部が `MoveFileExW(REPLACE_EXISTING)` +
  `FileRenameInfoEx` fallback を提供） |

### File sync / Directory sync

| 段階 | unix | Windows |
| --- | --- | --- |
| `sync_file` | `File::sync_all()`（fsync） | `File::sync_all()`（`FlushFileBuffers`） |
| `sync_directory` | `File::open(dir)`+`sync_all()` | `OpenOptions{read+write}`+`custom_flags(FILE_FLAG_BACKUP_SEMANTICS)` で dir open + `sync_all()`（`FlushFileBuffers` は GENERIC_WRITE を要求 — read-only dir や FAT/ReFS では正直に失敗 → evidence は `FileSynced` 止まり） |

### `validate_publish`

全 `PublishMode`×`VisibilityRequirement`×`Durability` 組合せを本 backend は
試行可能（`BestEffort` visibility は要求緩和であり、実装上は常に atomic
プリミティブを使う — 弱い経路への降格はしない）。`Ok(())` を返し、
FS 実能力は publish プリミティブのアトミック試行とエラー分類で
正直に報告する（無検証 `Supported` を返さない — capability は試行結果）。

### エラー分類ポリシー

`publish` のエラーは 2 軸に分類する（`ApplyOutcome` × `TempDisposition`）:

- **確定的失敗 → `NotApplied`**: 宛先/FS の状態として「適用されなかった」
  ことが定義上確かなもの — `AlreadyExists`/`NotFound`（親不在・temp 消失）/
  `PermissionDenied`/`IsADirectory`/`NotADirectory`/`FilesystemReadOnly` 相当/
  容量等の事前拒否。`Unsupported` 系 errno（`EPERM`/`EOPNOTSUPP`/`ENOSYS`
  を link 系 API が返した場合）→ `PublishFailureKind::Unsupported` へ写像。
- **あいまい失敗 → `Unknown` + temp `Unknown`**: ネットワーク FS/リダイレクタ
  経由で「適用されたか不明」なもの。unix: `EIO`/`ETIMEDOUT`/`ESTALE`/
  `ENOLINK`/`EREMOTEIO`/`ECOMM`。Windows: `ERROR_UNEXP_NET_ERR`/
  `ERROR_BAD_NET_RESP`/`ERROR_NETNAME_DELETED`/`ERROR_SEM_TIMEOUT`/
  `ERROR_NETWORK_BUSY`/`ERROR_NETWORK_UNREACHABLE`/`ERROR_HOST_UNREACHABLE`/
  `ERROR_CONNECTION_*`/`ERROR_IO_DEVICE` の集合（定数で列挙・文書化）。
- **確定エラーでも `temp` は `Retained`**（link/rename 失敗で temp は残る）。
- **`link` 成功後の `unlink` 失敗 → `Applied` + temp `Retained`**（宛先は
  確立済み。temp の unlink は Cleanup 段階に委譲）。
- `EINTR`/`ERROR_IO_PENDING` 系は bounded retry（最大 3 回）してから分類。

## Data and Control Flow

`StagedFile::create` → `staged_file()` → `OsFileOps::create`:

1. `destination.file_name()` から temp 名生成 → `parent` 解決
   （相対 `file` なら `.`、明示 parent 優先、root-only は create 前段で
   既に `InvalidInput`）。
2. `OpenOptions{read+write+create_new}` で temp open（失敗でリトライ/
   `Io`）。`RecoveryInfo.temp_path` に記録。
3. Write 段階: `File` カーソルで `write`/`seek`。readback は positioned
   I/O（`rpos`）。
4. Publish パイプライン（`staged.rs` 不変）: validate →
   （要求時）`sync_file` → `prepare_for_publish`（close）→
   `publish(mode)` →（要求時）`sync_directory` → `cleanup_temp`。
5. 失敗時 `RecoveryHandle` が `OsFileOps` を保持し、inspect/cleanup は
   `temp_path` ベースで動作。

## 保証範囲（文書化対象 — AC4/AC6）

- **Temp identity**: temp は宛先 dir 内の sibling・同一 FS。名前は
  `*.rvf-tmp-*` パターンの診断用一意名。`RecoveryInfo.temp_path` は
  同一性証明ではなく観測時点の記録。
- **Path root 信頼境界**: 宛先 dir の信頼は caller の責務。本クレートは
  `destination` のパス検証（file_name 存在）のみ行い、dir 自体の
  信頼性・権限・世界書込み性は保証しない。悪意ある同一 dir 共有者が
  temp/宛先を操作する脅威はスコープ外（temp 名は秘密ではない）。
- **Symlink**: publish プリミティブ（rename/link/MoveFileEx）は宛先
  symlink を「対象」でなく「エントリ」として扱う — `Replace` は symlink
  自体を置換する（リンク先を上書きしない）。宛先 dir の構成要素が
  symlink の場合はリンク先 FS の振る舞いに従う（dir 横断で宛先 dir が
  別 FS を指す場合も temp は宛先 dir 側に作るため同一 FS 保証は維持）。
- **Network FS**: rename/link/fsync は通るが、ネットワーク切断系エラーを
  `Unknown`/`Indeterminate` に正規化するのが本 backend の責務。
  NFS/SMB 等の FS 実装差で追加の不確定性があり得ることは文書化する。
- **Windows 共有 Mode**: temp は std 既定 share（`FILE_SHARE_READ|WRITE|
  DELETE`）で開く — 自ハンドルは publish 前に close するため rename は
  制約を受けない。他プロセスが宛先を `FILE_SHARE_DELETE` なしで開いて
  いる場合の Replace は `ACCESS_DENIED`→`NotApplied`。`MOVEFILE_WRITE_THROUGH`
  は `fs::rename` に含めず、dir sync で要求達成を観測する設計とする。
- **Close 段階の限界**: POSIX `close(2)` の遅延エラーは std では観測不能。
  `FileSync` 以上は `sync_file` が観測点、`BestEffort` は未主張で一貫。

### Capability matrix（設計時点の主張 — 実測で検証）

| OS/FS | CreateNew | Replace | FileSync | DirSync |
| --- | --- | --- | --- | --- |
| Linux ext4/xfs/btrfs | renameat2 / link fallback | rename | fsync | dir fsync |
| Linux vfat/exFAT | renameat2（VFS 層保証） | rename | fsync | dir fsync（FS 依存で失敗あり得る→Err+Published） |
| Linux NFS | renameat2/link（server 依存・`Unsupported` になり得る） | rename | fsync | dir fsync |
| macOS APFS | renamex_np / link fallback | rename | fsync | dir fsync |
| Windows NTFS | MoveFileExW(0) | MoveFileEx 系 | FlushFileBuffers | BackupSemantics+FlushFileBuffers |
| Windows FAT32/exFAT | MoveFileExW(0) | MoveFileEx 系 | FlushFileBuffers | DirSync は失敗し得る→Err+Published |
| Windows ReFS | MoveFileExW(0) | MoveFileEx 系 | FlushFileBuffers | FS 依存 |
| 上記以外の unix/その他 OS | link+unlink または `Unsupported` | rename / `Unsupported` | fsync / `Unsupported` | `Unsupported` |

試験済み/未試験の実測欄は実装後に README/impl 記録へ反映する。

## Non-Functional Constraints

- `unsafe` は syscall 呼出し点の `#[expect(unsafe_code, reason=…)]` +
  SAFETY コメントのみ（plugin-host `l1::diag` と同型）。
- 依存は `libc 0.2`（`cfg(unix)`）・`windows-sys 0.61`
  （`cfg(windows)`・`Win32_Foundation`+`Win32_Storage_FileSystem`）に
  限定 — workspace 既出版に合わせる。`std` のみの公開契約文面は
  「std + target 限定 FFI 依存」へ正確に更新する。
- 非 RT・alloc 可の既存契約不変。MSRV 1.89・edition 2024・
  `workspace.lints`・`-D warnings` ゲート。
- クレート DAG 不変（ruviform-* への依存ゼロ維持）。

## Risks and Trade-offs

- **link+unlink fallback の temp 残留**: `link` 成功・`unlink` 失敗で temp
  が宛先への hardlink として残る。temp 名除去は宛先に影響しないため
  `Applied`+`Retained`（Cleanup 段階が回収）で安全側に倒す。
- **Windows `ACCESS_DENIED` 分類の heuristic**: CreateNew で宛先 readonly
  存在時に `symlink_metadata` probe を入れる。race で「不在」になれば
  kind が `Io`（`DestinationExists` でなく）に揺れるが publish が失敗した
  事実・effect は偽らない。probe は atomic 性に関与しない。
- **dir sync の FS 依存失敗**: FAT32 等で DirSync が失敗 →
  `Err{Published, FileSynced}`。要求不達を `Ok` で偽装しない（契約どおり）。
- **`fs::rename` 内部仕様への依存**: Windows Replace を std に委譲する
  （`MoveFileExW(REPLACE_EXISTING)`+`FileRenameInfoEx` fallback は std
  実装の現行仕様でありドキュメント化された同等性）。
- **EINVAL 二重義**: `renameat2` の EINVAL を無条件に「非対応 FS」と
  みなさず、std と同じ再検査規則で誤用を隠さない。

## Deferred Design Concerns

- `FileRenameInfoEx` 直接呼出しによる Windows CreateNew の
  `ACCESS_DENIED` 分類精緻化 | heuristic で十分な実害が出たら | 誤分類
  レポートが出た時点 | `DestinationExists`→`Io` の kind 揺れ。
- `O_TMPFILE`/`linkat(AT_EMPTY_PATH)` 等の別 temp 方式 | 実需が出たら。
- BSD/その他 unix の `renameat` 系プリミティブ追加 | CI/需要が出たら。

## Verification Notes

- [ ] 設計の syscall 選択が std `rename_noreplace` 内部実装
      （linux `renameat2`→`link_then_unlink`・apple `renamex_np`→同・
      windows `MoveFileExW(0)`）と整合していること — 実測済み。
- [ ] 全段階が `PlatformFileOps` trait 形状のまま実装可能であること
      （公開 API・パイプライン不変）。
- [ ] capability matrix と受入 AC1–AC6 が test-spec に落ちること。
