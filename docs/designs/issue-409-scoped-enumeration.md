---
id: "DESIGN-034Tnjz1ofC4OXppkHjsgB"
type: "design"
status: "approved"
title: "Issue #409 scoped re-root + bounded enumeration 設計"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/409"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/issue-409-scoped-enumeration.md"
    - "rooted-read-capability.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-409-scoped-enumeration.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #409 scoped re-root + bounded enumeration 設計

## Context

`rooted` capability（#398 導入済）は「保持 directory handle が権威」
の境界を持つが、Consumer（SV PR180）接続で二つの不足が判明した
（spec 正本: `docs/specs/issue-409-scoped-enumeration.md`）:

1. 保持 handle を**新しい floor** として委譲する API がなく、
   Consumer は `canonical_root` pathname 再 open + Strong `NodeId`
   照合で workaround している（Weak FS 拒否・rename 後 scope 失敗）。
2. `read_dir`→`VfsDir::list` が全件 Vec 化のため、列挙中に
   max_entries/cancel/deadline を適用できない。

所有契約: capability 境界の責務は `ruviform-io-file` の rooted/vfs
層が持つ。 Consumer 側 workaround を恒久化せず、この境界で修復する。

## Scope

### In Scope

- `RootedDir::open_scoped_root`（re-root API）。
- `RootedDir::read_dir_each` + `EntryFlow`/`EnumerationOutcome`
  （bounded 列挙 API）、`read_dir` の collect 化。
- vfs `VfsDir::list_each` 必須 op + `list()` default collect
  （unix/windows/unsupported/mock 全 backend）。
- mock の handle-lifetime 忠実化（rename 後も保持 handle が生きる
  tree-model）と lazy 列挙 + counter/fault 注入。
- crate README・doc 更新、必要最小の mise task。

### Out of Scope

- 新外部依存・汎用 FS framework・iterator 公開 API。
- 既存 API の意味変更（`open_dir` floor 共有、`canonical_root()`
  シグネチャ、`read_dir` 全件返却、unsafe 新規拡散）。
- archive 側利用形態・Consumer 側 workaround 撤去（別担当）。

## Components and Boundaries

### `open_scoped_root(&self, rel, mode) -> Result<RootedDir, FileError>`

- `self.resolve(rel, mode)` で `Resolved::Dir` を得る（Leaf →
  `NotDirectory`）。
- 新 `RootedDir` は `chain = [resolved_dir]`（解決した handle のみ —
  元 chain の祖先を引き上げない）。これにより新 scope 内の `..`
  （link target 由来）は floor で `EscapesRoot`。
- 新 `RootShared`: options は親を引き継ぎ（Copy）、lexical anchor は
  `resolved_dir.canonicalize_handle()` のみ。失敗（`Unsupported` 含む
  全 error）は `FileErrorKind::Unsupported` で API 失敗 —
  **path 文字列からの再 bind・anchor なしの黙示縮小は行わない**。
  - 判断記録: anchor なし scope（absolute link 全拒否）は
    coordinator 指針と Issue「保証捏造禁止」に照らし、scoped
    capability として absolute-link 契約を果たせないため typed
    `Unsupported` を採用（`pin`/`open_pinned` の Weak-identity
    Unsupported と同じ fail-closed 系）。これは identity 比較では
    なく lexical anchor の提供可否という別軸であり、Weak NodeId
    の一律拒否とは区別される。
- 権威: 返された handle が唯一の authority — `NodeId` 照合・
  Strong identity 要求は一切行わない（Weak FS でも動作）。
- root rename/replacement 後も `canonicalize_handle` は handle の
  現行 canonical を返し、新 scope anchor は新 floor の真値となる。

### `read_dir_each(&self, rel, mode, visit) -> Result<EnumerationOutcome, FileError>`

- `resolve_dir(rel, mode)` で dir handle を得て
  `VfsDir::list_each` に visitor を委譲。
- `visit: &mut dyn FnMut(DirEntry) -> EntryFlow`;
  `EntryFlow::{Continue, Stop}`（`#[non_exhaustive]`）。
- `EnumerationOutcome::{Complete, Stopped}`（`#[non_exhaustive]`）。
  OS error は `Err(FileError)`。完走 / 早期停止 / error の三態を
  正直に区別。
- `read_dir` は `read_dir_each` で全件 `Vec` 化する薄い wrapper
  に改修（列挙規則の単一正本）。公開契約・戻り値は不変。

### vfs `VfsDir::list_each`

```rust
fn list_each(&self, visit: &mut dyn FnMut(VfsEntry) -> VisitFlow)
    -> io::Result<ListingOutcome>;
fn list(&self) -> io::Result<Vec<VfsEntry>> { /* default: collect via list_each */ }
```

- `VisitFlow::{Continue, Stop}`、`ListingOutcome::{Complete, Stopped}`。
- unix: `openat(dirfd,".")` 新規 fd + `fdopendir` で独立 `DIR*`。
  per-entry `readdir` → `dirent_kind`（`DT_UNKNOWN` はその entry の
 み `dir.probe`、visitor 前に一括 probe しない）→ callback。
  `Stop` で即時 return（以後の readdir/probe/callback なし）。
  `DIR*` は **Drop guard** で閉じる（visitor panic でも unwind 中に
  `closedir` — panic 安全）。
- windows: 既存 `NtQueryDirectoryFile` 64KB 固定 batch を維持
  （Issue が許容する OS 内部固定 batch）。取得済み record を
  callback し、`Stop` で次の `NtQueryDirectoryFile` を発行せず
  return。callback panic は OS 資源を新規確保しないため自然に安全。
- unsupported backend: `Unsupported`。
- mock: tree-model 化 — `TestNode` に `children: BTreeMap`（dir）と
  `path` を持たせ、`MockVfsDir` は node `Arc` を保持（path-keyed
  lookup 廃止 → rename 後も handle が生きる）。列挙は cursor 付き
  lazy pull + `enum_pulled`/`probe` counter・`list_fail_at` 注入で
  「停止後に OS 進捗なし」を検証可能にする。`list()` 自体は
  trait default で collect。

## Data and Control Flow

- `open_scoped_root`: `rel` resolve（親 scope 契約）→ dir handle →
  `canonicalize_handle` → 新 `RootShared{canonical, comps, options}`
  + `chain=[dir]` → 以後の resolve は新 floor を root として
  既存二状態 walk（TEXTUAL prefix は新 canonical）。
- `read_dir_each`: `rel` resolve → dir handle → backend `list_each`
  → entry ごとに `DirEntry` 変換 → `visit` → `Stop` で backend が
  enumeration state を即時閉じ `Stopped` / 完走で `Complete` /
  OS error で `Err`。

## Non-Functional Constraints

- non-RT only（既存契約どおり FS I/O・alloc を伴う）。
- memory bound: backend が保持するのは OS batch（unix `DIR*`
  内部 / windows 64KB buffer）と現在 entry のみ — 全量 Vec を
  backend で作らない。
- panic 安全: unix `DIR*` は Drop guard。visitor panic はそのまま
  伝播させ、次回列挙は独立 state で動作する。
- 列挙順は backend 順で非整列（`read_dir` 既存契約と同一）。

## Risks and Trade-offs

- anchor 必須化により Linux `/proc` 不在等の稀 backend で
  re-root が `Unsupported` — 縮小契約（anchor なし）は偽
  EscapesRoot を生むため採用せず、需要があれば別 Issue で検討
  （spec Deferred に記録）。
- mock tree-model 化は test double の internal 変更のみで、
  既存 `put`/`remove` ヘルパの使用形態（parent-first 順）を維持し
  archive mock（別物）へ影響しない。
- windows batch は `Stop` 時点で batch 内の未 visit entry を
  破棄するだけで追加 OS call なし — Issue の明示許容範囲。
- 既知限界（既存・fail-closed）: UNC canonical anchor（`UNC`
  成分を含む）と link-bridge の `\\srv\sh` 分解で成分形が一致せず、
  UNC root 配下の absolute link はすべて `EscapesRoot` —
  `open_scoped_root` もこの既存限界を継承するが、escape を
  生まない保守側の失敗であり本 Issue の範囲外（#409 差分では
  変更しない）。
- windows の列挙独立 state は FILE_OBJECT 単位 — `NtOpenFile`
  empty-name 再 open で呼出ごとの scan handle を確保する
  （共有 handle のままでは再入/並行列挙が scan position を
  奪い合い静かに truncate する）。unix は `openat(fd,".")` の
  fresh fd で同等。

## Deferred Design Concerns

- Consumer 接続（synvelis `open_scope`/`collect_entries` 改修）—
  別担当の後続作業。
- anchor なし縮小 scope — 需要確認時に別 Issue。
- `read_dir_each` の generic-error 返却型 — 現行は consumer 側が
  自身の error を capture slot に保持し `Stop` 後に回復する形。
  要望が出た場合に別 Issue で検討。

## Verification Notes

- [ ] mock: root rename 後に保持 root の `open_scoped_root` が継続
  動作・新 scope の `..`/sibling/absolute escape 拒否・anchor 失敗
  で `Unsupported`・Weak identity で拒否なし。
- [ ] mock: lazy 列挙で `enum_pulled` が prefix 件数に止まる・
  `list_fail_at` で error・callback 即時 Stop（max_entries=0 相当）・
  2 回連続列挙の独立 state・panic 後の再列挙成功。
- [ ] real FS（当該 OS 実測のみ）: rename/replacement・scoped
  escape・bounded prefix・`read_dir` 既存回帰・archive extract
  回帰。
- [ ] 正規 `mise run fmt-check/check/lint/test` + io 対象 lane、
  linux cross-check（`check-linux`）で unix backend を検査。
- [ ] 未実測 OS は NotRun 明記。
