---
id: "ADR-20261101-synvelis-driven-generic-execution-boundaries"
type: "adr"
status: "accepted"
title: "Ruviformの汎用Graph/Audio Execution/Stateful Processorとアプリ意味論の責務境界"
created: "2026-11-01"
updated: "2026-11-01"
owners: ["scarecrow173"]
metadata:
  adr:
    supersedes: []
    decision-makers: ["scarecrow173"]
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
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/media-io-backend-abstraction.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
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
    - "https://github.com/scarecrow173/ruviform/issues/189"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/191"
  defers: []
  deferred-by: []
---

# 汎用Execution境界の再定義（Synvelis-driven）

## Status

**Accepted — Architectural direction only (2026-11-01).** 本ADRは現行Audio Engine Snapshot SwapやPlugin RT Ownership、Media I/OなどのAccepted ADRを一律にSupersedeしない。ここで確定するのは再利用可能な実行基盤の**責務分離と必須不変条件**であり、Graph/Stateful Registryの具体API・新crate数・Sync方式はPoCで再判断する。

[詳細Target Design](../designs/synvelis-driven-target-architecture-and-migration.md)、[Migration #192](https://github.com/scarecrow173/ruviform/issues/192)、[Synvelis合意 #22](https://github.com/scarecrow173/synvelis/issues/22)。

## Context

Ruviformには28 crateのAudio/Video/Plugin/Media I/O/RT/UI基盤があり、これを汎用フレームワークとして維持する。Synvelisのv0.3〜v0.6要件ではTyped Multi-Port Graph、PDC、Timed Events、Variable PPQN、Stateful Plugin Plan切替、録音安全性、実WAV出力とStateful Offline Render、Window/Job寿命分離、Node/Port Probeが必要。

既存\`ruviform-audio\`のGraphは順序実行リストであり、GraphDefinition/Compiler/ExecutionStateの境界を持つ目標Runtimeへ移行する必要がある。一方、Plugin Host/Audio Engineがすでに本番依存循環を形成しているわけではない。現行Plugin Host→Audio Engineの参照はCargo \`dev-dependencies\`であり、過大なHost解体を前提としてはいけない。

## Decision

1. **Generic Owner:** RuviformはMedia/Time/Graph/Event/Plugin/Device/Offline/UI/Probeの汎用処理契約を所有。Project/Track/Clip/Take/Comp、Automation Write Mode/Undo、Remote Asset Provider/Import Commit、Stem/Freeze Recipe、Inspectorの診断解釈などDAWの制作意味論はSynvelisに置く。他ConsumerもRuviform基盤を単独利用できる。
2. **Semantic lowering boundary:** Synvelis Project Snapshot等からRuntime Graph/Timed Event/Media Sourceへ非RTでLoweringする。Ruviform GraphにSynvelis TrackIdやDAW-specific Stateを埋め込まない。
3. **Graph separation:** 非RT Graph IR/Port/Edge/Capability ValidationとAudio専用Execution Compiler/Buffer/PDC/Prepared Runtimeを異なる責務として設計する。Graph IRがHost/DeviceをInstantiateせず単独検証可能であること。独立\`ruviform-graph\` crate抽出の採否とAPIはPoCに委ねる。
4. **Single-stateful RT owner:** Immutable Schedule/Plan/GenerationとStateful Processor/Plugin Instanceの所有権・寿命を分離して考える。旧/新Planから同一Stateful Processorが並行実行されず、NonRT RetirementやTarget Generation検証を保つ。Plan-owned vs Stable Runtime Slotの最終選択はPoC。
5. **Plugin composition:** 非RT Plugin Host/RT Endpointを分離する既存Accepted ADRを維持。Plugin HostをAudio Engineへ通常依存させず、Audio EngineにもHost/特定Plugin ABIを直接依存させない。Port-level Adapter/Compositionを接続責務とし、Bridge専用crateの設置は条件付き。
6. **Prepared execution vs DAW state:** RuviformはGeneric Beat/PPQN/Tempo/Clock/Span/Timed Eventの処理、Codec/Prefetch/Output、RT Probe/Offline Driverを提供。Project Store/Recording Take/Job Registryや製品固有FeedbackのOwnerはSynvelis。Window CloseでProject Jobが暗黙に消滅しない。
7. **Incremental migration:** 旧Audio Graph/PlanSwapと各Consumerを互換Pathで維持し、新Typed RuntimeはMock-first PoCで検証。新API/Backend/Compiled Graphを実装済みと偽らず、RT Safety/Plugin ABI/Codecの受入Gateに合格してから段階切替。

## Alternatives

| Alternative | Decision |
| --- | --- |
| RuviformをSynvelis DAW Coreへ改造 | 他Consumer汎用性を失うため不採 |
| 現行28 crateの配置に全要件を合わせる | 責務の歪みと重複を生むため不採 |
| 全crateの一括再編・互換API即廃止 | 危険で回帰可能性を損ねるため不採 |
| 全候補crateを即新設 | 未検証の抽象分割・保守負担のため不採 |
| Audio EngineがPlugin Hostを直接依存 | 単体Engineと依存方向が悪化するため不採 |
| Stable Runtime Slotを最初から唯一方式として固定 | Sync/State/Retire検証不足のため保留 |
| **責務境界先行＋Compat＋PoC＋条件付き抽出** | **採用** |

## Compatibility and Consequences

- 現行[Audio Engine Snapshot Swap ADR](audio-engine-snapshot-swap.md)は依然Accepted。 \`RwLock<Arc<Plan>>\`/\`try_read\`を実際に置換するときには、PoC結果・互換影響・代替方式を示す**新しい後継ADR**が必要。本ADRを既存の同期方式を変更する許可と解釈しない。
- [Plugin RT Ownership Split ADR](plugin-rt-ownership-split.md)、[Realtime Contract ADR](realtime-contract-and-verification-policy.md)、[Media I/O Boundary ADR](media-io-backend-abstraction.md)、[UI Hybrid ADR](ui-architecture-webview-hybrid.md)は原則維持。
- [Variable PPQN successor ADR](configurable-ppqn-resolution-independent-musical-time.md)はAcceptedだが既存Codeは移行前。BeatPosition/APIの具体RangeはPoC対象。
- \`ruviform-edit\`はNLE汎用モデルとして維持し、SynvelisのDAW Track/Take/Compを強制統合しない。
- Stateful DSP OfflineのChunk並列は独立性の証明が必要。既存\`ruviform-exec::OfflineDriver\`の独立Chunk保証を無条件に拡大しない。
- RTでalloc/dealloc/無期限Lock・I/O/危険Dropをしない。不足Capacity/退役Overflow/NoteOff Lossを明示的にFault扱いし、実行しない実装を正常完了と表示しない。
- 段階移行・別Consumer互換のTest Suiteを先行し、必要なAPI Deprecation/Re-export/Migration Adapterを導入する。具体移動は#192を追跡。

## Confirmation / Validation

本ADRの方針をTA-001〜TA-012で検証する。特に、Cargo Production DAG非循環、Graph単体検証、Typed Audio Multi-Port/PDC、Plugin Host↔Audio Engine非循環、Single Stateful RT Owner、RT no-alloc/no-block/NonRT Retire、Event Generation、Variable PPQN/Transport、WAV Output、Window Job Lifetime、Legacy Audio/Video/Plugin/CLI/UI互換が必要。

詳細は[Target Design §7](../designs/synvelis-driven-target-architecture-and-migration.md)および[Synvelis #22](https://github.com/scarecrow173/synvelis/issues/22)。

**今は設計判断がAcceptedであるだけで、M0〜M7やTA-Gatesの実装・検証済みを意味しない。**
