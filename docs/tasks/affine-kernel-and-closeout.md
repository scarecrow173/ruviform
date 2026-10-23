---
id: "TASK-034KSVq7PFSELAOY00SIZG"
type: "task"
status: "todo"
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

- [ ] `AffineTransform`/`Rect`/`RectMap` が design の形状で実装される
  （`new()` 無引数 + `Default`・`process(src, dst, map)`・
  写像演算 `i128`/`u128`・境界検査 `i64` 拡幅・セル中心最近傍・
  `map.src` 検証 `InvalidValue("src rect")`・`map.dst` キャンバス
  非交差はエラーとせず全画素 transparent black・`src` のメタデータ
  伝播・`**RT contract:**` ラベル + 参照実装/bit-exact 契約の doc）
- [ ] test-spec 全ケースパス（`CompositeOver`/`Crossfade` 分を含む —
  拡大複製 trap vector・負 `map.dst.x`・空/範囲外矩形・検証拒否・
  `rt_process_*`）
- [ ] `lib.rs` の「Kernel API shape」段落・カーネル一覧・re-export が
  実装状況と整合し予約記述が差替えられる
- [ ] crate README + `crates/README.md` 索引行・検証列全パス・
  doc 監査チェーン起因 findings 0

## Verification

- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス
- [ ] `doc_status.js audit` findings 0・依存が
  `ruviform-video-core`/`ruviform-media-core` のみ（Cargo.toml 確認）
