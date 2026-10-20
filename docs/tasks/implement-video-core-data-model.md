---
id: "TASK-034J5KR5ueilYgXdyyF4ZU"
type: "task"
status: "done"
title: "ruviform-video-core のデータモデル実装"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/18"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-core/src/color.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/frame.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/pixel.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/plane.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-video-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-core/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-18-data-model.md"
  implemented-by:
    - "../impl/ir/implement-video-core-data-model.md"
  depends-on:
    - "../plans/implement-issue-18-data-model.md"
    - "implement-media-core-data-model.md"
  blocks:
    - "sync-docs-and-verify-issue-18.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-18-data-model.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/media-data-model-api.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-video-core のデータモデル実装

## Work

plan `PLAN-034J5K603qXDNmT82wI9v7` の Phase C。`tmp/issue-18/api-brief.md`「Crate 3 — ruviform-video-core」+ Global constraints を正本として、plane/stride/pixel/色・PTS の動画データモデル API を実装する（実績としての遡及記録）。

- [x] `Cargo.toml` に `ruviform-media-core = { path = "../ruviform-media-core" }` を追加（`Cargo.lock` 更新）
- [x] `pixel` モジュール: `#[non_exhaustive] PixelFormat { Yuv420p, Yuv422p, Yuv444p, Rgba8, RgbaF16 }`（`plane_count` / `is_planar` / `bytes_per_sample` / `chroma_subsampling`、全 `const fn`）
- [x] `color` モジュール: `ColorPrimaries` / `TransferCharacteristics` / `MatrixCoefficients` / `ColorRange`（各 `#[default] Unspecified`）・`Chromaticity`・`MasteringDisplayInfo`・`ContentLightLevel`・`HdrMetadata`・`ColorInfo`
- [x] `plane` モジュール: 借用 `Plane<'a>`（`Copy`）・`PlaneMut<'a>`・`SharedPlane { data: Arc<[u8]>, stride }`（stride は `width×bytes` と独立・パディング明示、stride < row_bytes と不足 backing を構築時に拒否）
- [x] `frame` モジュール: `VideoFormat`・`VideoFrame`（`new_owned` ゼロ初期化・`from_shared_planes` 検証済み zero-copy 供給・`MediaTime` PTS・`Option<MediaDuration>`・`Clone` は `Arc` 共有・共有中 `plane_mut` は `None`）
- [x] 全公開 API に `**RT contract:**` ラベル、`lib.rs` crate doc 更新 + 再エクスポート + `version()` 維持
- [x] `rt_` テスト 3 本（`rt_plane_access_and_row_slicing`・`rt_plane_mut_row_write`・`rt_timestamp_access`）と doctest/単体テスト整備

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-18-data-model.md](../plans/implement-issue-18-data-model.md) — `PLAN-034J5K603qXDNmT82wI9v7`（completed）。Phase C の正本
- [../specs/implement-media-data-model-api.md](../specs/implement-media-data-model-api.md) — `SPEC-034J5FYaYfeL5zkzb4sIV4`（implemented）。動画データモデル要件の正本
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。「動画データモデル」節の根拠

## Dependencies

- 前提タスク: [implement-media-core-data-model.md](implement-media-core-data-model.md)（`TASK-034J5KJgwtjFonTMQQIvpX`）— path 依存先の `MediaTime` / `MediaDuration` / `Error` / `Result` を提供
- 後続をブロック: [sync-docs-and-verify-issue-18.md](sync-docs-and-verify-issue-18.md)（総合検証対象を提供）
- `ruviform-audio-core` への依存は作らない（層内順序ルール）。外部クレート依存ゼロ・`unsafe` 不使用

## Affected Files

- `crates/ruviform-video-core/src/pixel.rs` — 新規
- `crates/ruviform-video-core/src/color.rs` — 新規
- `crates/ruviform-video-core/src/plane.rs` — 新規
- `crates/ruviform-video-core/src/frame.rs` — 新規
- `crates/ruviform-video-core/src/lib.rs` — crate doc・再エクスポート・`rt_` テスト更新
- `crates/ruviform-video-core/Cargo.toml` — `ruviform-media-core` path 依存追加
- `Cargo.lock` — path 依存の記録

## Done When

- [x] brief の全型・署名が公開 API として存在し、crate root から再エクスポートされる
- [x] plane 寸法が `PixelFormat::chroma_subsampling()` から導出され（420/422 は `ceil` chroma）、stride・backing 長が検証される
- [x] `Arc` 共有 backing 中は `plane_mut` が `None` を返す
- [x] 実施記録 [../impl/ir/implement-video-core-data-model.md](../impl/ir/implement-video-core-data-model.md) に検証結果が残っている

## Verification

- [x] `mise exec -- cargo test -p ruviform-video-core` — 23 単体テスト + 6 doctest 合格（修正 wave 後の最終値。初回実施時は 20 + 6）
- [x] `mise exec -- cargo clippy -p ruviform-video-core --all-targets -- -D warnings` — 0 warnings
- [x] `mise exec -- cargo test -p ruviform-video-core rt_` — `rt_` プレフィックス 3 テストのみ選択・合格（A2 指摘の `view_rejects_short_backing` は `view_rejects_undersized_backing` へ改名済み）
- [x] `mise run fmt-check` — 差分なし
