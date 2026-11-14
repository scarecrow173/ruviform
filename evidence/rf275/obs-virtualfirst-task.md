# 内部 Task — Issue #275 Virtual-first Owner/受入整合（Deferred Owner/Trigger 再指向）

- 文書種別: 内部 task（`obs-virtualfirst-plan.md` の作業分解相当）
- 日付: 2026-11-15
- 対象 Revision: `d5aa1b4b`（origin/main HEAD、clean tree）

## Tasks

- [x] T1: `gh issue view` で RF #627/#628/#629/#630/#631/
      #281/#294/#313/#314・AL #127・SV #28 の状態を照合し、
      POLICIES §2.1・Issue #275 本文・#628〜631 設計正本題名と
      突合する。
- [x] T2: 内部 doc chain（spec/design/plan/task）を
      `evidence/rf275/` に作成する。
- [ ] T3: canonical 設計正本を更新する — front-matter
      `updated`、Deferred Owner/Trigger 表（実 OS GUI・実
      Device 行）と dated note、Consumer Gate 割当の Windows
      L2/L3 行と dated note。
- [ ] T4: `obs-acceptance-ledger.md` に §7 追記専用
      セクションを追加する（既存 §1〜§6 は不変）。
- [ ] T5: `obs-virtualfirst-evidence.md` に verification
      記録（コマンド・exit code・照合結果・差分要約）を作成
      する。
- [ ] T6: `mise run fmt-check`・`mise exec -- prek run
      --all-files`・`git diff` 範囲確認を実施し、
      review-protocol で blocker を解消する。
- [ ] T7: `git fetch origin main && git merge origin/main`
      → commit → push → PR 作成（`Refs #275`）→ worker_done
      報告。

## 判断メモ

- doc chain を canonical `docs/` ではなく `evidence/rf275/`
  に置くのは、編集範囲が `evidence/rf275/` と指定文書に
  限定されているため。内部文書として spec→design→plan→
  task→evidence（ir/exp 相当）の連鎖を維持する。
- `route_graph.js`（`.agents/skills/doc-driven-dev-graph/
  scripts/route_graph.js`）は `focus-required` と本 slice
  無関係の既存 broken-relation（issue-253/313/410 等）で
  blocked を返した（2026-11-15 実測）。canonical docs を
  増やさない本 slice では router の対象 artifact が新規に
  存在しないため、フローは内部文書チェーンで代替した
  （前 slice `obs-acceptance-audit-task.md` と同一判断）。
- ledger §3・§4.2 の `RF #293/#313/#314`・「real-window
  lanes」記述は 2026-11-14 監査時点の記録として保持し、
  本 slice では改変しない。現行 Owner は ledger §7 と
  設計正本の dated note が正本。
