---
id: "SPEC-034QkwrLNvylpSOCZVs2bC"
type: "spec"
status: "proposed"
title: "Issue #186 Parameter Control Events — capability, delivery and observability contract"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/186"
    - "https://github.com/scarecrow173/synvelis/issues/6"
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
    - "docs/specs/issue-190-transport-span-scheduler.md"
    - "docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/specs/issue-189-rational-beat-variable-ppqn-m3.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-186-param-control-events-inventory.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-186-param-control-events-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/synvelis/issues/20"
    - "https://github.com/scarecrow173/synvelis/issues/22"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/188"
  deferred-by: []
metadata:
  gate: "M3"
  issue: 186
---

# Issue #186 Parameter Control Events — capability, delivery and observability contract

## What Should Be Built

アプリが算出した（または将来の汎用 Control Processor が算出した）制御結果を、対応する処理対象へ**安全かつ検証可能な精度で伝達する最低限の API/Capability 契約**を確定し、不足分を PoC 実装・検証する。

1. **Capability Matrix（パラメータ単位）**: `ParamInfo` + adapter 能力から「受け付ける更新手段（timed EventList / block-boundary のみ / per-note）・利用可能精度・`READONLY`/`STEPPED`/`AUTOMATABLE`/`MODULATABLE` の意味・配送保証・fallback」を照会する契約。
2. **Parameter ID 境界**: plugin ローカル `ParamId` とアプリ内安定 `TargetId` の写像、binding generation、未解決/無効/復元失敗の状態表現（DAW 固有 ID モデルは Synvelis 所有）。
3. **配送契約の実 API 仕様**: `TransportPlan<EventKind>` → `PluginNode` → `EventList` 経路を正本とし、同一 offset 競合・0-frame flush・block 境界・容量超過を明文化。
4. **タイムスタンプ精度**: `BeatPosition`/`MediaTime` → block frame offset の変換規約（`TempoMap::beat_to_media` 単一 NearestEven 丸め）、テンポ変更・Loop 境界・処理レイテンシ・offline/live 再現性。
5. **defined rejection**: 非対応精度・パラメータ型・Plugin 状態で無言の誤適用をしない。
6. **ホスト解決値の可観測性**: scheduled / dispatched / plugin-reported の区別と、評価時刻・revision・欠落・非対応の照会（Plugin 内部真値は無保証、#185 Probe 契約と混同しない）。
7. **規格差分の正直な宣言**: CLAP/VST3/L0/L1 の ParamEvent・sample-accurate/per-note・latency・量的上限をコードと PoC で実測し、誇大な互換性保証をしない。
8. **汎用 Control 評価の判断**: Source→Transform→Target DAG 共通化の実需評価 — 本 Issue では判断と境界契約のみ（実装は分割 Issue 候補）。

## Why Now

Synvelis #6 の Reference Value→個別 Source 寄与→Host-resolved Value 評価が Ruviform 側の配送・能力・時刻契約を必要とする。#189（BeatPosition/TempoMap）・#190（span scheduler/Receipt/stale 切断）・#196（bridge/PluginNode/capability matrix）がマージされ配送骨格は揃ったが、パラメータ単位の能力照会・Target 写像・観測区別・coalescing・defined rejection は未実装で、本 Issue が G3（M3）受入証拠を出す位置にある。

## Users and Value

- **Synvelis**: ParameterTarget への timed 配送、Capability 照会による UI への制限返却、Host-resolved/dispatched/reported の区別表示。
- **audio-lens 等の他 consumer**: 同一の中立契約で plugin param を安全に制御。
- **Ruviform 自身**: M3 gate（G3/G4）の受入証拠。

## Scope

### In Scope

- 棚卸し記録（discovery doc、実コード根拠付き）と DAW 固有部分の分離。
- Capability Matrix / 正規化値 / Time Domain / 同一 offset イベントの実 API 基づく仕様記述。
- plugin-core へのパラメータ write 契約型（更新手段・defined rejection・同一 offset 規約）追加。
- bridge への TargetId↔ParamId binding・dispatch ledger（scheduled/dispatched/reported 区別）追加。
- `cap.rs` のパラメータ精度差分拡張（正直な宣言）。
- Mock/代表 adapter（L0 in-process plugin + bridge PluginNode）での配送検証テスト。
- 時間変換/Loop/tempo change/block 境界/非同期更新の再現テスト。
- 非RT 準備/RT 有界評価/旧 Plan 維持/retire の PoC と `rt_` test lane 整合。

### Out of Scope

- Synvelis 固有: Track/Automation Lane/Curve/Region/Touch/Latch/Write/Gesture/Macro/LFO プリセット/Inspector UI/Project Undo/既定加算合成ポリシー。
- 新規 Control Graph クレート・汎用 Modulation Source 群の先行実装（判断のみ。実装は実需が示された場合に分割 Issue で ADR 提案）。
- Plugin 内部 DSP 制御を超える観測保証。全 Plugin での sample-accurate/audio-rate modulation の無根拠有効化。
- plugin→host パラメータ報告経路の実装（観測区別の API 形は定義するが、CLAP output events / VST3 outputParameterChanges の配線は分割 Issue）。
- MIDI/Note 系イベント（#188 の責務）、Audio Bus/Channel Routing（#184 とは別問題）。
- L1 wire protocol への新 verb 追加（現行 wire で表現可能な範囲に留める）。

## Requirements

- R1: パラメータ単位で「受け付ける更新手段・精度・保証・fallback」を照会できる。`Unknown`/`Declared` は supported 扱いしない（capability honesty）。
- R2: `ParamId`↔`TargetId` 写像に resolved/unresolved/defunct の状態と binding generation を持つ。未解決配送は defined rejection。
- R3: Timed ParamEvent は `TransportPlan<EventKind>`/`install_plan` 経路を正本とする。`NodeEvent` は param 配送に使わない。
- R4: 同一 offset・同一 param の複数イベントは安定順（source_sequence 順）で配送され、適用意味論（last-wins）は中立契約として明文化する。
- R5: `READONLY`・非 `AUTOMATABLE`（timed 用途）・per-note/即時など未サポート手段への書込みは defined rejection を返す。
- R6: `STEPPED` パラメータへの連続値適用は量子化規則が明示できる場合のみ。規約未定義の量子化は行わない。
- R7: `BeatPosition`/`MediaTime` → frame offset 変換は `TempoMap::beat_to_media`（単一 NearestEven 丸め）を権威とし、offline/live で同一結果。
- R8: 容量超過は defined fault（既存 `EventBudgetExceeded`/`EventScratchOverflow`/`VaultExceeded`）か、明示 opt-in の continuous-param coalescing（同一 param の連続値のみ・最新値残し）に限る。不可逆イベント・別 param を巻き込む coalescing は行わない。
- R9: 非RT での binding 検証・plan 構築・capability 照会、RT での有界配送は no-alloc/no-block/no OS I/O。旧 Plan は vault で維持し retire は非RT。
- R10: host-resolved value の追跡は `Scheduled`（plan 内 pending）→`Dispatched`（物理 call へ frame offset 確定）→`Reported`/`Unavailable` の区別を照会でき、評価時刻・revision を持つ。Plugin 内部真値は保証しない。
- R11: RT パターンは既存世代タグ切断（plan/transport/binding generation, clock epoch）と一致する。generation 不一致は defined fault、silent delivery/drop しない。
- R12: 公開 enum は `#[non_exhaustive]`、公開 API 例は doctest、`rt_` 契約テストは `assert_no_alloc` 測定窓。

## Acceptance Criteria

- [ ] AC-1: 棚卸し記録が `docs/discovery/issue-186-param-control-events-inventory.md` に存在し、各制御イベント契約の不足と DAW 固有部分の分離を含む。
- [ ] AC-2: Capability Matrix・正規化値・Time Domain・同一 offset イベントの扱いが実 API に基づき仕様記述される（design doc）。
- [ ] AC-3: 複数時刻・複数対象の ParamEvent が期待順序・位置・値で届くことを L0 mock plugin + bridge `PluginNode` で検証するテストが存在する。
- [ ] AC-4: READONLY/per-note/即時手段/未解決 Target/非 Processing 状態で defined rejection が返るテストが存在する。
- [ ] AC-5: 時間変換（beat→frame）・Loop・tempo change・block 境界・非同期（control channel 経由 install）の挙動を再現するテストが存在する。
- [ ] AC-6: 非RT 準備/RT 有界評価/旧 Plan 維持/retire の PoC が `rt_` lane に整合し `mise run test-rt` が green。
- [ ] AC-7: scheduled/dispatched/reported/unavailable の区別を照会する API と、そのテストが存在する。
- [ ] AC-8: Control Graph 共通化の判断が ADR/設計で記録され、必要な場合のみ分割 Issue が提案される。
- [ ] AC-9: 後方互換と機能不足一覧が discovery doc に更新済み（E9/E10）。
- [ ] AC-10: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`verify-ci` が green。consumer 影響分類（synvelis/audio-lens: impacted/no impact/unknown）を記録。

## Deferred Design Concerns

- plugin→host パラメータ報告経路（CLAP output events/gesture/rescan、VST3 outputParameterChanges/IComponentHandler の中立配線）— 別 Issue 分割候補。Reported 観測の配線が必要になった時。
- `NodeEvent` の param 対応拡張（即時 Fast Control）— 即時配送が実需で必要になった時。#196 設計の拒否理由（opcode+u32/f32 の表現力不足）を先に解消する必要あり。
- `PortFormat::Control` の typed payload — Control ドメインのポート伝送が実需になった時。
- 汎用 Control 評価基盤（Source→Transform→Target、複数寄与、cycle rejection、計算順序固定）— 2 つ目の consumer または Synvelis PoC の実需が出た時に ADR で分割提案。
- `STEPPED` の量子化粒度（step count/choices）の中立表現 — 製品側が量子化規則を必要とした時。
- L1 wire の capability/param verb 拡張 — bus enumeration と同様に L1 照会が実需になった時。

## Sources and Relations

- Issue: ruviform#186（要件正本）、synvelis#6/#18/#20/#22（consumer 要求）、ruviform#185/#188/#189/#190/#193/#196。
- 棚卸し: `docs/discovery/issue-186-param-control-events-inventory.md`（derives-from 元）。
- 前提契約: #190 spec/ADR（generation/span/receipt）、#189（BeatPosition/TempoMap）、#196（bridge/capability matrix/PlanVault）、境界 ADR（ownership split）。
