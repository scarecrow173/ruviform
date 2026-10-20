---
id: "TASK-034J6PiHv9GsKby7yH2eMO"
type: "task"
status: "done"
title: "全公開 enum への `#[non_exhaustive]` 適用と exhaustive_enums allow の撤去"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/41"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/error.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/time.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/transport.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/layout.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/color.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/lib.rs"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/issue-18-followup-enum-and-time-fix.md"
  implemented-by:
    - "../impl/ir/apply-non-exhaustive-to-public-enums.md"
  depends-on:
    - "../plans/issue-18-followup-enum-and-time-fix.md"
  blocks:
    - "fix-time-arithmetic-saturation.md"
    - "small-fixes-from-followup-triage.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/issue-18-followup-enum-and-time-fix.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/followup-enum-compat-and-time-arithmetic.md"
  references: []
  defers: []
  deferred-by: []
---

# 全公開 enum への `#[non_exhaustive]` 適用と exhaustive_enums allow の撤去

## Work

plan `PLAN-034J6OTwge4rS96WZWkZgd` の Phase T1。enum ADR `ADR-034J6OnZouPI8HQy6cIg9D`（公開 enum `#[non_exhaustive]` 統一方針）を data-model 3 クレートに適用する。

- [x] 9 件の公開 enum の `pub enum` 直上に enum レベルの `#[non_exhaustive]` を付与する（variant レベルではない）。対象:
  - `crates/ruviform-media-core/src/error.rs` — `Error`
  - `crates/ruviform-media-core/src/time.rs` — `Rounding`
  - `crates/ruviform-media-core/src/stream.rs` — `StreamKind`
  - `crates/ruviform-media-core/src/transport.rs` — `TransportStatus`
  - `crates/ruviform-audio-core/src/layout.rs` — `ChannelLayout`
  - `crates/ruviform-video-core/src/color.rs` — `ColorPrimaries`・`TransferCharacteristics`・`MatrixCoefficients`・`ColorRange`
  - 既存の `#[non_exhaustive]`（`crates/ruviform-audio-core/src/format.rs` `SampleFormat`、`crates/ruviform-video-core/src/pixel.rs` `PixelFormat`）は維持・変更不要
- [x] 3 クレートの `lib.rs` から `#![allow(clippy::exhaustive_enums)]` を撤去し、隣接ポリシーコメントを「`publish = false` 内部クレートとして exhaustive matching に依存する」旨から「全公開 enum に enum レベル `#[non_exhaustive]` を適用する統一ポリシー（enum ADR への参照付き）」旨へ書き換える（`#![allow(clippy::module_name_repetitions)]`・`#![allow(clippy::exhaustive_structs)]` は現行維持）
- [x] `AGENTS.md` 「暫定規約」節にポリシー 1 行を追記する（例: 「公開 enum: クレート外から到達可能な全公開 enum に enum レベルの `#[non_exhaustive]` を付与する（`docs/adr/public-enum-non-exhaustive-policy.md`）」）
- [x] クレート間でこれら enum への exhaustive `match` が存在しないことを再 grep で確認する（現時点で存在しないことを確認済み。`match self` 等の同一クレート内 match は `#[non_exhaustive]` 下でも合法）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/issue-18-followup-enum-and-time-fix.md](../plans/issue-18-followup-enum-and-time-fix.md) — `PLAN-034J6OTwge4rS96WZWkZgd`（approved）。Phase T1 の正本
- [../adr/public-enum-non-exhaustive-policy.md](../adr/public-enum-non-exhaustive-policy.md) — `ADR-034J6OnZouPI8HQy6cIg9D`（accepted）。適用規則（enum レベル・閉集合包含・兄弟クレート=外部・内部 enum 免除）の正本
- [../specs/issue-18-followup-enum-and-time-fix.md](../specs/issue-18-followup-enum-and-time-fix.md) — `SPEC-034J6OCXHiP1izKRzbMmgz`（approved）。AC1 の正本

## Dependencies

- 前提: plan のみ（`relations.depends-on`）。先行する他 task への依存なし（T1 が先頭）
- 後続をブロック: `fix-time-arithmetic-saturation.md`・`small-fixes-from-followup-triage.md`（順序 T1→T2→T3。T3 は audio-core `layout.rs`/`lib.rs` を本 task と共有）
- 外部クレート依存・`unsafe` なし（属性付与とコメント/doc 編集のみ）

## Affected Files

- `crates/ruviform-media-core/src/error.rs`、`time.rs`（`Rounding` のみ。時間演算修正は T2）、`stream.rs`、`transport.rs` — enum 属性付与
- `crates/ruviform-audio-core/src/layout.rs` — `ChannelLayout` 属性付与（`Custom` 構造変更は T3）
- `crates/ruviform-video-core/src/color.rs` — 色 enum 4 件属性付与
- `crates/ruviform-media-core/src/lib.rs`、`crates/ruviform-audio-core/src/lib.rs`、`crates/ruviform-video-core/src/lib.rs` — allow 撤去 + コメント更新
- `AGENTS.md` — 暫定規約 1 行追記
- 確認のみ（変更不要）: `crates/ruviform-audio-core/src/format.rs`、`crates/ruviform-video-core/src/pixel.rs`

## Done When

- [x] 監査リストの全 11 enum に enum レベル `#[non_exhaustive]` が付き（9 件新規 + 2 件維持）、クレート内部限定 enum には付与されていない
- [x] 3 `lib.rs` の `#![allow(clippy::exhaustive_enums)]` が撤去され、コメントが enum ADR を指す内容に更新されている
- [x] `AGENTS.md` 暫定規約にポリシー行がある
- [x] `mise run lint`・`mise run check`・`mise run test` がグリーン（allow 撤去下で 0 warnings、兄弟クレートが non_exhaustive enum に依存したままビルド・テスト通過）

## Verification

- [x] `mise exec -- cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings（`exhaustive_enums` が新規 exhaustive enum を検出する状態）
- [x] `mise exec -- cargo test -p ruviform-media-core`・`mise exec -- cargo test -p ruviform-audio-core`・`mise exec -- cargo test -p ruviform-video-core` — 合格
- [x] `grep -rn "#\[non_exhaustive\]" crates/*/src` — 11 enum（`Error`・`Rounding`・`StreamKind`・`TransportStatus`・`ChannelLayout`・`ColorPrimaries`・`TransferCharacteristics`・`MatrixCoefficients`・`ColorRange`・`PixelFormat`・`SampleFormat`）に付与を確認
- [x] `grep -rn "exhaustive_enums" crates/*/src/lib.rs` — 0 件
- [x] クレート間 exhaustive `match` 確認（`grep` で media-core の enum への `match` が audio/video-core に存在しないこと）— 現時点で存在しないことを確認済みの再確認
- [x] `mise run fmt-check` — 差分なし
