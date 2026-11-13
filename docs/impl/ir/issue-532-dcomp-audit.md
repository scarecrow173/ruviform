---
id: "IMPL-034WZFUVrjVFDc3zOgWJHp"
type: "impl"
status: "completed"
title: "Issue 532 DComp audit doc chain"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-532-dcomp-audit.md"
      - type: "file"
        path: "docs/discovery/issue-532-dcomp-native-dock.md"
      - type: "file"
        path: "docs/designs/issue-532-dcomp-native-dock.md"
      - type: "file"
        path: "docs/specs/issue-532-dcomp-audit.md"
      - type: "file"
        path: "docs/designs/issue-532-dcomp-audit-design.md"
      - type: "file"
        path: "docs/plans/issue-532-dcomp-audit-plan.md"
      - type: "file"
        path: "docs/tasks/issue-532-dcomp-audit-task.md"
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-audit.md"
      - type: "file"
        path: "docs/impl/exp/issue-532-dcomp-audit.md"
    modified:
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
  implements:
    - "docs/tasks/issue-532-dcomp-audit-task.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-dcomp-native-dock.md"
    - "docs/specs/issue-532-dcomp-audit.md"
    - "docs/designs/issue-532-dcomp-audit-design.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/plans/issue-532-dcomp-audit-plan.md"
    - "docs/test-specs/issue-532-dcomp-audit.md"
    - "docs/impl/exp/issue-532-dcomp-audit.md"
    - "docs/impl/ir/issue-388-os-l1-ci-evidence.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-532-dcomp-audit-task.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-532-dcomp-audit.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 532 DComp audit doc chain

## Summary

Issue #532 の残件を headless で処理する文書監査スライス。`crates/ruviform-ui*` の Fake/`NativeSurfaceSlot` 契約・headless 試験の既存被覆を調査し、対象 SHA・検査方法・観測を `docs/discovery/issue-532-dcomp-native-dock.md`（正本）に記録した。L2/L3 再開条件と方式 A/B/C 調査手順・担当（#388/#389/SV #176）を `docs/designs/issue-532-dcomp-native-dock.md` に記録した。Blocked by 誤登録の解消提案を記録した（誤登録は観測されず、除外提案の実作業は不要と判断）。コード変更はなく、文書のみの slice である。

## Context

Issue #532（GPUI 0.2.2 DComp topmost 合成制約）の先行 Slice（DComp 制約の検知と #388 への Partial/Unsupported 記録）は PR #501 で完了済み。残る未チェック受入項目は headless に可能な監査・文書スライスであり、実機 L2/L3 調査は利用要件発生まで Deferred。検証レベルの規範は `POLICIES.md` §2〜§5 に従う。

## Implementation

### Changes

- `docs/discovery/issue-532-dcomp-native-dock.md` — 正本監査記録（F1 制約・F2 L1 被覆棚卸し・F3 CI lane 対応付け・F4 依存状態・L2/L3 再開条件・方式 A/B/C 調査手順・Blocked by 除外提案・Evidence）。
- `docs/designs/issue-532-dcomp-native-dock.md` — 実機調査設計（再開条件表・Step 1〜4 手順・方式 A/B/C 採否条件・契約維持条件・リスク）。
- `docs/specs/issue-532-dcomp-audit.md` — 本 slice の受入条件（R1〜R6・AC）。
- `docs/designs/issue-532-dcomp-audit-design.md` — 本文書チェーンの構成設計。
- `docs/plans/issue-532-dcomp-audit-plan.md` — 実施計画。
- `docs/tasks/issue-532-dcomp-audit-task.md` — 実施タスク記録。
- `docs/test-specs/issue-532-dcomp-audit.md` — 検証仕様（R-DSC-01〜07・EX-DSC-01〜04）。
- `docs/impl/ir/issue-532-dcomp-audit.md` — 本記録。
- `docs/impl/exp/issue-532-dcomp-audit.md` — 実験ログ（監査手法の試行記録）。
- `docs/<type>/README.md` — index 再生成（`doc-maintenance` の `index apply` で全対象 dir を再生成。`docs/impl/exp` の生成 index は in-sync で変更不要のため、手書き .md レポート一覧のみ手動追記）。

### Decisions During Implementation

- **正本の分離**: `docs/discovery/issue-532-dcomp-native-dock.md` を観測事実・証拠・判定の正本とし、`docs/designs/issue-532-dcomp-native-dock.md` を実機調査手順の設計として分離。将来の実装 slice が設計文書を参照できるようにした。
- **CI 証拠の SHA 整合**: CI run `43428172820` の対象 SHA `e93c438a` と監査 HEAD `8944a360` の差分を `git diff` で確認し、契約層ファイル（`native_slot.rs`/`fake_native_slot.rs`/`native_slot_contract.rs`/`driver_contract.rs`）は無変更、`surface.rs` は doc comment rename のみであることを確認。discovery に「同一 SHA の新規 run は未取得」と明記して誤読を防いだ。
- **Blocked by 除外提案**: Issue #532 本文の「着手条件（Blocked by）」は「なし」であり、GitHub の正式依存としての誤登録は観測されなかった。除外提案の実作業は不要と判断し、根拠のみを記録。
- **index 再生成**: `doc-maintenance` の `index apply` で全対象 dir を再生成（`docs/impl/ir` を含む）。`docs/impl/exp` の生成 index（.jsonl 表）は in-sync で変更不要のため、手書き .md レポート一覧のみ手動追記。

## Related Experiments

### Adopted

- なし（本 slice は文書監査であり、実験的手法の採用はない）。

### Rejected

- なし。

## Validation

- `mise run fmt-check` — exit 0（Markdown/構成ファイルの軽量チェック）。
- `mise exec -- prek run --files <変更ファイル>` — exit 0（Markdown/リンクの軽量チェック）。
- `doc-maintenance` の `index apply` + `doc-status` — index 整合・front matter relations 解決を確認。
- 実機・実 plugin・実 GUI は対象外（POLICIES.md §5 の文書のみ変更の軽量検証）。

## Risks

- SHA 不一致（CI 証拠 `e93c438a` vs 監査 HEAD `8944a360`）の誤読 → `git diff` で契約層無変更を確認済み・discovery に明記。
- fake/headless の過大評価 → 各所で「実 Dock PASS ではない」「未確証」を明記。
- index の扱い → `docs/impl/ir` は `index apply` で再生成。`docs/impl/exp` の生成 index は in-sync、手書き .md 一覧のみ手動追記。

## Follow-ups

- 実機 L2/L3 調査（方式 A/B/C の採否・実 plugin 画素の実証）は Windows L2/L3 利用要件発生時に `docs/designs/issue-532-dcomp-native-dock.md` の手順で実施。
- `ruviform-poc-native-dock` の `gpui`/`vst3` feature の CI compile 追加は別 Issue 判断。
- `consumer-verify.yml` の Consumer Oracle run は次回 dispatch 時。
