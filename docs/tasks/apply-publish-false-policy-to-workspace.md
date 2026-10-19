---
id: "TASK-034IRYXA0oq14nlgmC4Bch"
type: "task"
status: "todo"
title: "workspace への publish = false 方針の適用"
created: "2026-10-20"
updated: "2026-10-20"
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
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
  implemented-by: []
  depends-on:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
  blocks:
    - "update-agents-interim-principles-to-fixed-contracts.md"
    - "verify-issue-1-repo-reflection.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# workspace への publish = false 方針の適用

## Work

plan `PLAN-034IR3qOr15LGNY9mEindJ` のスライス T-PUB（Phase P-1〜P-3）。`[workspace.package]` に `publish = false` を追加し、`crates/ruviform-core` で `publish.workspace = true` 継承を宣言する。`edition`/`rust-version`/`license`/`lints`/`profiles`/`members` の値行は変更せず、`#1 で再評価` を示すコメントのみ評価結論への送り先に更新する。

- [ ] P-1. ルート `Cargo.toml` の `[workspace.package]` に `publish = false` を追加し、未決定クレートは公開しない licensing ADR 原則であり、メンバーは `publish.workspace = true` で継承する旨のコメントを付ける。`edition`/`rust-version` の `# 仮置き。#1 で edition / MSRV を再評価`・`# 仮置き MSRV。` コメントは、評価結論「専用の確認タスク（design が挙げる edition/MSRV/publish 反映 Issue）へ送付・現段階で変更しない」を指す文言に更新する。`license = "MIT"` とその仮置きコメントは無変更（design: 仮置きのまま維持・未決定の明記を確認）
- [ ] P-2. `crates/ruviform-core/Cargo.toml` に `publish.workspace = true` を追加する（`license.workspace = true` 等の既存継承行と同じ方式。値行のみ・他フィールド無変更）。あわせて L2 の `# 仮置き名。#1 で最終クレート分割を再評価` コメントを、評価結論「Phase 1 で `ruviform-*` クレート群への分割置換として削除予定（design A.8「`crates/ruviform-core` の処遇」）」を指す文言に更新する
- [ ] P-3. `mise exec -- cargo metadata --format-version 1 --no-deps` を実行し、`ruviform-core` の `publish` が `[]`（publish 不可）であることを確認する（V-1）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/apply-issue-1-architecture-baseline-to-repo.md](../plans/apply-issue-1-architecture-baseline-to-repo.md) — `PLAN-034IR3qOr15LGNY9mEindJ`（approved）。Phase T-PUB / V-1 の正本
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。「feature flag・公開方針」（L203–L210）の publish 制御・A.8 差分タスク・「`crates/ruviform-core` の処遇」の引用元
- [../adr/licensing-and-distribution-policy.md](../adr/licensing-and-distribution-policy.md) — `ADR-034IO02s3n3KHsxnHckpRn`（accepted）。未決定クレートは `publish = false` 原則の根拠

## Dependencies

- 共有前提: plan S-1（変更前ベースライン記録）はいずれのタスクにも属さない pre-dispatch ステップとして、本タスク着手前に実施済みであること
- 後続をブロック: `update-agents-interim-principles-to-fixed-contracts.md`（AGENTS のワークスペース構成節が `publish = false` の存在を前提とする）、`verify-issue-1-repo-reflection.md`（V-1 の機械確認）
- 注意: `[workspace.package]` の値は `*.workspace = true` 宣言でのみ継承される。`publish.workspace = true` の記述漏れでは member に効かず、`publish` が `null` のまま残るため V-1 で即検出できる

## Affected Files

- `Cargo.toml` — `[workspace.package]` に `publish = false` 追加＋edition/rust-version コメント更新（値行不変）
- `crates/ruviform-core/Cargo.toml` — `publish.workspace = true` 追加＋L2 仮置き名コメント更新（値行のみ・他フィールド無変更）

## Done When

- [ ] ルート `Cargo.toml` に `publish = false`、member `crates/ruviform-core/Cargo.toml` に `publish.workspace = true` が宣言されている
- [ ] `edition`/`rust-version`/`license`/`lints`/`profiles`/`members` の値行が変更前と不変
- [ ] `cargo metadata` で `ruviform-core` の `publish` が `[]` である

## Verification

- [ ] `mise exec -- cargo metadata --format-version 1 --no-deps` の出力で `ruviform-core` の `publish` フィールドが `[]` を返す（V-1）
- [ ] `grep -n "publish = false" Cargo.toml` と `grep -n "publish.workspace = true" crates/ruviform-core/Cargo.toml` が各々一致を返す
- [ ] `git diff Cargo.toml crates/ruviform-core/Cargo.toml` で `edition`/`rust-version`/`license`/`lints`/`profiles`/`members` の値行に差分がない
