---
id: "TASK-034Jglv8LtJHUgWH3wlu0o"
type: "task"
status: "done"
title: "ruviform-audio-core の device ポート定義と契約テスト"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/23"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-core/src/device.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-core/src/buffer.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-audio-device-port-and-cpal-poc.md"
  implemented-by:
    - "../impl/ir/implement-audio-core-device-port.md"
  depends-on:
    - "../plans/implement-audio-device-port-and-cpal-poc.md"
  blocks:
    - "implement-audio-device-cpal-adapter.md"
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

# ruviform-audio-core の device ポート定義と契約テスト

## Work

plan `PLAN-034JgkWCimxdvr81k6b0ow` の Phase A。設計正本 `DESIGN-034JcC9QK924kDbu4WhibC`「`ruviform-audio-core` の `device` モジュール（ポート定義）」節のシグネチャどおりに実装する。

- [x] `buffer.rs`: `AudioBufferViewMut<'a, S>::try_new(data: &'a mut [S], channels: usize, frames: usize) -> Result<Self>` を追加する（借用のみ・非確保・RT-suitable。`channels == 0` または `data.len() != channels * frames` で `Err(Error::InvalidValue)`。`new`=panic/`try_new`=`Result` の既存命名規約に整合）。失敗テストを先に書き red 確認してから実装する
- [x] `device.rs`（新規）: 設計どおり `DeviceError`（`#[non_exhaustive]`・`Copy+Clone+Debug+PartialEq+Eq+Hash`・`&'static str` payload のみ・`Display`/`core::error::Error` 実装）と `DeviceResult<T>` を定義する
- [x] `device.rs`: 値型を定義する — `OutputStreamConfig`（`{ format: AudioFormat, buffer_size: BufferSize }`、Copy+Clone+Debug+PartialEq）、`#[non_exhaustive] BufferSize`（`Default`/`Fixed(usize)`、Copy+Default）、`#[non_exhaustive] SupportedBufferSize`（`Unknown`/`Range { min, max }`、Copy）、`SupportedOutputFormat`（Clone・`{ channels, min_sample_rate, max_sample_rate, buffer_size, sample_format }`）、`DeviceId` opaque newtype（`new`/`From<String>`）、`OutputTimestamp`（`{ callback: MediaTime, playback: Option<MediaTime> }`、Copy）、`OutputCallbackInfo`（`{ timestamp: OutputTimestamp }`、Copy）、`DEVICE_CLOCK_TIMEBASE`（ns = `TimeBase::new(1, 1_000_000_000)`。const-eval 可否を実装時に確認し `const`/`fn`/`OnceLock` を決める）
- [x] `device.rs`: コールバック型を定義する — `OutputDataCallback = Box<dyn for<'a> FnMut(AudioBufferViewMut<'a, f32>, OutputCallbackInfo) + Send + 'static>`、`OutputErrorCallback = Box<dyn FnMut(DeviceError) + Send + 'static>`。doc に RT 契約（alloc/dealloc 禁止・ブロッキング/I/O/無制限待機/非有界処理の制限・全フレーム書込義務・panic 禁止・コールバック内からのストリーム制御 API 再入禁止）を明記する
- [x] `device.rs`: 3 trait を定義する — `DeviceEnumerator`（`output_devices()`/`default_output_device() -> DeviceResult<Option<Box<dyn Device>>>`・非 RT・スレッド境界なし）、`Device: Send`（`name`/`id`/`default_output_format`/`supported_output_formats`/`open_output_stream`・非 RT）、`OutputStream: Send`（`start`/`stop` 非 RT・`format()` RT-suitable・`buffer_size()`/`now()` は制御経路 API・Drop で送出停止）。全メソッドに `**RT contract:**` ラベル
- [x] `lib.rs`: `pub mod device;` を追加し crate doc を更新する（crate root へのフラット再エクスポートはしない — `ruviform_audio_core::device::*` 名前空間とする設計判断）。既存モジュールの再エクスポートは変更しない
- [x] `#[cfg(test)]` 契約テスト: `Box<dyn DeviceEnumerator>`/`Box<dyn Device>`/`Box<dyn OutputStream>` の構成可能性、`Box<dyn Device>`/`Box<dyn OutputStream>` の `Send` 適合、`FnMut(AudioBufferViewMut<'_, f32>, OutputCallbackInfo) + Send + 'static` クロージャの `OutputDataCallback` への強制変換（HRTB）、`DeviceError` の `Copy`・`Display`・`Error` 実装、全公開 enum の `#[non_exhaustive]`（clippy `exhaustive_enums` ゲートでも強制）
- [x] doctest はモック実装で組み立てるか `no_run`/`ignore` を指定し、実デバイスに触れない形にする

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-audio-device-port-and-cpal-poc.md](../plans/implement-audio-device-port-and-cpal-poc.md) — `PLAN-034JgkWCimxdvr81k6b0ow`（approved）。Phase A の正本
- [../designs/audio-device-boundary-and-poc.md](../designs/audio-device-boundary-and-poc.md) — `DESIGN-034JcC9QK924kDbu4WhibC`（approved）。シグネチャ・trait 分解理由・`RtContext` 非搭載理由・`DeviceError` 選択理由の実装正本
- [../specs/audio-device-backend-evaluation-poc.md](../specs/audio-device-backend-evaluation-poc.md) — `SPEC-034Jc9QEqYLVfg1CXXTiPV`。受け入れ基準「ポート定義・非露出・`#[non_exhaustive]`」の正本

## Dependencies

- 前提: なし（Phase A は先頭タスク）。`ruviform-audio-core` → `ruviform-media-core` の既存 path 依存で `MediaTime`/`TimeBase`/`Error`/`Result` を利用する
- 後続をブロック: [implement-audio-device-cpal-adapter.md](implement-audio-device-cpal-adapter.md)（`TASK-034JglAvWUFQoLhvYXOw2O`）— ポート trait と `try_new` が adapter 実装の前提
- 外部クレート依存は追加しない。`unsafe` 不使用

## Affected Files

- `crates/ruviform-audio-core/src/device.rs` — 新規
- `crates/ruviform-audio-core/src/buffer.rs` — `try_new` 追加
- `crates/ruviform-audio-core/src/lib.rs` — `pub mod device`・crate doc・`#[cfg(test)]` 契約テスト

## Done When

- [x] 設計シグネチャどおりの全公開型・trait が `ruviform_audio_core::device` 名前空間に存在する
- [x] `try_new` の正常・`channels==0`・長さ不一致のテストが red→green で検証される
- [x] object safety・`Send`・HRTB 強制変換・`DeviceError: Copy` のコンパイルレベル検証が存在する
- [x] 全公開 enum に `#[non_exhaustive]`、全公開 API に `**RT contract:**` ラベルが付く
- [x] 公開面に外部 backend の型・エラー・feature 名が存在しない

## Verification

- [x] `mise exec -- cargo test -p ruviform-audio-core` — 40 unit + 28 doctest 合格（exit 0）
- [x] `mise exec -- cargo clippy -p ruviform-audio-core --all-targets -- -D warnings` — warnings 0（exit 0）
- [x] `mise exec -- cargo doc -p ruviform-audio-core --no-deps` — 成功・warnings 0・生成物に cpal 由来の型・エラー・feature 名なし
- [x] `mise run fmt-check` — 差分なし（exit 0）
