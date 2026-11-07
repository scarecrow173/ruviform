---
id: "SPEC-034T6RWQv8Hm2Lj4pD9sT6"
type: "spec"
status: "proposed"
title: "Issue #387 Driver-neutral Component/View Specs・Headless Behavior・GPUI Controls/Viz Renderer"
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
    - "../specs/issue-384-driver-neutral-gui-architecture.md"
    - "../specs/issue-385-dtcg-resolved-theme.md"
    - "../specs/issue-386-driver-neutral-gui-host.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/issue-387-component-view-inventory.md"
    - "../specs/issue-390-driver-contract-lab.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/issue-387-component-view-inventory.md"
  derived-by:
    - "../designs/issue-387-component-view-specs.md"
  verifies: []
  verified-by:
    - "../test-specs/issue-387-component-renderer.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #387 Spec: Component/View Specs・Headless Behavior・GPUI Renderer

Issue #387 v2 (2026-11-07 横断レビュー確定仕様) に基づく。
`WindowDriver`(RF386) が開いた Window の中に載る Component/View の
driver-neutral 契約、最小 headless behavior projection、および
GPUI Controls/Viz renderer を提供する。

## 背景

- RF386 は Window の生成/lifecycle/typed event 配送までを所有し、
  Window 内に何を描くかは未定義だった(現状 `RvfRoot` は固定
  fixture)。本 Issue はその内側を埋める。
- 既存 TypeScript 実装(`ui/core`, `ui/binding`, `ui/components`,
  `ui/viz`)に gesture/value/receipt の正本意味論が確立済み。
  Rust 側へは同じ意味論を契約として port し、DSL/実装の写経はしない。

## 要件

### R1 中立 Component/View schema(`ruviform-ui::view`)

- `ComponentKey` — View 内 stable な component 識別子(文字列 newtype)。
- `ViewSpec` — 木構造の spec。root `ComponentSpec` を持つ。
- `ComponentSpec` — `#[non_exhaustive]` enum。本 Issue の最小 kind:
  `Panel` / `Tabs` / `Fader` / `Seek` / `Waveform` / `NativeSlot`。
  `NativeSlot` は #388 `NativeSurfaceSlot` 用の非対話 layout 領域で、
  runtime `SurfaceSlotId` は spec に含めず driver 側
  `bind_native_slot` で `key` に関連付ける(design 参照)。
- Props/Variant: kind 毎の最小 props。連続値コントロール(Fader/Seek)は
  `ValueDescriptor` を持つ。`readOnly`/`disabled`/`constant` は
  descriptor の宣言 flag(availability の正本)、`error` は
  `ComponentVisualState` の visual+a11y flag で gesture を block しない。
- Accessibility semantics: `role`/`label`/`orientation`/`value_min`/
  `value_max`/`value_text` を中立契約に含める(renderer が backend の
  a11y へ写像する責務を持つ)。
- `Waveform` の data payload は解決済み `WaveformBin`
  (`{start,end,min,max,rms,count}` — `ruviform-viz::ViewBin`/`LodBucket`
  と同一 field 意味)列 + `sample_rate`/`channels`/`frames` 要約 +
  viewport 範囲。bounded(`MAX_WAVEFORM_BINS` 上限)。viz→contract 変換は
  adapter/consumer 責務で、`ruviform-ui` は viz 非依存を維持する。
- GPUI/DOM/OS/toolkit 型は schema に含めない。

### R2 mount/reconcile/unmount と identity(`ViewHost`)

- `ViewHost` は spec を mount/update/unmount する headless projection
  engine。`ViewInstanceId` 毎に generation を管理する。
- `update(spec)`: `ComponentKey` で reconcile し、key 一致は
  `updated`、消えた key は `removed`、host 破棄は `unmounted`。
  projection op 列を typed 公開する。
- stale generation の spec/input は `StaleGeneration` error で拒否し、
  現行 instance を mutate しない。
- 未対応 kind を renderer が受けた場合は typed `UnsupportedKind`
  (panic しない)。
- 同 key update は semantic identity を維持する(進行中 gesture は
  `ValueDescriptor` 変更時のみ detach+safe cancel のうえ再構築。
  それ以外の props 変更は committed value と gesture を保持する)。

### R3 Headless behavior(`ruviform-ui::behavior`)

- `ValueDescriptor`: min/max/defaultValue/step/stepDomain/fineStep/
  fineStepDomain/mapping/outOfRange/unit/readOnly/disabled/constant —
  pure data(関数は `ValuePorts` で別注入、serialize 境界を越えない)。
  `StepDomain::Plain|Normalized`、`ValueMapping::Linear|Log|Custom`、
  `OutOfRangePolicy::Clamp|Reject`。`ValueMapper`: range→snap→convert
  の `resolvePlain`/`resolveNormalized`、step 量子 `adjustNormalized`、
  format/parse、`verifyMapping`。
- `InteractionControl` gesture 機械。不変条件(TS canonical 同値):
  `Begin → Preview* → Commit|Cancel`。`begin` は idle 時のみ受理 —
  別 gesture 進行中/重複 id は `rejected(gesture-already-active)`、
  terminated id は `rejected(already-terminated)`。`preview`/`commit`/
  `cancel` は unknown id `noop(no-active-gesture)`、terminated id
  `rejected(stale-gesture|already-terminated)`。commit の invalid value
  は rejected で gesture は継続。`detach`/`focus-lost` は lifecycle
  input(policy 既定 `cancel`、reason `unmount`/`lost-focus`)。
  `set`/`adjust`/`reset` は gestureId 無しで atomic begin+commit、
  gestureId 有りで auto-begin+preview。`InputSeq` で duplicate noop /
  out-of-order rejected(lifecycle input は drop 免除)。terminated
  history bounded(既定 32)。`setAvailability` は active gesture を
  `cancel(explicit)` して unavailable 化。
- `InteractionEvent`/`ControlInput`/`InputSource`/`CancelReason`/
  `UnavailableReason`/`RejectReason`/`NoopReason` を型で表す。
- `InteractionDispatcher`: 複数 control を `ControlId` で route、
  spec 更新による再構築、`detachAll`、snapshot。
- Command 語彙: `CommandIntent`(`Begin|Preview|Commit|Cancel|Action`)、
  `CommandReceipt`(`ReceiptStatus`)、`StateProjection`、
  `CommandPhase`(`idle|preview|pending|accepted|applied|observed|
  rejected|unsupported|timeout|stale` — canonical 10 phase)。preview は
  coalescible、terminal は決して落とさない。`CommandBinding` は
  pending/receipt rank/projection revision guard/detach→stale+
  generation bump を機械として持つ(ack timeout は caller 供給 `now`
  で判定)。
- `Controlled display`(preview)/`Applied`(receipt)/`Observed`
  (projection)/Project 真値を混同しない — `CommandPhase` と
  event で区別を維持する。

### R4 GPUI renderer(`ruviform-ui-gpui`, feature `gpui`)

- `ThemeMap`: `ResolvedTheme` → 私有 palette への一方向写像。
  canonical semantic token 名を中立 contract として定義
  (`surface`/`surface.raised`/`text`/`text.dim`/`border`/
  `interactive`/`interactive.hover`/`interactive.active`/`focus.ring`/
  `status.error`/`status.warning`/`fader.track`/`seekbar.track`/
  `seekbar.buffered`/`viz.grid`/`viz.series.1`/`viz.waveform.fill`/
  `viz.gap.fill`)。未解決 token は既定値 fallback。
- `RvfView`: `ViewHost` を保持する GPUI `Render` entity。spec 木を
  element へ投影し、mouse/scroll/keyboard を `ControlInput` に
  normalize して `ViewHost` へ dispatch、emit された
  `CommandIntent` を `IntentSink` 経由で consumer へ返す。
- `Waveform` は `gpui::canvas` で `WaveformBin` を描画(fill/rms/gap/
  baseline)。LOD/NaN/gap 意味論は payload が既に保持。
- 未対応 kind は `UnsupportedKind` を sink/typed error へ返し
  panic しない。
- GPUI widget tree を一般 GUI DSL へ昇格しない — spec kind の写像のみ。

### R5 2 vertical slice

- `LAB-SV-FADER-DOCK`: `Panel` + `Tabs` + `Fader` の ViewSpec と
  `DockWorkspace`(既存)の view move で `ViewInstanceId` identity が
  維持されること、detach→reattach で stale generation が拒否されること。
- `LAB-AL-WAVEFORM-SEEK`: `Waveform` + `Seek` の ViewSpec が
  2 scale factor で同一 semantic state 遷移を持ち、bounded bins 経路で
  NaN/gap が意味論として維持されること。

### R6 検証・証跡

- `UI-DVR-02`/`03` の fake binding を `cargo-test` mechanism で pass へ。
- 両 story の fake binding を `cargo-test` mechanism で pass へ。
- gpui binding: `gpui::TestAppContext` headless semantic 試験を試行。
  不可能なら `not_run` + rerun 条件を維持(実機 lane)。
- `real-os`/`manual` binding は継続 not_run/blocked(実機別 gate)。
- 2 ResolvedTheme(fixture DTCG: synvelis-dark 系/audiolens-dark 系
  canonical token 語彙)で同一 spec+gesture 列から同一 semantic
  event/receipt 列が得られ、かつ palette が異なることを示す。
- Pass/Fail/Unsupported/NotRun/Blocked を分離して報告。Fake/headless
  の green を GPUI 実 window/実 OS の pass としない。

## Non-goals

- 全 widget の網羅(Knob/Toggle/NumericInput/Transport は Issue 記載の
  初期対象だが本 slice では未実装。kind を追加する際は schema 拡張)。
- consumer 側 screen 構成、ViewLink/binding transport の完全 port
  (preview coalescing queue 等は transport 責務。契約は
  `coalescible` 属性のみ)。
- `post_event`/`WindowDriver`/dock tree/NativeSlot/Session の変更。
- real OS window/IME/a11y 実証、pixel-perfect parity。
- GPUI 本体以外の toolkit 依存。

## 不変条件

- 中立 schema/behavior/host から `gpui`/`wry`/`DOM`/OS handle を
  参照しない。`check-ui-driver-boundary` で機械検証済みのまま保つ。
- Preview は coalescible、terminal(discrete action/commit/cancel)は
  drop しない。
- GPU/resource teardown は Project/RT ownership を drop しない
  (ViewHost/unmount は view 状態のみ破棄)。
- RT へ GUI lock/block を伝播させない — ViewHost は同期 pure
  projection、render thread の work のみ。
