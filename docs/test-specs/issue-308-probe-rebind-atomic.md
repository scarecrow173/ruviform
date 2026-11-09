---
id: "TSPEC-034Ui3L8Aap8bdwyrLbDGp"
type: "test-spec"
status: "approved"
title: "Issue #308 — Probe再Bindingの検証契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/308"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-308-probe-rebind-atomic.md"
    - "docs/designs/issue-308-probe-rebind-atomic.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-308-probe-rebind-atomic.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #308 — Probe再Bindingの検証契約

## Purpose

Issue #308 の staged all-or-nothing rebind・部分失敗 Receipt・Capture boundary policy が、`typed_probe.rs` の統合テストで実際に保証されることを固定する。#185 の既存 acceptance（ex01–ex16）を壊さないことも同時に検証する。

## Feature

Generation 切替時の Probe 再 Binding — 全件整合・部分失敗契約・Capture policy

## Rules

### Rule: Rebind は all-or-nothing — queue 飽和のどの点で失敗しても live bindings は不変

- 1/2/N probes を install した状態で `command_capacity` を調整し、`on_publish` の push 列（Begin → Stage×N → Commit）が (a) Begin 前に満杯、(b) Begin 直後、(c) StageBind 途中、(d) Commit 直前、で失敗するようにする。
- 失敗時 `on_publish` は `Err(ProbeError::CommandChannelFull)` を返し、`process` を回しても **新 generation の `Installed`/`Metric`/`Captured` が1件も出ない**（部分適用ゼロ）。旧 generation binding は generation guard で `Unbound` として skip されるのみ。
- 失敗後に queue を drain（`process` 実行）して retry すると全件 commit され、全 probe が新 generation の record を出す。

### Rule: Commit は観測に対して atomic — 誤 slot 読取・silent corruption なし

- 高速連続 publish（gen1→2→3）と `on_publish` を連続実行しても、telemetry/chunk の `meta.generation` が当時の active plan generation と一致しない record は 0 件。slot の中身が別 plan の PCM を誤って観測しない。
- `unbound`/`stale_commands` counter が stale 系 command の drop を正直に計数する。

### Rule: unresolved point は明示 — 静かに消えない

- publish で point が解決不能になった probe は `RebindReceipt.unbound` に列挙され、commit 時に `StatusKind::RebindDropped` が発行される。他 probe の binding は影響を受けない。

### Rule: Capture は境界で無断繰越ししない

- 既定（`StopAtBoundary`）: Capture 中に publish+`on_publish` すると、境界より前の chunk は旧 `meta.generation`、境界以降は chunk が出ない（Monitor に戻る）。`CaptureStopped { reason: GenerationBoundary }` が発行され、`RebindReceipt.capture_stopped` に列挙される。残 budget は破棄され、明示的な `capture()` 呼出しなしに新世代で PCM を出さない。
- `Restart` policy: `set_capture_policy` で設定した probe は commit で `CaptureStarted`（満額 budget）が発行され、新 segment の chunk が新 `meta.generation` で始まる。旧 segment の残 frame は混入しない。
- `stop_capture` → `CaptureStopped{Requested}` の status を collector で確認してから publish する Ack 契約経路も既存のまま動く。

### Rule: Probe OFF / Mode は音声を変えない — Golden 一致と drop 別 Counter

- publish+rebind を跨いでも、probe が Off/Monitor/Capture のいずれでも sink 出力 PCM が bit-identical（gain×2 processor に対し入力 ramp が厳密に2倍）。
- `lost_blocks`（chunk pool 枯渇）と `lost_records`（telemetry 溢れ）は別 counter のまま。

## Examples

- Given 3 probes installed with `command_capacity` tuned so `on_publish` fails at StageBind 3件目, When `on_publish` returns `Err(CommandChannelFull)` and one `process` block runs, Then zero gen-N metrics appear and `collector.counts().unbound` reflects only generation-guard skips; When the queue is drained and `on_publish` retried, Then all 3 probes emit `Installed` + gen-N metrics.
- Given a probe in `Capture` mode mid-budget, When generation 1→2 publish + `on_publish` (default policy) commits, Then telemetry contains `CaptureStopped{GenerationBoundary}` and no `CaptureChunk` with `meta.generation == 2` appears until `capture()` is called again.
- Given the same but `set_capture_policy(Restart)`, When commit lands, Then `CaptureStarted` fires and subsequent chunks carry `meta.generation == 2` with a fresh full budget.

## Guarantees

- 部分適用された deck 状態が観測可能な形で存在しない（all-or-nothing）。
- 失敗は型付き（`ProbeError::CommandChannelFull` = PendingRetry）で、成功を偽装しない。
- RT 側は no alloc/no block/no destructor を維持する（`assert_no_alloc` 系の既存 RT 検証を壊さない）。
- 公開 enum は `#[non_exhaustive]` を維持する。

## Non-goals

- `RuntimeCommand`（processor install/remove）の原子性 — publish 失敗時の last-good rollback で既に安全。
- Clock Epoch・Latency の Segment 品質分類（#309/#280）。
- 実 OS / 実デバイスでの Consumer 受入（Synvelis #107/#109・AudioLens 側の責務）。
- `spsc.rs` への batch push API 追加（staged 方式では不要と判断）。

## Risk

- queue 容量 `2+N`（Begin+Stage×N+Commit）に対しテストが容量を誤設定すると失敗点がずれる — テストは `command_capacity` を probe 数から算出して飽和点を決定的に作る。
- telemetry channel が小さいと commit status が `lost_records` で欠落し Installed が見えない — テストは telemetry_capacity を十分に取り、counter 検証は別テストで行う。
