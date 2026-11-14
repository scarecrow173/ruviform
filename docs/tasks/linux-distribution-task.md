---
id: "TASK-034X9Vy4vI2WPlFTQsXqKD"
type: "task"
status: "done"
title: "Linux 配布設計の既存 CI 照合と文書確定（Issue #127）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/127"
  changes:
    added:
      - type: "doc"
        path: "evidence/rf127/linux-ci-crosscheck.md"
      - type: "doc"
        path: "docs/specs/linux-distribution.md"
      - type: "doc"
        path: "docs/designs/linux-distribution-design.md"
      - type: "doc"
        path: "docs/test-specs/linux-distribution.md"
      - type: "doc"
        path: "docs/plans/linux-distribution-plan.md"
      - type: "doc"
        path: "docs/tasks/linux-distribution-task.md"
      - type: "doc"
        path: "docs/impl/ir/linux-distribution.md"
      - type: "doc"
        path: "docs/impl/exp/linux-distribution.md"
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
    generated: []
  implements:
    - "docs/plans/linux-distribution-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/linux-distribution-plan.md"
    - "docs/designs/linux-distribution-design.md"
    - "docs/test-specs/linux-distribution.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/linux-distribution-plan.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/linux-distribution-plan.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/linux-distribution.md"
  references:
    - "evidence/rf127/linux-ci-crosscheck.md"
    - "docs/designs/issue-127-linux-dist-design.md"
    - "docs/test-specs/issue-127-linux-dist-design.md"
    - "docs/plans/issue-127-linux-dist-design.md"
    - "docs/tasks/issue-127-linux-dist-design.md"
  defers: []
  deferred-by: []
---

# Linux 配布設計の既存 CI 照合と文書確定（Issue #127）

## Work

Issue #127 の残件スライス。既存 Ubuntu CI（`ci.yml`/`daily-ci.yml`/`consumer-verify.yml`）の lane 定義と main 上の成功 run を `gh` で照合し、対象 SHA・run URL・job・結果・テスト集計を `evidence/rf127/linux-ci-crosscheck.md` に台帳化する。あわせて先行設計（PR #529/#573 由来の `issue-127-linux-dist-design.md` チェーン）を既存 CI 証拠で更新・確定した docs チェーン（spec/design/test-spec/plan/task + IR/exp）を作成し、管理 index を再生成する。docs/evidence のみで、コード・CI・依存・パッケージ成果物の変更は行わない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/linux-distribution-plan.md`（本 task の plan）
- `docs/specs/linux-distribution.md`（slice の spec）
- `docs/designs/linux-distribution-design.md`（approved・確定版設計）
- `docs/test-specs/linux-distribution.md`（検証規範 R1〜R8）

## Dependencies

- `gh` CLI 認証と GitHub Actions read アクセス（run/job/log/artifact 照合用）。
- 文書生成・監査 script は repo 内 `.agents/skills/`（spec-doc/design-doc/test-spec-doc/plan-doc/task-doc/impl-doc/doc-maintenance/doc-status/review-protocol）。
- 編集範囲は task spec の占有範囲のみ。先行 `issue-127-linux-dist-design.md` チェーン・`docs/*/README.md`（index は生成のみ）・`Cargo.*`/`mise.*`/`.github/` は編集しない。

## Affected Files

- `evidence/rf127/linux-ci-crosscheck.md`（新規）
- `docs/specs/linux-distribution.md`・`docs/designs/linux-distribution-design.md`・`docs/test-specs/linux-distribution.md`・`docs/plans/linux-distribution-plan.md`・`docs/tasks/linux-distribution-task.md`（新規）
- `docs/impl/ir/linux-distribution.md`・`docs/impl/exp/linux-distribution.md`（新規）
- `docs/{specs,designs,test-specs,plans,tasks}/README.md`・`docs/impl/{ir,exp}/README.md`（doc-maintenance/impl-doc による再生成）

## Done When

- [x] 既存 Ubuntu CI の対象 revision と成功 Run を照合し `evidence/rf127/` に台帳化。CI 定義済みのみの項目を `NotRun` と区別した。
- [x] Ruviform 共通 runtime 最低要件・distro/配布方式の採否/未確証・AL/SV 製品 packaging Owner 境界を文書で確定した。実 deb/AppImage/Flatpak 作成・署名を主張していない。
- [x] macOS/Linux L1（必須）: Native Build/Link・必要 Feature・Headless/RT の既存 CI を同候補 SHA（`365c74ac`）で確認。HEAD（`621490e8`）は `NotRun(Pending)`。
- [x] 能力別判定で実環境未測定・Fail・NotRun を Pass にせず、対象 OS/機能/Device/Format/実行 URL・Deferred/NotRequired の理由を分離した。
- [x] docs チェーンと IR/exp が front matter・relation・index の監査を通過し、`mise run fmt-check` を実行した。
- [x] Conventional Commits でコミットし `docs/127-linux-distribution` を push、`gh pr create` で日本語 PR を作成した。

## Verification

- [x] `gh run view 43439434737` 系の実測照合（全 job success・テスト集計・checkout-sha artifact）
- [x] `audit_docs.js`（spec/design/test-spec/plan/task・artifact + materialized）・`audit_impl_record.js`・`audit_experiment_log.js`・`doc_status.js` で blocking finding なし
- [x] `doc_maintenance.js index` で管理 index を再生成し `in-sync` を確認
- [x] `mise run fmt-check` 実行・exit code 記録（exit 0）
- [x] review-protocol 実施（客観チェック + cooperative/adversarial レビュー・CONFIRMED P0/P1 修正・blocker なし）
