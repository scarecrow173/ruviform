---
id: "SPEC-034Uj8oH3kcr44vuCWxYrC"
type: "spec"
status: "approved"
title: "Issue #309 — 複数Probeの汎用Frame/Clock/Epoch/Latency整列可能性・欠落比較契約（PROBE-TIME）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/309"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/308"
    - "https://github.com/scarecrow173/synvelis/issues/107"
    - "https://github.com/scarecrow173/synvelis/issues/109"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-309-probe-time-context-and-alignment.md"
      - type: "file"
        path: "docs/adr/issue-309-probe-time-context-placement-and-alignment-contract.md"
      - type: "file"
        path: "docs/test-specs/issue-309-probe-time-alignment.md"
      - type: "file"
        path: "docs/plans/issue-309-probe-time-context-and-alignment.md"
      - type: "file"
        path: "docs/tasks/issue-309-probe-time-context-and-alignment.md"
    modified:
      - type: "file"
        path: "docs/designs/overview.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/309"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-185-generic-probe-rt-capture.md"
    - "docs/specs/issue-308-probe-rebind-atomic.md"
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/overview.md"
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
    - "docs/adr/issue-309-probe-time-context-placement-and-alignment-contract.md"
    - "docs/test-specs/issue-309-probe-time-alignment.md"
    - "docs/designs/issue-308-probe-rebind-atomic.md"
    - "docs/designs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/issue-185-generic-probe-contract-placement-and-rt-handoff-model.md"
    - "../../AGENTS.md"
  refines:
    - "docs/specs/issue-185-generic-probe-rt-capture.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/314"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/280"
  deferred-by: []
---

# Issue #309 — 複数Probeの汎用Frame/Clock/Epoch/Latency整列可能性・欠落比較契約（PROBE-TIME）

## What Should Be Built

Issue #185 で確立した汎用 Probe の非 RT 観測面に対し、「任意の 2 点以上の Node/Port 観測が同じ時間座標系で比較・整列できるか」を型と契約で判定する **PROBE-TIME** 層を設計する。具現は次の 3 要素:

- **`ProbeTimeContext`（型名暫定）**: 1 観測点（または 1 runtime の frame cursor 系列）が属する時間座標系の identity を表す非 RT メタデータ。clock domain・clock epoch・runtime instance・plan generation・frame interval・sample rate/time base・format/channel・PDC/effective latency（既知/不明）・gap/loss/validity 状態を含み、`RecordMeta` の `generation`+`sample_position` が「同じ波形上の位置」を意味するための前提条件を与える。
- **`align_observation_points(a, b, policy)` 相当の非 RT 契約**: 2 つの観測区間について `Aligned{basis, offsets, error_budget}` / `Conditional{…}` / `Incomparable{reason}` の三値で整列可能性を返す。basis（同一 frame cursor / EngineFrame 写像 / device clock 等）・frame 単位 offset・誤差上限・欠落区間を証拠として返す。latency 不明・別 epoch・rate 変換不明・over/underrun・data loss・sample discontinuity を無条件 `Aligned` へ昇格しない。Resampling・PCM 相関の自動実行は行わない。
- **loss mask / 区間有効性**: `CaptureChunk` 列・`StatusRecord`・counter（`lost_records`/`lost_blocks`/`unobserved`/`unbound`）・#308 `RebindReceipt`/`CapturePolicy` の世代境界から、観測区間の欠落・未観測・世代切替境界を再構成する非 RT 表現。測定されたデジタル無音（Peak=0/RMS=0 の連続区間）と `Unobserved`/`Stale`/`LostBlocks`/`LostRecords`/Probe OFF/`Unsupported`/`Unavailable`/stale generation を区別し、loss mask 付き波形を連続 PCM 列として扱わせない。

本 Issue は **設計 Issue** であり、`ProbeTimeContext`/`align_observation_points` の型・語彙・配置・RT/非 RT 責務分界を文書として確定する。実装は後続 dispatch で行う。

## Why Now

- `RecordMeta`（`crates/ruviform-audio/src/typed/probe.rs`）は `generation`+`sample_position`+`effective_latency` を持つが、Device/Transport clock domain・epoch identity・runtime instance を持たない。`sample_position` は `RtState.frame_cursor`（callback 毎に `wrapping_add(frames)` で前進、`publish` で reset されない）であり、seek/loop/device restart/runtime 再構成を区別できない。
- plan `generation` は `GraphRuntime` ごとの単調 counter であり runtime instance 間で一意でない。異なる runtime の `generation=3`+`sample_position=48000` を同一座標と誤認する経路が残る。
- device callback の `OutputCallbackInfo`/`InputCallbackInfo`（device clock timestamp・`frame_position`）は audio-core に語彙として存在するが probe 経路へ未配線であり、`RtContext` は ZST、typed `RtRuntime` は現状 offline render 経路でのみ駆動される。Clock 語彙（`ClockDomainId`/`ClockEpochId`/`InputStamp`/`TimestampProvenance`/`ClockAnchor`/`map_stamp_to_frame`/`EngineFrame`）は `ruviform-media-core` に揃っており、OBS-05（#280）が RT record bridge + `ClockContext` の正本を定義済み — これを再利用し別 Clock 正本・別 RT writer を作らない。
- Synvelis SF-04（#107: Track/Bus/Sidechain 診断）が本契約の第一 consumer であり、SF-06（#109）と Ruviform #294（AudioLens f64 との cross-consumer gate）が整列結果に依存する。
- #308（PR #492 で main 統合済み）が世代切替の atomic boundary・`RebindReceipt`・`CapturePolicy` を確立し、clock epoch/port identity/segment quality を明示的に #309 へ defer した — 本 Issue がその deferred 境界を閉じる。

## Users and Value

- **Synvelis（第一 consumer）**: Signal Flow 診断が Node/Port 観測を `Aligned`/`Conditional`/`Incomparable` で分類し、異 clock/epoch/generation の波形を同一時間座標として比較しない。DAW graph reasoning・原因候補・UI は Synvelis 側責務のまま。
- **Ruviform（engine 所有者）**: 観測メタデータの正本として frame/clock/epoch/latency identity と品質分類を所有し、consumer が個別に時計を推測しない構造にする。
- **AudioLens / 将来 consumer**: f32 probe 値と native f64 解析値の精度境界を `Aligned` の basis と provenance で判別可能にし、bit-exact 主張のない比較を型で防ぐ。
- **本 dispatch（設計）**: 後続実装 dispatch が判断に迷わない型名・語彙・配置・RT 制約を文書として固定する。

## Scope

### In Scope

- `ProbeTimeContext`（型名暫定）の設計: clock domain・clock epoch・runtime instance・plan generation・frame interval・sample rate/time base・format/channel metadata・effective latency/PDC（既知/不明の区別）・observation validity/drop/gap を含む identity 契約。`RecordMeta` 全注入 vs `generation+clock-context-id` の nonRT lookup の選択を PoC/RT コストで決定する（Issue の未決問 — 結論は design/ADR に記録）。
- `align_observation_points(a, b, policy)` 相当の非 RT 契約設計: `Aligned{basis, offsets, error_budget}` / `Conditional{…}` / `Incomparable{reason}` の三値結果、basis 語彙、offset の単位・丸め規則、error budget の算出根拠、missing/lost 区間の表現。
- `sample_position`（frame cursor）への runtime instance/epoch 識別子の与え方の設計（`ClockDomainId`/`ClockEpochId` 語彙の流用を前提に、device `OutputCallbackInfo` 未配線・`RtContext` ZST の現状を考慮）。
- #308 の `RebindReceipt`/`CapturePolicy` による世代切替区間 boundary/gap 通知との接続設計。
- loss mask の表現設計（chunk 列 + status + counts からの再構成 vs 型付き gap）と Peak/RMS 比較の `ChannelLevel`/`MetricRecord` 再利用方針。
- Issue #309 の実装受入 5 件を downstream AC として設計文書に記録する（本 dispatch では実装しない）。

### Out of Scope

- `crates/` 以下の実装変更（型・API・配線は後続 dispatch。本 dispatch は `docs/` の文書のみ）。
- device session callback への `OutputCallbackInfo`/`InputCallbackInfo` 配線そのものの実装（設計上の接続点は定義するが配線実装は別 Issue/dispatch）。
- Synvelis 固有の診断ルール・Track/Bus/Sidechain 解釈・原因候補・UI/Inspector（Synvelis #107/#109 側責務）。
- Resampling・PCM 相関の自動実行、waveform の自動補間。
- OBS-05 RT record bridge 自体の再設計（#280 の既存契約を再利用するのみ）。
- 永続ストレージ形式・外部ファイル保存・cross-session probe 安定識別子。

## Requirements

- Rust edition 2024・workspace MSRV。`unsafe` 不使用。公開 enum は enum レベル `#[non_exhaustive]`（`exhaustive_enums` を無効化しない）。
- **RT 契約**: 整列・比較は非 RT bounded 処理。RT 経路（`RtRuntime::process`）に新規の alloc・blocking・lock・OS I/O・destructor を追加しない。clock/epoch 識別子は RT に配線する場合も `Copy` POD のみ。
- **正直性**: clock domain/epoch/instance が不明・不一致の場合に `Aligned` を返さない。`UNKNOWN == UNKNOWN` は同期証拠にならない（audio-core `same_clock_domain` 規則を踏襲）。latency 不明を既知 0 として扱わない。
- **世代と epoch の分離**: plan `generation`（slot binding staleness key）と clock epoch（frame cursor 系列の restart 境界）は別 identity であり、互いを含意しない。#308 の rebind boundary は観測の世代境界として消費するが、clock epoch の証拠としては扱わない。
- **Clock 正本の一元化**: OBS-05（#280）の `ClockContext`/`ClockDomainId`/`ClockEpochId`/`InputStamp`/`TimestampProvenance`/`ClockAnchor`/`map_stamp_to_frame`/`EngineFrame` を再利用し、probe 専用の別時計語彙・別 RT writer・別 SPSC bridge を作らない。
- **欠落の正直性**: telemetry/chunk の drop・未観測・世代境界・stale binding を区別して記録し、loss mask のある区間を連続有効 PCM として返さない。測定無音と未観測を混同しない。
- **所有権**: Ruviform は観測メタデータ・整列可能性・品質分類を所有し、DAW graph reasoning・原因候補・UI は Synvelis 側に残す。AudioLens f64 解析値と f32 probe 値の精度境界を偽らない。
- **consumer 影響分類**: synvelis = `impacted`（新規 API 提供）、audio-lens = `no impact`（既存挙動不変の additive API — #219 contract で実装時に検証）。

## Acceptance Criteria

- [ ] **AC-1 (`ProbeTimeContext` 設計)**: clock domain・epoch identity・runtime instance・plan generation・frame interval・sample rate/time base・format/channel・effective latency/PDC（既知/不明）・validity/gap を含む identity 契約が設計され、`RecordMeta` 全注入 vs nonRT lookup の選択が RT コスト根拠つきで決定される。
- [ ] **AC-2 (`align_observation_points` 非 RT 契約)**: `Aligned{basis,offsets,error_budget}` / `Conditional{…}` / `Incomparable{reason}` を返す API が設計され、latency 不明・別 epoch・rate 変換不明・over/underrun・data loss・sample discontinuity を無条件 `Aligned` にしない。Resampling/PCM 相関の自動実行なし。
- [ ] **AC-3 (frame cursor identity)**: `sample_position`/`frame_cursor` に runtime instance/epoch 識別子を与える方式が設計され、seek/loop/device restart/runtime 再構成での cursor 連続性誤認を型で防ぐ。
- [ ] **AC-4 (rebind 境界接続)**: #308 `RebindReceipt`/`CapturePolicy` の世代切替区間 boundary/gap 通知が `ProbeTimeContext`/coverage 再構成へ接続される設計がある。
- [ ] **AC-5 (loss mask)**: chunk 列+status+counts からの欠落区間再構成 vs 型付き gap の選択が決定され、Peak/RMS 比較の `ChannelLevel`/`MetricRecord` 再利用方針が明記される。
- [ ] **AC-6 (downstream AC 記録)**: Issue #309 の実装受入 5 件（実 Audio Fixture で offset/Peak/RMS/gaps 一致 等）が downstream AC として設計文書に記録される。
- [ ] **AC-7 (文書チェーン)**: spec→design→ADR（+test-spec/plan/task）文書が `docs/` に作成され、既存 spec/design/ADR との矛盾がない。`docs/designs/overview.md` が更新される。
- [ ] **AC-8 (検証記録)**: 実行した文書検証（index 再生成・`mise run fmt-check`）と review-protocol 結果が PR に記録される。

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 判断・移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| device session への clock timestamp 配線実装 | probe 経路の identity 契約のみ設計。`OutputCallbackInfo` 等の実配線は別 Issue | OBS-10 debug session / device adapter 側 dispatch | live device で epoch が常に Unknown のまま |
| EngineFrame/media time への写像実装 | `ClockAnchor`/`map_stamp_to_frame` 語彙の再利用方針のみ確定。anchor 生成は consumer/driver 側 | #280 OBS-05 record bridge / render driver | offline render でのみ Aligned、live で Incomparable 継続 |
| PCM waveform の実差分・可視化 | 整列結果と coverage のみ提供。描画・差分強調は consumer 側 | Synvelis #107/#109・Ruviform #292/#314 | consumer が独自に時計を推測する余地が残る |
| cross-session / 永続 probe ID | `ProbeId` は broker 生存期間内の ID のまま | consumer 側 Semantic ID 対応付け | UI 再構成時の ID 崩れ（#185 から継続） |

## Sources and Relations

- 本チェーンの文書: [design](../designs/issue-309-probe-time-context-and-alignment.md)・[ADR](../adr/issue-309-probe-time-context-placement-and-alignment-contract.md)・[test-spec](../test-specs/issue-309-probe-time-alignment.md)・[plan](../plans/issue-309-probe-time-context-and-alignment.md)・[task](../tasks/issue-309-probe-time-context-and-alignment.md)。
- 一次要件: GitHub Issue [#309](https://github.com/scarecrow173/ruviform/issues/309)（`relations.source`/`implements`）。上流 consumer: Synvelis [#107](https://github.com/scarecrow173/synvelis/issues/107)・[#109](https://github.com/scarecrow173/synvelis/issues/109)。
- 前提 spec（`relations.depends-on`）: [#185 generic probe](issue-185-generic-probe-rt-capture.md)（観測面の正本 — 本 spec はその時間整列可能性を refine）、[#308 probe rebind](issue-308-probe-rebind-atomic.md)（世代境界・`RebindReceipt`・`CapturePolicy`）、[#280 OBS-05](issue-280-rt-record-bridge-clock-correlation.md)（Clock 語彙・RT bridge の正本）。
- refine 関係: 本 spec は #185 spec の「同一 generation+sample_position で整列可能」という記述を「clock/epoch/instance identity を含む整列可能性契約」へ精緻化する（`relations.refines`）。
- 関連 ADR（`relations.related`）: [RT 契約](../adr/realtime-contract-and-verification-policy.md)、[公開 enum 非網羅](../adr/public-enum-non-exhaustive-policy.md)、[#185 probe 配置 ADR](../adr/issue-185-generic-probe-contract-placement-and-rt-handoff-model.md)。
- 参考: [#280 OBS-05 spec](issue-280-rt-record-bridge-clock-correlation.md)、[#294 cross-consumer gate](https://github.com/scarecrow173/ruviform/issues/294)、[#314 semantic/visual golden](https://github.com/scarecrow173/ruviform/issues/314)。
