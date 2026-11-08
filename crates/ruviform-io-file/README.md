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

## Rooted read capability（`rooted` module）

`rooted` は「開いた root ディレクトリハンドルに対してのみ相対解決する」
読み取り専用 capability。canonicalize + 後続 open の分離に起因する
TOCTOU（検査時点と open 時点で名前が指す物が変わる問題）を、
コンポーネントごとに open handle 相対で辿る構造で除去する
（Synvelis SV85 / Issue #304 系の脆弱性パターンへの基盤側回答）。

- `RootedDir` — `open(path)` は open-first で、canonical root は
  handle 由来（Linux `/proc/self/fd`・Windows `GetFinalPathNameByHandleW`・
  Apple `fcntl(F_GETPATH)`）で取得する唯一の path 信頼境界。以後は
  path 文字列を一切再解決しない。`open_dir` で scoped capability を
  作り、解決済み ancestor chain を Arc 共有する（`..` は chain 内を
  pop できるが capability root を割ると `EscapesRoot`）。
- `open_scoped_root(rel, mode)` — 既に保持している capability 上で
  `rel` を解決し、**解決した directory handle 自体を新しい
  capability floor** とする再 root 化（Issue #409）。`open_dir` と
  違い元 root の祖先は新 floor に引き上げないため、新 scope 内の
  link target の `..`・sibling・scope 外 absolute link は
  `EscapesRoot`。権威は保持 handle のみ — pathname 再 open も
  `NodeId`/Strong identity 照合も行わず、root rename/replacement
  後も継続動作する。lexical anchor は handle 由来の
  `canonicalize_handle` のみで導出し、path 文字列への再 bind・
  anchor 捏造は行わない（導出不能 backend は `Unsupported`）。
- `read_dir_each(rel, mode, visit)` — 全件 Vec 化しない bounded
  列挙（Issue #409）。`EntryFlow::Stop` で任意 prefix（consumer の
  `remaining+1` 件など）に打切れ、各 step で cancel/deadline を
  観測できる。`Stop` 後は後続の OS 列挙・entry probe・callback を
  発行しない（OS 固定 batch 内の破棄は許容）。結果は
  `EnumerationOutcome::{Complete, Stopped}` で完走/早期停止を
  区別し、OS 失敗は `Err`。`read_dir` は同一列挙規則の collect
  wrapper。
- `RelativePath` — `/` 区切り UTF-8 locator。`""` は self。
  絶対・`..`・`.`・`\`・`:`・NUL・制御文字・Windows 予約名・
  trailing `.`/space・空成分・255 bytes 超を `InvalidInput` で拒否。
- `FollowMode::{NoFollow, WithinRoot}` — leaf/intermediate 双方で
  no-follow probe → dir は open・link は hop 上限内で解決
  （`WithinRoot` のみ）。絶対 link target は canonical prefix の
  TEXTUAL walk + 残 suffix の FD walk。「一度でも root prefix を外れた
  target は tainted」— 外を経由して戻る形も保守的に `EscapesRoot`。
- canonical 前提（Issue #468）— `open`/`open_scoped_root` に渡す
  path の綴りは canonical でなくてよい（anchor は handle 由来の
  canonical で導出される）。前提が効くのは **tree 内の絶対 link
  target の綴り**で、`canonical_root()` の成分列で綴るか相対
  target にする必要がある。非 canonical alias 綴り
  （macOS `/var` vs `/private/var`・Windows 8.3 名・junction・
  `subst`）の in-root 絶対 target は kernel 解決では内側でも
  保守的に `EscapesRoot`（wrong-reject は fail-closed であり、
  root 外の byte/fact は一切返さない）。consumer が root 内に
  絶対 symlink を作る場合は、当該 capability の `canonical_root()`
  を基底に target を組み立てるか、相対 target を使う。判断の
  正本は `docs/adr/issue-468-rooteddir-canonical-premise.md`。
- `open_regular` — Linux は `O_PATH|O_NOFOLLOW` pin→`fstat`→
  `/proc/self/fd` reopen + identity 検証。他 unix は `O_NONBLOCK`
  open+`fstat`。Windows は `FILE_NON_DIRECTORY_FILE|
  FILE_OPEN_REPARSE_POINT|FILE_SYNCHRONOUS_IO_NONALERT`。
  非 regular は `NotRegularFile` で通常 open に到達させない。
- `metadata`/`read_dir` — 同一 object の facts（kind・len・mtime・
  identity・`IdentityStrength`）と no-follow 分類済み列挙。
  `metadata` は `leaf_kind` で「leaf が link だった」も報告する。
- `pin`/`open_pinned` — object-identity token（volume+index+tag+
  discriminator+kernel handle）による open 時照合。`Weak` FS では
  `Unsupported` と正直に報告する（`NodePin` は digest ではない —
  消費側の digest 検証と併用）。
- `FileErrorKind` — `NotFound`/`NotDirectory`/`NotRegularFile`/
  `EscapesRoot`/`LoopDetected`/`IdentityMismatch`/`LimitExceeded`/
  `Unsupported`/`Io`/`InvalidInput`。生 `io::Error` は `source` に保持。
- `RootedOptions` — `max_hops`/`max_depth`/`max_components_total`/
  `max_open_dirs` の budget（既定 40/40/512/64）。
- 内部 `#[doc(hidden)] vfs` module — `openat`/`O_NOFOLLOW`/`fstatat`
  （unix）と `NtCreateFile`/`NtOpenFile`/`OBJECT_ATTRIBUTES.
  RootDirectory`/`OBJ_DONT_REPARSE`/`FILE_OPEN_REPARSE_POINT`
  （Windows）の no-follow・handle 相対プリミティブを workspace 共有で
  集約。`ruviform-io-archive` の staging backend もここを使う。
  サポート不能な target（Windows 10 RS1 未満を含む — `OBJ_DONT_REPARSE`
  が silent-ignore される環境）では全 op `Unsupported`。
- 限界: 保証は mount namespace 内の FS tree に scope される（unix
  mount/bind mount 横断は kernel の実体として通過する）。root
  `canonicalize` の fallback がある backend では open→canonicalize の
  わずかな race が残留する（doc 明記の residual）。

## 層・許可依存

- 層: 汎用ファイル I/O 基盤（メディア I/O より下位・Primitives 同様の
  相互依存なし位置）。
- 許可依存: `libc`（`cfg(unix)` のみ、`renameat2`/`renamex_np`・
  `openat`/`statx`/`name_to_handle_at` FFI）・`windows-sys`
  （`cfg(windows)` のみ、`MoveFileExW`・NT syscall FFI）。それ以外は
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
- Issue #304 関連: rooted read capability — handle 相対の no-follow
  traversal + 共有 vfs（`rooted` module・`docs/designs/
  rooted-read-capability.md`）
