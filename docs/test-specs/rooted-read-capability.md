---
id: "TSPEC-wJA20tmm9AN7JyNH6uMDQ6"
type: "test-spec"
status: "approved"
title: "ruviform-io-file Rooted Read Capability テスト仕様"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "docs/specs/rooted-read-capability.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-file/tests/rooted_read.rs"
      - type: "file"
        path: "crates/ruviform-io-file/tests/common/mod.rs"
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
    - "docs/designs/rooted-read-capability.md"
  derived-by: []
  verifies:
    - "docs/specs/rooted-read-capability.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform-io-file Rooted Read Capability テスト仕様

## Purpose

`rooted` 公開契約 — `RootedDir` capability・`RelativePath` 検証・
`FollowMode::{NoFollow, WithinRoot}`・`metadata`/`open_regular`/
`pin`/`open_pinned`/`read_dir`/`open_dir` — が、root 外 object の
byte/facts を返さないことを、`#[cfg(test)]` scripted vfs mock の
決定的 barrier/fault 試験と実 FS 回帰で pin する。
`ruviform-io-archive` への vfs 集約（同一 OS ルールの単一実装化）
の非回帰は既存 `extract_*` suite で担保する。

## AC 対応（spec 受入 1–6 への写像）

| spec AC | 検証箇所 |
| --- | --- |
| 1. canonicalize→swap→open 回帰 (leaf/intermediate × read/stat/pin/browse) | mock: `resolve_swap_*` / 実FS: `*_swap_*` |
| 2. root 内 symlink 正常系 + NoFollow leaf 可視 | 実FS: `within_root_*` / `no_follow_*` |
| 3. escape/loop/limit/nonregular/reparse/失敗分類 | mock+実FS: `escape_*` `loop_*` `limit_*` `nonregular_*` `kind_*` |
| 4. pin→swap→open_pinned で bytes を返さない | 実FS: `open_pinned_*` |
| 5. archive 非回帰 | `cargo test -p ruviform-io-archive` |
| 6. 正規入口 | `mise run fmt-check/check/lint/test` |

## Mock 試験（`#[cfg(test)]` scripted vfs — crate unit test）

擬似 FS tree（`TestNode{kind,id,target,data,outside}`）を
`RefCell`/shared で持ち、test が call 間に mutate できる scripted
`VfsOps`/`VfsDir`。`outside=true` node は root 外 object を表し、
その `data`/`facts` が返却・digest・metadata に混入しないことを
assert する。OS 非依存で resolve 規則の全分岐を網羅する。

- `resolve_swap_leaf_open_regular`: probe 相当の step 後に leaf を
  outside file への link に差し替え → `open_regular` が typed 失敗
  し outside data を返さない。
- `resolve_swap_intermediate`: `a/b/f` の `a` を outside dir への
  link に差し替え → `metadata`/`open_regular`/`open_dir` が
  `EscapesRoot`（または `NotFound`/`IdentityMismatch`）で失敗。
- `resolve_swap_open_pinned`: pin → 差し替え → `open_pinned` が
  `IdentityMismatch`。
- `resolve_browse_descend`: `read_dir` 結果の entry を `open_dir`
  した先が outside を指すよう差し替え → scoped dir が outside
  entry を列挙しない。
- `resolve_absolute_link_in/out`: 絶対 link が canonical root
  prefix 内のみ受理、外は `EscapesRoot`（lexical 正規化で
  `root/../x` 等の畳込みも検査）。
- `resolve_dotdot_overflow`: link target `../../..` で stack が
  root を割る → `EscapesRoot`。
- `resolve_loop_and_hops`: `a→b`,`b→a` 環 → `LoopDetected`；
  `max_hops` 超過、展開後総成分が `max_components_total` 超過 →
  `LimitExceeded`。
- `resolve_no_follow_*`: `NoFollow` で leaf link → `NotRegularFile`
  （open）/`kind=Symlink`（metadata）、中間 link → 失敗。
- `resolve_case_/`name 正規化: `RelativePath` 否定行列（`..`/`.`/
  絶対/空/`\`/`:`/NUL/制御文字/予約名/末尾ドット・空白/255+byte）。

## 実 FS 試験（`tests/rooted_read.rs`）

`tests/common/mod.rs` に tempdir・symlink/junction helper（権限
不足は `eprintln` + early-return の明示 skip）・fifo helper
（`mkfifo`、unix のみ）を置く。

- `within_root_leaf_link` / `within_root_intermediate_link`:
  root 内 link を `WithinRoot` で stat/open/descend できる。
- `within_root_absolute_link`: root 内 target を指す絶対 link が
  受理される（canonical root prefix 一致）。
- `escape_absolute_link` / `escape_relative_link_to_outside`:
  `WithinRoot` でも root 外 target は `EscapesRoot`。
- `swap_leaf_open_pinned`: `pin` → leaf を outside file への link
  に差し替え → `open_pinned` が `IdentityMismatch`/`EscapesRoot`
  で失敗し `Read` から outside byte が出ない。
- `swap_intermediate_open_regular`: 中間 dir を outside への link
  に差し替え → `open_regular`/`metadata` が typed 失敗。
- `browse_descend_link_dir`: link→dir（root 内）を `open_dir`
  で降下し、link→outside dir は失敗する。
- `loop_link_chain`: link 環で `LoopDetected`。
- `nonregular_dir_open_regular`: dir に `open_regular` →
  `NotRegularFile`。
- `nonregular_fifo`（unix `mkfifo` 可能時のみ）: `open_regular`
  が block せず `NotRegularFile`（`std::thread`+timeout 観測で
  待ちが発生しないことを示す）。
- `no_follow_leaf_link`: `NoFollow` で leaf link の
  `metadata.kind == Symlink`、`open_regular` は `NotRegularFile`。
- `kind_and_identity_classification`: 全 typed 失敗が
  `FileErrorKind` に正規化される（`EscapesRoot`/`LoopDetected`/
  `NotRegularFile`/`NotFound`/`NotDirectory`/`IdentityMismatch`/
  `LimitExceeded`）。
- windows reparse: junction 差し替え・`FILE_OPEN_REPARSE_POINT`
  による link 可視化（helper が作れない環境では明示 skip）。
- doctest: `RootedDir` の公開 example が `cargo test` で実行される。

## 非回帰

- `cargo test -p ruviform-io-archive`（`extract_path`/
  `extract_secure`/`extract_staging`/`extract_records`/`extract_cleanup`
  等の既存 suite）が vfs 集約後も全 pass。
- `cargo test -p ruviform-io-e2e`（所有 lane の結合）。

## 未検証の正直な扱い

- 実電源断・network FS・`openat2` 前提経路・未実行 OS（本 run は
  Windows 実機のみ実行）は Pass と報告しない。unix 分岐は
  `#[cfg]` でコンパイルされるが未実測であることを結果に明記する。
