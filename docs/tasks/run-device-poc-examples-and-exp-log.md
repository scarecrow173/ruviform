---
id: "TASK-034JglHs8cNMDOvyZFeUpc"
type: "task"
status: "done"
title: "PoC example（list_devices・play_sine）実機手順と実験ログ"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/23"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-device/examples/list_devices.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/examples/play_sine.rs"
      - type: "file"
        path: "docs/impl/exp/*.md（実験ログ）"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-audio-device-port-and-cpal-poc.md"
  implemented-by:
    - "../impl/ir/run-device-poc-examples-and-record-experiment-log.md"
  depends-on:
    - "../plans/implement-audio-device-port-and-cpal-poc.md"
    - "implement-audio-device-cpal-adapter.md"
  blocks:
    - "finalize-backend-adr-and-sync-docs.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-audio-device-port-and-cpal-poc.md"
    - "../designs/audio-device-boundary-and-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/audio-device-port-and-cpal-adapter.md"
  references: []
  defers: []
  deferred-by: []
---

# PoC example（list_devices・play_sine）実機手順と実験ログ

## Work

plan `PLAN-034JgkWCimxdvr81k6b0ow` の Phase C。設計正本 `DESIGN-034JcC9QK924kDbu4WhibC`「PoC 実証フロー」「実機手動検証手順」節どおり、手動検証 example を実装し実機で実行して `docs/impl/exp/` に実験ログを記録する。本タスクは実機依存であり CI では実行しない。

- [x] `examples/list_devices.rs` を実装する: `available_host_ids()`・各 `Host` のデバイス列挙・既定デバイス・`default_output_format()`・`supported_output_formats()` のダンプ（非 RT 診断用途）
- [x] `examples/play_sine.rs` を実装する: `default_host()` → `default_output_device()` → `default_output_format()`（`Err`/非 F32 時は `supported_output_formats()` から `select_f32_format` 補助関数で F32 `AudioFormat` を合成するフォールバック）→ `OutputStreamConfig { format, buffer_size }` で `open_output_stream` → `start()` → 数秒 sine 送出 → `stop()`。コールバック内の観測は `AtomicUsize`/`AtomicU64` のみ（コールバック回数・min/max 実フレーム数・xrun 回数・最終 `OutputTimestamp`）とし、I/O・確保を行わない。`stop()` 後に非 RT 側で要約を出力する
- [x] 実機で `cargo run -p ruviform-audio-device --example list_devices` を実行し結果を記録する（既定構成では PipeWire host id は列挙されない — Linux=ALSA。ALSA vs PipeWire 比較は `--features pipewire-backend` 有効化の任意検証・要 `libpipewire-0.3-dev`）
- [x] 実機で `cargo run -p ruviform-audio-device --example play_sine` を実行し「列挙→既定構成→ストリーム開始→コールバック到達（planar f32 境界変換）」を観測する。任意で `--features realtime` でも実行する
- [x] `docs/impl/exp/` に `node .agents/skills/impl-doc/scripts/new_experiment_log.js` で実験ログを作成し、環境（OS・ホスト API・cpal バージョン）・手順・観測値・判定を記録する。記録事項: コールバック到達・可変フレーム実測・xrun 頻度（WASAPI は出力 xrun 未報告のため観測 0 は「未発生」と「未報告」を区別しない旨も記録）・timestamp 整合・`BufferSize::Fixed` 尊重度・`Send`/`Sync` ばらつき・cpal API の「要実装時確認」項目の実測結果・依存追加理由（cpal・`assert_no_alloc`）・alloc 検証の方式と限界（`assert_no_alloc` は adapter 自前コードのみ。cpal 内部込みの直近手段は example への計数型 `#[global_allocator]` 仕込み。統合不能時はコードレビュー＋静的 grep 証跡）
- [x] discovery の Open Questions（コールバック内 alloc/ブロッキング・Fixed 尊重度・Send/Sync ばらつき・WASAPI exclusive/ASIO 必要性・ALSA vs PipeWire・RtContext 適合）それぞれについて観測できた範囲を記録し、未観測項目は ADR 確定タスクへ引き継ぐ

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-audio-device-port-and-cpal-poc.md](../plans/implement-audio-device-port-and-cpal-poc.md) — `PLAN-034JgkWCimxdvr81k6b0ow`（approved）。Phase C の正本
- [../designs/audio-device-boundary-and-poc.md](../designs/audio-device-boundary-and-poc.md) — `DESIGN-034JcC9QK924kDbu4WhibC`（approved）。PoC 実証フロー・実機手順・実験ログ方針・ADR 確定計画の正本
- [../discovery/evaluate-audio-device-backends.md](../discovery/evaluate-audio-device-backends.md) — `DISC-034Jc8wchRH37hqfhNLRPJ`。実機で確認すべき Open Questions の正本

## Dependencies

- 前提タスク: [implement-audio-device-cpal-adapter.md](implement-audio-device-cpal-adapter.md)（`TASK-034JglAvWUFQoLhvYXOw2O`）— adapter の公開 API・feature 構成を提供
- 後続をブロック: [finalize-backend-adr-and-sync-docs.md](finalize-backend-adr-and-sync-docs.md) — 実験ログが ADR 昇格判定の証跡
- 実機（オーディオ出力デバイス）が必要。Linux では `libasound2-dev`、任意で `libpipewire-0.3-dev`

## Affected Files

- `crates/ruviform-audio-device/examples/list_devices.rs` — 新規
- `crates/ruviform-audio-device/examples/play_sine.rs` — 新規（`select_f32_format` 補助関数同梱）
- `docs/impl/exp/*.md` — 実験ログ（`new_experiment_log.js` で生成）

## Done When

- [x] 両 example が `cargo build -p ruviform-audio-device --examples` でコンパイルする
- [x] 実機で「列挙→既定構成→ストリーム開始→コールバック到達」が観測され、観測値が実験ログに記録される
- [x] 実験ログが環境・手順・観測値・判定・依存追加理由・alloc 検証の限界を含み、discovery の Open Questions へ観測結果/未観測の対応が記録される
- [x] コールバック内の観測が `AtomicUsize`/`AtomicU64` のみで実装されている（I/O・確保なし）

## Verification

- [x] `mise exec -- cargo build -p ruviform-audio-device --examples` exits 0
- [x] `cargo run -p ruviform-audio-device --example list_devices` が実機でホスト・デバイス・既定構成をダンプする（手動・実機依存。結果は実験ログに記録）
- [x] `cargo run -p ruviform-audio-device --example play_sine` が実機でコールバック到達・min/max フレーム・xrun・最終 `OutputTimestamp` を観測する（手動・実機依存。結果は実験ログに記録）
- [x] `docs/impl/exp/` に本 PoC の実験ログファイルが存在し、所定の記録事項を含む
- [x] `mise run fmt-check` / `mise run lint` exits 0（example コードも lint 対象）
