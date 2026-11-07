---
id: "SPEC-034Tnjp8f4gexhi9BsQuq1"
type: "spec"
status: "implemented"
title: "Issue #409 Rooted capability の再 root 化と停止可能な bounded directory 列挙"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/409"
    - "https://github.com/scarecrow173/ruviform/issues/398"
  changes:
    added: []
    modified:
      - path: "crates/ruviform-io-file/src/vfs.rs"
      - path: "crates/ruviform-io-file/src/vfs/unix.rs"
      - path: "crates/ruviform-io-file/src/vfs/windows.rs"
      - path: "crates/ruviform-io-file/src/vfs/unsupported.rs"
      - path: "crates/ruviform-io-file/src/vfs/mock.rs"
      - path: "crates/ruviform-io-file/src/rooted.rs"
      - path: "crates/ruviform-io-file/src/rooted/tests.rs"
      - path: "crates/ruviform-io-file/tests/rooted_read.rs"
      - path: "crates/ruviform-io-file/README.md"
      - path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../plans/issue-409-scoped-enumeration.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "rooted-read-capability.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../designs/issue-409-scoped-enumeration.md"
    - "../plans/issue-409-scoped-enumeration.md"
    - "../test-specs/issue-409-scoped-enumeration.md"
  verifies: []
  verified-by:
    - "../test-specs/issue-409-scoped-enumeration.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #409 Rooted capability の再 root 化と停止可能な bounded directory 列挙

## What Should Be Built

`ruviform-io-file` の `rooted` capability に二つの公開契約を追加する
（Issue #409 本文を正本とする）。

1. **再 root 化 API（`open_scoped_root`）**: 既に保持している
   directory capability 上で相対 locator を解決し、解決した
   directory handle を **新しい capability floor** とする
   `RootedDir` を返す。解決は保持 capability 相対で行い、
   pathname の再 open も Strong `NodeId` 照合も不要とする
   （保持 handle 自体が権威）。返された capability の
   `WithinRoot` 解決は、`..` による ancestor escape も
   absolute link target の新 floor 外への解決も拒否する。
   既存 `open_dir`（元 root floor を共有する scoped clone）は
   契約不変とする。
2. **bounded 列挙 API（`read_dir_each`）**: directory 列挙を
   全件 `Vec` 化せず、entry ごとに consumer の visitor へ
   渡す callback 形式とする。consumer は `remaining + 1` 件の
   prefix で打切りを選択でき、各 step で cancel/deadline を
   観測できる。早期停止後は後続の OS 列挙・probe・callback を
   継続しない（OS 内部の固定サイズ batch は明示して許容）。
   完走 / 早期停止 / error を正直に区別し、no-follow の
   entry kind と独立した列挙 state を保持する。
   既存 `read_dir` は同一実装上で collect する形に保つ。

## Why Now

Refs #398・Synvelis scarecrow173/synvelis#85・SV PR180。
Rooted read 導入後の Consumer 接続で二つの能力不足が判明した:

- SV PR180 608c1c7 は scope 取得を `canonical_root` pathname からの
  再 open + Strong `NodeId` 照合で行うため、Weak FS では scope
  取得自体が拒否され、取得済み root の rename 後は scope のみが
  失敗する。別 root への誤結合を防ぐ安全修復だが、保持
  capability の責務を完結できない。
- `RootedDir::read_dir` → `VfsDir::list` は全件 Vec 化後にしか
  Consumer へ制御を戻さず、`max_entries` / cancel / deadline を
  列挙中に適用できない（巨大論理 directory で全量列挙→
  truncate となり、memory bound と打切り契約を満たせない）。

## Users and Value

- **Consumer（synvelis-library 等の asset provider）**: scope
  capability を保持 root の権威から直接派生でき、root rename /
  replacement 下でも取得済み scope の継続と別 authority への
  誤結合防止を Ruviform 側の契約で得られる。また列挙を
  `remaining+1` prefix で打切り、cancel/deadline を各 step で
  観測できるため、browse 契約の memory bound が守れる。
- **ruviform 利用者一般**: rooted capability の保証境界を弱めずに
  scoped browsing と bounded 列挙を使える。

## Scope

### In Scope

- `crates/ruviform-io-file`（`rooted.rs`・`rooted/resolve.rs`・
  `vfs.rs`・`vfs/unix.rs`・`vfs/windows.rs`・`vfs/unsupported.rs`・
  `vfs/mock.rs`）および直接関連する unit/integration test・
  crate README。
- `VfsDir` の列挙プリミティブを callback 形へ拡張し、既存
  `list()` をその上の collect として維持（単一正本）。
- 検証入口に不足があれば最小の mise task 追加。

### Out of Scope

- 新外部依存・汎用 filesystem framework・visitor 以外の列挙
  framework（自由形 iterator 公開など）。
- 既存公開 API の意味変更（`open_dir` の floor 共有契約、
  `canonical_root() -> &Path` のシグネチャ、`read_dir` の
  全件返却契約、unsafe の新規拡散）。
- `ruviform-io-archive` の利用形態変更（vfs を共有する現行を
  回帰させないことのみ要求）。
- Consumer（synvelis）側の workaround 撤去・接続 — 別担当の
  後続作業。SV PR180 は本能力不足解消まで BLOCKED のまま。
- `pin`/`open_pinned` の identity 契約変更、一律 Strong identity
  必須化。

## Requirements

1. `open_scoped_root(&self, rel: &RelativePath, mode: FollowMode)
   -> Result<RootedDir, FileError>`（名称は Issue 例示に倣う）:
   - `rel` を現 capability 内で解決する。非 directory leaf は
     `NotDirectory`。`rel == ""` はこの directory 自身を新 floor
     とする scoped clone。
   - 返される capability の floor は **解決した directory handle
     のみ**（元 capability の祖先 chain は新 floor へ引き上げ
     ない）。新 scope 内の `..`（link target 由来）が新 floor を
     下回れば `EscapesRoot`。
   - 新 floor の lexical anchor は解決した handle の
     `canonicalize_handle()` のみを用いる。path 文字列からの
     再 bind は行わない。handle 由来の canonical を取得できない
     backend は `FileErrorKind::Unsupported` を返す（保証の捏造
     禁止）。absolute link 判定 anchor を欠いた状態での黙示的な
     縮小は行わない。
   - root の rename/replacement 後も、取得済み handle からの
     scope 取得は継続し、旧 pathname の置換物へ別 authority を
     取得しない。
   - identity strength による拒否は行わない（handle が権威であり
     `NodeId` 比較を必要としない）。保証不能な backend のみが
     typed `Unsupported`。
2. `read_dir_each(&self, rel: &RelativePath, mode: FollowMode,
   visit: &mut dyn FnMut(DirEntry) -> EntryFlow)
   -> Result<EnumerationOutcome, FileError>`:
   - `rel` を解決した directory を列挙し、各 entry を visitor
     callback へ渡す。`EntryFlow::Continue` で継続、
     `EntryFlow::Stop` で早期停止。
   - 早期停止後は OS 列挙 call・per-entry probe・callback を一切
     継続しない。OS 内部の固定サイズ batch（Windows
     `NtQueryDirectoryFile` の buffer 等）は取得済み分の処理として
     許容する。
   - 結果は `EnumerationOutcome::{Complete, Stopped}` で完走と
     早期停止を区別し、列挙途中の OS 失敗は `Err` で報告する。
   - 各呼出しは独立した列挙 state（unix は `openat`+`fdopendir`
     による独立 `DIR*`、Windows は `RestartScan`）で、no-follow
     の entry kind（link は `Symlink` のまま）を保持する。
   - visitor が panic しても OS 列挙資源（`DIR*`・fd・buffer）が
     解放されること（panic 安全・Drop guard）。
   - `read_dir` は `read_dir_each` 上で全件 collect する形に
     維持し、列挙規則の単一正本を共有する。
   - 列挙順は backend 列挙順であり、整列を保証しない
     （`read_dir` 既存契約と同一）。
3. vfs 層は `VfsDir::list_each(visitor)` を各 OS backend が
   実装する必須 op とし、`list()` を `list_each` の collect
   default 実装で提供する（単一正本・archive 非回帰）。

## Acceptance Criteria

- [ ] 旧 pathname への root replacement で別 authority を取得
  しない。root rename 後も保持 handle の scope 取得が継続し、
  新 scope 内の ancestor/sibling escape が拒否される。
- [ ] Weak identity を理由に新 API を一律拒否しない。handle 由来
  canonical を提供できない backend では `Unsupported` を返し、
  path-based fallback で保証を捏造しない。
- [ ] 大きい論理 directory で、prefix 件数・OS 列挙進捗の停止・
  error/cancel/deadline の各 step 観測・memory bound を fault /
  mock 回帰で実証する。全量列挙後の truncate は不可。
- [ ] `max_entries = 0` 相当（初回 callback での即時 Stop）・
  早期停止後の後続 OS call 不発行・panic 後の資源解放と次回
  列挙の独立性を検証する。
- [ ] Win/Linux/macOS backend の責務を整合させ、既存
  rooted/archive regression を保持する。実測できない OS は
  NotRun と明記する。
- [ ] 正規 mise 検証（fmt-check/check/lint/test + 対象 lane）と
  doc-driven chain・独立 review を通す。新依存・汎用 filesystem
  framework を導入しない。

## Deferred Design Concerns

- Consumer（synvelis-library `local.rs`）の `open_scope`
  workaround 撤去と `collect_entries` の bounded 列挙接続 —
  Consumer 側変更は別担当の後続作業（SV #85 / PR180 側）。
- 真に handle 由来 canonical を提供できない backend での
  scoped capability — anchor なしの縮小契約（absolute link 全
  拒否）は本 spec では採用せず `Unsupported` とする。需要が
  確認された場合に限り別 Issue で縮小契約を検討する。

## Sources and Relations

- Issue #409（本文・必要な契約・受入条件の正本）。
- Refs #398（rooted read capability 本体 —
  `docs/specs/rooted-read-capability.md`、
  `docs/designs/rooted-read-capability.md`、merged PR #401）。
- Synvelis `scarecrow173/synvelis` Issue #85、PR180 head
  608c1c7（Consumer 参照実装 — `local.rs` `open_scope` の
  Strong `NodeId` workaround と `collect_entries` の全 Vec
  `read_dir` が不足箇所。後続 docs 66f8596）。
