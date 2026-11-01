---
id: "DESIGN-20261101-synvelis-driven-ruviform-target-architecture"
type: "design"
status: "approved"
title: "Synvelis要件を基準としたRuviform目標アーキテクチャと段階的Migration v0.7.1"
created: "2026-11-01"
updated: "2026-11-01"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/synvelis/issues/22"
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
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/media-io-backend-abstraction.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/synvelis/issues/17"
    - "https://github.com/scarecrow173/synvelis/issues/18"
    - "https://github.com/scarecrow173/synvelis/issues/19"
    - "https://github.com/scarecrow173/synvelis/issues/20"
    - "https://github.com/scarecrow173/synvelis/issues/21"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/186"
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/189"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/191"
  defers: []
  deferred-by: []
---

# Ruviform Target Architecture & Migration v0.7.1

**Status: Approved architectural direction (2026-11-01).** 本書はSynvelisの全体要件を満たすためのRuviform汎用責務・依存境界・段階的Migrationを合意済みベースラインとして記録する。ただし**実装、最終crate数、公開API名、具体RT同期方式、PoCと性能は未確定/未実施**。概念API/構成図は現存Rust型を主張しない。

- Tracking: [Ruviform #192](https://github.com/scarecrow173/ruviform/issues/192) / [Synvelis #22](https://github.com/scarecrow173/synvelis/issues/22)。
- Synvelis側正本: [Synvelis-driven Ruviform Target v0.7.1](https://github.com/scarecrow173/synvelis/blob/main/docs/designs/synvelis-driven-ruviform-target-architecture-v07.md)。
- 先行承認: [Synvelis #17 Project](https://github.com/scarecrow173/synvelis/issues/17)、[#18 Graph](https://github.com/scarecrow173/synvelis/issues/18)、[#19 Time](https://github.com/scarecrow173/synvelis/issues/19)、[#20 Events](https://github.com/scarecrow173/synvelis/issues/20)、[#21 Media/Export/UI](https://github.com/scarecrow173/synvelis/issues/21)。

## 1. 現行実装について確認したこと

現在のCargo Workspaceには28 crateがあり、Primitives、Domain Core、Media I/O、Audio/Video Engines、Plugin Host/CLAP/VST3/OFX、UI/Viz/CLIが存在する。全体設計は[旧Approved Baseline](ruviform-architecture-design-baseline.md)を参照。新設計は旧Baselineを暗黙に廃止しない。将来の公開DAG/層構造に関する差分は専用ADR/Docによって段階的に反映する。

**訂正済みの依存事実:**

- [plugin-host/Cargo.toml](../../crates/ruviform-plugin-host/Cargo.toml)の \`ruviform-audio\` は**\`dev-dependencies\`**にのみ存在し、通常の\`[dependencies]\`にはない。現状すでに通常依存のHost→Audio結合があると誤認しない。
- [plugin-host/endpoint.rs](../../crates/ruviform-plugin-host/src/endpoint.rs)の\`PluginEndpoint::process_audio\`が直接使うのは\`ruviform-audio-core\`のBuffer等の型で、\`ruviform-audio\` Engineの具象型を必要とするわけではない。
- [audio/graph.rs](../../crates/ruviform-audio/src/graph.rs)は現在\`AudioNode::pull\`の順序リストで、型付き複数Port/BusのGraph IR・Compilerの完成実装ではない。
- [audio/plan.rs](../../crates/ruviform-audio/src/plan.rs)は\`RwLock<Arc<GraphPlan>>\`/\`try_read\`とRTのlast-good cache、非RTの\`retire_channel\`/collectorを利用。退役OverflowでRT Destructorを避けるためリークするFail-safeと計数がある。新実行系では容量・Admission・Reclamationの明示検証が必要。
- [plugin-core/event.rs](../../crates/ruviform-plugin-core/src/event.rs)のEventListはFrame Offset昇順と同Offset順序を保証するが、現行EventKindはParamValue中心。Timed MIDI/Note Ledger/Target Generationの完成実装ではない。
- [media-core](../../crates/ruviform-media-core/src/lib.rs)の現在のMusical Timeは旧固定960実装。後継[可変PPQN ADR](../adr/configurable-ppqn-resolution-independent-musical-time.md)はAcceptedだが移行未了。
- [exec/offline.rs](../../crates/ruviform-exec/src/offline.rs)は独立Chunkの場合にのみ並列決定性を保証。Stateful DSPを任意時点から並列実行できることは保証しない。
- [observability](../../crates/ruviform-observability/src/lib.rs)は現在Counter Pair主体、Node/Port Probeは[#185](https://github.com/scarecrow173/ruviform/issues/185)で設計/PoC。
- [edit](../../crates/ruviform-edit/src/lib.rs)はNLE用汎用非破壊編集モデル。Synvelis Track/Take/Comp/DAW Undoの実体として再利用しない。
- [ui](../../crates/ruviform-ui/src/lib.rs)、[viz](../../crates/ruviform-viz/src/lib.rs)は汎用ホスト/表示Primitiveを保有。JobやInspectorの製品意味論はアプリ側。

## 2. System Ownership Boundary

| Capability | Ruviform owns | Synvelis owns |
| --- | --- | --- |
| Time | Rational Beat/PPQN/TempoMap/Clock Conversionの汎用型・検証 | Project PPQN初期値960、Timebase選択、Timeline/Clip Source Mapping |
| Graph | Typed Node/Port/Edge/Validation、Execution Schedule/Buffer/PDC | Track/Bus/Send/Automation等Semantic Graph、ID対応、Project Command |
| Processor | RT/NonRT Lifecycle、Prepared Endpoint、単一RT実行Owner | Pluginを含むProject Instance Identity、UndoとPersistence意味 |
| MIDI/Control | Sample Offset Event、Transport Span、Bounded Delivery、Plugin Capability | Note Source/Editing/Recording、Automation Write/Touch/Latch、Comp |
| Media | Container/Codec/Reader/Decoder/Encoder/Muxer/Prefetchの汎用能力 | Remote Provider、Pinned Asset、Project Verified Blob、Source/Asset/Clip登録 |
| Render | OfflineDriver、Stateful Runtime Execution、Output Writer | Master/Stem/Bounce/FreezeのRecipe、Project Commit |
| Job/UI | Generic Executor/UI Host/Dock/Bounded Binary Channel | Application/Project JobRegistry、Inspector/Workspace/Browser |
| Diagnostics | Generic Counter/Probe/Frame/Generation/Loss Evidence | 原因分析/Provenance/Signal Flow UI/Configured vs Inferred |

**非目標:** RuviformからSynvelis Project/Track/Take/Comp/Automation Write Mode、Git/P4/S3 Provider固有型、Export RecipeやUndoを要求しない。Plugin ABI、Audio Device、OS/UIの具象実装をCoreの必須依存へ持ち込まない。

## 3. Target Capability DAG（主要実行依存）

~~~mermaid
flowchart TB
  S["Synvelis / Other Consumers"]
  C["Runtime Composition / Glue"]
  G["Graph IR / Validator (candidate crate)"]
  A["Audio Compiler / Prepared Engine"]
  H["Plugin Host / RT Endpoint"]
  P["Plugin Core"]
  R["rt Primitives"]
  AC["audio-core"]
  M["media-core Time"]
  E["exec / Offline"]
  I["media-io + codec adapter"]
  O["observability"]
  S --> C
  C --> G
  C --> A
  C --> H
  C --> E
  C --> I
  A --> G
  A --> R
  A --> AC
  G --> M
  G --> AC
  H --> P
  H --> R
  P --> AC
  P --> M
  E --> R
  A --> O
  H --> O
~~~

矢印=production crate「利用側→依存先」。これは**主要DAGの設計概念**であり、全28 crateの完全なCargo edge列ではない。将来のM0では通常/Build/Dev/Optional/Feature別にCargo metadataから完全DAGを生成し、実API/feature/ターゲット別に確認する。

禁止する**通常依存**: Foundation→Synvelis、Graph IR→Audio Engine/Plugin Host/GUI、Audio Engine→Plugin Host、Plugin Host→Audio Engine、Observability Primitive→重いPlugin/Graph/UI依存。既存テスト用\`dev-dependencies\`の上位参照を同じ意味の循環と扱わない。FFmpeg/CPAL/CLAP/VST3固有型はAdapterに閉じる。

## 4. 既存crateの維持・拡張・条件付き抽出

| Current | Target disposition | Change/notes |
| --- | --- | --- |
| math / numeric | Retain | SIMD/Offline DSP Primitive、独立運用 |
| rt | Retain + extend | RT契約、所有権移譲、退役容量/Fault |
| media-core | Extend | Rational Beat/Validated PPQN/TempoMap/Time、個別time crateは条件付き |
| audio-core / video-core | Retain + extend | Buffer、Format、Device Port等の意味的型 |
| video-proc / video | Retain | Video EngineをDAW Audio DAGに従属させない |
| dsp / audio-analysis | Retain | DSPと分析、rt/nonRT契約を明記 |
| media-io / io-symphonia / io-avi | Retain + extend | Decode基盤維持、Encode/Mux/Finalizeの実Backend追加 |
| audio-device | Extend | Input/Duplex/Clock Epoch/Latency、Adapter依存隔離 |
| plugin-core | Extend | Event/MIDI/Param/Capabilities、規格中立 |
| plugin-clap / plugin-vst3 / plugin-ofx / plugin-export | Retain + extend | 各ABI Adapter/Runnerを隔離 |
| plugin-host | Retain + refine | NonRT Control/RT Endpoint保持。具象Audio Engineへの通常依存なし |
| audio | Major redesign | Ordered Graphの互換Path維持。Typed DAG Compiler/PDC/Plan/RT State |
| edit | Retain | NLE modelのまま。Synvelis DAW Projectには流用しない |
| exec | Retain + extend | OfflineDriverは汎用Primitive。Audio Stateful RenderはCompositionで解決 |
| observability | Extend | Counter→RT bounded records/Node Port Probe/NonRT collector |
| ui / ui-webview | Extend | Window/Close/IPC/Job Scope、不要なDAW型なし |
| viz | Retain + extend | Waveform/Spectrogram/Time Projection/Ranged Binary |
| cli-sample | Retain | 汎用能力のCompatibility Consumer |

**新規/分離候補の採否:**

| Candidate | Decision status | Validation |
| --- | --- | --- |
| \`ruviform-graph\` | 抽出有力、未確定 | Graph IRがDevice/Hostと独立・異なるConsumerが必要 |
| \`ruviform-io-wav\`相当 | 実Backend新設推奨、命名未確定 | 実PCM WAV Encode/Finalize/Reopen、Capability/Test |
| \`ruviform-audio-plugin-bridge\` | **条件付き・即新設しない** | Consumer内Glueや既存port adapterより独立性の利益が大きいか |
| \`ruviform-time\` | 当面新設しない | media-core内で型/Ownerを先に安定 |
| \`ruviform-audio-render\` | 当面新設しない | Audio Engine/ExecへのCompositionでPoC |
| \`ruviform-job\` | 当面新設しない | exec＋Application/Project Job RegistryでPoC |

## 5. Core API Contract（概念モデル）

### 5.1 NonRT Graph

GraphSpec = Node/Port/Edge/SignalDomain(Audio/TimedEvent/Control)/Role/Direction/Layout等。GraphValidatorは接続方向、Multiplicity、Type/Format compatibility、Cycle/Delay legality、必要能力を検証。Graph定義はPlugin HostやDeviceをInstantiateせず検証可能。

Graph IRは構造/論理Stable Idを持つが、Synvelis TrackIdなどのDAW型を持たない。Channel/Rate Conversionは明示的なLink/Conversion Planか許可済みCompiler TransformationとしてEvidenceに残す。

### 5.2 Audio Compiler / Prepared Runtime

非RT Graph CompilerはValidated Graph＋Node Capability/Latency/SampleRate/MaxBlock/Port Layout/Budgetを受け、Execution Schedule/Buffer Lifetime、PDC、Timed Event Routing、Resource Estimates、Diagnosticを生成。RTは準備済みSchedule/Buffer/Endpointで有界処理。CompilerはAudio Device開通/Plugin Host共有状態を暗黙制御しない。

**Immutable Prepared Schedule ≠ immutable Stateful Processor**。RT Processor Instanceは状態付きの単一実行Ownerが更新。旧/新Scheduleが同一Instanceを重複実行しない。Plan Generation、Plugin Instance/Target Binding Generation、TransportGenerationを混同しない。Retirementは非RTに回収する。

### 5.3 PlanSwap Migration

現行[Audio Snapshot Swap ADR](../adr/audio-engine-snapshot-swap.md)は\`RwLock<Arc<Plan>>\`/\`try_read\`をAcceptedとして採用。これを設計合意だけで置換しない。新Typed Runtimeは互換Pathと並行PoC:

1. 現行PlanがProcessorを所有。
2. 旧/新Scheduleから独立したStable Runtime Slot Registry（推奨候補）。

双方でSend、RT NoAlloc、無限待機なし、退役、Plugin State維持、Plan/Event Generationの安全性、観測できる適用時点、Worst-caseの処理コストを測る。方式の決定はPoC結果および必要な**後継ADR**による。Silent Gap-free Swapは無条件保証しない。旧Retire QueueのOverflowではRT Dropを避けるが、永久的Leak正常運用にならないAdmission/Drainを設計する。

### 5.4 Transport / Event / Plugin

- \`media-core\`: Canonical Rational Beat、validated variable PPQN（Synvelis default=960はアプリPolicy）、immutable TempoMap、explicit rounding、Frame/MediaTime converter。
- RT Transport Span: Command/Prepared/Applied Receipt、Half-open [start,end)、ClockEpoch vs TransportGenerationを区別。
- Timed Event: target/plan/transport generation、frame_offset、Stable Same-frame Source Order、Loop Occurrence、EventKind別Fault Policy。旧Plugin EventListのOffset/Zero-frame flush互換を守る。
- MIDI: Raw Native Source Timestamp保全、NoteLedger/CC64/NoteOff Lossの安全対応。全Plugin内部Voiceの完全保証を謳わない。
- Plugin Host: NonRT Host Command/SaveState/RestoreState＋RT Send Endpointを維持。Audio Graph AdapterはAudio RuntimeとHostの接着を行い、CLAP/VST3/OFX固有型をCoreへ漏らさない。State Capture Consistency/Active Ownerを分けて評価。

### 5.5 Media / Offline / UI / Diagnostics

- Source resolution: Synvelis Stable Asset/Source ID→Verified Blobまでアプリ。RuviformはGeneric Source/Decoder/NonRT Prefetch/RT Bufferを担当。
- Offline: Stateful DSPはSequentialに評価し、独立性が確認されたChunkだけ並列化。Liveと同じTime/Event semanticsを維持し、別Plugin Instanceを使用。
- Output: Encoder→Drain→Muxer Finalize→Output Validation→Safe Publish。未完成OutputをCompletedとしない。[WAV output #191](https://github.com/scarecrow173/ruviform/issues/191)がP0。
- Job: Generic Executor/Cancel/ProgressのPrimitiveはRuviform。Synvelis Application/Project Job RegistryがRender/Import/Freeze/Analysisを所有。Window CloseとProject Job Cancelは独立。
- Probe: Node/Port/Graph/Frame/Generation/Validity/Lossの汎用Evidenceを保持。製品固有Inspectorの原因推定はSynvelisが担当。

## 6. Migration M0–M7

| Stage | Implementation target | Gate / rollback |
| --- | --- | --- |
| M0 | 28 crateの通常/dev/optional/feature DAG、Public API/ABI、Known Consumers、Accepted ADR、Baseline tests | Legacy build/testとDAG inventory。ベースラインからRollback可能 |
| M1 | Typed Graph IR/ValidatorをAudio Host/Deviceに非依存に実装 | Topology/Port/Cycle/Sidechain/FormatのMock PoC。既存Graph維持 |
| M2 | Audio Compiler/Buffer/PDC/Prepared Runtime、Stateful Ownership PoC | Fan-out/in、PDC、Retire、NoAlloc。Legacy PlanSwap保持 |
| M3 | Variable PPQN/Beat/Transport/Timed MIDI/Parameter Contract | Project/Loop/Seek/Frame/Event test（#186/#188/#189/#190） |
| M4 | Audio Plugin Glue、L0/L1/MultiBus、Latency/State/Generation | 同一Stateful instanceの重複RT実行なし。Plugin/Audio互換 |
| M5 | Audio Input/Duplex/Clock、Media Reader/Prefetch、Recording/Playback | Mock Device Raw Timestamp/Fail-stop/Placement/PDC |
| M6 | PCM WAV実Encoder/Muxer、Stateful Sequential Offline Render、Job、Publish | E2E再Open/Validate、Cancel/DiskFull/NoOverwrite |
| M7 | MultiWindow/Docking、Probe、Compat Path Cutover、Deprecated API Cleanup | Legacy Audio/Video/Plugin/CLI/UI、RT/Adapter tests |

**並列性:** WAV Backend、Device Duplex、UI Docking/Probeの独立PoCはM1–M3と並走可能。M0–M7は厳格な単一列ではなく依存DAGを示す。

**最初のProduction-oriented vertical slice:** Local WAV Import→Verified Blob→Synvelis Track/Clip→Typed Audio Graph→Playback→Offline Master WAV Encode/Mux→Reopen Validation。高度なPlugin/Remote/Stem/Freeze/Multiwindowは最初のSliceの必須条件ではないが、Time/Graph/Mediaを暫定専用モデルで二重実装しない。

## 7. TA-001〜TA-012 Architecture Gates

| ID | Required evidence |
| --- | --- |
| TA-001 | 本番通常依存のCargo DAG非循環、dev/optional/feature別監査 |
| TA-002 | Foundation/Graph/Coreの公開型にDAW Project/Track/Take/Undoの混入なし |
| TA-003 | Graph IR/ValidatorをDevice/HostなしでBuild/Test |
| TA-004 | Fan-out/in、Sidechain、Explicit Mapping、PDCとBuffer Lifetime正しさ |
| TA-005 | PluginHostはAudio Engine通常依存なし、Audio Adapterは正方向のみ |
| TA-006 | 同一Stateful Plugin/DSPが旧/新RT Planから同時にProcessされない |
| TA-007 | RTにAlloc/Dealloc、Blocking、OS I/O、非RT Destructorがない（既存RT test＋review＋stress） |
| TA-008 | Plan/Transport/Target/Instance Generation不整合なEventを誤配送しない |
| TA-009 | Variable PPQN/Beat/Tempo/Loop→Frame/Event PrecisionとStable Order |
| TA-010 | 実PCM WAV Encode→Finalize→Reopen→Sample/Duration Verify |
| TA-011 | Window CloseによるApplication/Project Job無断Cancelなし |
| TA-012 | Audio/Video/CLAP/VST3/OFX/CLI/UI等既存ConsumerがCompatibility Pathで回帰 |

関連テスト: Synvelis EX-001〜020、TT-001〜024、MR-001〜028、PPQN-01〜12、およびRecording Vertical Sliceの既存AC。正式な完了は**テスト実行結果**に基づく。GitHubのIssue closeやDesign Approvedを実装完了の証拠としない。

## 8. ADR / Compatibility Policy

| Existing ADR | Treatment |
| --- | --- |
| Realtime Contract / RT ownership | 維持。新しいProcessor/Schedulerにも適用 |
| Plugin RT Ownership Split | 維持。HostとAudio接続はPort Adapterとして補足 |
| Audio Snapshot Swap | **維持・再評価**。方式を変更するときだけ後継ADR |
| Media I/O Backend Abstraction | 維持。WAV実Backendで必要なAPI変更のみ判断 |
| Variable PPQN successor | Acceptedのまま実装移行（#189） |
| UI WebView Hybrid | 維持。Window/Job ScopeのPoC結果で補完 |
| NLE Editing Model | 維持。Synvelis Track/Compを持ち込まない |
| General Architecture Baseline | 目標Design/Accepted boundary ADRで補足。現行実装の履歴を保持 |

**未決定として保留:** 新Graph crate正式採用/公開API、Bridge独立crateの必要性、Stable Slotの実装方式、旧PlanSwapの置換、PPQN/分母上限、RT/span/event容量、Plugin各ABIの実Adapter制限、Codec/FS安全なPublish/Recover、UI native surfaceの形態。PoCを先行し、必要なADRを作成してからAPI互換破壊を行う。

## 9. Rollback / Recovery / Incremental Adoption

- 新Graph IR/Typed RuntimeはまずMock device/feature-gated新APIとして独立実装。旧Graph/Session/PlanSwapを急に削除しない。
- NonRT Prepareに失敗した場合はActive Last-good Planを維持し、Requested/Prepared/Applied/Rejectedを明確に区別。
- Plugin State/Native Source/Project Saved RevisionはMigrated stateと別保存。旧Source Snapshotを直接破壊しない。
- Retire Queue Saturation、Stale Event、Clock Epoch断絶、File Finalization失敗は明示Fault/Degraded。RT安全なFailureとユーザーへ観測できるStateを分離。
- 互換APIのdeprecationはConsumerの採用と実テスト後。Video/CLI/既存Audio Consumerの後方互換を破壊してまで未検証の設計を強制しない。

**結論:** 全28 crateの総作り直しは不要。Graph IR/Audio Runtimeの責務分離、Stateful Processor Owner、Time/Event/Media/Jobの横断接続を合意済みの目標とし、具体APIやcrate抽出はPoCで最終化する。
