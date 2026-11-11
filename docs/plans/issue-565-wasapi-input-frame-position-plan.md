---
id: "PLAN-zmhvig6ptUxYZGsi0NwJ4MGJBX"
type: "plan"
status: "approved"
title: "Issue #565 WASAPI-native input frame_position 実装 plan"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/565"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-565-wasapi-input-frame-position-design.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-565-wasapi-input-frame-position.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
---

# Issue #565 implementation plan

## File map

| file | 変更 |
| --- | --- |
| `crates/ruviform-audio-device/Cargo.toml` | `[target.'cfg(windows)'.dependencies] windows = "0.62"` + features 追加 |
| `crates/ruviform-audio-device/src/lib.rs` | `#[cfg(windows)] mod wasapi_input;` 追加 + doc 更新 |
| `crates/ruviform-audio-device/src/wasapi_input.rs` | **新規** — WASAPI-native input stream（~700 LoC、unsafe 局所化） |
| `crates/ruviform-audio-device/src/device.rs` | `CpalDevice` に `host_name`/`default_input` 追加、`open_input_stream` に native 分岐、`clock_profile` doc 微修正 |
| `crates/ruviform-audio-device/src/host.rs` | `CpalDevice::new` 呼出を新 signature に更新（4 箇所） |
| `crates/ruviform-audio-device/src/stream.rs` | `instant_to_media_time` を `pub(crate)` 化、doc の `frame_position` 記述を WASAPI 限定に修正 |
| `crates/ruviform-audio-device/tests/real_input_e2e.rs` | WASAPI host で `frame_position` が `Some` かつ連続増加することを assert |
| `Cargo.lock` | 依存追加に伴う再生成（追跡継続） |

## Task 分解（単一 task doc に集約 — `docs/tasks/issue-565-t1-...`）

1. **T1-a 依存と骨格**: Cargo.toml `windows` dep、`lib.rs` mod 宣言、`wasapi_input.rs` の COM guard・定数・HRESULT→DeviceError 写像・WAVEFORMATEXTENSIBLE 構築・pure helper（`PositionTracker`/`qpc→instant`/`packet info`）+ 単体 test。
2. **T1-b stream 本体**: endpoint 解決（default async activate / specific immdevice）、`Initialize`〜`GetService` 初期化、worker thread + command channel + wait loop、`process_input` 相当（per-packet `device_position`/`qpc_position`/flags → `InputCallbackInfo{frame_position: Some}` → `read_interleaved` → `ReleaseBuffer`）、discontinuity → `ClockDiscontinuity`。
3. **T1-c `WasapiInputStream` port impl**: `start/stop/format/buffer_size/now/clock_domain/input_latency` + Drop（Terminate→join→monitor drop）+ `unsafe impl Send`。
4. **T1-d 統合**: `device.rs`/`host.rs`/`stream.rs` の配線、`real_input_e2e.rs` 更新。

## 検証マトリクス

| 検証 | lane |
| --- | --- |
| fmt / check / lint / test / build | `mise run fmt-check` `check` `lint` `test` `build` |
| RT non-alloc | `mise run test-rt`（`rt_` prefix test を wasapi_input.rs に追加 — Windows のみ compile、`test-rt` は ubuntu lane なので Windows 限定 test は local 実行を証拠にする） |
| MSRV clippy | `mise run test-msrv` |
| headless real-input lint | `mise run lint-audio-real-input` |
| 実機 | `mise run test-audio-real-input` — 実機・capture 権限があれば実行、なければ NotRun |

## リスクと rollback

- `windows` crate features 不足 → compile error で検出、features 追加で解消。
- `as_inner()` doc-hidden API の cpal 更新リスク → cpal 0.18 pin、PR に記録。
- native path の実機不具合 → `#[cfg(windows)]` 分岐を revert すれば cpal 経路に即復帰（`frame_position` は `None` に戻るだけで他契約は不変）。
