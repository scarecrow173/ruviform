---
id: "TASK-034JPwYfiOnSaZJgGJQQyY"
type: "task"
status: "done"
title: "dispatch_probe ベンチ追加"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-56-dispatch-analysis.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-56-dispatch-analysis.md"
  blocks:
    - "run-dispatch-remeasure.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-56-dispatch-analysis.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# dispatch_probe ベンチ追加

## Work

1. `crates/ruviform-math/benches/simd_poc.rs` に `dispatch_probe` ベンチグループを追加
2. case: `level_new` = `black_box(Level::new())`・`dispatch_noop` = `dispatch!(Level::new(), simd => black_box(simd))`・`token_avx2` = `black_box(Level::new().as_avx2())`
3. 小サイズ帯補足: `add` カーネルを N=16/32 で `fearless_dispatched`・`fearless_avx2`・`scalar` の3列で計測（固定コスト支配域の確認用）
4. 公開 API のみ・unsafe 不使用・criterion_group に登録

## Classification

- `normal-plan-task`

## Upstream Documents

- <!-- plan, spec, ADR, or task path -->

## Dependencies

- 先行タスクなし（本チェーン先頭）

## Affected Files

- `crates/ruviform-math/benches/simd_poc.rs`

## Done When

- [x] `dispatch_probe` グループが `cargo bench -p ruviform-math -- dispatch_probe` で実行される
- [x] 4 case（level_new/dispatch_noop/token_avx2 + 小サイズ add）が計測される
- [x] 既存ベンチ matrix・等価性テストに変更なし

## Verification

- [x] `cargo bench -p ruviform-math -- dispatch_probe` が完走する（bench-smoke 対象外でも単体実行確認）

## Completion Notes

- `dispatch_probe` グループを `simd_poc.rs` に追加: `level_new`・`token_avx2`・`dispatch_noop` の部品プローブ + `add_{scalar,fearless_avx2,fearless_dispatched}` の N=16/32 小サイズ帯（9 case）。
- `dispatch!` は各 arm が異なる opaque `impl Simd` を返すため、`black_box(&simd)` で `()` に統一。トークン生成経路は保持。
- 私の `use fearless_simd::{Level, dispatch}` import 追加で既存の `fearless_simd::Level::new()` 完全修飾が `unused_qualifications` に — `Level::new()` に短縮（2箇所）。
- `cargo bench -p ruviform-math -- dispatch_probe --test` 完走確認。lint/check/fmt-check パス。
