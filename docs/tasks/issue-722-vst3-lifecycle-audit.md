---
id: "TASK-722vst3LifecycleAudit05"
type: "task"
status: "done"
title: "Issue #722 production VST3 adapter lifecycle 監査 — task"
created: "2026-11-19"
updated: "2026-11-19"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-722-vst3-lifecycle-audit.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-722-vst3-lifecycle-audit.md"
  implements:
    - "docs/plans/issue-722-vst3-lifecycle-audit.md"
  implemented-by:
    - "docs/impl/ir/issue-722-vst3-lifecycle-audit.md"
  depends-on:
    - "docs/plans/issue-722-vst3-lifecycle-audit.md"
    - "docs/discovery/issue-722-vst3-lifecycle-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #722 task

- [x] T1: production `component.rs` の lifecycle 監査 — terminate /
  setupProcessing / bus info / controller を SDK 契約・fixture 正本と
  照合し結論を discovery に記録
- [x] T2: `terminate` 非終端化 — `Destroy` 検証後に
  `Lifecycle::new()` + `config = None` で `Created` へリセット
- [x] T3: `setupProcessing` 受理集合を `Initialized | Activated` に拡大
- [x] T4: lifecycle doc table / doc comment の同期
- [x] T5: unit test 追加 — `terminate_allows_reinitialize`・
  `setup_processing_accepted_while_initialized`（TDD red→green 確認済み）
- [x] T6: `run.mjs` に `--vst3-package` と bundle 名導出を追加
  （fixture 既定動作は不変）
- [x] T7: 正規検証 — fmt-check / check / lint / test /
  test-l1-real-plugin / node --test / ABI validator lane
- [x] T8: `evidence/rf722/` に監査報告と validator 実走証拠を記録
- [x] T9: review-protocol レビュー・blocker 解消
- [x] T10: Conventional Commits でコミット・push・PR 作成
