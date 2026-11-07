---
id: "PLAN-034T7ZX1VmpTfXPra5KRin"
type: "plan"
status: "in-progress"
title: "RF384/RF385: Driver-neutral GUI contract + ruviform-ui-theme DTCG interpreter (Issue #384/#385)"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
  changes:
    added:
      - type: "file"
        path: "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
      - type: "file"
        path: "docs/specs/issue-384-driver-neutral-gui-architecture.md"
      - type: "file"
        path: "docs/designs/issue-384-driver-neutral-gui-contract.md"
      - type: "file"
        path: "docs/specs/issue-385-dtcg-resolved-theme.md"
      - type: "file"
        path: "docs/designs/issue-385-dtcg-resolved-theme.md"
      - type: "file"
        path: "crates/ruviform-ui-theme"
    modified:
      - type: "file"
        path: "docs/adr/ui-architecture-webview-hybrid.md"
      - type: "file"
        path: "docs/adr/ui-tech-selection-poc.md"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
  implemented-by: []
  depends-on:
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "docs/designs/issue-384-driver-neutral-gui-contract.md"
    - "docs/designs/issue-385-dtcg-resolved-theme.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
test-spec-skip: "検証正本は crates/ruviform-ui-theme/tests/pipeline.rs の 18 contract tests + fixtures（正常/override/alias cycle/unknown-ref/type-mismatch/部分 alias/予約キー/missing profile/modeless）と workspace mise 検証。独立 test-spec doc は本 scope（契約文書+単一クレート）では作成しない"
---

# RF384/RF385: Driver-neutral GUI contract + ruviform-ui-theme DTCG interpreter (Issue #384/#385) Implementation Plan

## Goal

後継 ADR と契約設計で Driver-neutral 構造正本を確立し（旧 2 ADR は superseded・履歴保持）、`ruviform-ui-theme` クレートで deterministic な DTCG→typed `ResolvedTheme` パイプラインを実装・検証する。

## Scope

単一ストリームで実施。境界: ADR/spec/design の文書化 + `ruviform-ui-theme` crate + 旧 ADR supersession + workspace member/README 追加のみ。#386 以降の Driver 実装・`ruviform-ui` の API 移動・旧 `ui/*` 削除は他 worker/別 Issue。

## Upstream Documents

- `docs/specs/issue-384-driver-neutral-gui-architecture.md`
- `docs/specs/issue-385-dtcg-resolved-theme.md`
- `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`

## Design Inputs

- `docs/designs/issue-384-driver-neutral-gui-contract.md`（approved）
- `docs/designs/issue-385-dtcg-resolved-theme.md`（approved）

## File Map

- `crates/ruviform-ui-theme/src/lib.rs` — crate doc・再 export・`SCHEMA_VERSION`
- `crates/ruviform-ui-theme/src/errors.rs` — `TokenError`/`ResolveError` 分類
- `crates/ruviform-ui-theme/src/doc.rs` — profile metadata・token tree parse/validate（strictness）
- `crates/ruviform-ui-theme/src/resolve.rs` — precedence merge・mode 選択・alias DFS 解決
- `crates/ruviform-ui-theme/src/snapshot.rs` — `ResolvedTheme`/`ResolvedValue`/typed accessor・normalized JSON
- `crates/ruviform-ui-theme/tests/fixtures/*.json` — 12 fixture（valid/invalid/cycle/unknown-ref/type-mismatch/reserved-key/partial-alias/missing-profile/modeless）
- `crates/ruviform-ui-theme/tests/pipeline.rs` — 18 contract tests
- `docs/adr/*`・`docs/specs/*`・`docs/designs/*` — 上記 doc 群
- `Cargo.toml`（members 追加）・`crates/README.md`（行追加）

## Implementation Sequence

- [x] RF384 spec + 後継 ADR + 契約 design の作成（依存方向図・移行表・Capability Matrix・Native Dock Gate・縮退順序・責務マップ・3repo 境界）
- [x] 旧 2 ADR の status→`superseded` + `superseded-by` リンク + 履歴注記
- [x] RF385 spec + design（rvf-dtcg-profile.v1: 型・値・strictness・precedence・エラー分類）
- [x] `ruviform-ui-theme` crate 実装（serde/serde_json のみ依存）
- [x] fixtures + contract tests
- [x] 独立レビュー（Reviewer A/B → Validator/Judge）+ P0/P1 修正
- [x] `mise run check`/`lint`/`test` 実施（scoped green。workspace test は plugin-host L1 flake 1件・再実行で pass）
- [ ] plan/task doc・index 再生成
- [ ] origin/main merge・fmt-check・commit・push・日本語 PR・evidence report

## Dependencies and Risks

- GPUI 技術採用の最終承認は #262/#386/#388/#389 の実機証拠後（本 plan では行わない — ADR に逆反映欄を確保）。
- 共有 `Cargo.lock`/`docs/*/README.md` は他 worker と競合し得る — 最小差分で変更し coordinator の union merge に委ねる。
- `docs/specs/plugin-gui-poc.md`・`docs/designs/issue-28-ui-poc.md`・`docs/designs/ruviform-architecture-design-baseline.md` が superseded ADR を upstream に持つ stale-upstream finding（doc_maintenance 検出）は、履歴的導出として正しいため本 plan では再ポイントしない（PR に記録）。

## Task Breakdown

- task doc: `docs/tasks/` に実装タスクとして切出し（new_task.js）。

## Verification

- [x] `mise run check` — pass（workspace --all-targets）
- [x] `mise run lint` — pass（clippy -D warnings）
- [x] `mise exec -- cargo test -p ruviform-ui-theme` — 18 tests + 1 doctest pass
- [x] `mise run test`（workspace） — plugin-host L1 e2e `wedged_child_write_converges_and_teardown_is_bounded` が SpawnFailed{Timeout} で 1 件失敗 → 単独再実行で pass（環境 flake、他 crate・本変更と無関係）
- [x] `audit_adr.js` — 新 ADR は error 0（info 1 件: Pros and Cons of the Options → 追加済み）
- [x] `doc_maintenance.js plan` — 自 doc の finding 解消（derived-by reciprocal・orphan-navigation）
- [ ] `mise run fmt-check`（最終確認）

## Open Questions / Gaps

- GPUI mapper 消費契約の確定形は #386 に委譲（本 plan は `ResolvedTheme` の型と不変性のみ確定）。
- `ResolveError::Document` は将来の複合 helper 用に保持（現行は From<TokenError> 経由で使用可）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 worker 自身が実施済み）
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- レビュー焦点: ADR の移行表網羅性・Capability Matrix の declared/Pass 分離、profile v1 の deterministic 規約（fail-fast・BTreeMap 順序・wholesale override）、crate の依存境界（serde/serde_json のみ）。
