---
id: "DESIGN-20261103-reusable-ui-component-system"
type: "design"
status: "draft"
title: "Reusable UI Component System — Interaction, Controls, Visualization, Binding, Theme and Renderer"
created: "2026-11-03"
updated: "2026-11-03"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/257"
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
    - "../adr/ui-architecture-webview-hybrid.md"
    - "../adr/ui-tech-selection-poc.md"
    - "issue-156-gui-host-foundation.md"
    - "issue-157-viz-primitives.md"
    - "shared-foundations-and-modularization-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "issue-181-multi-window.md"
    - "issue-182-dock-tree.md"
    - "synvelis-driven-target-architecture-and-migration.md"
    - "issue-219-cross-repo-ci-gate-credential-runbook.md"
  refines: []
  refined-by: []
  derives-from:
    - "issue-156-gui-host-foundation.md"
    - "issue-157-viz-primitives.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/258"
    - "https://github.com/scarecrow173/ruviform/issues/259"
    - "https://github.com/scarecrow173/ruviform/issues/260"
    - "https://github.com/scarecrow173/ruviform/issues/261"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/263"
    - "https://github.com/scarecrow173/synvelis/issues/47"
    - "https://github.com/scarecrow173/synvelis/issues/48"
    - "https://github.com/scarecrow173/audio-lens/issues/131"
    - "https://github.com/scarecrow173/audio-lens/issues/132"
  defers: []
  deferred-by: []
---

# Reusable UI Component System — 横断詳細設計

> Status: **アーキテクチャの責務・不変条件・実装分解は合意済み方向性**。
> 本文はそれを具体化する **review用詳細設計（draft）** であり、公開TypeScript/Rust API名、
> package分割、Native compositorの採否、性能SLO、全OS実機完了を確定したものではない。
> マージ・実装・PoC成功・本番Consumer GUIの完成はそれぞれ別の証拠で管理する。

- Parent: [Ruviform #257](https://github.com/scarecrow173/ruviform/issues/257)
- Provider work: [#258 headless](https://github.com/scarecrow173/ruviform/issues/258) / [#259 web controls](https://github.com/scarecrow173/ruviform/issues/259) / [#260 viz components](https://github.com/scarecrow173/ruviform/issues/260) / [#261 bindings](https://github.com/scarecrow173/ruviform/issues/261) / [#262 native PoC](https://github.com/scarecrow173/ruviform/issues/262) / [#263 compat/showcase](https://github.com/scarecrow173/ruviform/issues/263)
- Consumers: [Synvelis #47 controls](https://github.com/scarecrow173/synvelis/issues/47), [#48 viz](https://github.com/scarecrow173/synvelis/issues/48); [AudioLens #131 controls](https://github.com/scarecrow173/audio-lens/issues/131), [#132 viz](https://github.com/scarecrow173/audio-lens/issues/132)

## 1. 目的・範囲と既存実装

Synvelis（DAW、live engine、Transport/Automation）およびAudioLens（高精度offline分析、長尺Dataset、Validity/Confidence/Provenance）を対等なConsumerとする。Knob/Fader/Seek、Waveform/Meter/Spectrum/Feature Plot等について、各製品の見た目・業務意味を保持したまま**重複する操作契約・描画資産・テスト**をRuviformへ還元する。GUIは実計算やProject/Analysis真値のOwnerにならない。

既存確認事項（基準は各リンク先のmainコード・README、設計時点）:

| 既存能力 | 責務 / 実装 | 残る不足 |
| --- | --- | --- |
| [ruviform-ui](../../crates/ruviform-ui/README.md) | Host/IPC v1、Service/Job、bounded DataRegistry、Dock/Window、RenderTier/NativeSurfaceHost *port* | UI操作部品・Gesture/Binding/View Rendererを提供しない |
| [ruviform-ui-webview](../../crates/ruviform-ui-webview/README.md) | wry/tao Host、rvf://app/data、window/session、feature webview | 製品非依存のSolid UI部品なし |
| [ruviform-viz](../../crates/ruviform-viz/README.md) | f64/NaN、Waveform LOD、Spectrogram tile、Timeline/Viewport、Cache、RVBW/RVST encode、非RT | 描画/View/ユーザー操作の本体なし |
| [ruviform-audio-analysis](../../crates/ruviform-audio-analysis/README.md) | LevelMeter、SpectrumAnalyzer、WaveformPeaks | UI snapshot配信・描画と分離済み |
| [UI ADR](../adr/ui-tech-selection-poc.md) | WebView-first、Solid、DOM/Canvas2D/WebGL2、条件付きwgpu/native adjacent surface | Component System採用は未確定 |
| [#181/#182](https://github.com/scarecrow173/ruviform/issues/182) | Multiwindow・Dock logical View ownership | 実機検証残件[#243](https://github.com/scarecrow173/ruviform/issues/243) |

既存UI Host/Visualization/RT/Pluginの責務を書換えず、既存ADRを暗黙に上書きしない。

## 2. 採用する境界と禁止する依存

### 2.1 4つの契約領域

1. **Semantic / Data**: 表示可能な値・単位・範囲・時間軸・viewport・series metadataを扱う。Data truthはConsumer/engine/dataset、Ruviformは汎用のvalue projectionとviz primitives。
2. **Headless Interaction**: Pointer/keyboard/touch/wheel/focusからBegin/Preview/Commit/Cancelへ変換。可視状態やCommand Gatewayに依存しない。
3. **Binding / State projection**: Consumer Command/Query/Subscription portをUIイベントと接続。target/owner/expected revision、receipt、stale/errorを表現できる。実認可・DAW Undo・解析JobはConsumerの責務。
4. **Presentation / Rendering + Theme**: Solid DOM controls、必要なCanvas/WebGL2 visualization、Token/variant/slots。Nativeは実需とPoCを通したoptional adapter。各コンポーネントに単一Renderer APIを強制しない。

### 2.2 依存DAG（概念、実package/型名は候補）

```text
Synvelis UI / AudioLens GUI (consumer app / Product Controllers)
  ├── Product Binding Adapters ──> Product Command/Query/Project/Dataset/Engine
  ├── Reusable Web Controls (Solid) ──> Headless Interaction + Theme
  ├── Reusable Viz Components ──> Viz Fetch Port + Renderer + Theme
  └── Ruviform WebView Host ──> ruviform-ui (headless host)
                 optional: Native Renderer adapter ──> NativeSurfaceHost port

Visualization adapter (product/nonRT) ──> ruviform-viz ──> media-core/numeric/exec
RT snapshots ──> bounded nonRT broker ──> Product data provider ──> Viz UI
```

- `ruviform-ui` の production dependencies は引き続きPrimitivesのみ。Vue/React/Solid/DOM/wry/DAW/Analysis型を混ぜない。
- `ruviform-viz` は `ruviform-ui`/WebViewと独立のnonRT数値・表示用データ処理を保つ。RVBW/RVSTの接着はapplication/nonRT adapter側。データ処理のGUI無依存・CLI再利用を守る。
- 新しいTypeScript workspace/package（例: `ui/packages/core`, `solid`, `visualization`, `themes`）は**候補**。最初は凝集した少数モジュールから開始し、独立配布/外部依存/CI等の理由が生じた場合だけ分割。
- RuviformはSynvelis Track/Bus/Automation Write、AudioLens Feature/Diagnosis/Dataset schema、Product-specific command gateway、Undo/Job Registryを所有しない。
- Rust coreにNative/OS/フロントエンド専用の具体型を漏らさない。

## 3. Value / Parameterの精度・表示

操作対象（continuous/stepped control）で扱う**数値値**はfiniteで範囲整合性を検証する。元Datasetの観測値にはNaN/欠測が存在し得るため、「操作系の値検証」と「可視化系列の欠測規則」を統一してはならない。

### 3.1 Value descriptor（概念型）

```ts
type ValueDescriptor = {
  min: number; max: number; defaultValue?: number; // finite, min<=max
  step?: number; // positive finite, optional
  mapping: "linear" | "log" | "custom";
  format?: (plainValue: number) => string;
  parse?: (text: string) => number;
  readOnly?: boolean; disabled?: boolean;
};
```

これは説明用の例であって公開APIではない。実契約ではserialized descriptorとlocal formatter callbackを区別し、境界跨ぎに関数を流さない。

- normalized [0,1]、plain domain、display text/unitは別。stepは**plainまたはnormalizedのどちらの単位か**を明示し、double conversionをしない。
- linear/log/customは単調性・端点・有限性・可逆性（許容誤差）を検証。logの非正端点は拒否または定義済みpiecewise policyを使う。degenerate rangeはreadOnlyか明示constant。
- amplitude zeroのdB表記（−∞dB）は「表示の特別な状態」であり、finite control plain valueへ黙って `-Infinity` を流さない。
- NaN/±Infinity操作入力は拒否。範囲外有限値のclamp/拒否はdescriptor policyを明示、既存Plugin `ParamValue::new` のclamp規則をUI一般則と誤認しない。
- value rounding/quantizationは表示/入力の境界だけ。AudioLensのf64分析原値をf32/gpu表示精度へ書戻さない。

## 4. Interaction state machineと最小イベント契約

連続操作:

```text
Idle --begin--> Active --preview (0..N times)--> Active
                 ├─ commit(exactly once) --> Idle
                 └─ cancel(exactly once) --> Idle
```

GestureId/OriginView/ControlId/LogicalTarget/Sequenceの概念を持つ（型の所有先・wire表現は#258/#261で確定）。Beginで対象を確定し、途中のFocus/Selection変化で勝手にtargetを差替えない。Previewはローカル暫定値。Commitはユーザー操作の**終了意思**であり、Controller/Engine適用完了ではない。

| イベント | 不変条件 |
| --- | --- |
| Begin | 対象と入力元を固定、同一Controlの重複activeを明示拒否/整理 |
| Preview | 入力値を検証。連続値のみcapabilityの範囲内でbounded coalesce可能 |
| Commit | 正当な終端。Exactly-once発行、配送失敗時は結果を明示 |
| Cancel | Escape/lost-capture/unmount/dock transfer等。未確定previewを廃棄し、Product真値を編集しない |
| Discrete action | Play/Pause/Stop/Resume等は1発Command。連続Gestureと同一モデルに無理に押し込めず、non-drop result/receiptを扱う |

Wheelでの連続入力は開始・終端の境界を明示し、IME/text-editorの入力はグローバルTransport/DAWショートカットより優先する。Keyboardは矢印/Home/End/Page/文字入力等をコンポーネント役割別に定義し、Pointer/touchと意味を揃える。

## 5. State ownership・Gateway・Receipt

| State | Owner | 永続性 |
| --- | --- | --- |
| hover/focus/drag/preview | component local | transient |
| logical View selection/zoom/viewport | Consumer View State | save/restoreはConsumer |
| Project/Parameter/Transportの確定状態 | Consumer Controller / Engine（適切な責務別） | domainに従う |
| Analysis Dataset / validity / provenance | AudioLens | 永続化Dataset |
| Command auth/target/revision/Undo | Consumer Command Router / Gateway | consumer responsibility |
| render cache/GPU objects/stream subscription | renderer/adapter | transient; lifetime bounded |

UIのプロトコル状態として、少なくとも `idle/preview/pending/accepted/applied/rejected/unsupported/unavailable` の違いを表現できること。**accepted ≠ applied ≠ observed**。Device/Pluginの実観測が不可能な場合はunavailable/unknownとして示す。Pendingがtimeoutする場合も確定値を捏造せず、revision再取得または明示的failureを選択する。

Binding portは `dispatch(commandIntent)`, `subscribe(stateProjection)`, `query(viewportRequest)`, `cancel(queryId)` の**能力概念**を持てば十分。汎用のRuviform Command Gateway/State Storeを全製品へ強制しない。Product adapterがドメインID・権限・expectedRevision・タイムスタンプを管理する。

高頻度coalesceは同一Target/Generationの連続値preview等、順序置換可能で仕様上安全と判明したイベントに限る。Commit/Cancel/Transport/Record/Undo/非冪等操作はlatest-winsへ渡さない。Backendがsample-accurateに対応するかはRuviform #186/#190で判定し、UI表示・ドラッグ周期から精度を推定しない。

## 6. Visualization component / Source / Cache

- **Waveform**: `ruviform-viz::WaveformSeries` のLOD/直読みに依存し、zoom/pan/channel、selection/markersは表示層・Consumer annotationを分離。
- **Spectrum / Spectrogram**: `SpectrumSource`/Tileから必要範囲を読出し、複素STFT再解析を必須とせず、周波数軸/dB/colormapを表示変換で行う。
- **Meter**: `ruviform-audio-analysis::LevelMeter` 等の結果を非RT側broker経由で表示。測定rateとrender fpsを分離し、hold/clip/stale/loss/unavailableの意味を明確化。
- **Feature/Time-Series**: 値・系列・単位・timebase・gap・汎用error/uncertainty visualization slots。AudioLens validity/confidence/support/provenanceの判定・変換・表示文言はAudioLens Adapterに残す。
- **Timeline**: 既存Viewport/FrameGrid/SampleTimebaseを利用し、DAW musical timeなどの異なるtimebaseを無条件に浮動小数へ丸めない。

### 6.1 Query要求（概念）

```text
ViewportQuery {
  source_id, source_revision, view_instance_id, request_generation,
  time_range, frequency_range?, channels?, resolution_px?,
  max_bins?, max_bytes?, encoding_capabilities?
}
Reply {
  same source/revision/request_generation, data or explicit error,
  validity / missing-samples indicators where available
}
```

- 要求ごとに**表示範囲・表示解像度に応じた有界読出し**を行う。全PCM/STFT Datasetの一括転送を要求しない。
- Cache keyはSource identity・Revision/Generation・LOD/transform条件・time/freq rangeを考慮。Window/Dock移動/閉鎖・Source切替・古いResponseで誤再描画しない。
- 既存 `rvf://data` はimmutable named sourceをbounded range-pull（既定最大16 MiB/request）し、frontendは実返却byte数によってpagingする。追加メタデータ/wireが必要なら互換・version・capability検出を先に文書化。RVBW/RVSTの互換を勝手に破らない。
- メモリ・GPUテクスチャ・pending query数には設定可能な上限を持ち、cancel/generation invalidation/eviction後もresourceを回収する。
- NaN欠測を0へ変換して描画しない。無効/部分/未観測はgap/error/annotationに明示。表示用量子化は永続原値に影響しない。

## 7. Presentation / Theme / Native

- **WebView主経路**: SolidコンポーネントとしてDOM Controlsを実装し、波形/高頻度Meter等はCanvas2DまたはWebGL2を実測で使い分ける。in-WebView WebGPUを共通前提にしない。
- **Theme**: CSS custom properties/design tokens（color、font、size、spacing、density、motion、focus）、slot/variant/formatter。Synvelis/AudioLensは製品テーマのみ提供しGesture/Value Algorithmをforkしない。Contrast、High-DPI、reduced motion、focus ringを試験。
- **Native**: `ruviform-ui::render::NativeSurfaceHost` は現在portであり、production widgetの実装ではない。WebView2の隣接surface/z-order/DOM overlay制約を尊重。Native対応は独立PoC #262で必要性/利益/劣化を計測後、具体コンポーネントだけ採用。全Native Controlsを先行実装しない。
- **Accessibility**: Slider等のrole/name/range/current value/ARIA-valuetext、キーボード、Pointer/touch、IME、disabled/readOnly/error、screen-reader用代替情報はコンポーネントの契約。Canvas/WebGL表示に表示専用の数値データを置く場合も検索可能なテキスト・表・要約を提供する。

## 8. Multiwindow / Dock / Lifecycle

既存 `WindowId` / `ViewInstanceId` / Dock Ownerを尊重。View moveはDOM/GPUの移設ではなく新Surface再生成。編集中Gestureはmove/unmount時に定義済みCancel/Commit policyに従い、旧Viewから再送しない。復元後は正本Stateを再読出しし、subscriptionを新generationで構築する。

Window close、View unmount、renderer backend lossにより、Project、Engine、長寿命Import/Render/Analysis Jobを暗黙に取消・変更しない。View state（Selection/Zoom等）はConsumerの保存責務。複数Windowで同Targetを編集する場合の優先/競合/UndoはConsumer Controllerの責務。

## 9. 段階的導入・Package/Issue境界

| Phase | Ruviform Issue | 提供物・Gate | Consumer |
| --- | --- | --- | --- |
| 0 | #257 | 本設計、責務・DAG・API保留点の記録 | Synvelis #47/#48、AudioLens #131/#132 |
| 1 | #258 | Headless Gesture/Value + deterministic tests | 両Consumerのmock fixture |
| 2a | #259 | Solid Controls/Theme/A11y | Synvelis #47・AudioLens #131 |
| 2b | #260 | Visualization/WebGL2/Canvas + bounded query | Synvelis #48・AudioLens #132 |
| 3 | #261 | Generic Binding, receipt, generation/lifecycle | 4 Consumer Issue |
| 4 | #263 | Showcase+3OS CI + consumer E2E/precision/perf gates | #219 / Synvelis #28 / AudioLens #127 |
| Conditional | #262 | Native adjacent surface PoC / measured adoption decision | 必要なConsumerのみ |

Phaseは厳格な直列ではない。#259/#260は#258の契約確定後に並行でき、#261のport設計も並行可能。#262は#259/#260の実測から必要性を判断する。

### 9.1 Consumer Gate

**Synvelis**:
- Fader/Knob/Transport/Seek→明示Track/Bus/Plugin Target→Command/Receipt、Automation Touch/Latch/Write/UndoをDAW側で所有。
- Arrangement/Mixer/Inspector/Signal FlowへのWaveform/Meter/Spectrum/Timeline挿入。2WindowのDock move/Focus/Plugin GUI操作で誤編集なし。
- Live meterのstale/lossを認識し、GUI更新がRTをblockせず、UIだけでGraph再構築しない。

**AudioLens**:
- Source/Run/Analysis revision付きSeek/Range/Query、job管理はheadless serviceと一致。
- 永続f64 Analysis Datasetのwaveform/spectrogram/featuresを範囲読出しし、保存複素STFTの再解析を必須としない。
- Validity/Confidence/Support/Provenance、partial/unavailable、ambiguous alignmentの意味を欠落させず、無根拠な診断や補間をしない。CLIはGUI依存なし。

## 10. Test / Evidence / Compatibility Gates

| ID | Test/証拠 | 合格基準 |
| --- | --- | --- |
| UIC-01 | deterministic value mapping | finite・step・clamp/reject・log端点・表示−∞等が規約に合致 |
| UIC-02 | gesture reducer | Begin/Preview/Commit/Cancel, exactly-one終端, Escape/lost focus/unmount |
| UIC-03 | delayed/reordered/duplicate receipts | Pending/accepted/applied/rejected/observedを混同せず明示targetを守る |
| UIC-04 | 2 consumer theme and headless fixture | 同一操作ロジックを再利用、製品専用ロジックはadapterに隔離 |
| UIC-05 | Waveform/Feature f64/NaN/quantization golden | 保存原値不変、欠測を無断補間しない |
| UIC-06 | long-form ROI/LOD/STFT tile | 全量ロード無し、bounded cache/query/GPU、stale reply破棄 |
| UIC-07 | RT snapshot/meter load | RTにUI/IPC/alloc/blockを持込まず、loss/stale区別 |
| UIC-08 | WebView keyboard/pointer/ARIA/IME/DPI | 統一操作・明示Accessible Name・利用可能な代替表示 |
| UIC-09 | multiwindow Dock move / close | View ownership/resubscribe維持、Job/Engine/Projectに暗黙作用なし |
| UIC-10 | renderer/OS matrix | Windows/macOS/Linuxでbuild/testと実GUI検証を別集計、未観測明示 |
| UIC-11 | consumer compatibility (#219) | Synvelis/AudioLensを個別`impacted/no impact/unknown`分類し、未検証の追跡先を残す |
| UIC-12 | E2E | Synvelis Target→Receipt、AudioLens Dataset→View/Inspectで実動作証拠 |

定量目標は初回PoCで対象fixture/OS/GPU/CPU/解像度/同時View数のbaselineを取得して確定する。30/60fpsを目安にしても、未測定のCPU/GPU/latency/メモリ数値を保証値として書かない。p50/p95/p99 render-time、GPU/CPU負荷、bounded query数/byte、操作応答、RT影響を測定して比較する。

既存[#219](https://github.com/scarecrow173/ruviform/issues/219)のconsumer-verifyは情報レーンであり、この設計により必須checkへ暗黙昇格させない。Provider変更PRでは両Consumerの影響と実施/未実施を記録する。Consumer別の機能・精度・性能を一方の要求で暗黙に犠牲にしない。

## 11. 保留/PoCで決める項目

- 具体TS workspace/モジュール名・型名・package publish/semver。
- 実Custom mapping/formatterのserialization・型生成方針とRust/TS境界。
- Snapshot/Command ackの新wireが本当に必要か。必要な場合のバージョン/導入順と既存`rvf.ipc.v1`への影響。
- Level Meter UIの具体hold/clipアルゴリズムと観測保証・更新周期のConsumer別設定。
- Native/wgpuの本番採否・OS別surface composition/overlayの制限。
- 具体パフォーマンスSLO・GPU能力別の最適Renderer選択。
- AccessibilityのOS支援差分とプラグインGUI keyboard focusの詳細。

これらは設計方針の未決定ではなく、**公開API/PoCの根拠が得られた時点で確定する実装判断**である。採否は後続Issueに証拠を残し、受入Gateへの影響を反映する。
