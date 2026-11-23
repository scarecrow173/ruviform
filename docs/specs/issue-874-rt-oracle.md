---
id: "SPEC-rf874-rt-oracle-01"
type: "spec"
status: "approved"
title: "Issue #874 実 Window RT 非干渉 oracle（real-dock lane catalog case 追加）"
created: "2026-11-22"
updated: "2026-11-22"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/874"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "POLICIES.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-874-rt-oracle.md"
      - type: "doc"
        path: "docs/designs/issue-874-rt-oracle.md"
      - type: "doc"
        path: "docs/test-specs/issue-874-rt-oracle.md"
      - type: "doc"
        path: "docs/discovery/issue-874-rt-oracle.md"
      - type: "doc"
        path: "evidence/rf874/"
    modified:
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/874"
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-874-rt-oracle.md"
    - "docs/designs/issue-389-real-session-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-874-rt-oracle.md"
    - "docs/test-specs/issue-874-rt-oracle.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/874"
  derived-by:
    - "docs/designs/issue-874-rt-oracle.md"
    - "docs/test-specs/issue-874-rt-oracle.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-874-rt-oracle.md"
  references:
    - "evidence/rf874/"
    - "evidence/rf631/"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/872"
  deferred-by: []
---

# Issue #874 実 Window RT 非干渉 oracle

## What Should Be Built

`rf631.plugin-dock-real` lane（現 29 case）に、実 Window session
経路での RT 非干渉を観測する oracle を 3 case として追加する:

- `real-session-rt-samethread` — scenario caller thread 上で
  `InProcessHost::process` block を実 session verb の前後に
  interleave し、DSP の正しさ・継続性・lifecycle 維持を実測する
  same-thread leg。
- `real-session-rt-fault` — `test_fault` の `rt_alloc` fault を
  arm/disarm し、観測経路が既知の RT 違反（process 内 heap
  alloc → `output[0][0] = 200.0` marker）を捕捉し得ることを
  証明する negative-control leg。
- `real-session-rt-boundary` — cross-thread 同一 instance pump
  が現行 API では実現不可能である理由を、実 session 内で正直
  `Unsupported` + detail + evidence tag として記録する
  boundary leg（`real-session-crash`/`real-session-hang` 先例）。

## Why Now

`scarecrow173/ruviform#389` の AC「実 Window での RT 非干渉」
Required 脚が未消化。headless の DSP 非干渉は
`editor_session` tests で検証済みだが、
`evidence/rf631/plugin-dock-real-scenario.jsonl` には RT 観測
フィールドを持つ case 行が 0 件であり、実 Window 経路の実測
照合ができない。

## Users and Value

- Issue #389 系の受入判定者（Coordinator・統合 CI）— RT 非干渉
  の Required 脚に対して、fake/headless でない実測 verdict を
  得る。
- real lane の読者 — `Unsupported` boundary の理由と deferred
  先が verdict detail に明記され、未実装面が隠蔽されない。

## Scope

### 対象

- `crates/ruviform-plugin-dock-test` 内の real lane catalog /
  entries / case body のみ。doc chain（discovery/spec/design/
  test-spec）・README・evidence。
- oracle は観測のみ: GPUI/Win32 型・GUI driver 本体の RT 経路・
  `mise.toml` lane 構造・新規 CI workflow は触らない。

### 対象外（deferred・明示）

- cross-thread 同一 instance pump の Pass 化、`attach_endpoint`
  公開、`PluginHost.l0` 公開、`create_sendable` 契約変更 —
  `scarecrow173/ruviform#872`（L1 out-of-process host）または
  GUI-capable `create_sendable` 契約の将来 Issue。
- `abort`/`exit`/`hang` fault mode の real 化（in-process lane
  では process kill/wedge — 既存 boundary 先例どおり）。
- headless RT 検証（`test-rt` 既存 lane で網羅済み）、
  `AttachMode::Floating`、Issue close。

## Requirements

### R1: same-thread leg（`real-session-rt-samethread`）

`test_editor` の実 instance を `Processing` まで駆動し
（`activate_test_gain` + `HostCommand::StartProcessing`）、
`attempt_real_attach` 経由で実 session の verb を実行し、
その前後に `host.process` block を interleave する。interleave
点は最低でも次の 6 点:

| label | 位置 | verb との関係 |
| --- | --- | --- |
| `rt_pre_open` | `drive_open` 冒頭（`session.open` 前） | baseline |
| `rt_post_open` | `session.open(OpenMode::Docked)` 直後 | open を跨ぐ |
| `rt_attached_1` | `observe_attached_children` の後 | child 観測後 |
| `rt_attached_2` | `session.set_size(800×600)` 直後 | resize を跨ぐ |
| `rt_post_close` | `conclude_editor_session`（close+detached+converge）直後 | close を跨ぐ |
| `rt_processing_state` | `host.lifecycle_state(handle)` 読み取り | `Some(LifecycleState::Processing)` を期待 |

block 形状: mono・8 frame・input 全 sample `0.5f32`・
`EventList::empty`（fixture の pass-through 契約で `output ==
input` が成立する形状）。`HostInput`/`HostOutput` は
`PlanarBufferView`/`PlanarBufferViewMut` を包む erased view で、
事前確保した stack 配列を包む — block 内で alloc しない。

### R2: negative-control leg（`real-session-rt-fault`）

同一 cdylib の `descriptors()` から `name == "test_fault"` を
解決して instance を立て、`Processing` まで駆動してから次の
block 列を実行する:

| label | events | 期待 |
| --- | --- | --- |
| `rt_fault_baseline` | empty | `Continue`・`output == input`・`output[0][0] != 200.0` |
| `rt_fault_armed` | `ParamValue(ParamId(0), 0.625)` | `Continue`・`output[0][0] == 200.0`・`output[0][1..] == input[0][1..]` |
| `rt_fault_still_armed` | empty | `Continue`・`output[0][0] == 200.0`（armed 持続） |
| `rt_fault_disarmed` | `ParamValue(ParamId(0), 0.0)` | `Continue`・`output == input`（marker 消滅） |
| `rt_fault_post_disarm` | empty | `Continue`・`output == input`（marker 消滅の持続） |

normalized `0.625` は plain `5.0` → `rt_alloc`（plain `[5,6)`
帯）を指す。marker 値 `200.0`（`f32::from(0xC8)`）は fixture
契約の marker で、armed block の `process()` 末尾で
`output[0][0]` に書かれる。

### R3: boundary leg（`real-session-rt-boundary`）

実 session 内で実行し、`Unsupported` + detail + evidence tag
を記録する。detail は cross-thread pump 不可の各根拠
（`create_sendable`→`None`、Plain slot、`attach_endpoint` が
`pub(crate)`、`PluginHost.l0` 非公開）と deferred 先
（`scarecrow173/ruviform#872`）を明記する。`Pass`/`NotRun` に
偽装しない。

### R4: verdict lattice（観測→verdict 写像）

- `Pass` — spec で定義した全観測契約の成立:
  - samethread leg: 全 interleaved block が
    `ProcessStatus::Continue`、全 block で `output == input`
    （`rt_attached_2`/`rt_post_close` も同契約）、全 block
    で `Err` なし（`Ok(ProcessResult)` 以外は defined
    rejection として記録し verdict に含める）、
    `rt_processing_state` が `Some(LifecycleState::
    Processing)`、`session.open`/`set_size`/`close` が defined
    結果に収束（`OpenOutcome::Opened`/`set_size` defined
    landing/`conclude_editor_session` の close/detached/
    converged 契約）、`observe_attached_children` の child
    HWND 契約成立、`session.events` が
    `[Attached{Docked}, Closed]` 収束、teardown の
    `StopProcessing`/`Deactivate`/`Destroy` が defined 結果に
    収束。
  - fault leg: 全 block が `Continue`・`Err` なし・marker
    追従が R2 の表どおり（armed 区間のみ `output[0][0] ==
    200.0`、それ以外は `output == input`）・teardown が
    defined 結果に収束。
  - boundary leg: `Unsupported` のみ（`Pass` 化しない）。
- `Fail` — 契約不一致（非 `Continue`・`output` 不一致・
  `Err`/`defined rejection`・lifecycle が
  `Some(Processing)` 以外・verb の undefined 収束・観測証拠
  の欠落・marker が期待区間と不一致）。
- `NotRun` + `observed.environment_blocked` — session 不成立
  （`blocked_reports` 経由の fail-closed 写像が自動適用）。
  test-plugin artifact の prerequisite 欠落は既存 mapping
  どおり `NotRun`。
- `NotRun`（`blocked` 以外）— `rt_fault` で `descriptors()`
  から `test_fault` が解決できない場合（cdylib が古い等
  の環境要因）— 既存 `load_plugin` の `test_editor` 欠落を
  `Fail` とする先例と区別し、本 case は descriptor 欠落を
  `NotRun`+`environment_blocked`（`environment_blocked:
  test_fault descriptor missing`）に写像する。
- `Unsupported` — `rt_boundary` leg・`set_size`/`reattach`/
  `set_visibility`/`send_key`/`set_focus` の plugin-declined
  refusal（既存 mapping どおり）。`rt_samethread` では
  `set_size` が defined `Unsupported` landing でも DSP oracle
  の Pass を妨げない（verb 収束として記録）。

### R5: 観測対象と測定方法

verdict 入力（load-bearing）は次のみ:

1. `ProcessResult.status`（`Continue`/`Sleep`/`Error`）。
2. `ProcessResult` が `Ok`/`Err`（`Err(HostError)` = defined
   rejection）。
3. `output` sample 値（`output == input` の bit 等価、または
   `rt_fault` の marker 追従）。
4. `host.lifecycle_state(handle)` が `Some(LifecycleState::
   Processing)`。
5. session verb の defined 収束（`OpenOutcome`・`set_size`
   landing・`conclude_editor_session` の close/detached/
   converged 判定・`session.events` の exact sequence）。
6. `observe_attached_children` の child HWND 契約（実在・
   visibility・parent）。

observed 記録のみ（verdict 非入力）:

- 各 block の `wall_ns`（`Instant::now()` 前後の wall time —
  RT ではないが、session verb を跨いだ timing の参考データ）。
- `rt_fault` の armed 値・marker 検出位置・teardown 実行
  sequence。
- `rt_boundary` の detail テキストと evidence tag
  （`session:rt:cross-thread-boundary`）。

### R6: 閾値設計

- Pass oracle は defined 観測契約のみに寄せる — `Continue`・
  出力正しさ・lifecycle 維持・verb 収束・child 契約。
- `wall_ns` は observed 記録のみ — verdict 入力にしない
  （共有 host の flake 回避・block 単位の budget 超過閾値は
  設けない）。
- `rt_fault` は marker の有無が armed 状態と完全一致するかの
  boolean 判定 — 閾値ではなく marker presence の正確な追従。

## Acceptance Criteria

- [ ] `real-session-rt-samethread`/`real-session-rt-fault`/
  `real-session-rt-boundary` が catalog + entries に追加され、
  interactive run で verdict（pass/fail/not_run/unsupported
  のいずれか）が記録される — AC1 は interactive Windows host
  での `mise run test-ui-plugin-dock-real` 実走で coordinator
  gate（本環境が headless なら blocked-probe + 正直記録で
  scope verification とする）。
- [ ] 観測対象・測定方法・閾値が本 spec（R4/R5/R6）に記録
  される — AC2 正本は本節。
- [ ] `mise run test` が全 OS の CI で pass する — AC3。

## Deferred Design Concerns

- `attach_endpoint`/`PluginHost.l0`/`create_sendable` の公開は
  `scarecrow173/ruviform#872`（L1 out-of-process host）の設計
  範囲。
- `rt_runtime`（`ruviform-audio-test::RtRuntime`）を helper
  thread で pump する co-scheduled leg — ambient 負荷で同一
  instance 観測にならないため不採用（design doc 参照）。

## Sources and Relations

- 実装根拠: `docs/discovery/issue-874-rt-oracle.md`。
- 設計確定: `docs/designs/issue-874-rt-oracle.md`。
- 検証仕様: `docs/test-specs/issue-874-rt-oracle.md`。
- 先例: `docs/designs/issue-389-real-session-lifecycle.md`。
- policy: `POLICIES.md`（OS-L1/GUI-L2/EXT-L3、honest verdict、
  fail-closed）。
