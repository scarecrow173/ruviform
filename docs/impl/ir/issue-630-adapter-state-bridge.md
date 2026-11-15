---
id: "IMPL-034Y14g9JXH2bWmCtJyAoV"
type: "impl"
status: "completed"
title: "Issue #630 adapter-state-bridge slice — 実装記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-630-adapter-state-bridge.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-630-adapter-state-bridge.jsonl"
    rejected: []
---

# Issue #630 adapter-state-bridge slice — 実装記録

## Summary

両 adapter host bridge に標準 ABI の state 境界を実装した。
VST3 は `IBStream` の `ComWrapper` 実装 `BridgeStream` を新設し、
`IComponent`/`IEditController` の state 呼び出し列を
`[u32 LE comp_len][comp][u32 LE ctrl_len][ctrl]` framing で中立
`Vec<u8>` blob に橋渡しする。CLAP は `clap.ext.state` の
`PluginState::save`/`load` を `PluginMainThreadHandle` 経由で駆動し、
plugin 定義の生 byte stream をそのまま中継する。あわせて fixture
`GainController` の不正直な state stub（`kResultOk` 空 no-op）を
`kNotImplemented` に修正した。

## Context

`docs/tasks/issue-630-adapter-state-bridge.md`（status: done）に
基づく。上流は `docs/specs/`・`docs/designs/`・`docs/plans/`・
`docs/test-specs/` の同 slug 文書。検証手順は
`evidence/rf630/adapter-state-bridge-verification.md` に記録。

## Implementation

### Changes

- `crates/ruviform-plugin-vst3/src/host.rs`
  - `BridgeStream`（`IBStreamTrait` + `Class`）新設:
    `RefCell<BridgeStreamState>{data: Vec<u8>, pos: u64}`。
    `read`/`write` は部分転送を実数で報告、null buffer・負 size は
    `kInvalidArgument`、`RefCell` borrow 競合・負/overflow seek は
    `kResultFalse`。前方 seek は許可し write の gap は 0 埋め、
    成長は `WIRE_MAX_STATE_BYTES` で cap。
  - `frame_state`/`unframe_state`/`FRAME_HEADER_BYTES(8)` と
    `save_component_state`/`restore_component_state` free function。
  - `Vst3Plugin::save_state`/`restore_state`: phase gate
    （`phase < Initialized` → `PluginFailed`）、save は
    `component.getState` + distributed のみ `controller.getState`、
    restore は VST3 spec 順
    `component.setState` → `component.getState` →
    `controller.setComponentState`（正規化 stream を seek(0)）→
    `controller.setState`。single-component は controller 呼び出し
    を省略。`kNotImplemented` → `Unsupported`、その他 →
    `PluginFailed`。
  - module doc の defined-boundaries を新 contract に更新。
  - test: `TestStatefulComponent`/`TestStatefulController`（実
    vtable dispatch 経由の stateful COM double、共有 call log で
    呼び出し順を pin）+ `BridgeStream` 直接試験 + fixture の
    gate/decline 試験。
- `crates/ruviform-plugin-vst3/src/controller.rs`
  - `GainController::setState`/`getState` を `kResultOk` 空 no-op
    から `kNotImplemented` に修正（side finding — 成功偽装の
    解消）。`state_stream_reports_not_implemented` test 追加。
- `crates/ruviform-plugin-clap/Cargo.toml`
  - `host` feature に `"clack-extensions/state"` 追加
    （`Cargo.lock` 差分なし）。
- `crates/ruviform-plugin-clap/src/host.rs`
  - `PluginState` import、module doc 更新。
  - `ClapPlugin::save_state`/`restore_state`: `instance=None` →
    `PluginFailed`、extension 無し → `Unsupported`、`StateError` →
    `PluginFailed`。blob は無 framing で中継。
  - test `state_verbs_gate_and_fixture_declines` に書き換え
    （pre-init `PluginFailed` + fixture extension 無し
    `Unsupported`）。

### Decisions During Implementation

- **foreign blob 規則（design に記録済みの調整）**: framing の
  最小生成長は 8 byte。8 byte 未満の blob は本 adapter の
  serialization ではあり得ないため foreign state として plugin
  の `setState` へそのまま渡す — 厳密 unframe 先行では
  `RestoreState{[0xAA,0xBB]}` が plugin 呼び出しへ到達せず
  `PluginFailed` となり、lease 外 L1 e2e の `Unsupported` pin を
  破るため。8 byte 以上の corrupt frame は `PluginFailed`。
- **`BridgeStream` の write 上限**: plugin からの任意長書き込みを
  `Vec` で受け付けるが `WIRE_MAX_STATE_BYTES` を超える分は
  short write + `kResultFalse` で正直に報告 — wire 側 cap に
  一元化し adapter 独自 cap は設けない（design 通り）。
- **テスト double の構成**: distributed 用に
  `IComponent`/`IEditController` を別 object で実装し、
  single-component 分岐は `single_component=true` flag で
  controller 呼び出しが走らないことを call log で検証
  （component==controller の同一 object 性は init 経路の責務で
  本 slice の対象外）。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-630-adapter-state-bridge.jsonl`

### Rejected

- なし（foreign blob 規則は experiment log seq 4 に decision として
  記録 — 採用案）

## Validation

Windows OS-L1、`cargo`/`mise` toolchain（`mise.toml` 管理）。
詳細は `evidence/rf630/adapter-state-bridge-verification.md`。

- `mise run fmt-check` — exit 0
- `mise run check`/`lint`/`test ruviform-plugin-vst3` — exit 0
- `mise run check`/`lint`/`test ruviform-plugin-clap` — exit 0
- `mise run check-l1-real-plugin` / `lint-l1-real-plugin` /
  `test-l1-real-plugin` — exit 0（adapter e2e
  `real_adapter_l1_full_series` 3+3 pass、stateless fixture の
  `SaveState`/`RestoreState` は wire 越しに `Err(Unsupported)` を
  維持）
- 補助証拠: `cargo test -p ruviform-plugin-vst3 --features host
  --lib` 84 pass、`cargo test -p ruviform-plugin-clap --features
  host --lib` 67 pass
- `mise run test-rt` — exit 0（workspace `rt_` 全緑）
- `mise exec -- prek run --all-files` — exit 0

## Risks

- in-process COM double の成功は real-ABI oracle ではない —
  stateful round-trip の実証は `test_stateful` fixture slice まで
  NotRun。
- foreign blob 規則は spec 文言（「malformed blob →
  PluginFailed」）に対する確定済みの調整 — design/spec/test-spec
  に記録し PR 本文にも明記。

## Follow-ups

- `test_stateful` fixture slice（real state round-trip oracle）
- host `mark_dirty`/dirty 通知経路（CLAP `HostState`・VST3
  `IComponentHandler` 系）— 別 slice 判断
- 外部 SDK validator・非 Windows OS・Preset/preset-list — 後続
