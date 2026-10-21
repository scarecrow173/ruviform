---
id: "IMPL-034JymdvDZaaZVzUnlu3RS"
type: "impl"
status: "completed"
title: "Implement audio-device cpal adapter"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/23"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-device/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio-device/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/host.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/device.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/convert.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/error.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/tests/mock_backend.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/implement-audio-device-cpal-adapter.md"
  implemented-by: []
  depends-on:
    - "implement-audio-core-device-port.md"
  blocks:
    - "../../tasks/run-device-poc-examples-and-exp-log.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-audio-device-port-and-cpal-poc.md"
    - "../../designs/audio-device-boundary-and-poc.md"
    - "../../adr/select-audio-device-backend.md"
    - "../../specs/audio-device-backend-evaluation-poc.md"
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
    adopted: []
    rejected: []
---

# Implement audio-device cpal adapter

## Summary

task `TASK-034JglAvWUFQoLhvYXOw2O`（plan `PLAN-034JgkWCimxdvr81k6b0ow` Phase B）として、設計正本 `DESIGN-034JcC9QK924kDbu4WhibC`「`ruviform-audio-device` クレート構成」「マッピング設計」「コールバック配管」節どおりに cpal 0.18 adapter を実装した。新規クレート `ruviform-audio-device` を workspace に追加し、公開面は `Host`・`HostId`（opaque）・`default_host()`・`available_host_ids()`・`host(id)` の 5 項目のみ — `Device`/`OutputStream` は `Box<dyn>` でポート trait の背後に隠し、cpal 型・エラー・feature 名は一切露出しない。エラー正規化（`map_error(e, ctx)`）・F32 交渉・interleave 書込・`DataCallbackState` コールバック配管（scratch open 時 1 回確保・bounded chunk・`Arc`/`Mutex` 非共有）を実装し、単体 39 本 + 非 cpal モック結合 4 本 + doctest 6 本で検証。`rt_output_callback_path_allocates_nothing` が `assert_no_alloc` でコールバック経路の非 alloc を機械検証し `mise run test-rt` で選択・合格する。

## Context

- 実施タスク: [../../tasks/implement-audio-device-cpal-adapter.md](../../tasks/implement-audio-device-cpal-adapter.md)
- 上流: [plan](../../plans/implement-audio-device-port-and-cpal-poc.md) Phase B、[design](../../designs/audio-device-boundary-and-poc.md)（`DESIGN-034JcC9QK924kDbu4WhibC`・approved）、[ADR](../../adr/select-audio-device-backend.md)（`ADR-034Jc9RslnH7QSQ1nZEEHL` 採用条件: 非露出・adapter 責務・意図命名 feature）、[test-spec](../../test-specs/audio-device-port-and-cpal-adapter.md)（`TSPEC-034JgliZJcJ6cl2OeZLkuZ`）
- 前提: [implement-audio-core-device-port.md](implement-audio-core-device-port.md)（`IMPL-034JxNTyasfMjdgGXnvVPY`）— `device` ポート trait・値型・`AudioBufferViewMut::try_new` を提供
- 環境: Windows 11 x64・rust 1.99.0（workspace `rust-version = "1.85"`・edition 2024）・cpal は WASAPI host が既定

## Implementation

### Changes

- `Cargo.toml`（新規・37 行）— `cpal = "0.18"`（解決結果 `0.18.2`）・`ruviform-audio-core`/`ruviform-media-core` path 依存・features `default = []`/`realtime = ["cpal/realtime"]`/`pipewire-backend = ["cpal/pipewire"]`・`publish.workspace = true`・`[lints] workspace = true`・dev-dep `assert_no_alloc = "1.1"`（解決結果 `1.1.2`。`"^1.2"` は crates.io 未提供のため `"1.1"` に修正）。feature 命名は固有名詞を出さない意図命名の設計どおり。root `Cargo.toml` の `members` に `"crates/ruviform-audio-device"` を追加
- `src/lib.rs`（75 行）— crate doc（責務・許可依存層・RT 契約・cpal 非露出方針転記）+ `#[path = "host.rs"] mod host_impl` + `pub use host_impl::{Host, HostId, available_host_ids, default_host, host}`。`mod host` と `fn host` の名前衝突を `host_impl` リネームで解消。crate 例は `no_run` でハードウェア非依存
- `src/error.rs`（167 行）— `ErrorCtx { Control, ErrorCallback }` + `map_error(e: &cpal::Error, ctx) -> DeviceError` 1 関数集約。設計写像表どおり: `DeviceNotAvailable` は Control→`DeviceNotFound` / err_cb→`StreamInvalidated`、`DeviceChanged`→`Backend("device_changed")` 非致命アドバイザリ、`Xrun` は err_cb のみ `Xrun`（Control 経路は設計表「通常発火しない」のため unclassifiable として `Backend("other")` に倒す）、`HostUnavailable`/`PermissionDenied`/`InvalidInput`/`ResourceExhausted`/`BackendError`/`Other`+wildcard→`Backend("...")` 分類名。`cpal::Error::new(ErrorKind)` が値構成可能と確認できたため全分岐を合成エラーで網羅テスト（6 本）
- `src/convert.rs` — 純粋関数: `map_sample_format`（F32→F32・F64→F64・他 `None`）、`layout_for_channels`（1→Mono・2→Stereo・4→Quad・6→FivePointOne・8→SevenPointOne・他 `None` — 嘘の `Custom` mask を書かない設計どおり）、`NegotiatedConfig`/`negotiate`（F32 優先・非 F32 要求は `UnsupportedConfig`・`BufferSize::Fixed` は全一致候補のいずれかの報告 Range が受容すれば可・`Default` は一致候補の loosest bound を採用・上限未報告は `Unknown`）、`map_supported_range`（untrusted-data 境界の候補単位フィルタ — 非 F32・channels==0・rate bound 逆転/ゼロ上限の報告は候補ごと `None` で除外。buffer bound は leaf `map_supported_buffer_size -> Option` で判定: `min > max` の矛盾報告のみ `None` → 候補ごと drop（`Unknown` へ潰して `negotiate(Fixed)` の無制約受理を得ることを防ぐ）、cpal `Unknown` と PipeWire の未公開量子エンコーディング `Range{0,0}`（cpal 自身が `max_quantum==0` を「server clock metadata 未受信=bound なし」として扱う実証済み意味論）は `Unknown` として通過）。`negotiate` の candidates フィルタも `valid_sample_rate_bounds`+`valid_buffer_bounds` で両軸対称に防御（合成 port 値由来の矛盾 bound も `Default`/`Fixed` 双方で候補外）、`synthesize_default`（cpal 既定 + サポート空間から F32-open 可能形式を合成）、`write_interleaved`（`n = len/channels`・残尾 dst ゼロフィル・`n > frames_capacity` は scratch 再借用で bounded chunk 分割・`copy_to_interleaved` 失敗時 dst 残部ゼロフィル・露出領域呼出前ゼロフィル）。合成 `&mut [f32]` テスト 24 本（map→formats→negotiate(Fixed) 連鎖の境界回帰を含む）
- `src/host.rs`（219 行）— private `cpal::Host` wrap → `DeviceEnumerator` 実装（`output_devices`/`default_output_device`、列挙エラーは `map_error(Control)`）。`HostId` opaque newtype（Clone+Debug+`name() -> &'static str`・cpal `HostId` 非露出）+ `default_host() -> DeviceResult<Host>`/`available_host_ids()`/`host(id) -> DeviceResult<Host>`。レビューで `cpal::default_host()` の init `.expect` panic 経路が指摘されたため、既定ホストは `available_host_ids()[0]`（cpal の優先順位先頭 = 既定と同値）を fallible な `host(id)` で取得する形へ正規化 — backend 初期化失敗が `Backend("host_unavailable")` の回復可能エラーになる。全 public 関数に `no_run`/hardware-independent doctest
- `src/device.rs`（171 行）— private `cpal::Device` wrap → `Device` 実装。`default_output_format` は `synthesize_default` で open 可能 F32 を優先返却（既定が F64/整数の実機でも利用側が即 `UnsupportedConfig` に落ちない設計判断どおり）、`supported_output_formats` は `map_supported_range` 経由で F32-capable かつ sample-rate/buffer 上下限ともに構造妥当（`min <= max`・非零上限）なレンジのみ報告（不正報告は untrusted-data 境界で候補ごと drop）、`open_output_stream` は非 F32 config を `Err(UnsupportedConfig)` で拒否し `negotiate` 通過後 `build_output_stream_raw` で開く
- `src/stream.rs`（288 行）— `CpalOutputStream`（private `cpal::Stream`）→ `OutputStream` 実装（`start`/`stop`/`buffer_size`/`now` は制御経路・`map_error(Control)`）。`DataCallbackState`（`data_callback` + scratch `Vec<f32>` + channels + `frames_capacity` + 遅延 epoch anchor）を data クロージャへ `move` 所有、error callback は独立 `move` — `Arc`/`Mutex` 共有なし。scratch は open 時に `min(effective_max, FRAMES_CAPACITY_HARD_CAP = 16_384) * channels` で 1 回確保。timestamp は `StreamInstant::as_nanos()` u128→i64 主経路 + 飽和時 `saturating_duration_since(epoch)` 予備経路（二重飽和は `None`/origin 崩しで panic しない）。`rt_output_callback_path_allocates_nothing` を含む 5 テスト
- `tests/mock_backend.rs`（387 行）— cpal 非依存のフェイク `DeviceEnumerator`/`Device`/`OutputStream`（スクリプト発火・記録用 `Arc`/`Mutex` はテストスレッド側のみ）で enumerate→default→open→data/err callback 発火→start/stop→`now` を `Box<dyn>` 経由で検証。object safety・`Send`・HRTB 強制変換・adapter `Host` が同一ポート契約を満たすことの 4 テストでポートの差替え可能性を証明

### Decisions During Implementation

- **`host_impl` モジュール名** — `mod host` と公開 `fn host(id)` がモジュール名前空間で衝突するため、内部モジュールを `#[path] mod host_impl` とし `pub use` で公開面を維持
- **制御経路 `Xrun` は `Backend("other")`** — 設計写像表で制御経路 `Xrun` は「通常発火しない」（写像未定義）。test-spec「`Xrun` は err_cb のみ `Xrun`」に従い、万一の制御経路到達は unclassifiable として wildcard バケットに倒す
- **`DeviceChanged` は両経路で `Backend("device_changed")`** — 設計表は制御経路を「発火しない」とし err_cb のみ定義。万一の到達時に誤分類しないよう両経路同一の非致命アドバイザリとした（発火しない経路の写像は無害）
- **`FRAMES_CAPACITY_HARD_CAP = 16_384`** — 設計の「有界 scratch + chunk 吸収」規約に対し、backend 報告 `Range::max` が異常値（数百 MB〜GB 級）でも open 時確保が走り切らないよう設けた具体的な上限値。設計は上限値の具体値を実装時確認に委ねていた
- **epoch anchor は 2 箇所** — ストリーム側は open 時 `stream.now()`、`DataCallbackState` は初回 callback instant を遅延記録。`saturating_duration_since` 予備経路の起点として両方を保持（cpal のクロック観測は callback/`now` 実行時のみ可能）
- **`assert_no_alloc` 統合成功** — `#[global_allocator] AllocDisabler`（`debug_assertions` のみ）+ `assert_no_alloc(|| …)` で view 構成・`copy_to_interleaved`・テスト用 user cb・timestamp 写像を包む。`mise run test-rt` が `rt_` フィルタで同テストを選択し合格 — フォールバック方針（コードレビュー＋静的 grep）への切替えは不要
- **cpal 0.18.2 適合確認** — `cargo update -p cpal` 解決で `0.18.2`、workspace の edition 2024 / `rust-version = "1.85"` 設定下でコンパイル成功（実ツールチェーン 1.99.0）。差し戻し条件（不適合時の停止報告）は非該当

## Related Experiments

### Adopted

なし（実機観測は Phase C `run-device-poc-examples-and-exp-log.md` の責務）

### Rejected

なし

## Validation

- `mise exec -- cargo test -p ruviform-audio-device` — 単体 39 + 結合 4 + doctest 6、全て合格（exit 0）
- `mise exec -- cargo clippy -p ruviform-audio-device --all-targets -- -D warnings` — exit 0
- `mise run check` / `mise run lint` / `mise run test` — workspace 全体 exit 0
- `mise run test-rt` — `rt_output_callback_path_allocates_nothing` が選択・合格（adapter 1 本 + 既存 media-core 4 本 + video-core 3 本）
- `mise exec -- cargo metadata --format-version 1` — cpal は `ruviform-audio-device` の `[dependencies]` にのみ存在。`ruviform-audio-core`・他メンバーに cpal/adapter 依存なし（adapter 逆依存ゼロ）
- `mise exec -- cargo check -p ruviform-audio-core` — exit 0（adapter 無効構成でコア自立）
- `mise exec -- cargo doc -p ruviform-audio-device --no-deps` — 生成物に cpal 型名が公開 signature に現れないことを grep 確認（prose 中の cpal 言及のみ）
- `mise run fmt` + `mise run fmt-check` — exit 0
- `mise exec -- prek run --all-files` — 全 hook 合格

## Risks

- **実機未検証** — 実デバイス列挙・ストリーム実測・`BufferSize::Fixed` 尊重度・xrun 頻度・WASAPI 固有振る舞い・cpal/OS 内部の callback alloc は Phase C 実験ログへ引き継ぎ（CI 不能・test-spec「実機ハードウェア依存の振る舞い」節どおり）
- **`BufferSize::Fixed` の実尊重度** — adapter は `StreamConfig` に固定値を渡すが backend が近傍値に丸める可能性は実機依存
- **chunk 分割時 timestamp 近似** — `n > frames_capacity` の chunk 群は同一 `OutputCallbackInfo` を共用する近似（設計で PoC 明記・厳密な per-chunk 合成は将来検討）

## Follow-ups

- Phase C: [../../tasks/run-device-poc-examples-and-exp-log.md](../../tasks/run-device-poc-examples-and-exp-log.md) — play_sine example・実機実験ログ（`#[global_allocator]` counting で cpal/OS 内部 alloc 観測含む）
- Linux `realtime-dbus`（rtkit）feature は必要時に別途追加（現状 `realtime` は Windows/Android にのみ作用）
- `ChannelLayout::Custom` の任意カウント対応は設計 Deferred Concern どおり `ChannelLayout` variant 追加余地（`#[non_exhaustive]`）として保持
