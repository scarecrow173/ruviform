# RF398 rooted read capability — worker report

- Branch: `feature/rooted-read-capability`
- PR: https://github.com/scarecrow173/ruviform/pull/401
- Head at push: `38e66c8c1a95c8c917113537cebed6f9038185e4`（origin/main `e8adcc0` merge 済み）
- Tracking: ruviform#398 / Consumer: Synvelis SV85・PR180

## 実施内容

- `ruviform-io-file` に rooted read-only capability 追加:
  `src/rooted.rs` + `src/rooted/resolve.rs`（public API + two-state
  TEXTUAL/FD resolve walk）、`src/vfs.rs` + `vfs/{unix,windows,unsupported,mock}.rs`
  （`#[doc(hidden)]` 共有 handle-relative primitive 層）。
- `ruviform-io-archive` の private extract backend（`extract/unix.rs`,
  `extract/windows.rs`）を vfs へ抽出・削除し、`extract/fs.rs` を薄い
  adapter 化。staging 契約（0700/0600・exclusive create・ShareAccess=0・
  unlinkat flag 分離・identity-gated cleanup）維持。依存方向 archive→file。
- `FileError`/`FileErrorKind` に rooted 用 typed 失敗を追加
  （EscapesRoot/LoopDetected/IdentityMismatch/LimitExceeded/
  NotRegularFile/NotDirectory/Unsupported 等）。
- テスト: `tests/rooted_read.rs`（14件: leaf/intermediate link swap、
  absolute-link `..`・root floor・tainted 決定的 EscapesRoot、
  NoFollow vs WithinRoot、root 列挙、pin identity mismatch、
  weak-identity→Unsupported、nonregular 拒否、concurrent replacement）
  + `rooted::tests` unit + mock-VFS 分類テスト。
- doc chain: spec/adr/design/test-spec/plan/tasks（`docs/*/rooted-read-*`）。
  独立 A/B→Validator→Judge 2 cycle で blocking findings 全件解消し approved。

## 検証（本 worktree・Windows host・実測。`mise` 未経由で cargo 直接実行）

| command | result |
| --- | --- |
| `cargo fmt --all` / `-- --check` | PASS（exit 0） |
| `cargo check --workspace --all-targets` | PASS（exit 0・merge 後 head で再実測） |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS（exit 0・merge 後 head で再実測） |
| `cargo test --workspace` | io-file 71・rooted_read 14・archive 全件 PASS。`ruviform-plugin-host` e2e `l1_proxy_e2e` に pre-existing flaky 失敗 1 件（初回 `command_timeout_converges_failed_with_timeout_cause`、対象再実行では別 test `spawn_failed_attempt_record_captures_stderr` が落ちる非決定 flake。plugin-host は io-file/io-archive 非依存で本変更と無関係）。 |
| `cargo test -p ruviform-io-file -p ruviform-io-archive`（unix 修正後の最終 head） | 全件 PASS |
| `cargo check/clippy --target x86_64-unknown-linux-gnu -p ruviform-io-file -p ruviform-io-archive --all-targets` | PASS（unix 分岐の型/借用/lint を Linux target で実検査 — `stx_size` E0308 を修正済み） |
| `cargo clippy --target aarch64-apple-darwin`（同 scope） | PASS（非 Linux unix 分岐 `O_NOFOLLOW`/`O_NONBLOCK` 経路も lint 済み） |

補足: workspace 全体の `--target x86_64-unknown-linux-gnu` check は
`zstd-sys`/`alloca` の C build script が Linux toolchain を要求するため
Windows host では実行不可（環境制約。io-file/io-archive とその依存閉包は
上記の通り全 PASS）。**正規入口として `mise run check-linux` を追加済み**
（上記 linux-gnu scoped check+clippy と同等）。実 Linux 正規検証は
root 側の既存 CI dispatch に委ねる。

## Review fix-up round 2（本 head に含まれる修正）

- `vfs/windows.rs identity_parts`: `FileIdInfo` 失敗を `(0,0)`
  fabricated identity へ潰さず Err 伝播（旧 archive `identity_of`
  と同じ fail-closed。archive cleanup / `open_pinned` 双方の境界）。
  回帰: `file_id_failure_propagates_instead_of_fabricating_identity`
  （NUL device handle での実注入 — FileIdInfo が確実に失敗する
  object で Err を検証）。
- `filetime_to_system`: `ticks * 100` の u64 overflow（~year 30828
  以降で debug panic/release wrap）を secs/nanos 分解 +
  `checked_add` で修正。回帰: `filetime_far_future_does_not_overflow`,
  `filetime_normal_value_converts`。
- 契約明記（仕様確定・コード変更なし）: capability の authority は
  取得済 dir handle（inode subtree）への委譲であり path namespace
  の継続的包含ではない — scoped dir が取得後に root 外へ rename
  されても handle は移動した subtree を正当に読む。namespace 連続性を
  各 op で再検証する手段は存在せず、再 canonicalize は TOCTOU の
  再導入のため採用しない（cap-std 系の標準的意味論。spec/design に
  明記済み。現 SV canonicalize モデルと等価以下の露出ではない）。
- statx `stx_size` u64→i64 の正規化契約をコードコメントで明記
  （kernel inode size は loff_t なので >i64::MAX は実在不可 —
  到達不能値は wrap せず i64::MAX に clamp）。

## 残 Gate / 未検証

- `vfs/unix.rs` は Windows host で compile 未検証だった問題を
  `--target x86_64-unknown-linux-gnu`/`aarch64-apple-darwin` の type-check
  で解消（E0308 + 2 lint 修正済み）。**実行時の挙動**（statx/handle
  等の実 FS 動作）は未検証 — Linux CI で要確認。
- Windows junction/symlink 権限なし環境では該当 test は明示 skip。
- `RootedDir::open` の handle 由来 canonical 取得が無い backend では
  canonicalize→open の residual race が唯一の trust boundary（設計明記）。

## Consumer 引継ぎ（SV 担当向け candidate）

- `RootedDir::{open,open_with,metadata,pin,open_pinned,open_regular,
  read_dir,open_dir}`、`RelativePath`（空=self）、`FollowMode::{NoFollow,
  WithinRoot}`、`RootedFile`、`NodeInfo/NodeKind/NodeId/NodePin/DirEntry`、
  `RootedOptions`。
- pin/open_pinned は per-object Strong identity 必須。弱 FS は typed
  `Unsupported` → SV は digest verify 経路へ縮退可能。
- `FollowMode::{NoFollow, WithinRoot}` は SV の現 `FollowWithinRoot`
  契約を維持し黙示置換しない。全解決は root-relative handle
  traversal であり、取得済 scoped handle の rename 後も authority は
  handle（inode subtree）に委譲される（spec/design 明記済み）。
- 正規入口: `mise run fmt-check check lint test`。
- 契約詳細: `docs/specs/rooted-read-capability.md`,
  `docs/designs/rooted-read-capability.md`（approved）。
- 候補 SHA: 本 branch の最新 head（`git rev-parse HEAD` 参照 — PR
  #401 head と同一）。
