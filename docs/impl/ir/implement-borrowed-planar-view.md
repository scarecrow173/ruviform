---
id: "IMPL-fWVxUoqgJUdazDXNGbhtza"
type: "impl"
status: "completed"
title: "Implement borrowed planar buffer views"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-core/src/planar.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/processor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "docs/plans/implement-plugin-foundation.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/implement-borrowed-planar-view.md"
  implemented-by: []
  depends-on:
    - "create-plugin-core-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-plugin-foundation.md"
    - "../../specs/implement-plugin-foundation.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/plugin-foundation.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement borrowed planar buffer views

## Summary

task `TASK-034JgsGmcPdPRoeRTvicjW`（plan `PLAN-034JcvpHIlt5JmqvgOquvI` T2）として、独立 per-channel planar バッファのゼロコピー借用 view を実装した。配置は lean どおり `ruviform-audio-core/src/planar.rs` — `PlanarBufferView<'a, S: Sample = f32>` / `PlanarBufferViewMut<'a, S: Sample = f32>`。`&'a [&'a [S]]` / `&'a mut [&'a mut [S]]` をラップし、均一フレーム長を構築時検証、ライフタイムでコールバックスコープ脱退をコンパイル阻止（`compile_fail` doctest ×2 で固定）。plugin-core 側に `BufferView`/`BufferViewMut` trait impl を配置し `process` 契約に接続。unsafe 不使用。

## Context

- 実施タスク: [../../tasks/implement-borrowed-planar-view.md](../../tasks/implement-borrowed-planar-view.md)
- 上流: [plan](../../plans/implement-plugin-foundation.md)（配置確定結果を File Map に追記済み）、[spec](../../specs/implement-plugin-foundation.md)、[RT ADR](../../adr/realtime-contract-and-verification-policy.md)
- 前提: T1 で確定した `process` の view 契約（`BufferView`/`BufferViewMut` trait）に適合する具象型

## Implementation

### Changes

| File | 内容 |
| --- | --- |
| `planar.rs` | `PlanarBufferView`（`new` で均一フレーム長検証・`empty`・`channels`/`frames`/`channel`/`as_slice`・`compile_fail` doctest）、`PlanarBufferViewMut`（同系 + `channel_mut`・重複出力領域の `debug_assert!`）、module doc に lifetime/allocation/channel/in-place 契約を集約 |
| audio-core `lib.rs` | `mod planar` + crate doc 追記 + 再エクスポート |
| plugin-core `processor.rs` | `PlanarBufferView`/`PlanarBufferViewMut` への `BufferView`/`BufferViewMut` impl、f32 版のみ |
| plan | File Map に配置確定結果・根拠・in-place 評価を追記 |

### Decisions During Implementation

- **配置確定: lean（audio-core）採用**。根拠: 独立 per-channel 借用形状は plugin ABI 境界固有ではなく device/host 境界にも現れる汎用形状であり、`AudioBufferView`/`Plane` 先例と整合。`S: Sample` ジェネリックで f64 予約を閉塞しない
- **trait impl は plugin-core 側**: `BufferView`/`BufferViewMut` は plugin-core ローカル trait のため audio-core 型への impl を plugin-core に置ける（orphan rule 適合）。audio-core → plugin-core の逆依存は発生しない
- **f32 版のみ impl**: `BufferView`/`BufferViewMut` は `&[f32]`/`&mut [f32]` 固定の契約（canonical f32 経路）。`S=f64` 版は型形状として存在するが process 契約へは接続しない — f64 経路は spec の reserved 方針どおり未実装
- **`debug_assert!` の重複出力領域チェック**: `&mut` 重複は構築時点で UB になるため、safe コンストラクタでは見えない raw pointer の一致を debug ビルドで検査（non-empty スライスのみ — zero-len のダングリングポインタ誤検出を回避）
- **in-place 評価結果（T5 ADR 入力）**: bounce-scratch は全エイリアスパターンに効く uniform 安全デフォルト。binding が in-place ペアを分類する場合（`clack` `ChannelPair::InPlace`）のみ専用分岐が有効 — T5 で決定記録

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64・Git Bash。task の `## Verification` 全項目を実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| テスト | `mise exec -- cargo test -p ruviform-audio-core` | 39 単体 + 22 doctest（`compile_fail` ×2 含む）= 61 合格 |
| 連結確認 | `mise exec -- cargo test -p ruviform-plugin-core` | 29 単体 + 19 doctest 合格（trait impl 導入後） |
| lint | `mise exec -- cargo clippy -p ruviform-audio-core -p ruviform-plugin-core --all-targets -- -D warnings` | clean・unsafe なし |
| `compile_fail` | doctest 結果 | 借用のスコープ脱退がコンパイル阻止されることを 2 例で確認 |
| 変換ゼロ | ドキュメント契約 | `&'a [&'a [f32]]` / `&'a mut [&'a mut [f32]]` 借用ラップのみ — f32 経路で変換 0 回を型形状で担保 |

## Risks

- in-place 専用分岐は未実装 — T5 の方式 ADR で採否を決定。bounce-scratch が安全デフォルトとして常に利用可能
- `rt_` テストはアロケーションを観測しない（Issue #22 の検証レーン待ち）— no-alloc は型形状 + ドキュメント契約 + レビューで担保

## Follow-ups

- T3/T4: アダプタ側で activate 時にチャンネルリスト・bounce スクラッチを事前確保し、本 view の `new` に供給する
- T5: in-place 方式（専用分岐 vs bounce-scratch）を PoC 実測と併せて方式 ADR に決定記録
