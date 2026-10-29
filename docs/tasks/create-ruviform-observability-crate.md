---
id: "TASK-034OpseKB70K4vR6W46RMi"
type: "task"
status: "done"
title: "T1. ruviform-observability クレート新設（counter pair・workspace 登録）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/101"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-observability/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-observability/README.md"
      - type: "file"
        path: "crates/ruviform-observability/src/lib.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/impl/ir/rt-observability-poc.md"
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

# T1. ruviform-observability クレート新設（counter pair・workspace 登録）

## Work

`crates/ruviform-observability` を新設 — #101 foundation minimal-api の最初の実消費者面として L1 diagnostics が使う counter pair 型 **`Counter`（writer・`Relaxed` atomic 更新のみの RT-suitable 参照・clone で reader と共有）/`CounterReader`（非 RT snapshot・load のみ）** を提供する（#104 PoC `Counter`/`CounterReader` 参照形 — foundation 設計 sketch の同名型に対応。T4 の counter 計装はこの pair 型を必須使用する）。依存なしの最小クレート — `Cargo.toml`（`publish.workspace = true` 継承）・`src/lib.rs`・`README.md`（クレート責務・RT 制約・初消費者 L1 diagnostics の記述 = #101 trigger 証跡）を作成し、workspace `members` + `Cargo.toml` 先頭の per-Issue rationale コメント・`crates/README.md`（層・責務・依存方向索引 + クレート数カウント）・`Cargo.lock` を更新する。

**API 契約の明記**: 複数 counter の逐次 `Relaxed` 読みは同一時点の整合 snapshot を保証しないことを公開 API doc に書く（設計 — 読み出し语义の確定事項）。`reserved: [u64; 2]` 規約（#104）は参照形として維持。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-121-l1-diagnostics.md`
- `docs/designs/issue-121-l1-diagnostics.md`
- `docs/designs/foundation-observability-minimal-api.md`

## Dependencies

- なし（先行独立タスク）

## Affected Files

- `Cargo.toml`（workspace members + rationale コメント）
- `Cargo.lock`（再生成・commit 対象）
- `crates/README.md`
- `crates/ruviform-observability/{Cargo.toml,README.md,src/lib.rs}`

## Done When

- [ ] 新クレートが workspace `check` で compile される
- [ ] counter pair 単体テスト（writer 更新→reader 観測・Relaxed・clone 共有）がグリーン
- [ ] `crates/README.md` 索引・クレート数カウントが更新済み

## Verification

- [ ] `mise run check` exits 0
- [ ] `mise exec -- cargo test -p ruviform-observability` exits 0
- [ ] `git diff --check` exits 0
