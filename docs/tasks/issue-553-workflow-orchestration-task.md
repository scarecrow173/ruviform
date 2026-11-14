---
id: "TASK-tKTEtLLPQU3PUXYcRGkiMF"
type: "task"
status: "done"
title: "Issue #553 Workflow/CLI orchestration契約 文書チェーン完成・検証"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/553"
  changes:
    added:
      - type: "doc"
        path: "docs/tasks/issue-553-workflow-orchestration-task.md"
      - type: "doc"
        path: "docs/impl/ir/issue-553-workflow-orchestration.md"
      - type: "doc"
        path: "docs/impl/exp/issue-553-workflow-orchestration.md"
      - type: "doc"
        path: "evidence/rf553/verification.md"
    modified:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/adr/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/adr/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/plans/issue-553-workflow-orchestration-plan.md"
  implemented-by:
    - "docs/impl/ir/issue-553-workflow-orchestration.md"
  depends-on:
    - "docs/plans/issue-553-workflow-orchestration-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-553-workflow-orchestration.md"
    - "docs/designs/issue-553-workflow-orchestration-design.md"
    - "docs/adr/issue-553-workflow-cli-contract.md"
    - "docs/test-specs/issue-553-workflow-orchestration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-553-workflow-orchestration-plan.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-553-workflow-orchestration.md"
  references:
    - "evidence/rf553/contract-trace.md"
    - "evidence/rf553/consumer-applicability.md"
    - "evidence/rf553/verification.md"
  defers: []
  deferred-by: []
---

# Issue #553 Workflow/CLI orchestration契約 文書チェーン完成・検証 Task

## Objective

Issue #553 の DESIGN-ONLY スライスとして、Consumer-neutral Workflow/CLI orchestration 契約の文書チェーン（spec/design/ADR/test-spec/plan/task/IR/exp）と `evidence/rf553/` 証拠を完成させ、管理 index を再生成して検証を通す。

## Done When

- [x] spec/design/ADR/test-spec/plan/task/IR/exp の文書チェーンが `docs/` に存在し、front matter relation が内部整合する。
- [x] `evidence/rf553/` に contract-trace・consumer-applicability・verification の 3 証拠が存在する。
- [x] 触れた docs ディレクトリ（specs/designs/adr/test-specs/plans/tasks/impl/ir/impl/exp）の管理 index を `doc-maintenance index apply` で再生成し `in-sync` を確認する。
- [x] `mise run fmt-check` と `mise exec -- prek run --files <変更ファイル>` が Pass する。
- [x] `doc-status`/`audit_docs`/`audit_adr` で自スコープの error/blocking がない。
- [x] Runtime/Consumer/OS 実機の未実行項目を NotRun/Unsupported として `evidence/rf553/verification.md` に記録する。

## Verification

- `mise run fmt-check`（= `cargo fmt --all -- --check`）
- `mise exec -- prek run --files <変更ファイル>`
- `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan|apply --dir <dir>`
- `node .agents/skills/doc-status/scripts/audit_docs.js --type <type> --dir docs/<dir>`
- `node .agents/skills/adr-doc/scripts/audit_adr.js --dir docs/adr`
- `node .agents/skills/doc-status/scripts/doc_status.js audit --type all`（参考・pre-existing finding を区別）
