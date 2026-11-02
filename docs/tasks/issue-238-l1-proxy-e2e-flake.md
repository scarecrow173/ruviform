---
id: "TASK-M5TTFmL3gjHvlBOb7sPDjF"
type: "task"
status: "done"
title: "Issue #238 l1_proxy_e2e 壁時計 sleep 同期を barrier/有界 poll へ置換"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/238"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-238-l1-proxy-e2e-flake.md"
  implemented-by:
    - "docs/impl/ir/issue-238-l1-proxy-e2e-flake-implementation.md"
  depends-on:
    - "docs/plans/issue-238-l1-proxy-e2e-flake.md"
    - "docs/specs/issue-238-l1-proxy-e2e-flake.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-238-l1-proxy-e2e-sync.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-238-l1-proxy-e2e-flake.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #238 task — l1_proxy_e2e 壁時計 sleep 同期を barrier/有界 poll へ置換

## Objective

`docs/plans/issue-238-l1-proxy-e2e-flake.md` の sleep 監査表を実施する。`l1_proxy_e2e.rs` のみ変更し、assertion・プロトコル coverage を維持する。

## Checklist

- [ ] 内部 `pipeline_barrier(&mut L1Instance)` helper 追加（`SaveState` ×2、`Err`→counter dump panic）
- [ ] `composite_e2e::pipeline_barrier(&mut PluginHost, PluginHandle)` helper 追加
- [ ] 配送 paced call sites 9 箇所を call 毎 barrier へ
- [ ] `audio_counters_*` 3 テスト: reject 系は call 毎 barrier、hung 系は sleep 除去（構造的上界コメント付き）
- [ ] heir 3 テストの 700 ms 除去（`freeze()` の `drain_cv` wait に同期）
- [ ] `REPLY_LANDING_TIMEOUT` 等新規 bounded deadline を明示
- [ ] `mise run fmt-check` `check` `lint` `test` green
- [ ] `cargo test -p ruviform-plugin-host --test l1_proxy_e2e` 反復実行証跡を IR に記録
