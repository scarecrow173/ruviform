---
id: "IMPL-034J5L3UVnQerTiQRlC7w3"
type: "impl"
status: "completed"
title: "Implement media-core data model"
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
    - "../../tasks/implement-media-core-data-model.md"
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

# Implement media-core data model

## Summary

task `TASK-034J5KJgwtjFonTMQQIvpX`（plan `PLAN-034J5K603qXDNmT82wI9v7` Phase A）として、`ruviform-media-core` に有理数時間モデル・ストリーム記述・タイムコード・トランスポート・共通エラーの公開 API を実装した。全型は `Copy` 可能な値型で、変換は `i128`/`u128` 中間表現の純粋整数関数、オーバーフローは `Err(Error::TimeOverflow)` / `None` で表出。全公開 API に `**RT contract:**` ラベルを付与し、`Cargo.toml` は変更せず依存ゼロを維持した（PR #39・merge `d041ba9`）。

## Context

- 実施タスク: [../../tasks/implement-media-core-data-model.md](../../tasks/implement-media-core-data-model.md)
- 上流: [plan](../../plans/implement-issue-18-data-model.md)、[spec](../../specs/implement-media-data-model-api.md)、[設計ベースライン](../../designs/ruviform-architecture-design-baseline.md)「データモデル／時間モデル」節、[RT ADR](../../adr/realtime-contract-and-verification-policy.md)
- 正本要件: `tmp/issue-18/api-brief.md`「Crate 1 — ruviform-media-core」+ Global constraints（brief と baseline が競合する場合は brief 優先・報告義務あり）
- 前提制約: edition 2024・MSRV 1.85・stable・外部依存ゼロ・`unsafe` 不使用・f64 を中核表現にしない・全公開 API に doctest/単体テストと `**RT contract:**` ラベル

## Implementation

### Changes

| Module | Public items |
| --- | --- |
| `error` | `Error { InvalidTimeBase, TimeOverflow, InvalidValue(&'static str), UnsupportedDropFrameRate }`（Debug/Clone/Copy/PartialEq/Eq/Hash + `Display` + `core::error::Error`）、`Result<T>` |
| `time` | `TimeBase`（`new` gcd 正規化・`COMMON_90K`・`sample_rate`）、`MediaTime`（`ZERO`・`convert`/`convert_exact`・`checked_add`/`checked_sub`・`to_seconds_f64`）、`MediaDuration`（`ZERO`・`convert`/`convert_exact`・`to_seconds_f64`）、`MediaRange`、`Rounding`（5 モード・`#[default] NearestEven`） |
| `stream` | `StreamId(pub u32)`、`StreamKind { Audio, Video, Data }`、`StreamDescriptor`（pub fields） |
| `timecode` | `Timecode` + `from_media_time` / `from_media_time_drop`（SMPTE drop-frame、整数演算のみ） |
| `transport` | `TransportStatus { Stopped, Playing, Paused }`、`TransportSnapshot`（`reserved() -> [u64; 2]` 常に `[0; 2]`） |
| `lib.rs` | crate doc を実責務へ更新・`pub use` フラット再エクスポート・`version()` 維持・`rt_` テスト 4 本 |

- `MediaTime` / `MediaDuration` は手実装の cross-base `Eq`/`Ord`/`Hash`（`u128` マグニチュード比較は `< 2^128` に束ねられオーバーフローしない・最低項正規化ハッシュで `Eq` と一貫・0 は全 base で等値）
- レビュー修正 wave で `MediaDuration::to_seconds_f64`（A4 ruling）・`MediaDuration::base()` のカバレッジ・`version()` への RT ラベル（A3）を追加

### Decisions During Implementation

- **`MediaTime::ZERO` / `MediaDuration::ZERO`**: brief 任意項を採用（`0` at `1/1`。cross-base 等値により全 base の 0 と等しい）
- **`Add`/`Sub` → `checked_add`/`checked_sub`**: trait 実装は変換・i64 オーバーフローで panic/saturate が必要になり「no silent f64, no panic」契約に反するため、明示 `Result` 形を選択
- **`MediaRange::end() -> MediaTime`**: brief 署名を維持し i128 計算後に `i64::MIN`/`MAX` へ飽和するトータル関数として文書化
- **負タイムコード**: 0 へ floor せず `Err(InvalidValue("negative time"))`（pre-roll の暗黙誤ラベル防止）
- **drop-frame 判定は正規化後 `TimeBase`**: `60000/2002` を `30000/1001` として受理
- **`MediaDuration` は signed を維持**し `MediaTime` と同系の変換・cross-base 比較を実装
- **`TimeBase` に `Ord` を実装しない**（有理数のフィールド比較は意味論的に誤り）
- **lint 方針**: `module_name_repetitions` / `exhaustive_enums` / `exhaustive_structs` を crate-level allow（root 再エクスポート名と `publish = false` 内部クレートの判断。C5 ruling で維持確定）。`to_seconds_f64` のみ `#[expect]` 付きの display-only f64 例外
- **rulings（コーディネータ）**: `MediaDuration` 命名（std 衝突回避・`Media*` 系統）、timecode は media-core の表示変換責務として範囲内確定

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
| 型チェック | `mise exec -- cargo check -p ruviform-media-core --all-targets` | clean・0 warnings |
| lint | `mise exec -- cargo clippy -p ruviform-media-core --all-targets -- -D warnings` | clean・0 warnings |
| テスト | `mise exec -- cargo test -p ruviform-media-core` | 初回 23 単体 + 22 doctest = 45 合格。修正 wave 後の最終値は 24 単体 + 23 doctest |
| RT レーン | `mise exec -- cargo test -p ruviform-media-core rt_` | `rt_` 4 テスト（`rt_cross_base_ordering`・`rt_convert_exact_and_rounding`・`rt_checked_arithmetic`・`rt_drop_frame_roundtrip`）が選択・合格 |

レビューサイクル:

- Reviewer A（spec 適合）: media-core 関連では A3（`version()` の RT ラベル欠落・P3）・A4（`MediaDuration::to_seconds_f64` 仕様曖昧・P3）。Validator は A3 を CONFIRMED、A4 を UNVERIFIED（仕様判断として coordinator へ）。Judge は A3 を blocking 判定
- 修正: `version()` に `**RT contract:**` 追加、`MediaDuration::to_seconds_f64` 追加（A4 ruling。`MediaTime` 側と同一の display-only 文書化・`#[expect]` 処理）、`MediaDuration::base()` のテストカバレッジ追加（A1 の一部）
- ワークスペース最終検証（タスク 4 で実施）: `mise run check` / `lint` / `test` / `test-rt` 全 PASS

## Risks

- `rt_` テスト選択は substring マッチのため、将来 `rt_` を内包する非 RT テスト名（`short_`・`support_` 等）は RT レーンに混入する。命名規約注意を implementer レポートで後続クレートへ共有済み（video-core で A2 として実発生・改名で解消）
- `to_seconds_f64` は `MediaTime`/`MediaDuration` のみの表示用変換。他の f64 変換が増えると中核表現ルールが曖昧になるため、追加は個別の文書化判断とする

## Follow-ups

- `rt_` フィルタの substring 精度改善（厳密プレフィックスマッチ化）は test-rt レーン運用の別検討事項
- `MediaTime` / `MediaDuration` の算術は `checked_*` API のみ。`Add`/`Sub` trait 実装が必要になった場合は panic-free 契約を維持できる設計を再検討する
