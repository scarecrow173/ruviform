---
id: "SPEC-034XBGBcah0yYseU6Ypekr"
type: "spec"
status: "approved"
title: "Issue #628 Deterministic Audio/MIDI Test Runtime first slice — deterministic core"
created: "2026-11-14"
updated: "2026-11-14"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-628-deterministic-test-runtime.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/plans/issue-628-deterministic-test-runtime-plan.md"
  depends-on:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-628-deterministic-test-runtime-design.md"
    - "docs/test-specs/issue-628-deterministic-test-runtime.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-628-deterministic-test-runtime.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 Deterministic Audio/MIDI Test Runtime first slice — deterministic core

## What Should Be Built

Ruviform の公開 Device/Event/Clock/Buffer 契約を実際に利用する、test-only の
Deterministic Audio/MIDI Test Runtime の **first slice（deterministic core）**。
新規 test-only crate `crates/ruviform-audio-test` に、機材不要で Audio/MIDI の
精度・RT・復旧を検証するための deterministic core を提供する:

- **VirtualClock** — sample frame/epoch tick モデル、deterministic な
  drift/jitter/offset、pause/resume、timeout/deadline、monotonic 時間。
  RT-safe read は atomic・wait-free（`ruviform-rt` の `MasterClock` をモデル先例）。
- **Deterministic PCM fixtures** — sine / impulse / sweep / seeded noise /
  silence / multi-channel / 欠落 frame を byte- かつ sample-exact に生成し、
  時刻付 block として供給。format/channel/buffer 変更を表現できる。
- **Deterministic MIDI event scheduler/trace** — `ruviform-media-core` の
  MIDI 型上の timestamped scenario を、本番 `TransportScheduler` /
  `TransportPlan` / `SpanSink` 契約を通して trace として capture する。
- **独立 Oracle** — 固定テストベクトル + 独立な scalar/数値実装が期待
  PCM/MIDI trace を生成する。被試験コード自身で期待結果を生成しない
  （自己参照の正解値共用禁止）。
- **Run metadata** — scenario seed + fixture version + clock 設定 +
  `environment_kind`（`deterministic-backend`）を記録し、同一条件からの
  再現に使う。
- **RT contract tests** — `assert_no_alloc` を使う `rt_` prefix テストで
  clock-read / generator-fill 経路が alloc/lock/wait-free であることを証明。

本 slice は Issue #628 の first slice であり、Issue の残受入条件
（scripted port-trait backend、fault injector、production pipeline 配線、
CI lane 統合、consumer oracle）は後続 slice へ hand-off する。

## Why Now

- `scarecrow173/ruviform#627`（Closed）で 2026-11-15 仮想優先 POLICIES が
  main に反映され、Case Schema v1・`environment_kind`・Test Depth が確定した。
  物理機材・市販 Plugin を初回受入の必須条件にしない方針の下、Audio/MIDI の
  精度・RT・復旧を機材なしで検証する deterministic core が #628〜#631 の
  基盤として必要。
- AudioLens（f64/長尺/解析）と Synvelis（PCM/Project/RT/Recording/MIDI）の
  独立 Oracle が同一 Fixture version を共有するには、Ruviform 側に
  決定論的 fixture/clock/trace 生成の共通基盤が先に必要。
- 既存の `ruviform-audio-device/tests/mock_input_backend.rs` は scripted
  backend の先例だが、test-only crate として共有・拡張可能な形ではない。
  本番 `TransportScheduler`/`SpanSink`・`AudioBufferView`・`MasterClock`
  契約をそのまま使う共通 deterministic core を先に確立する。

## Users and Value

- **Ruviform 自身**: Audio/MIDI device・engine・plugin-host の契約試験を
  機材なし・3 OS で再現可能に実行できる。
- **AudioLens / Synvelis**: 同一 fixture version・clock 設定から生成される
  決定論的 PCM/MIDI 期待 trace を独立 Oracle で検証できる
  （双方の resolved Ruviform SHA を記録）。
- **後続 slice（#629〜#631）**: 仮想 OS Endpoint・実 ABI Test Plugin・
  GUI Runner が乗る deterministic backend の土台を提供する。

## Scope

### In Scope

- `crates/ruviform-audio-test/` の新設（test-only crate、publish しない、
  production crate は本クレートに依存しない）。
- VirtualClock（deterministic tick/monotonic、drift/jitter/offset、
  pause/resume、deadline、atomic wait-free read）。
- Deterministic PCM fixture 生成（pattern・block・scenario/segment、
  `fill` による alloc-free 生成経路）。
- Deterministic MIDI scenario → `TransportPlan<MidiEvent>` compile →
  `TransportScheduler` + `SpanSink` による trace capture。
- 独立 Oracle（PCM 数値/byte 比較、MIDI trace 期待列、固定ベクトル）。
- Run metadata（Case Schema v1 準拠の記録構造、`environment_kind`）。
- `rt_` prefix の `assert_no_alloc` RT contract テスト。
- Workspace 登録（root `Cargo.toml` members、`crates/README.md` 索引、
  `Cargo.lock` 再生成）と docs chain・evidence。

### Out of Scope

- scripted port-trait backend（`DeviceEnumerator`/`Device`/
  `InputStream`/`OutputStream` の script 駆動実装）— 後続 slice。
- fault injector（loss/reconnect、xrun、permission denial、time
  discontinuity、rate change、queue full、plugin hang/crash、
  cancellation）— 後続 slice。
- production pipeline への配線（Audio Input→本番 Recording、
  Output→本番 Playback、MIDI→本番 Timestamp/Host adapter）— 後続 slice。
- CI lane 統合・新規 mise task・workflow 変更 — `mise.toml` は別 Worker
  のリース。本 slice は既存 `mise run test-rt` の `rt_` name filter で
  収集される範囲に留める。
- consumer oracle（AudioLens f64/長尺、Synvelis PCM/Project/RT）—
  別 repo・別 Issue。
- Windows 仮想 OS Endpoint の実走検証 — `scarecrow173/ruviform#629`。

## Requirements

- test-only crate は製品配布に混入せず、production crate は本クレートに
  依存しない（依存方向は利用側層の test-shared infra）。
- path 依存は `ruviform-audio-core`・`ruviform-media-core`・
  `ruviform-rt` のみ。`assert_no_alloc` は dev-dependency。
- 公開 enum は全て `#[non_exhaustive]`（
  `docs/adr/public-enum-non-exhaustive-policy.md`）。
- RT-safe read/generate 経路は allocation-free・non-blocking・I/O なし。
- 同一 scenario seed + fixture version + clock 設定から決定論的
  PCM/MIDI 期待 trace を生成し、run metadata を保存する
  （本 slice の主張は pure-Rust の決定性。3 OS 安定性の実走証拠は
  後続統合 slice）。
- Oracle は固定ベクトルと独立実装を持ち、被試験コードと期待値生成を
  共有しない。
- run metadata は #627 Case Schema v1 の語彙（`environment_kind` 等）に
  従う。

## Acceptance Criteria

- [ ] `crates/ruviform-audio-test` が workspace member として build でき、
  `mise run check|lint|test ruviform-audio-test` が Pass する。
- [ ] 同一 seed/fixture version/clock 設定から PCM block・MIDI trace・
  run metadata が決定論的に生成され、再現テストが Pass する。
- [ ] `rt_` prefix テストが `assert_no_alloc` で clock-read /
  generator-fill の alloc-free を証明し、`mise run test-rt` で実際に
  収集・実行される（テスト名で確認）。
- [ ] 独立 PCM 数値/byte Oracle・MIDI trace Oracle が独立 fixture/実装で
  検証され、期待値を被試験コードから生成していない。
- [ ] test-only の公開範囲（crate README・`src/lib.rs` doc・本 spec）が
  明文化され、production crate からの依存がないことを確認できる。
- [ ] Issue #628 の残受入（backend/fault/pipeline/CI/consumer oracle）
  は本 slice の対象外として docs に明記されている。

## Deferred Design Concerns

- scripted backend・fault injector・production 配線・CI lane・consumer
  oracle・3 OS 実走証拠・Windows EXT-L3 — 後続 slice / 別 Issue が担当。
  本 spec の `relations.defers` は self-reference のため、実体は
  `docs/designs/issue-628-deterministic-test-runtime-design.md` の
  Deferred Design Concerns に記録する。

## Sources and Relations

- `scarecrow173/ruviform#628`（本 Issue）、`scarecrow173/ruviform#627`
  （Closed — Case Schema v1・`environment_kind`・Test Depth の契約元）。
- `docs/designs/issue-627-virtual-first-verification-contract-design.md`
  （Case schema・`environment_kind` taxonomy の正本）。
- `POLICIES.md` §2.1（仮想優先の受入・証拠要件）・検証レベル定義。
- 先例: `crates/ruviform-rt/src/clock.rs`（`MasterClock`）、
  `crates/ruviform-media-core/src/midi.rs`・`transport_schedule.rs`、
  `crates/ruviform-audio-device/tests/mock_input_backend.rs`
  （`assert_no_alloc` + `rt_` prefix）、`crates/ruviform-ui-test`・
  `crates/ruviform-io-e2e`（test-only crate 宣言）。
