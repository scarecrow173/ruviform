---
id: "TASK-034T81mttwNdGpBq1wCGwP"
type: "task"
status: "in-progress"
title: "Implement RF384/RF385: Driver-neutral contract docs + ruviform-ui-theme crate"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/rf384-rf385-driver-neutral-gui-contract-ruviform-ui-theme-dtcg-interpreter-issue.md"
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
    - "docs/plans/rf384-rf385-driver-neutral-gui-contract-ruviform-ui-theme-dtcg-interpreter-issue.md"
  implemented-by: []
  depends-on:
    - "docs/plans/rf384-rf385-driver-neutral-gui-contract-ruviform-ui-theme-dtcg-interpreter-issue.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/rf384-rf385-driver-neutral-gui-contract-ruviform-ui-theme-dtcg-interpreter-issue.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement RF384/RF385: Driver-neutral contract docs + ruviform-ui-theme crate

## Work

Issue #384: 後継 Driver-neutral ADR 作成 + 旧 2 ADR supersede + spec/design 契約ドキュメント。Issue #385: `rvf-dtcg-profile.v1` の deterministic interpreter クレート `ruviform-ui-theme`（parse/validate/alias/mode/override → 型付き不変 `ResolvedTheme`）と fixture/contract tests。独立レビュー（A/B→Validator→Judge）を経て CONFIRMED P0/P1 を修正済み。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/rf384-rf385-driver-neutral-gui-contract-ruviform-ui-theme-dtcg-interpreter-issue.md`
- `docs/specs/issue-384-driver-neutral-gui-architecture.md` / `docs/specs/issue-385-dtcg-resolved-theme.md`
- `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`

## Dependencies

- GPUI 採用最終承認は本 task 外（#262/#388/#389 実機証拠後、ADR 逆反映欄へ）。
- #386–#390 は別 worker 所有。共有 `Cargo.lock`/index は最小差分。

## Affected Files

- `crates/ruviform-ui-theme/{src/{lib,errors,doc,resolve,snapshot}.rs,tests/{pipeline.rs,fixtures/*.json},Cargo.toml,README.md}`
- `docs/{adr,specs,designs,plans,tasks}/` 上記ファイル + 各 README index
- `Cargo.toml`（members）、`crates/README.md`

## Done When

- [x] ADR + spec/design/plan/task の doc チェーンが揃い、旧 2 ADR が `superseded` + 双方向リンクで履歴保持
- [x] `ruviform-ui-theme` が serde/serde_json のみ依存で build、`ResolvedTheme` 公開 API に GPUI/toolkit 型なし
- [x] 18 contract tests + doctest green、deterministic normalized output
- [x] 独立レビューで CONFIRMED P0/P1 なし（Validator/Judge verdict 記録）
- [ ] origin/main merge・fmt-check・Conventional Commit・push・日本語 PR

## Verification

- [x] `mise run check` / `mise run lint` pass（workspace・clippy -D warnings）
- [x] `mise exec -- cargo test -p ruviform-ui-theme` 18+1 pass
- [x] workspace `mise run test`: plugin-host L1 e2e 1件 flake（単独再実行 pass、本変更無関係）
- [x] `audit_adr.js` error 0、`doc_maintenance.js` 自 doc finding 解消
- [ ] `mise run fmt-check`（最終）・PR 作成
