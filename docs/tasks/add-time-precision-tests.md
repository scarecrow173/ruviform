---
id: "TASK-034JNwUoAfb2TnY7nw07XH"
type: "task"
status: "todo"
title: "時間変換精度テストの追加（音声×映像行列・往復/連鎖誤差上界）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/58"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-58-time-type-extension.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-58-time-type-extension.md"
    - "implement-framerate-type.md"
  blocks:
    - "sync-docs-and-verify-issue-58.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-58-time-type-extension.md"
    - "../designs/time-model-extension.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/time-type-extension-contracts.md"
  references: []
  defers: []
  deferred-by: []
---

# 時間変換精度テストの追加（音声×映像行列・往復/連鎖誤差上界）

## Work

plan `PLAN-034JNstCXHhzzwOvo9iiCv` の Phase T3。丸め誤差累積ポリシー（呼び出しごと最大 1 回の丸め・安定アンカー・base-hopping 禁止）の検証可能な誤差上界を、`time.rs` の `mod tests` 内に deterministic 列挙テストとして追加する。誤差計測は `i128`/`u128` 有理数比較で行い `f64` を使わない。

- [ ] 精度行列テスト: 音声 base（`TimeBase::new(1, 44_100)`・`(1, 48_000)`・`(1, 90_000)`）× 映像 frame duration（`FrameRate` の `frame_duration()` で `24`・`25`・`30000/1001`・`60000/1001`）の全組み合わせ × 値集合（`0`・`±1`・大きな正値・負値）で `MediaTime::convert(_, Rounding::NearestEven)` の丸め誤差 ≤ 宛先 base の 0.5 tick を検証。負値での丸め対称性を含む
- [ ] 往復誤差上界テスト: `t → 中間 base → 元base` の 2 ホップで、累積誤差 ≤「中間 base の 0.5 tick を元 base 単位に換算した値 + 元 base の 0.5 tick」を検証。粗い中間 base 経由で誤差が大きくなり得る例（`1/48000 → 1/24 → 1/48000` で ~1000 tick）を含め上界内に収まることを確認
- [ ] N 連鎖プロパティテスト: 代表 base 集合 `{1/48000, 1/44100, 1/90000, 1001/30000, 1001/60000, 1/25, 1/24}` の巡回連鎖（3 ホップ以上・始点=終点を含む）で、累積誤差 ≤ Σ（各ホップ宛先 base の 0.5 tick）の共通単位換算を検証
- [ ] 対照実験: 同一始点・終点でアンカーからの直接変換（ポリシー遵守形）の誤差 ≤ 宛先 base の 0.5 tick を検証し、連鎖誤差がポリシー違反コストであることを定量化
- [ ] 誤差計測ヘルパをテストモジュール内に定義（tick 差の有理数絶対値計算。公開 API は増やさない）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-58-time-type-extension.md](../plans/implement-issue-58-time-type-extension.md) — Phase T3 の正本
- [../designs/time-model-extension.md](../designs/time-model-extension.md) — 誤差上界の形（往復・N 連鎖・アンカー直接）の正本
- [../test-specs/time-type-extension-contracts.md](../test-specs/time-type-extension-contracts.md) — Rules/Examples の検証対象

## Dependencies

- 前提: `implement-framerate-type.md`（映像側 base を `FrameRate::frame_duration()` で生成するため）
- 後続をブロック: `sync-docs-and-verify-issue-58.md`
- T2（`timecode.rs` 移行）とは独立だが同一ファイル群に近いため、T2 後の実施を推奨
- 外部クレート依存・`unsafe` なし（proptest/quickcheck 等新依存は導入しない。deterministic 列挙）

## Affected Files

- `crates/ruviform-media-core/src/time.rs` — `mod tests` に精度行列・往復・連鎖テストと計測ヘルパを追加

## Done When

- [ ] 音声 3 base × 映像 4 rate × 値集合の全組み合わせで単回変換誤差 ≤ 0.5 tick が検証される
- [ ] 往復誤差上界・N 連鎖誤差上界・直接変換対照の 3 形がテストとして存在し合格する
- [ ] 負値・ゼロ・境界値が値集合に含まれる
- [ ] テスト判定に `f64` を使用していない（整数有理数比較）

## Verification

- [ ] `mise exec -- cargo test -p ruviform-media-core` — 新規精度テスト群を含め合格
- [ ] `mise exec -- cargo clippy -p ruviform-media-core --all-targets -- -D warnings` — 0 warnings
- [ ] `mise run fmt-check` — 差分なし
