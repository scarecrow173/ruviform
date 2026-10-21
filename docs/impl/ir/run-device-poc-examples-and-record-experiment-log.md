---
id: "IMPL-034JywI99DQ33dDEZVnnqC"
type: "impl"
status: "completed"
title: "Run device PoC examples and record experiment log"
created: "2026-10-21"
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
        path: "docs/impl/exp/device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/run-device-poc-examples-and-exp-log.md"
  implemented-by: []
  depends-on:
    - "implement-audio-device-cpal-adapter.md"
  blocks:
    - "../../tasks/finalize-backend-adr-and-sync-docs.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-audio-device-port-and-cpal-poc.md"
    - "../../designs/audio-device-boundary-and-poc.md"
    - "../../specs/audio-device-backend-evaluation-poc.md"
    - "../../discovery/evaluate-audio-device-backends.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/audio-device-port-and-cpal-adapter.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "../exp/device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl"
    rejected: []
---

# Run device PoC examples and record experiment log

## Summary

task `TASK-034JglHs8cNMDOvyZFeUpc`（plan `PLAN-034JgkWCimxdvr81k6b0ow` Phase C）として、PoC 実証 example `list_devices` / `play_sine` を実装し、Windows 11 / WASAPI / cpal 0.18.2 の実機で実行して観測値を `docs/impl/exp/device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl` に記録した。「列挙 → 既定構成 → ストリーム開始 → コールバック到達（planar f32 境界変換）」を end-to-end で実証し、496 コールバック・可変フレーム実測（min=88 / max=441）・ストリーミング窓のプロセス全体 alloc 2 イベントという結果を得た。ADR 昇格判定に必要な実機証跡を Phase D へ引き渡す。

## Context

- 実施タスク: [../../tasks/run-device-poc-examples-and-exp-log.md](../../tasks/run-device-poc-examples-and-exp-log.md)
- 上流: [plan](../../plans/implement-audio-device-port-and-cpal-poc.md) Phase C、[design](../../designs/audio-device-boundary-and-poc.md)（`DESIGN-034JcC9QK924kDbu4WhibC`・approved）「PoC 実証フロー」「実機手動検証手順」節、[discovery](../../discovery/evaluate-audio-device-backends.md)（Open Questions の正本）、[test-spec](../../test-specs/audio-device-port-and-cpal-adapter.md)（`TSPEC-034JgliZJcJ6cl2OeZLkuZ`）
- 前提: [implement-audio-device-cpal-adapter.md](implement-audio-device-cpal-adapter.md)（`IMPL-034JymdvDZaaZVzUnlu3RS`）— adapter 公開面 `Host`/`HostId`/`default_host`/`available_host_ids`/`host(id)`
- 環境: Windows 11 x64・rust 1.99.0・WASAPI host・既定出力デバイス「Analogue 1/2 (2- Audient iD4)」

## Implementation

### Changes

- `examples/list_devices.rs`（121 行）— `available_host_ids()` → 各 host の `output_devices()` 列挙・`default_output_device()`・各デバイスの `default_output_format()`/`supported_output_formats()` をダンプする非 RT 診断プログラム。`SupportedBufferSize` の未知 variant は `#[non_exhaustive]` 前提で Debug フォールバック表示
- `examples/play_sine.rs`（454 行）— `default_host()` → `default_output_device()` → `default_output_format()`（`Err`/非 F32 時は `select_f32_format` で supported 空間から F32 `AudioFormat` を合成）→ `OutputStreamConfig` で `open_output_stream` → `start()` → 5 秒の 440 Hz sine → `stop()`。観測はコールバック内 `AtomicUsize`/`AtomicI64` のみ（コールバック回数・min/max フレーム・first/last callback ns・playback presence・xrun・最終エラー kind code）。コールバック内で I/O・確保を行わない。`#[global_allocator]` 計数アロケータ（局所 `#[expect(unsafe_code)]` + SAFETY コメント）で start→stop 窓のプロセス全体 alloc/dealloc イベントを計測（cpal/OS 内部を含む上限値）。30 秒 watchdog + コールバック未到達を `RESULT: FAIL`（exit≠0）として扱う判定構造
- `docs/impl/exp/device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl` — `append_experiment_event.js` 経由で 9 イベント（start/observation×6/decision/validation/summary）を記録。audit 0 findings

### Decisions During Implementation

- **`Fixed(480)` → `UnsupportedConfig` の自動フォールバック** — `open_with_fallback` が `BufferSize::Fixed` 要求を先行し、`UnsupportedConfig` 時に `Default` で再試行する。「固定バッファ拒否」と「ストリームオープン不能」を分離するための判定手順（設計の実機手順どおり）。実機では Fixed(480) が拒否され Default で開通した
- **計数 `#[global_allocator]` は example 内のみ** — 暫定規約の `unsafe` 局所許可（`#[expect(unsafe_code)]` + SAFETY コメント）を example に限定。計測はプロセス全体イベントでありスレッド帰属不能のため「上限値」として記録。adapter 自前コードの非 alloc は `assert_no_alloc` の `rt_output_callback_path_allocates_nothing` が別途保証
- **watchdog と FAIL 判定** — 手動 PoC がセッションをハングさせないよう 30 秒の watchdog（`process::exit(3)`）と「start 成功だがコールバック 0 回 = FAIL（exit 4）」を実装。実機では 496 コールバック到達で `RESULT: OK`
- **`step` は open 前に 1 回導出** — negotiated `AudioFormat.sample_rate` から `step = τ·440/rate` を制御側で計算し、closure へ `move` 所有（`f64` は trivial Drop で backend 任意スレッド drop 安全）

## Related Experiments

### Adopted

- [device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl](../exp/device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl) — 実機観測の一次記録（環境・手順・観測値・判定・依存追加理由・Open Questions 対応表）

### Rejected

なし

## Validation

- `mise exec -- cargo build -p ruviform-audio-device --examples` — exit 0
- `mise exec -- cargo run -p ruviform-audio-device --example list_devices` — exit 0。WASAPI host 1 件・出力デバイス 5 件・既定「Analogue 1/2 (2- Audient iD4)」をダンプ（`tmp/issue-23/list_devices.out.txt` に保存）
- `mise exec -- cargo run -p ruviform-audio-device --example play_sine` — exit 0・`RESULT: OK`。`Fixed(480)`→`UnsupportedConfig`→`Default` 開通、`stream.buffer_size(): Fixed(441)`、496 コールバック・min=88/max=441 フレーム・error 0・playback 全件 presence・窓内 alloc 2/dealloc 2（`tmp/issue-23/play_sine.out.txt` に保存）
- `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` / `mise run test-rt` / `mise run bench` — 全て exit 0（example コードも lint 対象で合格）
- `node .agents/skills/impl-doc/scripts/audit_experiment_log.js --dir docs/impl/exp --json` — findings 0

## Risks

- **初回コールバック timestamp=0** — `stream.now()`・後続コールバックが ~5.046e13 ns（WASAPI デバイス時計スケール）なのに対し初回 callback instant が 0。初回 `StreamInstant` 照会が epoch-0 を返したか adapter の予備経路が初回 callback で epoch anchor した可能性。後続値は単調で整合。follow-up 検証事項として記録
- **WASAPI xrun 未報告** — error event 0 は「未発生」と「未報告」を区別しない（WASAPI は出力 xrun を報告しない）。Linux/ALSA レーンでの再観測が必要
- **計数アロケータは上限値** — 窓内 2 alloc イベントの帰属スレッド（cpal 内部か adapter か）は未同定。assert_no_alloc 側が adapter 自前パスの 0 を保証するため実害なし

## Follow-ups

- Phase D: [../../tasks/finalize-backend-adr-and-sync-docs.md](../../tasks/finalize-backend-adr-and-sync-docs.md) — 本実験ログを証跡として ADR `ADR-034Jc9RslnH7QSQ1nZEEHL` 昇格判定
- 未観測項目（実験ログ summary イベントで引き継ぎ済み）: ALSA vs PipeWire 比較（Linux レーン・`--features pipewire-backend` 任意検証）、Send/Sync の backend 間ばらつき、WASAPI exclusive/ASIO 要否、初回 callback timestamp=0 の原因確認
