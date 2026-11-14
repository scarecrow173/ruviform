# 内部 Task — Issue #275 OBS-01〜11 受入照合台帳と Consumer Gate 割当

- 文書種別: 内部 task（`obs-acceptance-audit-plan.md` の作業分解相当）
- 日付: 2026-11-14
- 対象 Revision（監査基準）: `20d8e7c23bc365e26060a4ef82d463bb7eed38bc`

## Tasks

- [x] T1: Issue #275/#281・AL #127・SV #28 の現行受入条件・
      gate 構成・Case 組を `gh issue view` で取得し、本
      doc chain（spec/design/plan/task）を `evidence/rf275/`
      に作成する。
- [x] T2: PR #521/#547 の正本・契約 Fixture（`tests/
      rt_boundary.rs`・collector lifecycle test）と現行
      コード（module/feature/test target）を突合する。
- [x] T3: CI 証拠を収集する — 直近 Full CI run
      （43439434737 @ `365c74ac`）の job/log から
      observability の test 件数、consumer-verify run
      （43347745016 @ `3692c6fa`）の tuple/lane 結果、
      `git diff` による SHA 間 crate 同一性。
- [x] T4: ローカル補助証拠を取得する —
      `mise run test ruviform-observability --all-features`
      と `mise run test ruviform-plugin-host`（Windows）。
- [ ] T5: `evidence/rf275/obs-acceptance-ledger.md` を作成
      する — OBS-01〜11 照合表・Fixture×lane 証拠行列・
      Consumer Gate 割当表・SHA 引継規則・NotRun 棚卸し。
- [ ] T6: 統合設計正本へ台帳参照 + Gate 割当要約の追記型
      セクションを追加する（既存記述は不変）。
- [ ] T7: `mise run fmt-check` と `git diff` 範囲確認を実施
      し、review-protocol レビューで blocker を解消する。
- [ ] T8: `git fetch origin main && git merge origin/main`
      → commit → push → PR 作成 → worker_done 報告。

## 判断メモ

- doc chain を canonical `docs/` ではなく `evidence/rf275/`
  に置くのは、編集範囲が `evidence/rf275/` と指定 3 文書に
  限定されているため。内部文書として spec→design→plan→
  task の連鎖を維持する。
- `route_graph.js` は `focus-required` と本 slice 無関係の
  既存 broken-relation で blocked を返した。canonical
  docs を増やさない本 slice では router の対象 artifact が
  存在しないため、フローは内部文書チェーンで代替した
  （PR 本文に記録）。
