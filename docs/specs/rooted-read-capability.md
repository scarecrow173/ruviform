---
id: "SPEC-OSFxdu6zk2jDbA6edHhE38"
type: "spec"
status: "approved"
title: "ruviform-io-file Rooted Read Capability — dirfd/handle 相対 traversal・FollowWithinRoot/NoFollow・pin/identity"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/304"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-file/src/rooted.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/rooted/resolve.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/vfs.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/vfs/unix.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/vfs/windows.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/vfs/unsupported.rs"
      - type: "file"
        path: "crates/ruviform-io-file/tests/rooted_read.rs"
      - type: "file"
        path: "crates/ruviform-io-file/tests/common/mod.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-file/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract/fs.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-archive/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "docs/designs/issue-304-io-archive-extract.md"
    deleted:
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract/unix.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract/windows.rs"
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements: []
  implemented-by:
    - "docs/plans/rooted-read-capability.md"
  depends-on:
    - "docs/specs/issue-304-io-archive-extract.md"
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/specs/issue-296-io-file-os-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/rooted-read-capability-placement.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/designs/rooted-read-capability.md"
    - "docs/adr/rooted-read-capability-placement.md"
  verifies: []
  verified-by:
    - "docs/test-specs/rooted-read-capability.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-io-file Rooted Read Capability — dirfd/handle 相対 traversal・FollowWithinRoot/NoFollow・pin/identity

## Background

Synvelis #85 / PR180（head `d27bbee4372c2ca024207f9ad9b98e2f11a2a28e`）の
`LocalBlobDir` / `LocalFolderProvider` は、locator を
`fs::canonicalize` + canonical root への `starts_with` で検査したあと、
`File::open` / `fs::metadata` / `fs::read_dir` で **path を再解決**
している。検査と利用の間に leaf または中間 component を symlink /
reparse point / rename で差し替えられると、root 外の regular file の
bytes / facts が返り得る（SV85・独立 A/B/Validator 確認・root Judge
P1）。副次問題として、

- pin が外部内容の digest を計測すると hash oracle になる
  （EOF digest はすでに返した byte を取り消せない）、
- post-open の `is_file` 検査は open(2) 自体の FIFO/device 待ち・
  副作用を防がない。

Ruviform main (`59c7e71`) には公開 rooted read が存在しない一方、
`ruviform-io-archive` の `extract/{fs,unix,windows}.rs`（IO-IMP-10 /
Issue #304）に、handle/fd 相対・no-follow・排他 create・
`NodeIdentity` の私有 primitive が実装済みである。同設計は
「dir-relative primitive の IO-IMP-02 等との共通化」を Deferred
Design Concern として記録しており、本 spec がその読取側の
対応物である。

## Purpose

Ruviform 所有の**最小 read-only root capability** を
`ruviform-io-file` に追加する。root directory handle を保持し、
その handle に対する component 解決 / open / metadata / enumeration
が、root 外 object の byte / facts を返さないことを保証する
（保証の scope は root inode が属する FS tree — unix mount/bind
mount 横断は kernel namespace の実体として通過する。現 SV の
canonicalize+containment も同一の限界を持つ非回帰の範囲）。
capability の authority は**取得済み directory handle（inode
subtree）への委譲**であって、path namespace の継続的包含ではない:
取得後に scoped dir 自身が root 外へ rename されても、handle は
移動した subtree を正当に読む（cap-std 系の標準的意味論。
namespace 連続性を各 op で再検証する手段は存在せず、再
canonicalize は本 capability が排除する TOCTOU そのものを再導入
するため行わない）。
Synvelis の `stat` / `pin` / `open_preview` / `open_pinned` /
browse-descend に必要な同一保証を、path 再解決ではなく
handle-relative primitive で提供する。

## Requirements

### Functional

- `RootedDir::open(path)` が canonicalize 済み root の dir handle
  capability を返す。root path 解決が唯一の trust boundary であり、
  以後すべての操作は handle 相対に行う。
- `RelativePath`: `/` 区切りの検証済み相対 path（UTF-8・非空
  component・`..` / `.` / 絶対 / `\` / `:` / NUL / 制御文字 /
  Windows 予約名・末尾 `.`/空白・component ≤ 255 byte を拒否）。
  **空文字列は 0 成分として受理**し、この dir 自身の列挙・facts・
  scoped dir を表す（root 列挙の entry point）。
- `FollowMode` は `NoFollow`（link component を一切辿らない）と
  `WithinRoot`（bounded hops で root 内 symlink を辿る — 現 SV
  canonicalize+containment 契約の意味論）を区別する。`WithinRoot`
  は中間 component の link も leaf link も同じ規則で解決する。
- `metadata(rel, mode)` は leaf link 判定（no-follow leaf kind）と
  resolved object の `NodeInfo{kind, byte_len, modified, identity}`
  を返す（SV stat の「link であれば link として報告しつつ target の
  facts を出す」意味を構成できる）。
- `open_regular(rel, mode)` は同一 handle の metadata/length と
  `Read + Seek` reader を対応させて返す。nonregular（dir / link
  under NoFollow / FIFO / device / socket）は通常の blocking open
  へ流さず、typed 失敗で返す。
- `NodePin`（opaque identity token — object identity であり
  content digest ではない）を `pin` 相当に提供し、
  `open_pinned(rel, pin, mode)` は resolve した object の identity が
  pin と一致しない場合 typed `IdentityMismatch` で失敗する
  （差し替えられた object の bytes を返さない）。**identity が
  弱い FS**（kernel-exported handle も birth time もない FS）では
  `pin`/`open_pinned` は typed `Unsupported` を返し、差し替え検出
  の嘘の保証を名乗らない。
- `read_dir(rel)` は leaf の no-follow kind（link を可視化）を持つ
  entry 列を返し、`open_dir(rel, mode)` は同じ capability 内に閉じた
  scoped dir を返す。browse 列挙と後続 open は同じ capability 所有と
  する。`NodeId`（Eq + Hash）は browse 側の loop guard 集合に使える。
- `WithinRoot` の link 解決は fd-stack walk で行い、path 文字列を
  root 配下で再解決しない。絶対 link target は canonical root
  prefix 一致を lexical に判定したうえで、raw suffix を fd-stack
  で解決する（`..` は resolved ancestor handle の pop — kernel の
  post-substitution 意味に忠実。root floor を割ると `EscapesRoot`）。
  「外を経由して戻る」形の target は保守的に `EscapesRoot`（意図的
  な縮小）。hop 上限・loop 検出・component/depth/fd budget は
  typed 失敗で報告する。
- scoped dir（`open_dir` の返り値）は capability root からの
  resolved ancestor handle chain を保持し、link target 中の `..`
  は chain 内を pop できる（SV の `../b/x` 相当の合法的な in-root
  link target を scoped 相対でも辿れる）。

### Non-functional

- `unsafe` は `#[expect(unsafe_code)]` + SAFETY comment の局所許可の
  み（IO-IMP-02 / IO-IMP-10 と同規約）。lint gate を弱体化しない。
- 新規外部依存なし。`libc`（`cfg(unix)`）と `windows-sys`
  （`cfg(windows)`）のみ。`cap-std` 等の capability framework は
  導入しない。
- `unix` / `windows` 以外の target、および要求した保証を FS が
  提供できない場合は typed `Unsupported` を返し、path-based
  fallback に縮退しない。
- backend 型・OS error 型・feature 名を公開 API に露出しない。
- 全 API 非 RT — filesystem I/O にブロックし、alloc する。
- 公開 mock backend は追加しない（既存 private fault test の流儀を
  継続し、`#[cfg(test)]` scripted backend で検証する）。

## Out of Scope

- 書込み側 capability（create / remove / rename）の公開。
  archive の staging write 契約は archive の私有責務として残る。
- `ExtractedTree` への readback API・archive の公開契約変更。
- SV / AudioLens 側の consumer 実装修復（別 owner）。
- `openat2` / `RESOLVE_BENEATH` の kernel 追加保証（deferred 候補、
  Issue #304 と同じ扱い）。
- filesystem 全攻撃を一般に解く万能 framework・永続的な
  identity store・network FS の保証拡張。
- root 自身の path 解決（`RootedDir::open` 時点の canonicalize）
  より前の trust boundary — root 作成・権限管理は呼び出し側。

## Acceptance Criteria

1. `canonicalize → swap → open` 形状の決定的 barrier/fault 回帰が
   RED で具体化され、同一 handle 境界で GREEN になる。対象は leaf
   と intermediate の両方、`open_regular` / `metadata` / `open_pinned`
   / `read_dir` + `open_dir` の各経路。
2. root 内 symlink（leaf・中間・browse 降下の link→dir）の正常系が
   `WithinRoot` で維持される。`NoFollow` は link を辿らず、leaf kind
   `Symlink` を観測できる。
3. escape（絶対 link・`..` 溢れ・外 root への link target）・
   loop（link 環・hop 上限）・limit（component/depth budget）・
   nonregular（dir への open_regular・FIFO・link under NoFollow）・
   reparse（Windows junction/symlink 差し替え）・失敗分類
   （`EscapesRoot`/`LoopDetected`/`NotRegularFile`/`IdentityMismatch`/
   `NotFound`/`Unsupported` 等）が意味ある最低層で検証される。
   未実行 OS・実電源断等は Pass と報告しない。
4. 差し替え後に `open_pinned` が pinned identity と不一致なら
   bytes を返さず typed 失敗になる（pin → swap → open の決定的
   回帰）。
5. `ruviform-io-archive` の公開契約・`tests/extract_*.rs`・
   staging/cleanup/recovery 挙動に非回帰（同一 OS ルールを
   io-file 側へ集約した後も extract session は同一 primitive 経由で
   動く）。
6. `mise run fmt-check` / `check` / `lint` / `test`（workspace）と
   所有 lane の検証が実 head で実行・記録される。

## Constraints

- `unsafe` 局所許可・lint/CI ゲート・`publish = false` 継承・
  MSRV 1.89・公開 enum `#[non_exhaustive]` 規約に従う。
- SV 側の消費は別 owner（Synvelis 修復）— 本 spec は RF 側の
  最小候補契約を提供するのみで、SV provider 実装は変更しない。
- 他者 worktree と並行作業のため、共有 file（ルート `Cargo.toml`
  members・`Cargo.lock`・`crates/README.md`・docs index）の変更は
  必要最小限とし、`feature/rooted-read-capability` branch で完結する。
