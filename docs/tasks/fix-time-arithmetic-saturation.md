---
id: "TASK-034J6PpKd8jNFxEYS4CRdM"
type: "task"
status: "done"
title: "時間演算の早期飽和・Err 修正（i128 中間演算）"
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
        path: "crates/ruviform-media-core/src/time.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/issue-18-followup-enum-and-time-fix.md"
  implemented-by:
    - "../impl/ir/fix-time-arithmetic-saturation.md"
  depends-on:
    - "../plans/issue-18-followup-enum-and-time-fix.md"
    - "apply-non-exhaustive-to-public-enums.md"
  blocks:
    - "small-fixes-from-followup-triage.md"
  supersedes: []
  superseded-by: []
  related: []
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

# 時間演算の早期飽和・Err 修正（i128 中間演算）

## Work

plan `PLAN-034J6OTwge4rS96WZWkZgd` の Phase T2。`crates/ruviform-media-core/src/time.rs` の時間演算が「duration の自 base → start base 変換値が `i64` を超えた時点」で早期に飽和 / `Err` を返し、「最終的な和が `i64` で表現可能」でも誤値・過剰 `Err` になるバグ（Issue #41 スコープ 2）を修正する。

- [x] **回帰テストを先に追加して失敗を確認する**: `MediaRange::end()` / `MediaTime::checked_add` に「duration の変換値が `i64` を超えるが `start + delta` は表現可能」な入力を与えるテスト（例: `start = MediaTime::new(i64::MIN, TimeBase::new(1, 1))` + `duration` が base `~2^32/1` で `2^63` 相当の tick → 真の end は `0` または小さい値。現行実装は変換 `Err` を delta 方向へ飽和させて誤値、または `checked_add` が `Err` を返す）
- [x] **修正**: delta を `i128` の正確な有理数（`value * num * other_den / (den * other_num)`）として計算し、丸めは最後に 1 回だけ適用し、`i64` 範囲検査は最終値（`start + delta`）のみに行う。適用対象:
  - `MediaRange::end()` — トータル関数を維持し、飽和は「真の最終オーバーフロー（`start + delta` が `i64` 範囲外）時のみ」（現行は `duration.convert` の `Err` を `i64::MIN`/`MAX` へ飽和させてから加算しており、上記ケースで誤値になる）
  - `MediaRange::checked_end(rounding) -> Result<MediaTime>` — 新設。`Err(Error::TimeOverflow)` は最終値オーバーフローのみ
  - `MediaTime::checked_add` / `checked_sub` — 同型の早期 `Err` を除去し最終値のみで判定。`checked_sub` の否定は `i128` 上で行う（`i64::MIN` の duration を `i64` で否定するとオーバーフローする点に注意）
- [x] **テスト**: 回帰ケース + 負 duration（end が start より前）+ 全 `Rounding` モードの境界（非整数 delta で各モードが最終 1 回の丸めとして作用すること）
- [x] **ドキュメント**: 変更対象 API の `# Panics` / `# Errors` 記述と `**RT contract:**` ラベルを実態に合わせて更新する。`MediaTime::convert` / `MediaDuration::convert` の契約（表現可能なら厳密・表現不可能なら指定丸め・変換値自体が `i64` を超えるなら `Err`）は変更しない

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/issue-18-followup-enum-and-time-fix.md](../plans/issue-18-followup-enum-and-time-fix.md) — `PLAN-034J6OTwge4rS96WZWkZgd`（approved）。Phase T2 の正本
- [../specs/issue-18-followup-enum-and-time-fix.md](../specs/issue-18-followup-enum-and-time-fix.md) — `SPEC-034J6OCXHiP1izKRzbMmgz`（approved）。AC2 の正本
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — 時間モデル（正確な有理数演算・f64 を中核表現にしない）の根拠

## Dependencies

- 前提: plan・`apply-non-exhaustive-to-public-enums.md`（順序 T1→T2。T1 が `time.rs` の `Rounding` 属性付与を含む同一ファイルのため T1 完了後に実施）
- 後続をブロック: `small-fixes-from-followup-triage.md`（順序 T1→T2→T3）
- 外部クレート依存・`unsafe` なし（整数演算のみ）

## Affected Files

- `crates/ruviform-media-core/src/time.rs` — `MediaRange::end()` 修正、`MediaRange::checked_end` 新設、`MediaTime::checked_add`/`checked_sub` 修正、テスト・doc 更新（`Rounding` への `#[non_exhaustive]` 付与は T1 済み）

## Done When

- [x] 「duration の中間変換が `i64` 超過・最終和は表現可能」な回帰テストが追加され、修正前は失敗・修正後は合格する
- [x] `MediaRange::checked_end(rounding) -> Result<MediaTime>` が存在し、`Err` は最終値オーバーフローのみ
- [x] `checked_add`/`checked_sub` が同型の早期 `Err` を返さず、負 duration・全 `Rounding` モードの境界テストが合格する
- [x] `# Errors`/`# Panics` ドキュメントと RT ラベルが実態と一致する

## Verification

- [x] `mise exec -- cargo test -p ruviform-media-core` — 回帰テスト・負 duration・丸めモード境界を含め合格
- [x] `mise exec -- cargo clippy -p ruviform-media-core --all-targets -- -D warnings` — 0 warnings
- [x] `mise run fmt-check` — 差分なし
