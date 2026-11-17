---
id: "TASK-732clapLifecycleAudit05"
type: "task"
status: "done"
title: "Issue #732 production CLAP adapter lifecycle 監査 — task"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-732-clap-lifecycle-audit.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-732-clap-lifecycle-audit.md"
  implements:
    - "docs/plans/issue-732-clap-lifecycle-audit.md"
  implemented-by:
    - "docs/impl/ir/issue-732-clap-lifecycle-audit.md"
  depends-on:
    - "docs/plans/issue-732-clap-lifecycle-audit.md"
    - "docs/discovery/issue-732-clap-lifecycle-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #732 task

- [x] T1: clack 0.2.0 / clap-sys 0.5.0 binding の生存期間契約を確認
      （init 1 回制限・activate/deactivate の processor cell・
      destroy 時の自動 deactivate・不正状態 callback の拒否）
- [x] T2: `plugin.rs` lifecycle 経路の監査 — initialize/destroy/
      activate_inner/activate/process/deactivate/reset/
      start_processing/stop_processing/declare_extensions を CLAP
      契約・中立遷移表と照合
- [x] T3: `adapt.rs` の写像層監査 — AtomicLifecycle・
      map_process_result・port/event/transport 写像
- [x] T4: bus/port info 自己整合・latency/tail 報告経路の監査
      （audio-ports extension 宣言 vs process 実振る舞い）
- [x] T5: `host.rs` の報告経路対称性確認（read-only）
- [x] T6: 欠陥判定 — rf#722 3 パターン照合。terminalization/bus-info
      は欠陥なし、受理層で 2 欠陥（param module `/`・fractional
      sample rate reject）を発見 → T6b で修正
- [x] T6b: 欠陥修正（TDD red-green）— `plugin.rs` `module: b"/"`→`b""`
      （`gain_param_info()` 集約 + unit test）・`adapt.rs`
      `to_process_config` fractional reject → `rate.round()` 受理
      （fixture 同一写像）+ unit test 更新
- [x] T7: `run.mjs` production CLAP leg 拡張（`--clap-production`・
      `--clap-production-dll`・build list・artifacts・fail-closed 維持）
      + `run.test.mjs` contract test 追加
- [x] T8: `evidence/rf732/` に監査報告 + validator 実走/NotRun 証拠
- [x] T9: docs chain 整備 + mise 正規タスク検証 + prek
