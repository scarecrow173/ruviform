---
id: "DISC-rf874-rt-oracle-01"
type: "discovery"
status: "resolved"
title: "Issue #874 実 Window RT 非干渉 oracle 調査記録"
created: "2026-11-22"
updated: "2026-11-22"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/874"
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-874-rt-oracle.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks:
    - "docs/specs/issue-874-rt-oracle.md"
    - "docs/designs/issue-874-rt-oracle.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-874-rt-oracle.md"
    - "docs/designs/issue-874-rt-oracle.md"
    - "docs/test-specs/issue-874-rt-oracle.md"
    - "docs/designs/issue-389-real-session-lifecycle.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/874"
  derived-by:
    - "docs/specs/issue-874-rt-oracle.md"
  verifies: []
  verified-by: []
  references:
    - "evidence/rf631/"
    - "evidence/rf874/"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/872"
  deferred-by: []
---

# Issue #874 実 Window RT 非干渉 oracle 調査記録

## Summary

Issue `scarecrow173/ruviform#874` — `scarecrow173/ruviform#389`
の AC「実 Window での RT 非干渉」Required 脚に対応する oracle を
`rf631.plugin-dock-real` lane（現 29 case）へ追加する slice の
調査記録。headless での DSP 非干渉は
`crates/ruviform-plugin-host/tests/editor_session.rs` の
`l0_session_*_leaves_dsp_processing_untouched` 群で検証済みだが、
`evidence/rf631/plugin-dock-real-scenario.jsonl` には RT 観測
フィールドを持つ case 行が 0 件で、実 Window 経路の実測照合が
できない。本記録は oracle leg の実現可能性を実コードで確認し、
採否判断の入力を固定する。確度: 高（全て現行実装の API 契約に
基づく）。

## 現状棚卸し

- real lane は `RealOsGuiTest::run` 内で `real_entries`（
  `crates/ruviform-plugin-dock-test/src/real_dock.rs`）の 29 case
  を逐次実行し、headless/非対話 host では `blocked_reports` が
  全行 `NotRun`+`observed.environment_blocked` に fail-closed
  する。session lifecycle の boundary は
  `real-session-crash`/`real-session-hang` が「実 session 内で
  `Unsupported`+detail+evidence tag を記録する」先例。
- `load_plugin` は test-plugin cdylib を解決・hash・load し、
  `test_editor` descriptor のみを選択する。`test_fault` は同一
  cdylib・同一 `Vst3Library` の別 descriptor で、`descriptors()`
  から `name == "test_fault"` で解決できる。`register_factories`
  は全 factory を登録するため、同一 `InProcessHost` 上で両
  descriptor の instance を立てられる。
- 既存 case 群は `host.execute(HostCommand::..)` の制御面のみを
  駆動し、`InProcessHost::process`（DSP 経路）は real lane では
  一度も呼ばれていない。これが evidence の RT 観測欠落の直接
  原因。

## leg 実現可能性の確認

### (a) same-thread leg — 実現可能（Pass 可能面）

- `InProcessHost::process`（
  `crates/ruviform-plugin-host/src/host.rs:366`）は `pub` で、
  Plain slot の実 VST3 instance を直接駆動する。RT-suitable
  契約（nothing allocates/blocks）。`catch_unwind` なし —
  fault の発散系は直接呼び出し側へ伝播するため、本 lane では
  `rt_alloc` 等の非発散 mode のみ使用する。
- lifecycle 規約（`ruviform-plugin-core/src/lifecycle.rs` の
  transition table）: `process` は `Processing` 状態からのみ
  合法（`Activated` では defined rejection）。したがって
  `activate_test_gain`（Scan→Instantiate→Initialize→Activate）
  の後に `HostCommand::StartProcessing` が必要。`Destroy` は
  `Created`/`Initialized`/`Failed` からのみ合法なので、
  teardown は `StopProcessing`→`Deactivate`→`Destroy` の
  defined 順序が必要。
- scenario caller thread 上で session verb（`session.open`
  /`set_size`/`close`、いずれも `InProcessEditorOps` 経由の
  実 verb）と `host.process` block を逐次 interleave できる。
  headless では `l0_session_open_close_leaves_dsp_processing_
  untouched` が同構成（Processing 中の open/set_visibility/
  close）を実証済み。
- buffer/event 型は全て既存 dep で足りる: `HostInput`/
  `HostOutput`/`EventList`/`ParamEvent`/`ParamValue`/
  `ProcessStatus`/`LifecycleState`（`ruviform-plugin-core`）、
  `PlanarBufferView`/`PlanarBufferViewMut`（`ruviform-audio-core`）。
- `test_editor` は parameterless pass-through processor
  （`crates/ruviform-plugin-test-vst3/src/fault.rs` の
  `pass_through` を共有）— 出力==入力の bit 等価が DSP
  正しさの oracle になる。

### (b) negative-control leg — 実現可能

- `test_fault` の `Fault` param は `ParamId(0)`・plain `[0,8)`、
  `rt_alloc` は plain `[5,6)` → normalized `0.625` 付近。block 内
  `ParamValue` event で arm すると、arm した block の
  `process()` 末尾で 4 KiB heap alloc を実行し `output[0][0]` に
  `200.0`（`f32::from(0xC8)`）marker を書く。armed 値は block を
  跨いで持続し、plain `0.0` の event で disarm できる。
- `abort`/`exit`/`hang` は host process を kill/wedge するため
  in-process lane では使用禁止（fixture の規約どおり）。
  `rt_alloc` は `ProcessStatus::Continue` を返す非発散 fault で
  安全に観測できる。
- 観測設計: baseline（marker なし）→ armed block（marker あり）→
  armed 持続 block（marker 持続）→ disarm block（marker 消滅）。
  marker の有無が armed 状態に追従することを示せば、観測経路が
  既知の RT 違反を捕捉し得ることが証明できる。

### (c) cross-thread 同一 instance 観測 — 現行 API では不可

- `create_sendable` は VST3/CLAP とも `None`（`!Send`）を返す
  （`crates/ruviform-plugin-vst3/src/host.rs`、`crates/
  ruviform-plugin-clap/src/host.rs`）ため `Instantiate` は
  Plain slot を選択する。
- Plain slot への `prepare_endpoint`/`attach_endpoint` は
  定義済み `EndpointUnavailable`、`InProcessHost::attach_endpoint`
  は `pub(crate)`、`PluginHost.l0` は非公開。
- したがって「RT-class thread から同一 instance を pump し
  ながら UI thread が session verb を駆動する」観測は Pass 化
  できない。`real-session-crash`/`real-session-hang` と同型の
  正直 `Unsupported` boundary leg として記録する — Issue の
  「環境的に不可能な場合の `unsupported` 扱い」の適用面。
- deferred 参照: `scarecrow173/ruviform#872`（L1
  out-of-process host — endpoint が Send 可能な channel leg）
  または GUI-capable `create_sendable` 契約の将来 Issue。

### (d) `RtRuntime` co-scheduled workload leg — 不採用

- `ruviform-audio-test` の `RtRuntime`（既存 dep）を helper
  thread で pump する案は、同一 instance の観測ではなく ambient
  な並行負荷でしかない。OS scheduling 経由の干渉は理論上あり
  得るが、verdict の根拠になる観測（status/lifecycle/出力）は
  負荷の有無で変わらず、flake 要因だけが増える。GPUI event
  loop timing への影響も未検証。採否は design doc で不採用に
  確定。

## case 分割の判断材料

verdict 単位の正直性に合わせ、crash/hang が別行 `Unsupported`
で残る先例に倣い 3 行構成とする:

- `real-session-rt-samethread` — (a) leg。Pass/Fail/NotRun/
  Unsupported 可能な実測 oracle。
- `real-session-rt-fault` — (b) leg。marker 追従で観測経路を
  検証する negative control。
- `real-session-rt-boundary` — (c) leg。cross-thread boundary
  の正直 `Unsupported` 記録。

## 未解決・範囲外

- GPUI/Win32 型・GUI driver 本体の RT 経路・`mise.toml` lane
  構造・新規 CI workflow は対象外（oracle は観測のみ）。
- interactive Windows session の有無は dispatch host 依存 —
  AC1 の interactive 実走は coordinator gate。
- `attach_endpoint` 公開・`PluginHost.l0` 公開・`create_sendable`
  契約変更は本 slice 対象外。
