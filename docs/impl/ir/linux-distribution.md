---
id: "IMPL-034X9OsAFWEhAZF1OWqG4R"
type: "impl"
status: "completed"
title: "Linux distribution CI cross-check and doc finalization"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/127"
    - "docs/tasks/linux-distribution-task.md"
    - "docs/plans/linux-distribution-plan.md"
    - "docs/designs/linux-distribution-design.md"
    - "docs/specs/linux-distribution.md"
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
    - "docs/tasks/linux-distribution-task.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/gt-05-ci-offscreen-lanes-slice.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "evidence/rf127/linux-ci-crosscheck.md"
    - "docs/designs/issue-127-linux-dist-design.md"
    - "docs/test-specs/issue-127-linux-dist-design.md"
    - "docs/plans/issue-127-linux-dist-design.md"
    - "docs/tasks/issue-127-linux-dist-design.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/linux-distribution.md"
    rejected: []
---

# Linux distribution CI cross-check and doc finalization

## Summary

Issue #127 の残件スライスとして、既存 Ubuntu CI（`ci.yml`/`daily-ci.yml`/`consumer-verify.yml`）の lane 定義と main 上の成功 run を `gh` で照合し、結果を `evidence/rf127/linux-ci-crosscheck.md` に台帳化した。あわせて先行設計（PR #529/#573 由来の `issue-127-linux-dist-design.md` チェーン）を既存 CI 証拠で更新・確定した docs チェーン（spec/design/test-spec/plan/task + 本 IR + exp 記録）を作成し、管理 index を再生成した。コード・CI・依存・パッケージ成果物の変更はない。

## Context

- Task: [docs/tasks/linux-distribution-task.md](../../tasks/linux-distribution-task.md)（Issue #127・branch `docs/127-linux-distribution`）。
- Plan: [docs/plans/linux-distribution-plan.md](../../plans/linux-distribution-plan.md)。Spec: [docs/specs/linux-distribution.md](../../specs/linux-distribution.md)。Design: [docs/designs/linux-distribution-design.md](../../designs/linux-distribution-design.md)（先行版 `docs/designs/issue-127-linux-dist-design.md` を `refines`）。Test-spec: [docs/test-specs/linux-distribution.md](../../test-specs/linux-distribution.md)。Evidence: [evidence/rf127/linux-ci-crosscheck.md](../../../evidence/rf127/linux-ci-crosscheck.md)。Exp: [docs/impl/exp/linux-distribution.md](../exp/linux-distribution.md)。
- 先行版は GPUI Linux lane の hosted 初回実行を `NotRun` と記録していたが、その後 Issue #582 修正（PR #593）を経て main で全 lane 成功（run 43439434737 @ `365c74ac`）しており、文書の確定にはこの実績の反映が必要だった。
- 実 deb/AppImage/Flatpak 作成・署名・実機検証は本 slice の対象外（spec Out of Scope）。

## Implementation

### Changes

- `evidence/rf127/linux-ci-crosscheck.md` を新規作成: main HEAD と最新成功 run の対応、GPUI lane 系譜（追加 PR #566・失敗→Issue #582 修正 PR #593・初緑 run 43326874974・最新緑 run 43439434737）、Ubuntu lane 台帳（job URL・実行内容・verdict）、checkout-sha artifact 照合、受入条件カテゴリ写像、正直な記録（NotRun/Deferred 項目）を収録。
- `docs/specs/linux-distribution.md`・`docs/designs/linux-distribution-design.md`・`docs/test-specs/linux-distribution.md` を新規作成: spec→design→test-spec の確定版チェーン。design は先行版の構造（判定語彙・distro/Backend・runtime 依存・capability matrix・package 比較・Owner 境界・legacy 分離・検証手順・リスク・AC 写像）を継承し、ubuntu-24.04 hosted CI の実測を反映して更新。
- `docs/plans/linux-distribution-plan.md`・`docs/tasks/linux-distribution-task.md` を新規作成（design approved 後に script gate を通して生成）。
- `docs/impl/exp/linux-distribution.md` を新規作成: CI 照合手法の試行記録（`.md` 形式・issue-532 precedent）。
- `docs/{specs,designs,test-specs,plans,tasks}/README.md`・`docs/impl/{ir,exp}/README.md` を doc-maintenance/impl-doc の生成手順で再生成。

### Decisions During Implementation

- **新規チェーンを `issue-127-linux-dist-design.md` の更新版として別ファイルで作成**: 先行ファイルは編集範囲外のため、同名更新ではなく `linux-distribution*.md` の確定版チェーンを新設し、`refines`/`related`/`references` で先行版と接続した。先行版の逆方向 relation フィールドは空のため reciprocal 整合の問題は生じない。
- **design status は `draft`→レビュー後 `approved`**: `new_plan.js` の design gate は approved design を要求するため、review-protocol で blocker なしを確認後に approved へ遷移させて plan/task を生成した（ユーザー承認の代替として task contract の review-protocol 規定に従う）。
- **IR/exp のファイル名は scope 指定の `linux-distribution.md` に合わせた**: `new_impl_record.js` は title slug で生成するため生成後に `mv` で rename（impl-conventions の slug-only 命名に適合）。exp は `.md` 試行記録として手動作成（`.jsonl` 実験ログではない）。
- **`365c74ac` の CI 証拠を現行 HEAD `621490e8` の Pass にしない**: `365c74ac..621490e8` に `crates/`・`mise.toml`・`ci.yml` の差分があるため、HEAD は `NotRun(Pending)` として分離記録した。

## Related Experiments

### Adopted

- `docs/impl/exp/linux-distribution.md` — CI 照合手法（gh run/job/log/artifact 照合・SHA 差分評価・文書生成 script の出力先調整）の試行記録。design/evidence の記述根拠として採用。

### Rejected

- なし。

## Validation

- `gh run list`/`gh run view`/`gh api .../artifacts`/`gh run view --job --log` で run 43439434737 の全 job success・`real_window` 3 passed・`test result: ok` 201 block を確認。checkout-sha artifact（verify-ubuntu-latest・ui-gpui-verify-ubuntu-latest・ui-gpui-offscreen-verify）を取得し `365c74ac` と一致を確認。
- `audit_docs.js`（spec/design/test-spec/plan/task・artifact profile + materialized）・`audit_impl_record.js`・`audit_experiment_log.js`・`doc_status.js lint`/`audit` を実行（結果は PR 本文に記録）。
- `mise run fmt-check` を実行（docs/audit 系スライスのため crate 検証は不要・結果は PR 本文に記録）。
- `git diff --name-only origin/main...HEAD` が編集許可範囲 + 管理 index に限定されることを確認。

## Risks

- CI 証拠は SHA 固有であり、main HEAD `621490e8` の CI は未実施（`NotRun(Pending)`）。merge 後の対象 SHA の必須 CI は Coordinator が確認する。
- `P(L0)`（Xvfb+openbox+lavapipe）は実機 `Pass` ではない。実機 GUI/Plugin/Device は Deferred/NotRun のまま。
- deb/AppImage/Flatpak は全形式 `保留` のまま。package 作成・署名・install 検証は未実施。
- dlopen 実行時依存（`x11-dl`/`ash`）の網羅検査 lane・専用 ABI lane は CI に存在しない（`NotRun`）。

## Follow-ups

- merge 後の対象 SHA に対する必須 CI（daily-ci dispatch）結果の確認（Coordinator 責務）。
- 実機 Linux GUI/Plugin 検証・対象 distro 決定・package 形式採用は利用要件発生時に #419/#262/#388/#389・Consumer 側 Issue で実施。
