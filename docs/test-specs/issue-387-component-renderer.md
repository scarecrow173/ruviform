---
id: "TESTSPEC-034T6RWTu4Bx7Ng3hV0dK2"
type: "test-spec"
status: "proposed"
title: "Issue #387 Component/View + Headless Behavior + GPUI Renderer 試験仕様"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/387"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-387-component-renderer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-390-driver-contract-lab.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-387-component-view-specs.md"
  derived-by: []
  verifies:
    - "../specs/issue-387-component-view-specs.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #387 試験仕様

## 層別マトリクス

| layer | 場所 | 内容 |
| --- | --- | --- |
| unit (ruviform-ui) | `src/behavior.rs` `#[cfg(test)]` | value mapper linear/log/custom、clamp/quantize、out-of-range、parse、display special |
| unit (ruviform-ui) | `src/view.rs` `#[cfg(test)]` | spec validate、reconcile op 列、stale generation 拒否、Knob/Toggle/Meter/TimeAxis mount、Toggle 非 binary 拒否、Meter/TimeAxis payload 上限・NaN tick 拒否 |
| semantic fake | `ruviform-ui-test/tests/component_view.rs` | UI-DVR-02/03 機械 binding + 2 story fake layer + 2-theme 同値 |
| unit (gpui) | `ruviform-ui-gpui` `render.rs` `#[cfg(test)]` | ThemeMap token→palette、normalized→descriptor 写像 |
| semantic gpui | `ruviform-ui-gpui` `render.rs` `#[gpui::test]` | `TestAppContext` headless: Fader gesture(begin/preview/commit)、Toggle クリックで binary 反転、NativeSlot layout/clip。実 OS window は含まない |
| boundary | `mise run check-ui-driver-boundary` | 中立 module の非依存機械検証 |
| manifest | `mise run test-ui-driver-contract` | cases/stories validation |

## semantic fixtures(oracle 対応)

### UI-DVR-02 ViewSpec mount/reconcile/unmount

- mount: `Panel > Fader + Seek` spec → `Mounted` op が key 毎に発行。
- update 同 key props 差分 → `Updated`、identity(gesture/committed)保持。
- key 消滅 → `Removed` + active gesture は `cancel(reason=unmount)` で終端。
- `unmount` → 全 `Unmounted` + `detach_all`。
- stale generation の `update`/`dispatch_input` → `ViewError::StaleGeneration`、
  現行 instance 非 mutate。

### UI-DVR-03 gesture delayed arrival

- `Begin → Preview → Commit` 正常列 → `CommandIntent` が
  `begin/preview/commit` で発行。
- commit 後の遅延 `preview` → `rejected(stale-gesture)`、同 id 再
  `begin` → `rejected(already-terminated)`。
- 別 gesture 進行中の `begin` → `rejected(gesture-already-active)`、
  先発 gesture は継続。
- `detach` → `cancel(reason=unmount)`(既定)、`focus-lost` →
  `cancel(lost-focus)`、以降入力は stale reject。
- `setAvailability(unavailable)` → active `cancel(explicit)`。
- disabled/read_only spec → `begin`/`set` は `unavailable` event。
- `reject` policy の範囲外入力 → `rejected(out-of-range)`、非有限 →
  `rejected(non-finite-input)`。
- `seq` duplicate → `noop(duplicate-input)`、lower seq →
  `rejected(out-of-order)`、lifecycle input は drop 免除。
- gestureId 無し `set` → atomic `begin`+`commit` pair。

### LAB-SV-FADER-DOCK(fake)

- spec `Panel > Tabs(tabA: Fader ch1, tabB: Fader ch2)` mount。
- `DockWorkspace` の既存 view move で `ViewInstanceId` 不変、host は
  `reattach`(generation++)で stale input を拒否。
- Fader `Begin(drag)` → `Preview*` → `Commit` → intent 列が
  `begin/preview+/commit`。preview は coalescible、commit は必達。
- detach→同 ViewInstanceId で再 attach → committed value 維持、
  gesture は新規 generation。

### LAB-AL-WAVEFORM-SEEK(fake)

- `Waveform`(fixture bins: 有限値 + NaN gap + EMPTY 混在)mount。
  bins は spec payload のみで描画意味論として `ViewBin` を保持。
- `Seek` `Begin → Preview → Commit` が viewport をまたいでも
  semantic state(phase/preview/committed)が一致。
- scale factor 1.0/2.0 で同一 spec+入力列から同一 `ViewEvent` 列
  (scale は render 側のみ、semantic は不変)。
- bounded: bins 数上限超過 spec は `ViewError::InvalidSpec`。

### Knob/Toggle/Meter/TimeAxis(追補)

- `Knob`(Fader と同一 gesture 契約)、`Toggle`(binary)、`Meter`、
  `TimeAxis` を含む Panel を mount → 全 key が `mounted` で observe 可能。
- `Toggle` の descriptor が `{0,1,step 1}` 以外 → `InvalidSpec`。
- `TimeAxis` の tick 上限超過・非有限 `at`、`Meter` の channel 不一致 →
  `InvalidSpec`。(`Meter` の bins 上限は実装済みだが個別テストは未追加。)
- GPUI: `Toggle` クリック 1 回で committed が 0↔1 に反転(`rvf_view_toggle_click_flips_binary`)。
- GPUI: `Meter`/`TimeAxis` は描画されず `unsupported component: <kind>`。
  headless 契約のみで GPUI 描画は未達(follow-up)。

### 2-theme 同値

- fixture DTCG `synvelis-dark`/`audiolens-dark`(canonical semantic
  token 語彙、値は各製品色)を `ThemeResolver::resolve` → 2
  `ResolvedTheme`(正規化出力が互いに異なることを先に確認)。
- `ViewHost` は theme を持たない — 同一 spec+入力列を 2 回実行して
  `ViewEvent`/receipt 列が完全一致することで、semantic 機械が
  theme 非依存であることを示す。palette 差分は gpui `ThemeMap`
  unit test で担保。
- `ThemeMap` が 2 theme で異なる palette を出すこと(gpui unit test)。

## verdict 語彙

- `pass`: fake layer semantic は全 pass 期待。
- `gpui` layer: headless parity 可なら `pass`(cargo-test)、
  不可なら `not_run`+rerun(実機 window lane)。
- `real-os`/`manual`: 現行 not_run/blocked を維持。
- Fake/headless green を実 OS/実 GPUI window の pass にしない。

## evidence

- `mise run test-ui-driver-contract` + `test -p ruviform-ui-test` +
  `check/lint/test-ui-gpui` の exit code を IR に記録。
- OS/toolchain/`rustc --version`/`gpui 0.2.2`/exit code/TestID/3 repo
  SHA(実行時点)を evidence として記録。
