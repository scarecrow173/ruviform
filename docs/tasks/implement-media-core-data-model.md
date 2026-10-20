---
id: "TASK-034J5KJgwtjFonTMQQIvpX"
type: "task"
status: "done"
title: "ruviform-media-core のデータモデル実装"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/18"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-media-core/src/error.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/time.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/timecode.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/transport.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-18-data-model.md"
  implemented-by:
    - "../impl/ir/implement-media-core-data-model.md"
  depends-on:
    - "../plans/implement-issue-18-data-model.md"
  blocks:
    - "implement-audio-core-data-model.md"
    - "implement-video-core-data-model.md"
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

# ruviform-media-core のデータモデル実装

## Work

plan `PLAN-034J5K603qXDNmT82wI9v7` の Phase A。`tmp/issue-18/api-brief.md`「Crate 1 — ruviform-media-core」+ Global constraints を正本として、Primitives 層の値型のみで構成されるデータモデル API を実装する（実績としての遡及記録）。

- [x] `error` モジュール: `Error { InvalidTimeBase, TimeOverflow, InvalidValue(&'static str), UnsupportedDropFrameRate }` + `Result<T>`、`Display` + `core::error::Error` 実装
- [x] `time` モジュール: `TimeBase`（gcd 正規化・`COMMON_90K`・`sample_rate`）、`MediaTime`（`ZERO`・`convert`/`convert_exact`・`checked_add`/`checked_sub`・`to_seconds_f64` 表示用）、`MediaDuration`（同系アクセサ + `to_seconds_f64`）、`MediaRange`、`Rounding` 5 モード（既定 `NearestEven`）
- [x] `stream` モジュール: `StreamId` / `StreamKind` / `StreamDescriptor`（コンテナレベル・codec 非依存）
- [x] `timecode` モジュール: `Timecode` 表示変換（SMPTE drop-frame: 30000/1001・60000/1001、負時刻は `Err(InvalidValue)`、整数演算のみ）
- [x] `transport` モジュール: `TransportStatus` / `TransportSnapshot`（opaque `reserved: [u64; 2]`、常に `[0; 2]`）
- [x] `MediaTime` / `MediaDuration` の cross-base `Eq`/`Ord`/`Hash`（`u128` マグニチュード比較・最低項正規化ハッシュ・全 base で 0 等値）
- [x] 全公開 API に `**RT contract:**` ラベル、`lib.rs` crate doc を実責務へ更新 + `pub use` フラット再エクスポート + `version()` 維持
- [x] `rt_` テスト 4 本（`rt_cross_base_ordering`・`rt_convert_exact_and_rounding`・`rt_checked_arithmetic`・`rt_drop_frame_roundtrip`）と doctest/単体テスト整備。依存追加なし（media-core は依存ゼロを維持）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-18-data-model.md](../plans/implement-issue-18-data-model.md) — `PLAN-034J5K603qXDNmT82wI9v7`（completed）。Phase A の正本
- [../specs/implement-media-data-model-api.md](../specs/implement-media-data-model-api.md) — `SPEC-034J5FYaYfeL5zkzb4sIV4`（implemented）。時間モデル要件の正本
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。「データモデル／時間モデル」節の根拠

## Dependencies

- 前提: 先行 plan `apply-issue-1-architecture-baseline-to-repo.md` 適用済み（Phase 1 スケルトン・workspace lints・`publish = false` 継承）
- 後続をブロック: `implement-audio-core-data-model.md`・`implement-video-core-data-model.md`（path 依存先の公開 API を提供）、`sync-docs-and-verify-issue-18.md`（総合検証対象を提供）
- 外部クレート依存ゼロ・`unsafe` 不使用・f64 を中核表現に使わない（Global constraints）

## Affected Files

- `crates/ruviform-media-core/src/error.rs` — 新規
- `crates/ruviform-media-core/src/stream.rs` — 新規
- `crates/ruviform-media-core/src/time.rs` — 新規
- `crates/ruviform-media-core/src/timecode.rs` — 新規
- `crates/ruviform-media-core/src/transport.rs` — 新規
- `crates/ruviform-media-core/src/lib.rs` — crate doc・再エクスポート・`rt_` テスト更新
- `crates/ruviform-media-core/Cargo.toml` — 変更不要（依存ゼロ維持）

## Done When

- [x] brief の全型・署名が公開 API として存在し、crate root から再エクスポートされる
- [x] cross-base `Eq`/`Ord`/`Hash` が一貫し、5 モード丸めと `Err`/`Option` の失敗表出が動作する
- [x] 全公開 API に `**RT contract:**` ラベルが付き、`rt_` テストが `mise run test-rt` で選択される
- [x] 実施記録 [../impl/ir/implement-media-core-data-model.md](../impl/ir/implement-media-core-data-model.md) に検証結果が残っている

## Verification

- [x] `mise exec -- cargo test -p ruviform-media-core` — 24 単体テスト + 23 doctest 合格（修正 wave 後の最終値。初回実施時は 23 + 22）
- [x] `mise exec -- cargo clippy -p ruviform-media-core --all-targets -- -D warnings` — 0 warnings
- [x] `mise exec -- cargo test -p ruviform-media-core rt_` — `rt_` プレフィックス 4 テストが選択・合格
- [x] `mise run fmt-check` — 差分なし
