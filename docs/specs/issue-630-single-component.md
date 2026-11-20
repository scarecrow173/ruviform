---
id: "SPEC-630singleComponent01"
type: "spec"
status: "approved"
title: "Issue #630 single-component slice — VST3 `test_stateful_single` spec"
created: "2026-11-20"
updated: "2026-11-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-single-component.md"
    - "docs/specs/issue-630-test-stateful.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-630-single-component.md"
  implements:
    - "docs/discovery/issue-630-single-component.md"
  implemented-by:
    - "docs/designs/issue-630-single-component.md"
  depends-on:
    - "docs/specs/issue-630-test-stateful.md"
  related:
    - "evidence/rf630/single-component-verification.md"
---

# Issue #630 single-component slice — spec

## 目的

Issue #630 の残存 in-repo ギャップを埋める: production VST3
bridge の `single_component == true` 経路を、実バイナリ fixture
class で初めて実走する。全既存 class が distributed pair のため
該当経路は in-process COM double でしか検証されていなかった。

## スコープ

`crates/ruviform-plugin-test-vst3` に VST3 single-component
fixture class `test_stateful_single`（作業名）を追加する。
観測対象は `crates/ruviform-plugin-vst3/src/host.rs` の既存
single-component arm — 本 slice は fixture 側のみを実装し、
bridge 側は zero-diff（観測で実バグが判明した場合は直さず
証拠化してエスカレーション）。

## 受入条件（slice 確定版）

1. `test_stateful_single` が 1 COM object で `IComponent` +
   `IAudioProcessor` + `IEditController` を実装し、
   `getControllerClassId` が自分の CID を返す。factory の
   12 番目の class（suite order 末尾、`"Audio Module Class"` の
   1 行 — controller row なし）として登録される。
2. production `load → init → prepare → process → teardown`
   （L0 in-process host_path）と L1 `SaveState`/`RestoreState`
   wire round-trip を実走で通る。
3. `single_component == true` bridge 経路が実バイナリで踏まれる:
   `initialize`/`terminate` の二重呼出なし（`controller.*` が
   skip される）、`setComponentHandler` が shared object に
   1 回、`IConnectionPoint` pairing なし、保存 blob が
   `[u32 16][RFST comp][u32 0]` の `ctrl_len = 0` framing
   （24 bytes）。
4. 既存 lane は緑のまま — 11 → 12 class / 6 → 7 descriptor の
   pin 更新は全箇所に波及し、他 class の e2e は回帰しない。

## 振る舞い契約

- `test_stateful` と同一の `Gain` param（id 0、plain `[0.0,2.0]`、
  default 1.0）、同一 DSP law、同一 16-byte RFST blob law、同一
  activation-does-not-reset law — topology だけを変化させる。
- gain の真実は 1 cell（`SharedGain`）。component state verb と
  controller value/state verb は同一 cell に束縛（distributed
  pair の normalized mirror のような第二 source は作らない）。
- `terminate` は `Initialized` 状態でのみ合法 — shared shell の
  `Created` からも成功する lenient 契約を、検証 fixture として
  strict に狭める（二重 terminate を defined rejection に変える）。
  terminate は non-terminal のまま（re-init cycle は合法）。
- `setComponentHandler` は VST3 once-contract — 2 回目は
  `kResultFalse`。handler pointer は保持しない（`beginEdit`/
  `performEdit` 動作は fixture に存在しない — 回数を記録する
  honest recorded surface）。
- controller の `setComponentState`/`setState`/`getState` は
  honest 実装（本番経路で呼ばれなくても `kNotImplemented` で
  ごまかさない）。`createView` は null。
- 悪性入力は全て defined rejection — blob 長・magic・version・
  非有限値は `kResultFalse`、有限 range 外は param domain clamp。
- RT 経路は alloc/lock/wait-free を維持 — `process` は共有
  `TestComponent<TestStatefulProcessor>` への委譲で新規コード
  パスを増やさない。state verb は non-RT（`IBStream` drain の
  み allocate）。

## 検証要求

- in-crate COM-level unit test（own-CID、strict terminate、
  double-init rejection、single-cell 両 domain、malformed
  rejection、handler once-contract、domain verb）。
- `tests/host_path.rs`: discovery（7 descriptor・名前順 pin）、
  single param surface、`ctrl_len = 0` 24-byte framing、
  save → mutate → restore → save byte-exact、malformed →
  `PluginFailed`。
- `tests/l1_stateful_test_plugin_e2e.rs`: out-of-process での
  round-trip + malformed（delivered-audio ordering gate・audio
  oracle・clean teardown は既存先例どおり）。
- `mise run test-plugin-abi-validator` は provisioned 環境があれば
  optional leg — 新規 class を含む結果を記録。無ければ honest
  NotRun。
- deterministic evidence（in-crate + host_path）と real-run
  evidence（L1 out-of-process + 外部 validator）を分離して記録。
- `setState`/`getState` の defined rejection は Pass と書かない。

## 非目標

- `crates/ruviform-plugin-vst3` への変更（zero-diff）。
- CLAP 側の対応（single-component 相当の対象がない）。
- `test_fault` alloc/bad-timestamp モード、`mark_dirty`/
  `performEdit`、GUI-L2 real-dock lane、consumer oracle 照合、
  MSRV/`*-full` lane — 残件として PR 本文に記録。
- docs index（`docs/**/README.md`）登録 — Coordinator が統合時に
  整備。
