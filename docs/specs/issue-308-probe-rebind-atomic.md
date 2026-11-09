---
id: "SPEC-034Ui3cZpzwuwGdf2aHCCv"
type: "spec"
status: "approved"
title: "Issue #308 — Generation切替時のProbe再Bindingを全件整合・部分失敗可観測にする"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/308"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/synvelis/issues/107"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-308-probe-rebind-atomic.md"
      - type: "file"
        path: "docs/test-specs/issue-308-probe-rebind-atomic.md"
      - type: "file"
        path: "docs/plans/issue-308-probe-rebind-atomic.md"
      - type: "file"
        path: "docs/tasks/issue-308-probe-rebind-atomic.md"
      - type: "file"
        path: "crates/ruviform-audio/src/typed/probe.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/typed_probe.rs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/308"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-185-generic-probe-rt-capture.md"
    - "docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/specs/issue-185-generic-probe-rt-capture.md"
  refines:
    - "docs/specs/issue-185-generic-probe-rt-capture.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-308-probe-rebind-atomic.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/309"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/synvelis/issues/109"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/309"
    - "https://github.com/scarecrow173/ruviform/issues/280"
  deferred-by: []
---

# Issue #308 — Generation切替時のProbe再Bindingを全件整合・部分失敗可観測にする

## What Should Be Built

`ruviform_audio::typed::probe` の `ProbeBroker::on_publish` が行う Generation 切替時の Probe 再 Binding を、`ClearAll` + 個別 `Bind` の非原子的なコマンド列から、RT 側で **bounded all-or-nothing** に適用される staged rebind へ改める。部分失敗は `Partial / PendingRetry / Unavailable` を型付きで返し、「ClearAll だけ先行して完全成功を返す」動作を禁止する。部分失敗・境界通過は GUI が読める Receipt / StatusRecord / Counter で可観測にし、Capture 継続 Policy により旧 Generation の残 frame を新世代へ無断繰越しない。

## Why Now

現行実装（Issue #185 PoC）では `on_publish` が `ProbeCommand::ClearAll{generation}` を1件、続けて解決できた probe ごとに `ProbeCommand::Bind{generation, binding}` を個別に `try_push` する。bounded SPSC command channel が途中で `CommandChannelFull` になると、投入済み prefix（ClearAll と一部の Bind）だけが RT 側で適用され、**部分適用された deck** が残る。戻り値は `Err` でも RT 側の状態は「旧 binding 全消去 + 一部だけ新 binding」という中途半端な状態であり、Consumer はそれを成功とも失敗とも区別できない。

さらに `on_publish` は `capture_remaining` に `entry.budget_frames` を無条件で再設定するため、Capture 中の Graph 変更で残 frame budget が新 Generation へ**黙って繰り越され**、旧 Plan の PCM 区間と新 Plan の PCM 区間が同一 Capture stream に混在し得る。`on_publish` の doc（「entry flips to `Off`」）と実装（mode を一切変更しない）の不一致もあり、unresolved probe が静かに消える。

Synvelis SF-04（#107）は「Graph 切替で Capture 区間を終了し自動再 Capture しない」という Consumer Policy を要求しており、Ruviform 側が RT 適用 Ack / 原子的世代境界を提供しない限り Consumer は安全性を認定できない。

## Users and Value

- **Synvelis（Consumer）**: Semantic Point → RT Probe Point の mapping と Capture 継続方針を所有する側として、Generation 切替時に「全件適用 or 無変更 + 型付き失敗理由」を受け取り、誤 slot 観測・silent corruption・無断 Capture 繰越しを 0 にできる。
- **AudioLens（Consumer）**: 既存の PCM/Gain Golden・Counter を維持したまま、Probe 再 Binding の信頼性を得る。
- **Ruviform generic audio probe（Owner）**: 別 Probe Engine を作らず、#185 の `ProbeBroker`/`ProbeDeck`/`ProbeCollector` を拡張して契約を強化する。

## Scope

### In Scope

- `crates/ruviform-audio/src/typed/probe.rs` の再 Binding 経路の全件整合化（bounded all-or-nothing activation）。
- 部分失敗の型付き返却と GUI-readable Receipt（User Requested / Enqueued / RT Installed / Observed の区別、QueueFull・Point removed・plan superseded・late generation・shutdown/cancel の語彙）。
- Capture 継続 Policy（Generation 境界での停止 / 明示的再開）と旧 Generation 残 frame の無断繰越し禁止。
- 異なる Graph Generation / port identity を跨ぐ PCM の SegmentBoundary 相当の明示通知（既存 `RecordMeta.generation`/`sample_position` 足場の利用）。
- `on_publish` doc/impl 不一致の修正（unresolved probe の扱いを正直に）。
- `crates/ruviform-audio/tests/typed_probe.rs` の拡張（Queue 飽和点網羅・高速連続切替・Capture 境界の break/complete/lost 区別・Golden 一致・drop 別 Counter）。
- `docs/` 配下の spec → design → test-spec → plan → task 文書チェーンと impl record。

### Out of Scope

- 別 Probe Engine・別 RT Capture/Queue の新設（ユーザー指定により禁止）。
- `RecordMeta`・`StatusKind`・Counter の再利用を超えた新観測基盤の新設。
- Clock Epoch / Latency 比較可能性（Issue #309 PROBE-TIME が Owner。本 Issue では集約しない）。
- RT 固定長観測 / Clock Bridge（Issue #280 OBS-05）・Collector（Issue #289 OBS-07）。
- `ui/`・`crates/ruviform-ui-*`（別 Worker が変更中）、`crates/ruviform-observability`（Counter API を使うだけ）、`Cargo.toml` メンバー定義、`.github/`。
- 新クレート・新依存の追加。
- Consumer 恒久 workaround での問題隠蔽。

## Requirements

- R1: `ClearAll`＋個別 `Bind` の非原子的列を廃し、bounded all-or-nothing activation を実現する。安全な全件適用が不可能な場合は `Partial / PendingRetry / Unavailable` を**型付き**で返し、「ClearAll だけ先行して完全成功を返す」動作を禁止する。
- R2: 部分失敗を可観測にする。User Requested / Enqueued / RT Installed / Observed を分けた GUI-readable Receipt を提供し、QueueFull・Point removed・plan superseded・late generation・shutdown/cancel を網羅する。
- R3: Capture 継続 Policy を Consumer が事前設定でき、RT 上で確実に Capture を切断するか、`Stop` の RT 適用 Ack を取得してから Publish できる契約を提供する。**旧 Generation の残 frame を新世代へ無断繰越ししない**。
- R4: 異なる Graph Generation / Probe port identity を跨ぐ PCM は SegmentBoundary/Gap/Unverified として通知する（既存 `RecordMeta.generation`/`sample_position` 足場を使う）。
- R5: 検証として Queue 飽和点網羅（ClearAll 相当の前/直後/Bind 途中/最後）・高速連続切替での誤 slot 読取 0 / Silent corruption 0 / unresolved 明示・Capture 中 Graph 変更の break/complete/lost 区別・Probe OFF の PCM/Gain Golden 一致と drop 別 Counter を `typed_probe.rs` 拡張で RED→GREEN で示す。
- R6: `on_publish` doc（1026 行付近）と実装の不一致を修正し、unresolved probe の扱いを正直に記述する。

## Acceptance Criteria

- [ ] AC1: 1/2/N probes の Graph publish 時に command queue 容量を Begin 前 / Begin 直後 / StageBind 途中 / Commit 直前で飽和させても、戻り値・RT install 状態・GUI-readable Receipt/Retry が一致し、部分適用が発生しない。
- [ ] AC2: Graph generation の連続高速切替・旧 Command 遅延・RT 停止/再開・Probe 破棄/追加・物理 Port 変更で誤 slot 読取 0、Silent data corruption 0、unresolved 状態が明示される。
- [ ] AC3: Capture 中の Graph 変更は break/complete/lost の観測区間として判別可能。残 frame を別 Graph の同一区間として偽結合しない。Monitor の再 Binding 成功/失敗経路を区別する。
- [ ] AC4: Mode/Probe OFF は Bypass や DSP Connection を変更せず、PCM/Gain Golden 一致、Capture drop/telemetry drop を別 Counter で可視化する。
- [ ] AC5: `on_publish` の doc と実装が一致し、unresolved probe が `RebindDropped` 相当で明示される。
- [ ] AC6: Ruviform workspace の `mise run check` / `lint` / `fmt-check` / `test` / `test-rt` が通る。Synvelis Signal Flow / AudioLens の実機・OS 別 Pass/Fail/NotRun/Unsupported は本 PR では Consumer 側未検証として記録する（Consumer 追従が必要なら各 Repository に Issue）。

## Deferred Design Concerns

- Clock Epoch を跨ぐ PCM の比較可能性・品質判定 | Issue #309（PROBE-TIME）が Owner であり本 Issue の責務外 | #309 着手時 | epoch を跨ぐ区間の品質を誤認する恐れ（本 Issue では `RecordMeta.generation`/`sample_position` の足場までを保証）
- RT 固定長観測・Clock Bridge との Probe 記録の整合 | Issue #280（OBS-05）が Owner | #280 着手時 | Probe PCM と RT 観測の時刻対応が未検証のまま残る
- Collector 側の Segment 集約・GUI 表示 | Synvelis SF-04/SF-06 が Consumer 側責務 | Consumer 実装時 | Receipt/Status は提供されるが GUI 側の解釈は Consumer 依存

## Sources and Relations

- Issue #308 本文（旧 #185 設計・現行 `probe.rs` 実装の拡張指示、受入条件）。
- Issue #185 spec/design（`typed::probe` の既存契約 — 本 spec はその `on_publish` 経路を精緻化する `refines` 関係）。
- `docs/adr/realtime-contract-and-verification-policy.md`（RT 契約 — no alloc/no block/no destructor の境界）。
- Synvelis #107（Capture 継続禁止の Consumer 要求）・#109（横断受入 Oracle）。
- Issue #309/#280/#289（Clock/Collector 系 — 本 Issue では変更しない連携先）。
