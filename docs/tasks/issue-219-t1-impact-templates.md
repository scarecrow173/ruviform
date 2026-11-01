---
id: "TASK-034QdV8LJ4ZnIRqDUSRakb"
type: "task"
status: "done"
title: "T1 Consumer影響判定・証拠テンプレート（PR/Issue template群）"
created: "2026-11-01"
updated: "2026-11-01"
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
    - "docs/plans/issue-219-consumer-cross-repo-ci-gate.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-219-consumer-cross-repo-ci-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/consumer-pr-issue-template.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 Consumer影響判定・証拠テンプレート（PR/Issue template群）

## Work

PR template と Issue template 3 件を新設する。design §Components の表形式・項目定義に厳密に従う。
- `.github/PULL_REQUEST_TEMPLATE.md`: Consumer 影響分類表（Synvelis/AudioLens × impacted/no impact/unknown × 根拠・リンク）＋検証証拠表（lane・OS・feature・ruviform revision・pass/fail/not run・許容例外・Rollback）＋ unknown 時の未検証理由・解消先必須記述
- `.github/ISSUE_TEMPLATE/consumer-impact.yml`: 変更種別（API/feature/behavior/dependency/CI 基盤）・両 Consumer 分類・根拠・Ruviform↔Consumer 相互リンクの form
- `.github/ISSUE_TEMPLATE/consumer-followup.yml`: Consumer 追従用（対象 revision・追従 lane・例外・Rollback）の form
- `.github/ISSUE_TEMPLATE/config.yml`: synvelis/audio-lens Issue への contact link

## Classification

- Type: implementation
- Size: S

## Upstream Documents

- `docs/plans/issue-219-consumer-cross-repo-ci-gate.md`
- `docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md`

## Dependencies

なし（T2 と独立）

## Affected Files

- `.github/PULL_REQUEST_TEMPLATE.md`
- `.github/ISSUE_TEMPLATE/consumer-impact.yml`
- `.github/ISSUE_TEMPLATE/consumer-followup.yml`
- `.github/ISSUE_TEMPLATE/config.yml`

## Done When

- 4 ファイルが design の表・項目を全て持つ（AC1/AC2）
- `unknown` 必須記述と human gate 注記が含まれる
- YAML form が parse 可能（`python -c yaml.safe_load` 相当）

## Verification

- [ ] 各ファイルの必須欄を design §Components と照合
- [ ] Issue form の YAML syntax check
