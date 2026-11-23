---
id: "TSPEC-rf874-rt-oracle-01"
type: "test-spec"
status: "approved"
title: "Issue #874 実 Window RT 非干渉 oracle テスト仕様"
created: "2026-11-22"
updated: "2026-11-22"
owners: []
relations:
  source:
    - "docs/specs/issue-874-rt-oracle.md"
    - "docs/designs/issue-874-rt-oracle.md"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-874-rt-oracle.md"
      - type: "doc"
        path: "evidence/rf874/"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-874-rt-oracle.md"
    - "docs/designs/issue-874-rt-oracle.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-874-rt-oracle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-874-rt-oracle.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-874-rt-oracle.md"
  verified-by: []
  references:
    - "evidence/rf874/"
    - "evidence/rf631/"
  defers: []
  deferred-by: []
---

# Issue #874 実 Window RT 非干渉 oracle テスト仕様

## Purpose

`real-session-rt-samethread`/`real-session-rt-fault`/
`real-session-rt-boundary` の 3 case が catalog + entries に
追加され、headless/非対話 host では fail-closed、interactive
run では spec R4 の verdict lattice に従う verdict が記録
されることを検証する。AC 分解: AC1（interactive run verdict）
は coordinator gate、AC2（観測仕様）は
`docs/specs/issue-874-rt-oracle.md` R4/R5/R6、AC3（CI）は
`mise run test` 系。

## Feature

- real lane の catalog 行数が 29 → 32 に増える（3 行追加）。
- `real_catalog_matches_entries` / `blocked_reports_fail_closed`
  / `--blocked-probe` / `--list --include-real` が新行を
  正しく追従する。
- `rt_samethread` body が `attempt_real_attach` を
  `drive_open_rt_samethread`/`observe_rt_samethread_lifecycle`
  で駆動し、spec R1 の 6 点で `rt_block` を interleave する。
- `rt_fault` body が `test_fault` descriptor を解決して spec R2
  の block 列を実行し、marker が armed 状態に追従する。

## Rules

1. 新 case は `REAL_CATALOG`/`real_entries` の末尾に追加し、
   既存 29 行の verdict・行順・`required`/`spec_extras` を
   変更しない。
2. `rt_samethread`/`rt_fault`/`rt_boundary` は実 session 内で
   のみ `EnvironmentKind::RealWindow` を claim する —
   `blocked_reports`/`blocked_probe` 経由では `NotRun`+
   `environment_blocked`。
3. `rt_block` の verdict 入力は `Ok(ProcessResult)` +
   `status == Continue` + `output == input`（または
   `rt_fault` の marker 追従）。`wall_ns` は observed 記録
   のみで verdict 非入力。
4. `rt_fault` は `rt_alloc` のみ使用 — `abort`/`exit`/`hang`
   は in-process lane 禁止。
5. `rt_boundary` は `Unsupported` のみ — `Pass`/`NotRun` に
   偽装しない。
6. `set_size` の plugin-declined `Unsupported` landing は
   verb 収束として記録し、DSP oracle の Pass を妨げない。
7. `test_fault` descriptor 欠落は `NotRun`+
   `environment_blocked`（`load_plugin` の `test_editor` 欠落
   `Fail` 先例と区別）。

## Examples

### E1: catalog + entries 一致性

`real_catalog_matches_entries` が 32 行で pairwise 一致する
ことを assert する — `case`/`scenario`/`required` が両表で
一致。

### E2: blocked-probe fail-closed

`plugin-dock-real-scenario --blocked-probe` が 32 行すべてを
`NotRun`+`observed.environment_blocked` で出力し、
`environment_kind == "real-window"` の行が 0 であることを
assert する。

### E3: `--list --include-real` 投影

`tests/console.rs` の `list_include_real_appends_real_lane_rows`
が `rows.len() == 41`（9 headless + 32 real）を assert し、
新 case 3 行が expected list に含まれることを確認する。

### E4: `rt_samethread` block 序列

interactive run で `steps` に `rt_pre_open`/`rt_post_open`/
`rt_attached_1`/`rt_attached_2`/`rt_post_close`/
`rt_processing_state` が順に記録され、全て `Continue` +
`pass_through` + `Ok` なら `Pass`。

### E5: `rt_fault` marker 追従

`rt_fault_armed`/`rt_fault_still_armed` のみ
`output[0][0] == 200.0`、それ以外は `output == input` —
marker が armed 状態に追従すれば `Pass`。

## Guarantees

- 3 行追加後も `blocked_reports`/`blocked_probe` は全行を
  fail-closed に写像する（`real_entries` を読むため自動
  追従）。
- `--list --include-real` は catalog metadata を emit する
  ため、`real-dock` 非コンパイル環境でも selector 名を解決
  できる。
- `rt_samethread`/`rt_fault` の `spec_extras` は
  `editor_attach_extras`（`plugin_artifact` 付き）で、
  `rt_boundary` は `real_window_extras`。

## Non-goals

- interactive Windows host での AC1 実走（coordinator gate）。
- `wall_ns` を verdict 入力にする timing oracle（observed
  記録のみ）。
- `RtRuntime` co-scheduled leg（不採用 — ambient 負荷で同一
  instance 観測にならない）。
- host/plugin/GUI driver 側の API 公開変更（`attach_endpoint`/
  `PluginHost.l0`/`create_sendable` — `scarecrow173/
  ruviform#872` deferred）。

## Risk

- shared_file 競合（rf#873）— 新 case は末尾追加ブロックで
  独立性を保つ。
- `test_fault` descriptor 欠落（cdylib 古い等）— `NotRun`+
  `environment_blocked` に写像。
- headless/非対話 host での AC1 実走不能 — blocked-probe で
  fail-closed を実証し、AC1 は正直 `NotRun`+
  `environment_blocked` で coordinator gate に残す。
