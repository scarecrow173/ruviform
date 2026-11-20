---
id: "DSN-630singleComponent01"
type: "design"
status: "approved"
title: "Issue #630 single-component slice — VST3 `test_stateful_single` design"
created: "2026-11-20"
updated: "2026-11-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-single-component.md"
    - "docs/discovery/issue-630-single-component.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-single-component.md"
  implements:
    - "docs/specs/issue-630-single-component.md"
  implemented-by:
    - "docs/test-specs/issue-630-single-component.md"
  depends-on:
    - "docs/designs/issue-630-test-plugin-suite.md"
---

# Issue #630 single-component slice — design

## データ形状（先に確定）

`TestStatefulSingle` は 4 フィールドを持つ 1 COM object:

| field | 型 | 役割 |
| --- | --- | --- |
| `inner` | `TestComponent<TestStatefulProcessor>` | component/audio/lifecycle/component-state surface への委譲先。`controller_cid = Self::CID` で構築 → `getControllerClassId` が自分の CID を返す（single-component discriminator） |
| `gain` | `SharedGain` | plain-domain gain cell — RT processor・component state surface・controller value/state verb が共有する唯一の真実 |
| `domain` | `ParamController` | pure-domain verb の委譲先（param info・string format/parse・normalized↔plain mapping・null `createView`）。`value`/`stateful` field は一切駆動しない — 値を持つ verb は `gain` 側で自前実装 |
| `handler_calls` | `AtomicU32` | `setComponentHandler` 呼出回数 — once-contract |

`Class::Interfaces` = `(IComponent, IAudioProcessor,
IProcessContextRequirements, IEditController)` — binding の IID
query は tuple 順を問わず解決するため、shared object への
`cast::<IEditController>` が合法に成功する。

## 責務の割り当て

- **委譲（drift しない）**: `IComponent`・`IAudioProcessor`・
  `IProcessContextRequirements` は `inner` へ verbatim 委譲。
  bus negotiation・lifecycle event mapping・param event
  collection・`apply_gain` DSP・component-domain `setState`/
  `getState`（`SharedGain` の `ComponentStateSurface` 経由）は
  `test_stateful` と同一コードを踏む — fixture は topology を
  変えるのであって law を複製しない。
- **`initialize`**: `inner` へ委譲（`Created→Initialized` のみ
  合法 — bridge が shared object の二重 init を犯したら
  `IllegalTransition` → `kResultFalse` → host `init` 失敗として
  可視）。
- **`terminate`（strict）**: `inner.lifecycle_state() ==
  Initialized` のときのみ委譲。shared shell は `Created` からも
  成功する lenient 契約（neutral `Destroy` が `Created` から
  合法）だが、それでは bridge の二重 terminate が黙って成功
  し観測不能になる。検証 fixture の責務として
  `Initialized` 以外を defined rejection に狭めた —
  `TestComponent::lifecycle_state()` 読取 probe をこの目的で
  追加（borrow conflict は `Failed` 報告 → caller の strict
  check が decline する既存の defined-rejection 方針に整合）。
- **controller value/state verb**: `gain` cell 直結。
  `setParamNormalized` は `ParamController` と同じ normalized
  domain contract（非有限・範囲外 → `kInvalidArgument`）で
  denormalize → `gain.store`（`store` 内部で param domain clamp）。
  `getParamNormalized` は `gain.load` → `normalize`（finite は
  構築上保証、`else` は defined backstop）。
  `setComponentState`（plain blob）/`setState`（normalized
  blob）/`getState`（normalized blob）は RFST codec + `gain`
  cell — distributed pair と意味論一致、mirror drift なし。
- **`setComponentHandler`（once-contract）**: 呼出を count、
  2 回目は `kResultFalse`。pointer は保持しない — 回数が観測
  面。「少なくとも 1 回」は bridge 経由では構造的に観測不能で
  ある旨を evidence に記録する。
- **pure-domain verb**: `domain` へ委譲 — 値を持たない verb は
  instance state に依存しないため mirror 問題が存在しない。
- **`IConnectionPoint`**: 実装しない — bridge が single で
  pairing を試みれば QI 失敗が `init` 失敗として可視。

## factory 登録

`CLASS_ROWS` 末尾に 12 番目の row（index 11）—
`"Audio Module Class"`、`name = "test_stateful_single"`、
`sub_categories = "Fx"`、controller row なし。`countClasses`
= 12、`createInstance` dispatch に `TestStatefulSingle::CID`
arm を追加。descriptor scan は category フィルタで 7 件
（`test_spawn_fail` を含む）。

## 観測設計

| 契約 | 観測手段 |
| --- | --- |
| own-CID discriminator | in-crate test（`getControllerClassId` が `Self::CID` を書く）+ `params()` が scan 経由で Gain 面を返す |
| 二重 init なし | `initialize` が `Created` 以外で reject — bridge が犯せば host `init` 失敗 |
| 二重 terminate なし | strict `terminate` — bridge が犯せば teardown/`Destroy` 失敗 |
| handler が shared object に 1 回 | once-contract（2 回目で reject → `init` 失敗） |
| `IConnectionPoint` pairing なし | 非実装 — pairing 試行は QI 失敗 |
| `ctrl_len = 0` framing | save blob が 24 bytes `[u32 16][RFST][u32 0]` — L0/L1 の両方で byte レベル pin |
| state round-trip | save → mutate → restore → save byte-exact + audio oracle |

## 却下した設計

- **distributed pair の mirror を真似る**: `ParamController` の
  `value` field をそのまま共有すると component 側 cell と drift
  する第二 source になる。single component の正直な意味論は
  1 cell。
- **lenient `terminate` のまま**: bridge の二重 terminate を
  観測不能にする — fixture の存在意義を失う。
- **handler の retain/再利用**: `beginEdit`/`performEdit` 動作を
  fixture が持たないのに pointer を保持する必要はない。
- **`kNotImplemented` state verb**: 本番経路で呼ばれないからと
  いって spec surface を defined decline にするのは fixture の
  honest 契約に反する。

## 依存・境界

- 新規依存なし — `vst3` binding + `ruviform-plugin-core`/
  `ruviform-audio-core` の既存範囲。
- `stateful.rs` の RFST codec・`SharedGain`・stream helper・
  `TestStream` を再利用（`TestStream` は `#[cfg(test)]
  pub(crate) mod test_stream` へ切り上げ — 重複実装しない）。
- `component.rs` への追加は `lifecycle_state()` probe のみ —
  shared shell の契約自体は不変。
- RT 経路は `TestComponent<TestStatefulProcessor>` 委譲で新規
  コードを増やさない（state verb のみ non-RT で allocate 可）。
