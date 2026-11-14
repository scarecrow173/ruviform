---
id: "TASK-034XzvRtVDZG1xxfMh2b56"
type: "task"
status: "done"
title: "Issue #262 GPUI採用Gate verdict 文書チェーン・証拠完成・検証"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/262"
  changes:
    added:
      - type: "doc"
        path: "docs/tasks/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "docs/impl/ir/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "docs/impl/exp/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "evidence/rf262-gpui-verdict/verdict-matrix.md"
      - type: "doc"
        path: "evidence/rf262-gpui-verdict/report.md"
      - type: "doc"
        path: "evidence/rf262-gpui-verdict/handoff-419.md"
      - type: "doc"
        path: "evidence/rf262-gpui-verdict/verification.md"
    modified:
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
    deleted: []
    renamed: []
    moved: []
    generated:
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
    - "docs/plans/issue-262-gpui-verdict.md"
  implemented-by:
    - "docs/impl/ir/issue-262-gpui-verdict.md"
  depends-on:
    - "docs/plans/issue-262-gpui-verdict.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-262-gpui-verdict.md"
    - "docs/designs/issue-262-gpui-verdict.md"
    - "docs/test-specs/issue-262-gpui-verdict.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-262-gpui-verdict.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-262-gpui-verdict.md"
  references:
    - "evidence/rf262-gpui-verdict/verdict-matrix.md"
    - "evidence/rf262-gpui-verdict/report.md"
    - "evidence/rf262-gpui-verdict/handoff-419.md"
    - "evidence/rf262-gpui-verdict/verification.md"
  defers: []
  deferred-by: []
---

# Issue #262 GPUI採用Gate verdict 文書チェーン・証拠完成・検証 Task

## Work

Issue #262 の採用 Gate 評価スライスとして、A〜E Gate の verdict matrix・証拠 report・#419 引き渡し文面・検証記録を `evidence/rf262-gpui-verdict/` に作成し、spec/design/test-spec/plan/task/IR/exp の文書チェーンを完成させて検証を通す。コード・CI・ADR・Issue 投稿は対象外。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-262-gpui-verdict.md`
- `docs/specs/issue-262-gpui-verdict.md`
- `docs/designs/issue-262-gpui-verdict.md`
- `docs/test-specs/issue-262-gpui-verdict.md`

## Dependencies

- 先行証拠: `evidence/rf388-os-l1-ci/`、`evidence/rf388-389-native-dock/`、`evidence/rf388-native-dock-{layout,windowing}/`、`evidence/rf389-vst3-editor/`、`evidence/rf389-session-hardening/`、`evidence/rf313-os-capability-matrix/`、`evidence/rf243/`、`evidence/issue-243-multiwindow-verify/`、`evidence/rf465/`
- 参照 Issue: #262（本件）・#419（引き渡し先）・#388/#389/#532（Native Dock）・#465（Legacy 撤去）・#294（両 Consumer）

## Affected Files

- `docs/{specs,designs,test-specs,plans,tasks,impl/ir,impl/exp}/issue-262-gpui-verdict.md`
- `evidence/rf262-gpui-verdict/{verdict-matrix,report,handoff-419,verification}.md`
- `docs/{specs,designs,test-specs,plans,tasks,impl/ir,impl/exp}/README.md`（生成 index）

## Done When

- [x] spec/design/test-spec/plan/task/IR/exp の文書チェーンが `docs/` に存在し、front matter relation が内部整合する。
- [x] `evidence/rf262-gpui-verdict/` に verdict-matrix・report・handoff-419・verification の 4 証拠が存在する。
- [x] verdict matrix が A〜E Gate × OS × capability を CaseID/expected/observed/verdict/証拠で記録し、Fail/NotRun/Unsupported を Pass に繰り上げていない。
- [x] #419 引き渡し文面が「L1 のみ評価済み、L2/L3 非保証」と #465 独立判断境界を明記している。
- [x] 触れた docs ディレクトリの管理 index を `doc-maintenance index apply` / `sync_impl_index.js` で再生成し `in-sync` を確認する。
- [x] `mise run fmt-check` と `mise exec -- prek run --files <変更ファイル>` が Pass する。
- [x] `audit_docs`/`audit_impl_record`/`audit_experiment_log` で自スコープの error がない。
- [x] 証拠再実走（`test-ui-scenario-contract`/`check-ui-driver-boundary`/`test-ui-native-slot`/`test-ui-parity`/`test-ui-gpui`）を HEAD 上で実行し結果を `verification.md` に記録する。
- [x] verdict の決裁宣言・Issue 投稿・ADR status 変更を行っていない（Coordinator 権限として明記）。

## Verification

- `mise run fmt-check`（= `cargo fmt --all -- --check`）
- `mise exec -- prek run --files <変更ファイル>`
- `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan|apply --dir <dir>`
- `node .agents/skills/impl-doc/scripts/sync_impl_index.js --kind ir|exp --apply`
- `node .agents/skills/doc-status/scripts/audit_docs.js --type <type> --dir docs/<dir>`
- `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` / `audit_experiment_log.js --json`
- `mise run test-ui-scenario-contract` / `check-ui-driver-boundary` / `test-ui-native-slot` / `test-ui-parity` / `test-ui-gpui`（証拠再実走）
