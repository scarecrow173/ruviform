---
id: "IMPL-034ISfioxP51rGFoHh8MX3"
type: "impl"
status: "completed"
title: "issue-1 architecture baseline repo reflection"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  changes:
    added: []
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/ruviform-core/Cargo.toml"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/designs/overview.md"
      - type: "file"
        path: "docs/designs/ruviform-architecture-design-baseline.md"
      - type: "file"
        path: "docs/specs/architecture-design-baseline-deliverables.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../plans/apply-issue-1-architecture-baseline-to-repo.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/apply-issue-1-architecture-baseline-to-repo.md"
    - "../../designs/ruviform-architecture-design-baseline.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# issue-1 architecture baseline repo reflection

## Summary

Issue #1 の approved 設計ベースラインをリポジトリへ反映する計画（PLAN-034IR3qOr15LGNY9mEindJ）の5タスクを実施した。ワークスペースに `publish = false` とメンバー側 `publish.workspace = true` 継承を適用し、README／AGENTS の暫定表現を確定契約に置換、designs/overview の stale 注記を整合させ、双方向 relation と索引を整備した。

## Context

`docs/plans/apply-issue-1-architecture-baseline-to-repo.md`（approved）配下の5タスク: T-PUB・T-README・T-AGENTS・T-IDX・T-VERIFY。上流は approved 設計ベースライン DESIGN-034IPynWe2V30FLubzQenb。

## Implementation

### Changes

- `Cargo.toml`: `[workspace.package]` に `publish = false`（継承条件コメント付き）。edition/rust-version/license の値は不変（コメントのみ再評価送付先を明記）
- `crates/ruviform-core/Cargo.toml`: `publish.workspace = true` 追加、stale コメントを Phase 1 分割置換・削除予定に更新
- `README.md`: 「検討中（要 PoC）」・「決定済み・未確定」区分へ再構成、design 導線追加、RT 契約注意書き
- `AGENTS.md`: 暫定原則をアーキテクチャ確定契約（RT・SIMD 仮説・依存 DAG・members 段階追加）に更新
- `docs/designs/overview.md`: 3形式の stale 注記を approved 反映、#3 境界に #1 A.8 実施を追記
- spec/design の `derived-by` に plan 逆リンク、5タスク `done`、索引再生成

### Decisions During Implementation

- `publish.workspace = true` を唯一メンバーの ruviform-core にのみ適用（ライセンス未決定期の誤公開防止方針に整合）
- `test-spec-skip` に従い test-spec 非作成 — 外部観測変化は `cargo metadata` の publish 値に限定

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `cargo metadata --format-version 1 --no-deps`: ruviform-core の publish `null` → `[]`
- `prek run --all-files`: 全フック Passed
- `mise run fmt-check` / `check` / `lint` / `test`: すべてパス（doctest 1件含む）
- `audit_docs.js --type all`: 40 ファイル findings 0、`git diff --check` クリーン
- スコープ外確認: edition/rust-version/license 値・mise.toml・ci.yml・Cargo.lock 無変更、LICENSE 非作成、新規クレートなし

## Risks

- `publish = false` は将来の公開決定時にクレート単位で解除する運用が前提

## Follow-ups

計画に記録した19件の後続 Issue 提案（edition/MSRV 再評価、codec/container 決定、各種 PoC 等）は GitHub Issue 化をユーザー判断に委ねる。
