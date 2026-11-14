---
id: "IMPL-IbedjE9oj9KCJ7tyaLScCC"
type: "impl"
status: "completed"
title: "Issue #262 GPUI Native First採用Gate verdict 文書・証拠スライス実装記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/262"
  changes:
    added:
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
        path: "docs/specs/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "docs/designs/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "docs/test-specs/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "docs/plans/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "docs/tasks/issue-262-gpui-verdict.md"
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
    - "docs/tasks/issue-262-gpui-verdict.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-262-gpui-verdict.md"
    - "docs/designs/issue-262-gpui-verdict.md"
    - "docs/test-specs/issue-262-gpui-verdict.md"
    - "docs/plans/issue-262-gpui-verdict.md"
    - "docs/tasks/issue-262-gpui-verdict.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/impl/exp/issue-262-gpui-verdict.md"
    - "evidence/rf262-gpui-verdict/verdict-matrix.md"
    - "evidence/rf262-gpui-verdict/report.md"
    - "evidence/rf262-gpui-verdict/handoff-419.md"
    - "evidence/rf262-gpui-verdict/verification.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-262-gpui-verdict.md"
    rejected: []
---

# Issue #262 GPUI Native First採用Gate verdict 文書・証拠スライス実装記録

## Summary

Issue #262 の採用 Gate 評価スライスとして、A〜E Gate の verdict matrix・証拠 report・#419/#465 引き渡し文面・検証記録を `evidence/rf262-gpui-verdict/` に作成し、spec/design/test-spec/plan/task/IR/exp の文書チェーンを完成させた。実装コード・crate・CI workflow・依存変更・ADR status 変更・Issue 投稿は含まない。

## Context

- Task: `docs/tasks/issue-262-gpui-verdict.md`
- Plan: `docs/plans/issue-262-gpui-verdict.md`
- Spec: `docs/specs/issue-262-gpui-verdict.md`
- Design: `docs/designs/issue-262-gpui-verdict.md`（approved）
- Test-spec: `docs/test-specs/issue-262-gpui-verdict.md`
- Issue: https://github.com/scarecrow173/ruviform/issues/262

## Implementation

### Changes

- `docs/specs/issue-262-gpui-verdict.md` - 問題・Audience・Scope/Non-goals・受入条件・Evidence・Deferred を定義（SPEC-034Xzvx8VyUUbiqyFokB52）。
- `docs/designs/issue-262-gpui-verdict.md` - 判定方法論（D1 証拠階層/SHA 照合/verdict 語彙）・Gate A〜E 評価構造（D2〜D6）・推奨 verdict と引き渡し境界（D7）を定義（DESIGN-034XzvxSlT8Mm4uX42xwFb）。
- `docs/test-specs/issue-262-gpui-verdict.md` - verdict 記録の不変条件を Rule（R-EVIDENCE/R-VERDICT/R-GATE-A〜E/R-HANDOFF）と Example（EX-1〜4）へ固定（TSPEC-034XzvJurlMbzQjoWxrwvu）。
- `docs/plans/issue-262-gpui-verdict.md` - 文書・証拠スライスの実装計画（PLAN-034Xzw4vyjdTI0ZYgqUJr1）。
- `docs/tasks/issue-262-gpui-verdict.md` - 文書チェーン完成・検証タスク（TASK-034XzvRtVDZG1xxfMh2b56）。
- `evidence/rf262-gpui-verdict/verdict-matrix.md` - A〜E Gate verdict matrix 正本（CaseID/OS/driver/plugin/expected/observed/requirement/result/証拠 URL）。
- `evidence/rf262-gpui-verdict/report.md` - 証拠収集方法・SHA/Run 照合・Gate 別判定まとめ・既知ギャップ。
- `evidence/rf262-gpui-verdict/handoff-419.md` - #419/#465 向け引き渡し文面（「L1 のみ評価済み、L2/L3 非保証」・#465 独立判断境界）。
- `evidence/rf262-gpui-verdict/verification.md` - 検証実行記録（実コマンド・OS・toolchain・exit・結果）。
- `docs/{specs,designs,test-specs,plans,tasks,impl/ir,impl/exp}/README.md` - `doc-maintenance`/`sync_impl_index` で管理 index を再生成。

### Decisions During Implementation

- **verdict の決裁分離**: 本 slice は「推奨 verdict + 根拠表」のみを記録し、accepted/conditional/rejected の決裁宣言・Issue コメント投稿・Close 判定・ADR status 変更は行わない（Coordinator 権限）。spec/design/test-spec/IR で一貫して明記。
- **機構層と画素層の分離**: Gate D の HWND 機構 Pass（33 checks）を Dock PASS（画素表示を含む完了条件全体）に昇格させず、verdict-matrix で別 Case（GD-D1 機構層 Pass / GD-D2 画素層 Fail / GD-D3 Dock PASS 未達）に分離。各 report の「不成立/Partial」明記をそのまま転記。
- **Unsupported と NotRun の分離**: `ime`/`native_surface_slot`（macOS/Linux）/focus release/CLAP/AU/L1 cross-process は宣言済み `Unsupported`、未測定能力（a11y/DPI 動的/性能/Consumer Oracle）は `NotRun` として区別。POLICIES §3 の要求状態と実行結果の分離を verdict-matrix の requirement/result 列で固定。
- **ancestor SHA 証拠の扱い**: CI 証拠（`e93c438a`/`365c74ac`/`2613b238`）と先行 PoC 証拠は ancestor SHA として記録し、HEAD `7aab6302` の CI run 未実施を `NotRun`（Pending）として明記。最新 SHA の Pass に読み替えない（POLICIES §5）。
- **DComp 根拠の特定**: GPUI 0.2.2 `directx_renderer.rs:793` `CreateTargetForHwnd(topmost=true)` が child HWND を覆い、`WS_EX_NOREDIRECTIONBITMAP` で GDI redirection fallback がないことを `evidence/rf388-native-dock-windowing/report.md` と `docs/impl/exp/issue-532-dcomp-audit.md` で確認。#532 を open Blocker として併記。
- **Gate E の NotRun 固定**: `consumer-verify.yml` は workflow_dispatch 専用で対象 SHA run がなく、AL #166/SV #176 が製品 Owner。rf465 inventory の L1 互換記録を製品 Oracle の Pass に読み替えない。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-262-gpui-verdict.md` - 証拠照合・verdict 分類・文書規約適用の試行記録。Gate 別判定の根拠。

### Rejected

- なし（文書・証拠 slice のため実装アプローチの比較実験は不要。verdict 分類の試行は exp に記録）。

## Validation

- `evidence/rf262-gpui-verdict/verification.md` に実行コマンド・OS・toolchain・exit code・結果を記録。
- 証拠再実走（`test-ui-scenario-contract`/`check-ui-driver-boundary`/`test-ui-native-slot`/`test-ui-parity`/`test-ui-gpui`）を HEAD `7aab6302`・Windows で実行し全 Pass。
- 文書監査（`audit_docs`/`audit_impl_record`/`audit_experiment_log`/`doc_status`）で front matter・relation・index 整合を確認。
- `mise run fmt-check` + `prek run --files` で Markdown/リンクの軽量チェックを実行。
- 未実行項目（実 Native Dock・実 GUI・実 Plugin・Consumer 製品受入・性能測定・macOS/Linux 実機）は NotRun/Deferred/Unsupported と明記。

## Risks

- ancestor SHA の CI 証拠を最新 SHA の Pass と誤認 → design D1.2 の SHA 照合規則と HEAD run 未実施の NotRun 明記で対応。
- HWND 機構 Pass を Dock PASS に昇格 → verdict-matrix で機構/画素を別 Case に分離。
- `Unsupported`/`NotRun` 混同 → test-spec R-VERDICT で語彙を固定。
- Consumer Oracle 不在の Pass 化 → Gate E を NotRun に固定。
- verdict の決裁を Worker が行う誤り → spec/design/IR で Coordinator 権限を明記。
- 管理 index の手編集禁止 → `doc-maintenance`/`sync_impl_index` の手順に従う。

## Follow-ups

- #419: 「L1 のみ評価済み、L2/L3 非保証」を ADR に明記する決裁材料として `handoff-419.md` を引き渡し。
- #465: Legacy 撤去の独立判断境界（L1 機能/既存 Consumer 互換と未実 GUI 製品機能の破壊を分離）を `handoff-419.md` に記録。
- #532: Windows Native Dock 画素の根本 Blocker。対策決裁・実装が前提。
- #388/#389: Native Dock 残件（Dock layout/clip/tab・keyboard/IME routing・plugin 主導 resize 等）。
- Gate C 性能: 性能予算 Issue 確定後に定量測定。
- Gate E: `consumer-verify.yml` 対象 SHA run または AL #166/SV #176 受入。
- HEAD `7aab6302` の CI run: merge 後対象 SHA の CI 証拠（統合受入）。
