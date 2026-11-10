---
id: "PLAN-034Vj4WmuQbtJHrZ5oTcnm"
type: "plan"
status: "approved"
title: "Implement Issue #253 host-graph report consumer"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-253-hostgraph-reports.md"
    - "docs/designs/issue-253-hostgraph-reports.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-253-hostgraph-reports.md"
    - "docs/test-specs/issue-253-hostgraph-reports.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-253-hostgraph-reports.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-253-hostgraph-reports.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-hostgraph-reports.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [253]
---

# 実装計画: Issue #253 plugin→host report の host-graph consumer 配線

設計: `docs/designs/issue-253-hostgraph-reports.md`。
テスト仕様: `docs/test-specs/issue-253-hostgraph-reports.md`。
タスク: `docs/tasks/issue-253-hostgraph-reports.md`。

単一実装ストリーム — host 側 drain API 1 面とその検証のみ。
plugin-core / graph / adapter / wire の変更は無い。

## ファイルマップ

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-plugin-host/src/host.rs` | `ReportDrain` enum 追加（`BusReport` と同型・`#[non_exhaustive]`）、`InProcessHost::drain_output_reports` 追加（`Plain` 直接 / `Shared` は `shared_verb` 経由）、unit テスト（reporting fixture 含む） |
| `crates/ruviform-plugin-host/src/composite.rs` | `PluginHost::drain_output_reports` 追加（L0 委譲 / L1 audio・video → `Unavailable`）、unit テスト（L1 `test_fixture` スロット・video スロット・stale handle） |
| `crates/ruviform-plugin-host/src/lib.rs` | `ReportDrain` export、crate doc の RT contract 節に drain（non-RT）を追記 |
| `crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs` | report を出す `Send` fixture + `drain_output_reports` E2E（endpoint attach 中の Shared 経路）+ L1 `Unavailable` E2E |
| `docs/impl/ir/issue-253-hostgraph-reports.md` | 新規 IR（API 形状判断・capability 判断・検証・残 NotRun） |

`crates/ruviform-plugin-core`・`crates/ruviform-graph`・
`crates/ruviform-plugin-vst3`・`crates/ruviform-audio-plugin-bridge`・
`crates/ruviform-plugin-host/src/l1`・`wire.rs` は **無変更**
（plugin-core 契約は既存、graph は依存方向上不要、adapter emit は
PR #555 のまま、cap.rs は編集範囲外、L1 wire verb は別 slice）。

## 順序

1. `host.rs`: `ReportDrain` + `InProcessHost::drain_output_reports`
   + reporting fixture の unit テスト（red → green）。
2. `composite.rs`: `PluginHost::drain_output_reports` + unit テスト。
3. `lib.rs`: export + crate doc 追記。
4. `plugin_graph_e2e.rs`: fixture + E2E。
5. `docs/impl/ir`: IR 作成。
6. 検証: `git fetch origin main && git merge origin/main` の後
   `mise run fmt-check check lint test test-rt lint-l1-real-plugin
   test-l1-real-plugin`。

## リスク

- `shared_verb` の closure が `out: &mut Vec<Event>` を capture
  する — `FnOnce` なので問題ないが、borrow が `instance` の
  disjoint field（`slot`/`lifecycle`）に収まることを
  `buses` と同じ形で確認する。
- `Event`/`EventKind`/`ParamEvent`/`ParamValue`/`ParamId` の
  import — host.rs tests は `ruviform_plugin_core` から、
  e2e は `ruviform_media_core::MidiEvent` 系も必要になる場合が
  ある（`ParamValue` report のみで足りるなら media-core 追加は
  不要 — dev-dep 確認）。
- L1 `test_fixture` スロットで `drain_output_reports` が
  `Unavailable` を返す — plugin に到達しないので fixture の
  内容に依らない。

## 検証

- [ ] `cargo test -p ruviform-plugin-host`（補助証拠 — canonical は mise）
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` /
  `lint-l1-real-plugin` / `test-l1-real-plugin`
