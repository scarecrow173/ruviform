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

## 検証（本 worktree・Windows host・実測）

| command | result |
| --- | --- |
| `cargo fmt --all` / `-- --check` | PASS |
| `cargo check --workspace --all-targets` | PASS（merge 後 head で再実測） |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | io-file 71・rooted_read 14・archive 全件 PASS。`ruviform-plugin-host` e2e に pre-existing flaky 1 件（spawn timeout 系・再実行で別 test が落ちる非決定 flake。plugin-host は io-file/io-archive 非依存で本変更と無関係） |

## 残 Gate / 未検証

- `vfs/unix.rs` は `cfg(unix)` のため Windows host で compile 未検証。
  Linux CI で確認要。
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
- 正規入口: `mise run fmt-check check lint test`。
- 契約詳細: `docs/specs/rooted-read-capability.md`,
  `docs/designs/rooted-read-capability.md`（approved）。
- 候補 SHA: `38e66c8c1a95c8c917113537cebed6f9038185e4`（PR #401 head）。
