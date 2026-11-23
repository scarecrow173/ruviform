---
id: "TSPEC-891dirtyNoteVerifyQ2x"
type: "test-spec"
status: "approved"
title: "Issue #891 plugin→host state dirty 通知経路 — 検証仕様"
created: "2026-11-23"
updated: "2026-11-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/891"
    - "docs/specs/issue-891-dirty-notification.md"
    - "docs/designs/issue-891-dirty-notification.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-891-dirty-notification.md"
  implements:
    - "docs/specs/issue-891-dirty-notification.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-test-stateful.md"
  blocks: []
  verifies:
    - "docs/specs/issue-891-dirty-notification.md"
  verified-by: []
---

# Issue #891 plugin→host state dirty 通知経路 — test-spec

## 検証対象と検証入口

`ruviform-plugin-core`（`DirtyNotice`/`DirtyNoticeQueue`/wire v7
codec）・`ruviform-plugin-clap`（`HostState`/`mark_dirty` 捕捉）・
`ruviform-plugin-vst3`（`IComponentHandler(2)` 応答面）・
`ruviform-plugin-host`（L0 drain/composite routing/L1 pull loop）・
`ruviform-plugin-export`（子側 backlog dispatch）・両 test fixture
（発火手段）。

正規入口: `mise run test ruviform-plugin-core`・`test
ruviform-plugin-host`・`test ruviform-plugin-export`・`test
ruviform-plugin-test-vst3`・`test ruviform-plugin-test-clap`、adapter
変更面を通る `mise run check-l1-real-plugin`・`lint-l1-real-plugin`・
`test-l1-real-plugin`（`#[ignore]` e2e を選択実行する lane）。

## 試験一覧

| 試験 | 場所 | oracle |
| --- | --- | --- |
| flag latch coalescing | `boundary.rs::tests` | dirty→clean→dirty は最終 edge のみ; 二度目 drain は空 |
| clean edge | `boundary.rs::tests` | `StateClean` が drain される |
| edit FIFO + edit→dirty latch | `boundary.rs::tests` | edits 古い順 → flag edge; bare `ParamEdited` でも `StateDirty` |
| edit→clean | `boundary.rs::tests` | backlog → `StateClean` の drain 順 |
| edit overflow | `boundary.rs::tests` | `MAX_DIRTY_EDITS` 超過で最古 drop + `dropped_edits` 加算 |
| wire verb/reply round-trip | `wire.rs::tests` | `DrainDirtyNotices`(tag28)/`DirtyNotices`(tag9) 全 variant |
| `DirtyNotice` field codec | `wire.rs::tests` | variant round-trip + 未割当 tag `Malformed` + 非有限 `Malformed` + 短 payload `Truncated` |
| `DirtyNotices` bound | `wire.rs::tests` | count > `WIRE_MAX_DIRTY_NOTICES` は `LimitExceeded` |
| `WIRE_VERSION` pin | `wire.rs::tests` | `7` |
| VST3 handler 写像 | `plugin-vst3 host.rs::tests` | `performEdit`→`ParamEdited`+flag、`setDirty(0/1)`→`Clean`/`Dirty`、`restartComponent(kParamValuesChanged)`→`StateDirty`、非有限→`StateDirty`、他 flag・`requestOpenEditor`・group-edit は `kNotImplemented`、begin/end edit は `kResultOk` |
| CLAP `mark_dirty` 発火 | `plugin-test-clap stateful.rs::tests` | `PluginInstance` + `HostState` 登録 host で state `load` の値変更 edge が `mark_dirty` 1 回、同一値 restore は増えない |
| composite routing | `plugin-host composite.rs::tests` | L0 `Shared` → `Drained`+payload、L1 audio 未接続 → `Ipc`、L1 video → `Unavailable`、stale → `InvalidHandle` |
| L1 dirty e2e (CLAP) | `test-clap tests/l1_stateful_test_plugin_e2e.rs` | baseline 空 → mutate→`RestoreState`(変更) → drain `[StateDirty]` → 同一値 restore → drain 空 |
| L1 dirty e2e (VST3) | `test-vst3 tests/l1_stateful_test_plugin_e2e.rs` | baseline 空 → mutate→`RestoreState`(変更) → drain `[ParamEdited{0,0.5}, StateDirty]` → repeat restore → drain に同じ echo（`setComponentState` は verb 自体が change signal — 無条件 announce; controller `setState` は採用済み mirror と等しければ silent） |

## 主要な assert 不変条件

- pull のみ: drain を呼ばない限り notice は consumer 面に届かず、
  二度目 drain は enqueue がなければ空（since-last-drain）。
- `ParamEdited` の value は常に `ParamValue` 域（非有限は
  `StateDirty` に degrade — adapter 境界契約）。
- wire decode は bounded（`LimitExceeded` は allocate しない）・
  strict（未割当 tag は `Malformed`、短 payload は `Truncated`）。
- L1: `RestoreState` は同期 verb — `Ok` reply は子側が verb を
  適用済み（発火は reply 前に queue 済み）。e2e の delivered-audio
  gate は mutation event 着弾の証明にのみ使う（state verb 自体の
  順序保証は wire の request/reply）。
- `OutOfProcessVideo` は `Unavailable` — 「通知なし」と
  「drain 空」の混同を生まない。

## 判定

- `mise run test <各 package>`・`lint <各 package>`・`check <各
  package>` が green。
- `mise run test-l1-real-plugin` が両 suite の
  `real_stateful_test_plugin_l1_state_restore_announces_dirty` を
  `#[ignore]` 選択実行して pass。
- 未実施項目は `NotRun`/`Unsupported` と理由を PR 本文に記録する
  （`*-full`・`*-msrv`・3OS 横断・ABI validator lane は本 slice の
  対象外）。
