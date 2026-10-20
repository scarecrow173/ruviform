---
id: "IMPL-034J5L4LL7z9gp1QZKqwa9"
type: "impl"
status: "completed"
title: "Implement video-core data model"
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
    - "../../tasks/implement-video-core-data-model.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-issue-18-data-model.md"
    - "../../designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/media-data-model-api.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement video-core data model

## Summary

task `TASK-034J5KR5ueilYgXdyyF4ZU`（plan `PLAN-034J5K603qXDNmT82wI9v7` Phase C）として、`ruviform-video-core` に plane/stride/pixel/色・PTS の動画データモデル API を実装し、`ruviform-media-core` への path 依存を配線した。`#[non_exhaustive] PixelFormat`・色/HDR メタデータ・stride 独立 `Plane`/`PlaneMut`/`SharedPlane`・`Arc` backing `VideoFrame`（`MediaTime` PTS）を含む（PR #39・merge `d041ba9`）。

## Context

- 実施タスク: [../../tasks/implement-video-core-data-model.md](../../tasks/implement-video-core-data-model.md)
- 上流: [plan](../../plans/implement-issue-18-data-model.md)、[spec](../../specs/implement-media-data-model-api.md)、[設計ベースライン](../../designs/ruviform-architecture-design-baseline.md)「動画データモデル」節、[RT ADR](../../adr/realtime-contract-and-verification-policy.md)
- 正本要件: `tmp/issue-18/api-brief.md`「Crate 3 — ruviform-video-core」+ Global constraints + dependency wiring
- 前提: media-core 実装済み（task `TASK-034J5KJgwtjFonTMQQIvpX` の `MediaTime`/`MediaDuration`/`Error`/`Result` を使用）。外部依存ゼロ・`unsafe` 不使用

## Implementation

### Changes

- `crates/ruviform-video-core/Cargo.toml` — `ruviform-media-core = { path = "../ruviform-media-core" }` 追加（`Cargo.lock` 更新）
- `src/pixel.rs` — `#[non_exhaustive] PixelFormat { Yuv420p, Yuv422p, Yuv444p, Rgba8, RgbaF16 }`（`plane_count`/`is_planar`/`bytes_per_sample`/`chroma_subsampling`、全 `const fn`）
- `src/color.rs` — `ColorPrimaries`/`TransferCharacteristics`/`MatrixCoefficients`/`ColorRange`（各 `#[default] Unspecified`）・`Chromaticity`（CTA-861 1/50000 単位）・`MasteringDisplayInfo`（1/10000 cd/m²）・`ContentLightLevel`・`HdrMetadata`・`ColorInfo`（`Default` = all-unspecified + `hdr: None`）
- `src/plane.rs` — `Plane<'a>`（借用・`Copy`）・`PlaneMut<'a>`・`SharedPlane { data: Arc<[u8]>, stride }`・`pub(crate) required_len` 共有ヘルパ
- `src/frame.rs` — `VideoFormat`・`VideoFrame`（private fields、`new_owned` ゼロ初期化・`from_shared_planes` 検証済み zero-copy 供給、`pts`/`duration`/`color` アクセサ・`plane`/`plane_mut`、derive `Clone` は `Arc` 共有、手実装 `Debug` は plane 数のみ表示）
- `src/lib.rs` — crate doc（責務・許可依存層・RT 契約・契約ソース）+ `pub use` 再エクスポート + `version()` 維持 + `rt_` テスト 3 本

### Decisions During Implementation

- **plane 寸法導出を `frame::plane_dimensions` に一元化**（公開 `PixelFormat::chroma_subsampling()` 駆動。plane 0 = フルグリッド、以降は `ceil(w/sx) × ceil(h/sy)`。subsample 表の二重管理を回避）
- **`Yuv444p.chroma_subsampling()` は `Some((1,1))`**（brief は 420=`(2,2)`・packed=`None` のみ確定。非 subsample を `(1,1)` 係数とする統一グリッド解釈。doc 済み・open question として記録）
- **`bytes_per_sample` は範囲外 plane に `0` を返す**（`-> usize` 署名を panic なしでトータル化。doc 済み）
- **ビューは使用可能領域へ trim**: `data()` は `stride*(h-1) + row_bytes`（行間パディング含む・backing 末尾は除外）。`Plane::new`/`PlaneMut::new` は不足 backing と `stride < row_bytes` を `None` で拒否（C4 修正で所有レイヤーへ不変条件を移動）
- **`required_len` に `isize::MAX` 上限を追加**（C2 修正。`vec!` 容量 panic を `Err(InvalidValue("dimensions"))` へ。`from_shared_planes`・両ビュー構築にも同一境界が一貫適用）
- **`plane_mut` は `Arc::get_mut` ゲート**（共有 backing 生存中は `None`。clone 前後でテスト済み）
- **`VideoFrame` に `PartialEq`/`Eq`/`Hash` を実装しない**（`Arc` 内容を持つ identity 的セマンティクス）
- **`MediaTime`/`MediaDuration`/`Error`/`Result` は media-core から直接使用し再エクスポートしない**（brief 不要求）

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64・Git Bash。task の `## Verification` 全項目を実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| 整形 | `mise run fmt` / `fmt-check` | 差分なし |
| 型チェック | `mise exec -- cargo check -p ruviform-video-core --all-targets` | clean |
| lint | `mise exec -- cargo clippy -p ruviform-video-core --all-targets -- -D warnings` | clean（`doc_markdown`・`field_reassign_with_default` 修正含む） |
| テスト | `mise exec -- cargo test -p ruviform-video-core` | 初回 20 単体 + 6 doctest = 26 合格。修正 wave 後の最終値は 23 単体 + 6 doctest |
| RT レーン | `mise exec -- cargo test -p ruviform-video-core rt_` | `rt_` 3 テスト（`rt_plane_access_and_row_slicing`・`rt_plane_mut_row_write`・`rt_timestamp_access`）のみ選択・合格 |

レビューサイクル:

- Reviewer A: A1（`PlaneMut` 全ゲッタのカバレッジ欠落・P2 一部）・A2（`view_rejects_short_backing` が `rt_` フィルタと衝突・P3）・A3（`version()` RT ラベル・P3）。Reviewer B: C2（`new_owned` の `isize::MAX` panic・P2）・C3（A3 と同件）・C4（`stride >= row_bytes` 不変条件が所有レイヤーにない・P3 latent）・C5（exhaustive 列挙型・P3）
- Validator: 全件 CONFIRMED。Judge: A1・A2・A3/C3 blocking、C2・C4・C5 非 blocking。C5 は文書化済み判断（`publish = false` 内部クレート）として変更不要 ruling
- 修正: `mutable_view_exposes_plane_geometry`（A1）・`view_rejects_undersized_backing` 改名（A2）・`version()` RT ラベル（A3/C3）・`required_len` の `isize::MAX` 境界 + `owned_frame_rejects_oversized_planes`（C2）・`Plane::new`/`PlaneMut::new` の stride 拒否 + `view_rejects_overlapping_rows`（C4）
- ワークスペース最終検証（タスク 4 で実施）: `mise run check` / `lint` / `test` / `test-rt` 全 PASS

## Risks

- `Yuv444p` の `chroma_subsampling() == Some((1,1))` は統一グリッド解釈。`None` が意図なら `plane_dimensions` の特殊ケースが必要（plan Open Questions に記録）
- `PlaneMut::data_mut`/`row_mut` は view の borrow に束縛される再借用（`'a` ではない）。通常の `plane_mut` 使用では自然な形
- exhaustive 列挙型（`Error`・色 enum 等）は `publish = false` の内部クレート方針で維持。クレート公開時は `#[non_exhaustive]` ポリシー再評価が必要（C5 ruling）

## Follow-ups

- デコーダ zero-copy バッキングの具体方式は media-io 実装時の要 PoC 項目（`from_shared_planes` が型契約として許容済み）
- 色変換の対応範囲は `ruviform-video-proc` の要 PoC 項目（本クレートはメタデータ型のみ）
- `mise run test-rt` の substring フィルタ精度（`convert_exact` doctest の誤検出余地）は別検討事項
