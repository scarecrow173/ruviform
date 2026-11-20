---
id: "DSC-630singleComponent01"
type: "discovery"
status: "final"
title: "Issue #630 single-component slice — VST3 `test_stateful_single` 調査記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "evidence/rf630/adapter-state-bridge-verification.md"
    - "evidence/rf630/single-component-verification.md"
  related:
    - "../specs/issue-630-single-component.md"
---

# Issue #630 single-component slice — 調査記録

## 残存ギャップの確定

Issue #630 の in-repo scope で唯一残っていた未実走経路: production
VST3 bridge (`crates/ruviform-plugin-vst3/src/host.rs`) の
`single_component == true` arm。suite 全 11 class が distributed
component/controller pair のため、実バイナリが一度も
`controller_cid == component_cid` の分岐を踏んでいなかった。
`evidence/rf630/adapter-state-bridge-verification.md` が本 arm を
後続 slice へ明示 defer 済み — 本調査はその slice の確定記録。

## bridge 側契約の読み取り（zero-diff 対象・観測のみ）

`host.rs` の single-component 経路を読み、fixture が満たすべき
observable 契約を確定した:

- `enumerate_params`（:1579 付近）: component の
  `getControllerClassId` が **component 自身の CID** を返すと
  `component.cast::<IEditController>()` — 第二の `createInstance`
  は行われない。scan 時インスタンスは使い捨てで
  `controller.initialize` → param scan → `controller.terminate`
  を 1 回ずつ踏む（shared object に対し `initialize` は
  component 経由でも controller 経由でも同じ vtbl slot に届く）。
- `Plugin::init`（:2857-2929 付近）: 同じ own-CID 判定 →
  `component.initialize` 1 回 → `controller.setComponentHandler`
  を shared object に 1 回 → `IConnectionPoint` pairing skip
  （`single_component` guard 内）。
- teardown（:2225-2243 付近）: `controller.terminate` skip →
  `component.terminate` 1 回。
- `save_component_state`（:2708-2744 付近）: single は
  `component.getState` のみ → `frame_state(comp, [])` で
  **`ctrl_len = 0`**（24 bytes = header 8 + RFST 16 + ctrl_len 4）。
- `restore_component_state`（:2746-2810 付近）: single は
  `component.setState` → `component.getState`（normalize）—
  `setComponentState` / `controller.setState` は呼ばれない。
- `params()` は load 時 scan の cached table を返す — live instance
  への param surface 再呼出はない。
- `setComponentHandler` の呼出は `init` の 1 回のみ（:2891）。
- in-process double test `state_roundtrip_single_component_skips_controller`
  が既存の double 証拠（本 slice はその実バイナリ版を足す）。

## fixture 設計の確定

- 1 COM object が `IComponent` + `IAudioProcessor` +
  `IProcessContextRequirements` + `IEditController` を実装し、
  `getControllerClassId` が自分の CID（`uid(0x7A3E_5C91,
  0x2D4F_8B36, 0xE9A1_6C42, 0xB5D8_3F07)` — fresh uid）を返す。
- component/audio surface は共有 `TestComponent` shell に委譲
  （`controller_cid = Self::CID` + `SharedGain` state surface）—
  bus negotiation・lifecycle・DSP・component-domain state law を
  `test_stateful` と一致させ、drift する第二実装を作らない。
- gain の authoritative source は 1 cell（`SharedGain`）。
  controller value/state verb は mirror ではなく同一 cell に束縛
  — single component は「ずれる可能性のある複製」ではなく
  「単一の真実」を持つ（distributed pair の controller mirror と
  意味論が異なるのはこの点）。
- state blob は `crate::stateful` の 16-byte RFST codec をそのまま
  再利用 — component payload = plain、controller payload =
  normalized（本番経路では呼ばれないが honest 実装を維持）。
- `TestStream` を `stateful.rs` の `#[cfg(test)] pub(crate) mod
  test_stream` へ切り上げ、single の state-verb test も同じ
  stream double で駆動（重複実装を作らない）。

## 観測可能性の境界（honest に記録）

- `initialize`/`terminate` 二重呼出: shared shell の lenient
  lifecycle では `Created` からの `terminate` が黙って成功する
  （neutral `Destroy` が `Created` から合法のため）ため、bridge の
  二重 terminate バグは観測不能になる。fixture 側で strict
  contract を採用し、`Initialized` 以外からの `terminate` を
  defined rejection とした — 二重呼出は host `init`/teardown の
  failure として可視になる。
- `setComponentHandler` の「呼ばれること」: bridge の
  `BridgeComponentHandler` は acknowledge-only の no-op で、
  callback は観測不能。「せいぜい 1 回」は fixture の
  once-contract（2 回目 → `kResultFalse` → host `init` 失敗）で
  観測可能。「少なくとも 1 回」は bridge 経由では構造的に
  観測不能 — evidence に残る観測境界として記録。
- `IConnectionPoint` 非実装: fixture は `IConnectionPoint` を
  実装しない — bridge が single で pairing を試みると QI 失敗が
  `init` 失敗として可視。

## upstream 参照禁止への適合

SDK validator との照合は振る舞い・バージョン・ライセンスで記述し、
外部リポジトリへのリンク・issue 番号は新規記述に一切含めない
（`evidence/rf630/abi-validator/` 既存台帳の pin 値は read-only
参照）。
