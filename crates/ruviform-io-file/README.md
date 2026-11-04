# ruviform-io-file

Ruviform の汎用ファイル I/O 基盤 — IO-01（Issue #283 v0.4）の
staged-publish 契約を実装する非 RT 専用クレート。
AudioLens（#143）・Synvelis（#84）・Ruviform 本体（#305/#306）が共有する
「完成ファイルの安全な公開」プリミティブの型状態・結果語彙・回復所有権を
確立し、IO-IMP-02（Issue #296）で実 OS バックエンド（atomic publish・
ファイル/ディレクトリ sync・回復）を実装した。

## 責務

- `StagedFile` — `Write + Seek + Send` の temp 書込み面。内部 `File` や
  clone 可能ハンドルは一切公開しない。`seal(self)` で `SealedFile` へ
  消費遷移する。
- `SealedFile` — `Write`/`Seek` を持たない sealed 状態。publish・abort・
  読み戻し borrow（`open_readback(&mut self)` → `Read + Seek` の
  `ReadbackFile<'_>`、非 File ハンドル）のみ。読取 borrow 中は publish
  不可（`&mut` borrow が所有権をブロック）。
- `PublishOptions` — mode（`CreateNew`/`Replace`）× visibility
  （`RequireAtomic`/`BestEffort`）× durability（`BestEffort`/`FileSync`/
  `FileAndDirectorySync`）の独立3軸。
- `PublishReceipt` / `PublishFailure` — `PublishEffect`
  （`NotPublished`/`Published`/`Indeterminate`）・`PublishPhase`・
  `DurabilityEvidence`（要求値ではなく観測済みの証跡）・`TempState`。
  `Indeterminate` の temp は自動削除しない。
- `RecoveryHandle` / `RecoveryInfo` / `RecoveryObservation` —
  残存 temp の inspect/cleanup を行う所有権ハンドル。Drop は削除しない。
- `abort` / `Drop` — 明示 cancel と best-effort temp 掃除（Drop は
  成功経路でも完全経路でもない）。
- private `PlatformFileOps` — OS 操作の抽象境界。validate・file sync・
  close 準備・publish・directory sync・cleanup・inspect の各段階を
  分離し、`io::ErrorKind::Unsupported`/`AlreadyExists` の規約で公開
  kind へ正規化する。実 OS 実装は `src/os.rs` の `OsFileOps`。

## OS バックエンド（IO-IMP-02）

`StagedFile::create` は宛先の兄弟 temp（`<name>.rvf-tmp-<token>`）を
`create_new` で排他的に開き、publish は真の atomic primitive を使う。
`exists()` 後の `rename()` という組合せは一切使わない。

### Capability matrix

| OS / FS | `CreateNew`（非上書き） | `Replace` | ファイル sync | ディレクトリ sync |
| --- | --- | --- | --- | --- |
| Linux / Android（POSIX FS） | `renameat2(RENAME_NOREPLACE)`（生 syscall。`ENOSYS`/`EINVAL` のみ `link`+`unlink` fallback、std と同じ再検査規則） | `rename(2)` | `fsync` | 親 dir を open して `fsync` |
| macOS / iOS 系（POSIX FS） | `renamex_np(RENAME_EXCL)`（`ENOTSUP`/`ENOSYS` のみ `link`+`unlink` fallback） | `rename(2)` | `fsync` | 親 dir を open して `fsync` |
| その他 Unix | `link`+`unlink`（POSIX FS で atomic。hardlink 不可 FS は `Unsupported`） | `rename(2)` | `fsync` | 親 dir `fsync` |
| Windows / NTFS | `MoveFileExW(temp, dest, 0)`（`MOVEFILE_REPLACE_EXISTING` なし） | `MoveFileExW` + std `FileRenameInfoEx` fallback（`fs::rename`） | `FlushFileBuffers` | dir を `FILE_FLAG_BACKUP_SEMANTICS`+read-write で open し `FlushFileBuffers` |
| unix/windows 以外の target | `Unsupported` | `Unsupported` | `Unsupported` | `Unsupported` |

- `CreateNew` の `EEXIST`/`ERROR_ALREADY_EXISTS`/`ERROR_FILE_EXISTS` は
  `DestinationExists` + `NotPublished` に正規化される。
- Windows で `CreateNew` が `ERROR_ACCESS_DENIED` を返した場合のみ、
  「宛先が実在するのか真の拒否か」を区別するため**1回だけ**
  `symlink_metadata` で診断 probe する。これは分類専用で、
  atomicity の機構ではない（rename 自体が atomic 判定）。
- `link`+`unlink` fallback で `link` が `EPERM`/`EOPNOTSUPP`/`ENOSYS`/
  `ENOTSUP` を返した FS は hardlink 不可とみなし `Unsupported` を報告する
  — 保証不能な FS を検証なしに Supported と扱わない。
- `EIO`/`ETIMEDOUT`/`ESTALE`/`ENOLINK` 等の「適用されたか判断できない」
  errno・Win32 ネットワーク/デバイス系エラーは `Unknown` に分類し、
  `PublishEffect::Indeterminate` + `TempState::Unknown` へ正規化される。
  `Indeterminate` の temp は自動削除しない。

### 信頼境界・保証範囲

- **Path root 信頼境界**: temp・宛先・親 dir の各 path は create 時に
  絶対パス化して保持する。宛先 dir の権限・所有は呼び出し側の信頼境界内
  （同じ境界内の悪意ある差替えまでは防衛しない）。
- **Symlink**: publish の rename は宛先 path を置き換える。宛先が symlink
  でも「その名前」が公開対象であり、`CreateNew` は symlink 自身の存在を
  既存として拒否する。temp/宛先の中間 path component が symlink の場合は
  その先の FS の保証に従う。
- **Network FS / 共有 FS**: NFS・SMB 等では rename/link/fsync の保証が
  FS 実装依存になる。要求した保証が FS から得られない場合は失敗または
  `Indeterminate` として正直に報告し、成功とは見なさない。
- **Windows 共有モード**: temp は標準 `File` 共有（read/write/delete
  share）で開く。宛先を delete-share なしで排他 open している他者がいる
  と `Replace`/`CreateNew` は共有違反で `NotPublished` + `Io` になる
  （定義済みの確定的失敗）。
- **Temp identity**: `<name>.rvf-tmp-<token>` の token は時刻・pid・
  プロセスカウンタの混合で、暗号学的保証ではない。path は診断用識別
  （`RecoveryInfo.temp_path`）であり、所有権・ロックの証明ではない。
  `create_new` 排他 + retry が衝突安全性を担う。
- **Close 段階**: Rust `File` の drop では `close(2)` エラーを取得
  できないため、`prepare_for_publish` はハンドル解放のみを行い、
  durability エラーの観測点は `sync_file`（`fsync`）に置く。
- **Generic Directory Transaction / CAS / 複数 file transaction は
  実装しない**（Issue 本文の範囲外）。

### 検証状況

- 実 FS 結合試験は `tests/os_publish.rs`（`mise run test` で CI lane
  OS matrix に乗る）。複数プロセスの `CreateNew` 競合・既存宛先の非上書き・
  RecoveryHandle の実 temp inspect/cleanup・誘導可能な実故障を網羅する。
- 誘導不能な失敗（中途不明の publish・transport-level indeterminacy・
  電源断）は `src/staged.rs` の fault-injection mock で網羅する。

## 層・許可依存

- 層: 汎用ファイル I/O 基盤（メディア I/O より下位・Primitives 同様の
  相互依存なし位置）。
- 許可依存: `libc`（`cfg(unix)` のみ、`renameat2`/`renamex_np` FFI）・
  `windows-sys`（`cfg(windows)` のみ、`MoveFileExW` 等）。それ以外は
  std のみ。Media/Codec/Serde 非依存。
- `default = []`・feature なし・`publish = false` 継承・MSRV 1.89。

## RT 契約

- 全 API 非 RT — filesystem I/O にブロックし、alloc も行う。
  RT オーディオコールバックからは絶対に呼ばない。

## 実装 Issue

- Issue #283: IO-01 v0.4 設計契約（型状態・readback borrow・効果三分類）
- Issue #295: 本クレートの新設・公開契約・Mock 駆動 publish（IO-IMP-01）
- Issue #296: IO-IMP-02 — Linux/macOS/Windows atomic publish・
  durability・recovery の実 OS adapter（`src/os.rs`）・実 FS 試験
- Issue #305/#306: 旧 Media File API 撤去・消費者移行・構成/OS/障害/消費者
  受入ゲート
