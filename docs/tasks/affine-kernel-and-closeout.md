---
id: "TASK-034KSVq7PFSELAOY00SIZG"
type: "task"
status: "done"
title: "AffineTransform カーネル + 文書クローズ"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-proc/src/affine.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-97-composite-kernels.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-97-composite-kernels.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-97-composite-kernels.md"
    - "docs/tasks/composite-kernels.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# AffineTransform カーネル + 文書クローズ

## Work

`affine.rs` 新設 — `Rect`/`RectMap` パラメータ型と `AffineTransform`
（`process(src, dst, map)` — 整数矩形写像 + セル中心最近傍 +
transparent-black 充填の完結変換）を実装し、`lib.rs` crate doc の
「Kernel API shape」段落・カーネル一覧・re-export を更新、crate
README・`crates/README.md` 索引・検証列・doc 監査で Issue #97 の
doc チェーンを閉じる（plan 手順 4–6 + 文書クローズ）。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-issue-97-composite-kernels.md
- docs/tasks/composite-kernels.md
- docs/designs/composite-transition-kernels.md
- docs/specs/composite-transition-kernels.md
- docs/test-specs/composite-transition-kernels-tests.md

## Dependencies

- docs/tasks/composite-kernels.md（共有 f16 コーデック格上げ +
  `composite.rs` の検証/演算規約の先行）
- `ruviform-video-core`・`ruviform-media-core`（新規依存なし）

## Affected Files

- crates/ruviform-video-proc/src/affine.rs（`Rect`/`RectMap` pub
  フィールド plain data + `AffineTransform` + inline tests）
- crates/ruviform-video-proc/src/lib.rs（`mod affine` +
  `pub use affine::{AffineTransform, Rect, RectMap}`・「Kernel API
  shape」段落へコンポジット族の `process` 可変引数規約追記・
  カーネル一覧へ 3 カーネル追加・「Scaling and compositing kernels
  stay reserved for later issues」記述の差替え）
- crates/ruviform-video-proc/README.md（カーネル一覧・保留記述を
  実装状況へ更新）
- crates/README.md（`ruviform-video-proc` 索引行の説明更新）
- docs/*（チェーン frontmatter: task status・plan Verification
  チェック — design/spec/test-spec の reciprocal relation は本
  task 着手時点で完了済みのはず。不整合があればここで修正）

## Done When

- [x] `AffineTransform`/`Rect`/`RectMap` が design の形状で実装される
  （`new()` 無引数 + `Default`・`process(src, dst, map)`・
  写像演算 `i128`/`u128`・境界検査 `i64` 拡幅・セル中心最近傍・
  `map.src` 検証 `InvalidValue("src rect")`・`map.dst` キャンバス
  非交差はエラーとせず全画素 transparent black・`src` のメタデータ
  伝播・`**RT contract:**` ラベル + 参照実装/bit-exact 契約の doc）
- [x] test-spec 全ケースパス（`CompositeOver`/`Crossfade` 分を含む —
  拡大複製 trap vector・負 `map.dst.x`・空/範囲外矩形・検証拒否・
  `rt_process_*`）
- [x] `lib.rs` の「Kernel API shape」段落・カーネル一覧・re-export が
  実装状況と整合し予約記述が差替えられる
- [x] crate README + `crates/README.md` 索引行・検証列全パス・
  doc 監査チェーン起因 findings 0

## Verification

- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス
- [x] `doc_status.js audit` findings 0・依存が
  `ruviform-video-core`/`ruviform-media-core` のみ（Cargo.toml 確認）

## Completion Notes

- `affine.rs` 新設: `Rect {x: i32, y: i32, w: u32, h: u32}`・
  `RectMap {src: Rect, dst: Rect}`（pub フィールド plain data・
  `Copy`/`Clone`/`Debug`/`PartialEq`/`Eq`・左上原点・整数画素座標 —
  `dst` 側 `x`/`y` は負値許容）と `AffineTransform`（`new()` 無引数 +
  `Default` impl・`process(src, dst, map)`）を実装。`map` は `Copy`
  plain data のため値渡しとした（design 表記 `&RectMap` からの意図的
  変更 — 呼出し側が literal を借用せず渡せる形状）
- 写像演算: `cell_center_offset` ヘルパが `(2·offset·src_len +
  src_len)/(2·dst_len)` を `u128` で正確評価（`offset < dst_len`
  より商は構造的に `< src_len` — クランプ不要）。`map.dst` の
  キャンバスクリップと `map.src` のフレーム内検証は `i64` 拡幅で
  評価（`x + w` の桁上がりなし）。`map.src` 非空・`x`/`y` ≥ 0・
  `x + w ≤ width`・`y + h ≤ height` 違反は `InvalidValue("src
  rect")` で全検証を書込み前に実施
- 非交差/空 `map.dst` は `Option` の span 計算で空レンジへ畳み込み
  全行が black-fill パスを取る定義済み結果（エラーにしない）。
  `dst` は全画素書換え — span 外行・span 行の先頭/末尾は
  transparent black（両フォーマットで 0 byte）。画素値を演算しない
  `sample_bytes` 単位の複写のみのため `Rgba8`/`RgbaF16` は共通パス
  で bit-exact。`dst` メタデータへ `src` の
  `ColorInfo`/`pts`/`duration` を伝播
- テスト: affine.rs `#[cfg(test)] mod tests` に 18 件（恒等写像
  bit-exact・sub-rect crop+配置・1→2 複製 `[0,0]`・2→4 複製
  `[0,0,1,1]` 最終列 in-range・4→2 縮小 `[1,3]`・垂直方向・負
  `dst.x` クリップ・`dst` 全書換え・空/完全オフキャンバス矩形 →
  全 transparent black・`map.src` 検証拒否 6 パターン・遠端境界
  in-range・format/共有 backing 拒否・`RgbaF16` パス・メタデータ
  伝播・両側 padded stride 非破壊・`rt_process_*` 2 件）+
  doctest 2 件 — test-spec 全ケースパス
- `lib.rs`: `mod affine` + `pub use affine::{AffineTransform, Rect,
  RectMap}`。「Kernel API shape」段落へコンポジット族の `process`
  可変引数規約（`process(src, dst, opacity)` / `process(from, to,
  dst, t)` / `process(src, dst, map)`）を追記。カーネル一覧へ
  `CompositeOver`/`Crossfade`/`AffineTransform` を追加し「Scaling
  and compositing kernels stay reserved for later issues」を
  bilinear 以上・フルアフィン・追加 blend・planar YUV の保留記述へ
  差替え
- crate README: Issue #97 カーネル群を責務節へ追加し二相形状段落へ
  `process` 可変引数規約を追記・保留記述を更新。`crates/README.md`
  索引行を「色変換・コンポジット/遷移」へ更新。`docs/tasks/README.md`
  の両 task 行を `done` へ、plan の Implementation Sequence・
  Verification の全チェックを完了へ更新（spec の Acceptance
  Criteria は convention どおり未チェックのまま）
- 検証: `mise run fmt`/`fmt-check`/`check`/`lint`/`test`/`test-rt`
  全パス・`doc_status.js audit` findings 0。依存は
  `ruviform-video-core`・`ruviform-media-core` のみ（Cargo.toml
  確認済み）
