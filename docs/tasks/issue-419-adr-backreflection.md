---
id: "TASK-034XPQF7BrLnw2ekZ4Q3r0"
type: "task"
status: "done"
title: "Issue #419 後継ADR verdict逆反映 文書チェーン・検証タスク"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/419"
  changes:
    added:
      - type: "doc"
        path: "docs/tasks/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/impl/ir/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/impl/exp/issue-419-adr-backreflection.md"
    modified:
      - type: "doc"
        path: "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
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
      - type: "doc"
        path: "docs/adr/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/adr/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
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
  implements:
    - "docs/plans/issue-419-adr-backreflection.md"
  implemented-by:
    - "docs/impl/ir/issue-419-adr-backreflection.md"
  depends-on:
    - "docs/plans/issue-419-adr-backreflection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-419-adr-backreflection.md"
    - "docs/designs/issue-419-adr-backreflection.md"
    - "docs/test-specs/issue-419-adr-backreflection.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-419-adr-backreflection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-419-adr-backreflection.md"
  references:
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "evidence/rf262-gpui-verdict/verdict-matrix.md"
    - "evidence/rf262-gpui-verdict/report.md"
    - "evidence/rf262-gpui-verdict/handoff-419.md"
  defers: []
  deferred-by: []
---

# Issue #419 後継ADR verdict逆反映 文書チェーン・検証 Task

## Work

Issue #419 の「Coordinator 決裁済み verdict（outcome = CONDITIONAL）を後継 ADR へ逆反映する」docs-only slice として、対象 ADR の More Information 逆反映欄・Confirmation・frontmatter を更新し、slug `issue-419-adr-backreflection` の文書チェーン 7 ファイルを完成させ、触れた全 dir の管理 index を再生成して検証を通す。コード・CI・evidence・他 ADR・Issue 投稿は対象外。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-419-adr-backreflection.md`
- `docs/specs/issue-419-adr-backreflection.md`
- `docs/designs/issue-419-adr-backreflection.md`
- `docs/test-specs/issue-419-adr-backreflection.md`

## Dependencies

- 判定正本（読み取り専用）: `evidence/rf262-gpui-verdict/verdict-matrix.md`・`report.md`・`handoff-419.md`・`verification.md`
- 先例チェーン: `issue-262-gpui-verdict`（同一 stem 命名・docs-only slice）
- 参照 Issue: #419（本件）・#262（verdict 証拠）・#388/#389/#532（Native Dock/DComp）・#465（Legacy 撤去）・#294（両 Consumer）・#630/#631（仮想優先 Gate、残 DoD）

## Affected Files

- `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`（More Information 逆反映欄・Confirmation・frontmatter `updated`/`references`。`status` 不変）
- `docs/{specs,designs,test-specs,plans,tasks,impl/ir,impl/exp}/issue-419-adr-backreflection.md`（新規 7 文書）
- `docs/adr/README.md`・`docs/{specs,designs,test-specs,plans,tasks,impl/ir,impl/exp}/README.md`（生成 index）

## Done When

- [x] spec/design/test-spec/plan/task/IR/exp の文書チェーンが `docs/` に存在し、front matter relation が内部整合する。
- [x] ADR More Information 逆反映欄が「未記入」を含まず、outcome = CONDITIONAL・「L1 のみ評価済み、L2/L3 非保証」・標準 Driver 方針維持・Native Dock 無条件採用不可・宣言 Capability・既知制約/将来 Owner・`evidence/rf262-gpui-verdict/` 相対リンクを日付付きで記録する。
- [x] ADR Confirmation に逆反映実施の dated 記録と outcome・相互参照・無条件採用を宣言しない制約の維持がある。
- [x] ADR frontmatter `status` は `proposed` のまま、`updated` が当日日付、`references` に `#419`/`#532` が追加され、`id`/`created`/既存 `relations` が不変。
- [x] ADR の Decision Outcome・判定フロー・縮退順序・責務マップ・Capability Matrix・Verification の既存記述が不変。
- [x] 触れた docs ディレクトリの管理 index を `doc-maintenance index apply` / `sync_impl_index.js` / `docs/adr` index 手順で再生成し `in-sync` を確認する。
- [x] `mise run fmt-check` と `mise exec -- prek run --files <変更ファイル>` が Pass する。
- [x] `audit_docs`/`audit_impl_record`/`audit_experiment_log` で自スコープの error がない。
- [x] verdict の決裁宣言・Issue 投稿・Close・`evidence/` 編集・他 ADR/POLICIES/コード変更を行っていない（Coordinator 権限として明記）。

## Verification

- `mise run fmt-check`（= `cargo fmt --all -- --check`）
- `mise exec -- prek run --files <変更ファイル>`
- `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan|apply --dir <dir>`（`docs/adr` + 触れた全 dir）
- `node .agents/skills/impl-doc/scripts/sync_impl_index.js --kind ir|exp`（preview → `--apply`）
- `node .agents/skills/doc-status/scripts/audit_docs.js --type <type> --dir docs/<dir>`
- `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` / `audit_experiment_log.js --json`
- Rust 系 lane（`check`/`lint`/`test`/`build`/`verify`・`*-full`・`*-msrv`・Feature Matrix・3OS・実機 GUI/Native Dock/Consumer Oracle）は docs-only のため **NotRun**（POLICIES §5。統合検証は merge 後対象 SHA の CI 証拠）
