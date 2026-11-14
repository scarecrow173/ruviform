---
id: "PLAN-034X9Vq34lcX0EODzIuD9u"
type: "plan"
status: "completed"
title: "Linux 配布設計 CI 照合の実施計画（Issue #127）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/127"
    - "docs/specs/linux-distribution.md"
    - "docs/designs/linux-distribution-design.md"
    - "docs/test-specs/linux-distribution.md"
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
    - "docs/designs/linux-distribution-design.md"
  implemented-by:
    - "docs/tasks/linux-distribution-task.md"
  depends-on:
    - "docs/specs/linux-distribution.md"
    - "docs/designs/linux-distribution-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/linux-distribution.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/linux-distribution.md"
  derived-by:
    - "docs/tasks/linux-distribution-task.md"
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

# Linux 配布設計 CI 照合の実施計画（Issue #127） Implementation Plan

## Goal

Issue #127 の残件スライスとして、既存 Ubuntu CI lane の定義と main 上の成功 run を SHA 単位で照合した証拠台帳を作り、先行設計文書をその証拠で更新・確定した docs チェーンを完成させる。

## Scope

単一の docs/evidence ストリーム。コード・CI・依存・パッケージ成果物の変更を含まないため分割しない。編集範囲は task spec の占有範囲（`evidence/rf127/`・`docs/{specs,designs,plans,tasks,test-specs}/linux-distribution*.md`・`docs/impl/{ir,exp}/linux-distribution.md`・各 dir の管理 index 再生成）に限定する。

## Upstream Documents

- `docs/specs/linux-distribution.md`（本 slice の spec・Issue #127 受入条件の転記）
- `docs/designs/linux-distribution-design.md`（approved・CI 照合結果を反映した確定版設計）
- `docs/test-specs/linux-distribution.md`（本 plan の検証規範 R1〜R8）
- 先行チェーン: `docs/{specs,designs,test-specs,plans,tasks}/issue-127-linux-dist-design.md`（PR #529/#573・編集範囲外・`refines`/`related` で接続）

## Design Inputs

- `docs/designs/linux-distribution-design.md`（status: approved）
- `evidence/rf127/linux-ci-crosscheck.md`（CI 照合レジャー・正本）

## File Map

- `evidence/rf127/linux-ci-crosscheck.md` - 既存 CI run の対象 SHA・URL・job・結果・テスト集計の台帳
- `docs/specs/linux-distribution.md` - slice の spec（受入条件・scope・deferred）
- `docs/designs/linux-distribution-design.md` - 確定版設計（runtime 最低要件・capability matrix・package 保留・Owner 境界）
- `docs/test-specs/linux-distribution.md` - 証拠/設計の整合を保証する検証規範
- `docs/plans/linux-distribution-plan.md` - 本計画
- `docs/tasks/linux-distribution-task.md` - 実施タスク記録
- `docs/impl/ir/linux-distribution.md` - Implementation Record
- `docs/impl/exp/linux-distribution.md` - 照合手法の試行記録
- `docs/{specs,designs,test-specs,plans,tasks}/README.md`・`docs/impl/{ir,exp}/README.md` - doc-maintenance/impl-doc による管理 index 再生成

## Implementation Sequence

- [x] 既存 CI run の照合: `gh run list/view`・`gh api .../artifacts`・`gh run view --job --log` で run 43439434737（@ `365c74ac`）の全 lane 成功・テスト集計・checkout-sha artifact を確認し `evidence/rf127/linux-ci-crosscheck.md` に台帳化
- [x] lane 系譜・関連 workflow（daily-ci/consumer-verify）の最新 run を照合し、HEAD（`621490e8`）の CI 未実施を `NotRun(Pending)` として記録
- [x] spec→design→test-spec の確定版チェーンを作成し、design を review-protocol でレビュー後 approved に遷移
- [x] plan→task を script gate 経由で生成し内容を記述
- [x] IR/exp 記録を作成（IR は task 開始時に in-progress で作成・随時更新）
- [x] 管理 index を doc-maintenance/impl-doc の生成手順で再生成
- [x] `audit_docs`/`audit_impl_record`/`audit_experiment_log`/`doc_status` と `mise run fmt-check` を実行し結果を記録
- [x] Conventional Commits でコミット・`docs/127-linux-distribution` を push・日本語 PR を作成

## Dependencies and Risks

- **依存**: `gh` CLI の認証と GitHub Actions への read アクセス。文書生成 script（`new_*`/`doc_maintenance`/`sync_impl_index`/`audit_*`）は repo 内 `.agents/skills/` に存在。
- **リスク**: CI 証拠は SHA 固有。`365c74ac..621490e8` に `crates/`・`mise.toml`・`ci.yml` の差分があるため、HEAD の Pass には転用しない（`NotRun(Pending)`）。`P(L0)`/headless CI を実機 `Pass` に昇格させない。deb/AppImage/Flatpak は全形式 `保留` のまま。
- **収集中の HEAD 前進**: 収集中に main HEAD が `c49e2a81`→`621490e8` へ前進したが、いずれの SHA にも ci run はなく判定は変わらない。

## Task Breakdown

- `docs/tasks/linux-distribution-task.md` 単一タスク（docs/evidence slice のため分割しない）。

## Verification

- [x] CI 証拠の実測照合: run 43439434737 全 job success・`real_window` 3 passed・`test result: ok` 201 block・rt_ 166・l1-real/ui-gpui 内訳・checkout-sha artifact 3 leg 一致（`gh` 実測）
- [x] `git diff --name-only 365c74ac..origin/main` で HEAD 差分に `crates/`・`mise.toml`・`ci.yml` があることを確認し `NotRun(Pending)` 判定の根拠とした
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type <type>`（spec/design/test-spec/plan/task・artifact + materialized profile）で blocking finding なし
- [ ] `node .agents/skills/impl-doc/scripts/audit_impl_record.js`・`audit_experiment_log.js` で blocking finding なし
- [ ] `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan` で管理 index が `in-sync`
- [ ] `mise run fmt-check` の exit code を記録
- [ ] review-protocol: 目的・必須要件の確認 → 客観チェック → cooperative/adversarial レビュー → CONFIRMED P0/P1 の修正 → blocker なしで停止

## Open Questions / Gaps

- 現行 main HEAD（`621490e8`）の CI 結果は未取得（daily-ci dispatch 待ち）。merge 後の対象 SHA の必須 CI 確認は Coordinator 責務。
- 実機 Linux GUI/Plugin・対象 distro 決定・package 形式採用は利用要件発生まで Deferred（#419/#262/#388/#389・Consumer 側 Issue）。
- dlopen 実行時依存の網羅検査 lane・専用 ABI lane は CI に存在しない（`NotRun`・増設しない）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 docs slice・Worker 契約により subagent 不使用）
- Independent work streams: none
- User approval required before dispatch: yes → task contract の review-protocol で代替（blocker なしを承認として扱う）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提: CI 証拠は `evidence/rf127/linux-ci-crosscheck.md` の SHA/run URL が正本。レビュー時は `gh run view 43439434737` で再現可能。
- 注意点: `P(L0)`/`NotRun(Pending)`/`保留` の語彙が design・test-spec・evidence で一貫しているか。`365c74ac` の成功を HEAD の Pass にしていないか。
